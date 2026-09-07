//! Leitor ext2/ext3/ext4 (somente leitura): superbloco, descritores de grupo, inodes,
//! árvores de extents, blocos indiretos, dados inline, pastas e links simbólicos.

use std::cell::RefCell;
use std::collections::HashMap;
use std::io;
use std::rc::Rc;

use crate::device::BlockDevice;
use crate::fs::{read_all, BytesReader, Entry, FileReader, FileSystem, Kind, Native, RunReader};
use crate::util::*;

const MAGIC: u16 = 0xEF53;
const COMPAT_HAS_JOURNAL: u32 = 0x4;
const INCOMPAT_FILETYPE: u32 = 0x2;
const INCOMPAT_RECOVER: u32 = 0x4;
const INCOMPAT_META_BG: u32 = 0x10;
const INCOMPAT_EXTENTS: u32 = 0x40;
const INCOMPAT_64BIT: u32 = 0x80;
const INCOMPAT_FLEX_BG: u32 = 0x200;
const INCOMPAT_INLINE_DATA: u32 = 0x8000;
const INCOMPAT_ENCRYPT: u32 = 0x10000;
const INCOMPAT_CASEFOLD: u32 = 0x20000;
const RO_COMPAT_SPARSE_SUPER: u32 = 0x1;
const RO_COMPAT_EXT4_ANY: u32 = 0x8 | 0x10 | 0x20 | 0x40 | 0x400; // huge_file, gdt_csum, dir_nlink, extra_isize, metadata_csum

const FL_ENCRYPT: u32 = 0x800;
const FL_EXTENTS: u32 = 0x8_0000;
const FL_INLINE: u32 = 0x1000_0000;
const EXTENT_MAGIC: u16 = 0xF30A;
const ROOT_INO: u32 = 2;

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct ExtInode {
    pub ino: u32,
    pub raw: Vec<u8>,
}

#[allow(dead_code)]
impl ExtInode {
    pub fn mode(&self) -> u16 {
        le16(&self.raw, 0)
    }
    pub fn flags(&self) -> u32 {
        le32(&self.raw, 32)
    }
    pub fn size(&self) -> u64 {
        le32(&self.raw, 4) as u64 | ((le32(&self.raw, 108) as u64) << 32)
    }
    fn extra_isize(&self) -> usize {
        if self.raw.len() > 130 {
            le16(&self.raw, 128) as usize
        } else {
            0
        }
    }
    fn time(&self, off: usize, extra_off: usize) -> i64 {
        let mut t = le32(&self.raw, off) as i64;
        if self.raw.len() >= extra_off + 4 && 128 + self.extra_isize() >= extra_off + 4 {
            t += ((le32(&self.raw, extra_off) & 3) as i64) << 32;
        }
        t
    }
    pub fn mtime(&self) -> i64 {
        self.time(16, 136)
    }
    pub fn crtime(&self) -> i64 {
        if self.raw.len() >= 148 && 128 + self.extra_isize() >= 148 {
            self.time(144, 148)
        } else {
            0
        }
    }
    pub fn links(&self) -> u16 {
        le16(&self.raw, 26)
    }
    pub fn uid(&self) -> u32 {
        le16(&self.raw, 2) as u32 | ((le16(&self.raw, 120) as u32) << 16)
    }
}

pub struct Ext4 {
    dev: Rc<dyn BlockDevice>,
    bs: u64,
    inodes_count: u32,
    blocks_count: u64,
    first_data_block: u64,
    blocks_per_group: u32,
    inodes_per_group: u32,
    inode_size: usize,
    desc_size: usize,
    compat: u32,
    incompat: u32,
    ro_compat: u32,
    first_meta_bg: u32,
    label: String,
    pub uuid: [u8; 16],
    gd_cache: RefCell<HashMap<u32, Vec<u8>>>,
}

impl Ext4 {
    pub fn open(dev: Rc<dyn BlockDevice>) -> io::Result<Ext4> {
        let mut sb = vec![0u8; 1024];
        dev.read_at(1024, &mut sb)?;
        if le16(&sb, 56) != MAGIC {
            return Err(err("assinatura ext2/3/4 não encontrada"));
        }
        let log_bs = le32(&sb, 24);
        if log_bs > 6 {
            return Err(err("tamanho de bloco ext inválido"));
        }
        let bs = 1024u64 << log_bs;
        let rev = le32(&sb, 76);
        let inode_size = if rev >= 1 { le16(&sb, 88) as usize } else { 128 };
        if inode_size < 128 || inode_size as u64 > bs || !inode_size.is_power_of_two() {
            return Err(err(format!("tamanho de inode inválido: {}", inode_size)));
        }
        let incompat = if rev >= 1 { le32(&sb, 96) } else { 0 };
        let ro_compat = if rev >= 1 { le32(&sb, 100) } else { 0 };
        let compat = if rev >= 1 { le32(&sb, 92) } else { 0 };
        let is64 = incompat & INCOMPAT_64BIT != 0;
        let mut desc_size = if is64 { le16(&sb, 0xFE) as usize } else { 32 };
        if desc_size < 32 || desc_size > 1024 {
            desc_size = 32;
        }
        let blocks_count = le32(&sb, 4) as u64 | if is64 { (le32(&sb, 0x150) as u64) << 32 } else { 0 };
        let blocks_per_group = le32(&sb, 32);
        let inodes_per_group = le32(&sb, 40);
        if blocks_per_group == 0 || inodes_per_group == 0 {
            return Err(err("superbloco ext inválido"));
        }
        let mut uuid = [0u8; 16];
        uuid.copy_from_slice(&sb[104..120]);
        let fs = Ext4 {
            dev,
            bs,
            inodes_count: le32(&sb, 0),
            blocks_count,
            first_data_block: le32(&sb, 20) as u64,
            blocks_per_group,
            inodes_per_group,
            inode_size,
            desc_size,
            compat,
            incompat,
            ro_compat,
            first_meta_bg: le32(&sb, 0x104),
            label: cstr(&sb[120..136]),
            uuid,
            gd_cache: RefCell::new(HashMap::new()),
        };
        if incompat & INCOMPAT_ENCRYPT != 0 {
            eprintln!("aviso: o volume usa criptografia de pastas (fscrypt); arquivos criptografados não poderão ser lidos");
        }
        if incompat & INCOMPAT_RECOVER != 0 {
            eprintln!("aviso: o volume não foi desmontado corretamente (journal pendente); os últimos arquivos gravados podem faltar");
        }
        Ok(fs)
    }

    pub fn version(&self) -> &'static str {
        if self.incompat & (INCOMPAT_EXTENTS | INCOMPAT_64BIT | INCOMPAT_FLEX_BG | INCOMPAT_META_BG) != 0
            || self.ro_compat & RO_COMPAT_EXT4_ANY != 0
        {
            "ext4"
        } else if self.compat & COMPAT_HAS_JOURNAL != 0 {
            "ext3"
        } else {
            "ext2"
        }
    }

    fn groups(&self) -> u32 {
        ((self.blocks_count - self.first_data_block + self.blocks_per_group as u64 - 1) / self.blocks_per_group as u64) as u32
    }

    fn has_super(&self, g: u32) -> bool {
        if self.ro_compat & RO_COMPAT_SPARSE_SUPER == 0 || g <= 1 {
            return true;
        }
        fn is_pow(mut n: u32, b: u32) -> bool {
            while n % b == 0 {
                n /= b;
            }
            n == 1
        }
        is_pow(g, 3) || is_pow(g, 5) || is_pow(g, 7)
    }

    fn group_desc(&self, g: u32) -> io::Result<Vec<u8>> {
        if let Some(d) = self.gd_cache.borrow().get(&g) {
            return Ok(d.clone());
        }
        let dpb = (self.bs as usize / self.desc_size) as u32;
        let meta_bg = self.incompat & INCOMPAT_META_BG != 0 && g >= self.first_meta_bg.saturating_mul(dpb);
        let (block, off) = if !meta_bg {
            (self.first_data_block + 1 + (g / dpb) as u64, (g % dpb) as usize * self.desc_size)
        } else {
            let mg_first = (g / dpb) * dpb;
            let mut b = self.first_data_block + mg_first as u64 * self.blocks_per_group as u64;
            if self.has_super(mg_first) {
                b += 1;
            }
            (b, (g % dpb) as usize * self.desc_size)
        };
        let mut d = vec![0u8; self.desc_size];
        self.dev.read_at(block * self.bs + off as u64, &mut d)?;
        self.gd_cache.borrow_mut().insert(g, d.clone());
        Ok(d)
    }

    fn inode_table(&self, g: u32) -> io::Result<u64> {
        let d = self.group_desc(g)?;
        let lo = le32(&d, 8) as u64;
        let hi = if self.incompat & INCOMPAT_64BIT != 0 && self.desc_size >= 64 { le32(&d, 0x28) as u64 } else { 0 };
        Ok(lo | (hi << 32))
    }

    pub fn read_inode(&self, ino: u32) -> io::Result<ExtInode> {
        if ino == 0 || ino > self.inodes_count {
            return Err(err(format!("número de inode inválido: {}", ino)));
        }
        let g = (ino - 1) / self.inodes_per_group;
        if g >= self.groups() {
            return Err(err(format!("inode {} fora dos grupos do volume", ino)));
        }
        let idx = ((ino - 1) % self.inodes_per_group) as u64;
        let table = self.inode_table(g)?;
        if table == 0 || table >= self.blocks_count {
            return Err(err(format!("tabela de inodes do grupo {} inválida", g)));
        }
        let mut raw = vec![0u8; self.inode_size];
        self.dev.read_at(table * self.bs + idx * self.inode_size as u64, &mut raw)?;
        Ok(ExtInode { ino, raw })
    }

    fn read_block(&self, b: u64) -> io::Result<Vec<u8>> {
        if b == 0 || b >= self.blocks_count {
            return Err(err(format!("bloco {} fora do volume", b)));
        }
        let mut v = vec![0u8; self.bs as usize];
        self.dev.read_at(b * self.bs, &mut v)?;
        Ok(v)
    }

    fn extent_node(&self, node: &[u8], out: &mut Vec<(u64, u64, u64)>, depth_guard: u32) -> io::Result<()> {
        if node.len() < 12 || le16(node, 0) != EXTENT_MAGIC {
            return Err(err("cabeçalho de extents inválido"));
        }
        if depth_guard > 8 {
            return Err(err("árvore de extents profunda demais"));
        }
        let entries = le16(node, 2) as usize;
        let depth = le16(node, 6);
        let max = (node.len() - 12) / 12;
        if entries > max {
            return Err(err("árvore de extents corrompida"));
        }
        for i in 0..entries {
            let e = &node[12 + 12 * i..24 + 12 * i];
            if depth == 0 {
                let lblk = le32(e, 0) as u64;
                let len = le16(e, 4) as u64;
                if len > 32768 {
                    continue; // extent não inicializado: lê como zeros
                }
                let phys = le32(e, 8) as u64 | ((le16(e, 6) as u64) << 32);
                if len > 0 && phys != 0 {
                    out.push((lblk * self.bs, phys * self.bs, len * self.bs));
                }
            } else {
                let leaf = le32(e, 4) as u64 | ((le16(e, 8) as u64) << 32);
                let blk = self.read_block(leaf)?;
                self.extent_node(&blk, out, depth_guard + 1)?;
            }
        }
        Ok(())
    }

    fn indirect(&self, b: u64, level: u32, next_logical: &mut u64, out: &mut Vec<(u64, u64, u64)>) -> io::Result<()> {
        let per = self.bs / 4;
        if b == 0 {
            *next_logical += per.pow(level);
            return Ok(());
        }
        let blk = self.read_block(b)?;
        for i in 0..per as usize {
            let p = le32(&blk, i * 4) as u64;
            if level == 1 {
                if p != 0 {
                    out.push((*next_logical * self.bs, p * self.bs, self.bs));
                }
                *next_logical += 1;
            } else {
                self.indirect(p, level - 1, next_logical, out)?;
            }
        }
        Ok(())
    }

    /// Mapeamento (offset lógico, offset físico, tamanho) em bytes.
    fn runs(&self, ino: &ExtInode) -> io::Result<Vec<(u64, u64, u64)>> {
        let mut out = Vec::new();
        if ino.flags() & FL_EXTENTS != 0 {
            self.extent_node(&ino.raw[40..100], &mut out, 0)?;
        } else {
            let mut logical = 0u64;
            for i in 0..12 {
                let p = le32(&ino.raw, 40 + 4 * i) as u64;
                if p != 0 {
                    out.push((logical * self.bs, p * self.bs, self.bs));
                }
                logical += 1;
            }
            // um ponteiro zerado é um buraco (arquivo esparso), não o fim do mapeamento
            for (slot, level) in [(12usize, 1u32), (13, 2), (14, 3)] {
                let p = le32(&ino.raw, 40 + 4 * slot) as u64;
                self.indirect(p, level, &mut logical, &mut out)?;
            }
        }
        out.sort_by_key(|r| r.0);
        // junta trechos contíguos
        let mut merged: Vec<(u64, u64, u64)> = Vec::with_capacity(out.len());
        for r in out {
            if let Some(last) = merged.last_mut() {
                if last.0 + last.2 == r.0 && last.1 + last.2 == r.1 {
                    last.2 += r.2;
                    continue;
                }
            }
            merged.push(r);
        }
        Ok(merged)
    }

    /// Conteúdo de um arquivo/pasta com dados inline (i_block + xattr system.data).
    fn inline_data(&self, ino: &ExtInode) -> Vec<u8> {
        let size = ino.size() as usize;
        let mut d = ino.raw[40..100].to_vec();
        let start = 128 + ino.extra_isize();
        if ino.raw.len() >= start + 4 && le32(&ino.raw, start) == 0xEA02_0000 {
            let base = start + 4;
            let mut p = base;
            while p + 16 <= ino.raw.len() {
                let nlen = ino.raw[p] as usize;
                let idx = ino.raw[p + 1];
                let voff = le16(&ino.raw, p + 2) as usize;
                let vsize = le32(&ino.raw, p + 8) as usize;
                if nlen == 0 {
                    break;
                }
                let name_end = p + 16 + nlen;
                if name_end > ino.raw.len() {
                    break;
                }
                if idx == 7 && &ino.raw[p + 16..name_end] == b"data" {
                    let vs = base + voff;
                    if vs + vsize <= ino.raw.len() {
                        d.extend_from_slice(&ino.raw[vs..vs + vsize]);
                    }
                    break;
                }
                p = (name_end + 3) & !3;
            }
        }
        d.truncate(size);
        d
    }

    fn open_inode(&self, ino: &ExtInode) -> io::Result<Box<dyn FileReader>> {
        if ino.flags() & FL_ENCRYPT != 0 {
            return Err(err("arquivo criptografado (fscrypt): não é possível ler sem a chave"));
        }
        if ino.flags() & FL_INLINE != 0 {
            return Ok(Box::new(BytesReader(self.inline_data(ino))));
        }
        let size = ino.size();
        let runs = if size == 0 { Vec::new() } else { self.runs(ino)? };
        Ok(Box::new(RunReader { dev: self.dev.clone(), size, runs }))
    }

    /// (nome, inode, tipo) das entradas de uma pasta.
    fn dir_entries(&self, ino: &ExtInode) -> io::Result<Vec<(String, u32, u8)>> {
        let (data, mut off) = if ino.flags() & FL_INLINE != 0 {
            (self.inline_data(ino), 4usize) // os 4 primeiros bytes são o inode pai
        } else {
            (read_all(self.open_inode(ino)?.as_mut())?, 0usize)
        };
        let filetype = self.incompat & INCOMPAT_FILETYPE != 0;
        let mut out = Vec::new();
        while off + 8 <= data.len() {
            let inode = le32(&data, off);
            let rec_len = le16(&data, off + 4) as usize;
            let (name_len, ftype) = if filetype { (data[off + 6] as usize, data[off + 7]) } else { (le16(&data, off + 6) as usize, 0) };
            if rec_len < 8 || rec_len % 4 != 0 {
                break;
            }
            if inode != 0 && name_len > 0 && off + 8 + name_len <= data.len() {
                let name = String::from_utf8_lossy(&data[off + 8..off + 8 + name_len]).into_owned();
                if name != "." && name != ".." {
                    out.push((name, inode, ftype));
                }
            }
            off += rec_len;
        }
        Ok(out)
    }

    fn entry_from_inode(&self, name: String, ino: ExtInode) -> Entry {
        let kind = match ino.mode() & 0xF000 {
            0x4000 => Kind::Dir,
            0x8000 => Kind::File,
            0xA000 => Kind::Symlink,
            _ => Kind::Other,
        };
        Entry {
            name,
            kind,
            size: if kind == Kind::Dir { 0 } else { ino.size() },
            mtime: ino.mtime(),
            crtime: ino.crtime(),
            mode: ino.mode() & 0o7777,
            compressed: false,
            id: ino.ino as u64,
            native: Native::Ext(ino),
        }
    }
}

impl FileSystem for Ext4 {
    fn fs_type(&self) -> &'static str {
        self.version()
    }
    fn label(&self) -> String {
        self.label.clone()
    }
    fn case_sensitive(&self) -> bool {
        self.incompat & INCOMPAT_CASEFOLD == 0
    }
    fn root(&self) -> io::Result<Entry> {
        let ino = self.read_inode(ROOT_INO)?;
        Ok(self.entry_from_inode(String::new(), ino))
    }
    fn read_dir(&self, dir: &Entry) -> io::Result<Vec<Entry>> {
        let ino = match &dir.native {
            Native::Ext(i) => i.clone(),
            _ => self.read_inode(dir.id as u32)?,
        };
        let mut out = Vec::new();
        for (name, num, ftype) in self.dir_entries(&ino)? {
            match self.read_inode(num) {
                Ok(i) => out.push(self.entry_from_inode(name, i)),
                Err(e) => {
                    eprintln!("aviso: '{}' (inode {}): {}", name, num, e);
                    let kind = match ftype {
                        1 => Kind::File,
                        2 => Kind::Dir,
                        7 => Kind::Symlink,
                        _ => Kind::Other,
                    };
                    out.push(Entry { name, kind, size: 0, mtime: 0, crtime: 0, mode: 0, compressed: false, id: num as u64, native: Native::None });
                }
            }
        }
        out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        Ok(out)
    }
    fn open(&self, e: &Entry) -> io::Result<Box<dyn FileReader>> {
        match &e.native {
            Native::Ext(i) => self.open_inode(i),
            _ => Err(err("entrada sem inode carregado")),
        }
    }
    fn read_link(&self, e: &Entry) -> io::Result<String> {
        let ino = match &e.native {
            Native::Ext(i) => i,
            _ => return Err(err("entrada sem inode carregado")),
        };
        if ino.flags() & FL_ENCRYPT != 0 {
            return Err(err("link criptografado"));
        }
        let size = ino.size() as usize;
        let data = if ino.flags() & FL_INLINE != 0 {
            self.inline_data(ino)
        } else if size < 60 && ino.flags() & FL_EXTENTS == 0 {
            ino.raw[40..40 + size].to_vec()
        } else {
            read_all(self.open_inode(ino)?.as_mut())?
        };
        Ok(String::from_utf8_lossy(&data).trim_end_matches('\0').to_string())
    }
    fn summary(&self) -> String {
        let mut feats = Vec::new();
        if self.incompat & INCOMPAT_64BIT != 0 {
            feats.push("64bit");
        }
        if self.incompat & INCOMPAT_INLINE_DATA != 0 {
            feats.push("inline_data");
        }
        if self.incompat & INCOMPAT_ENCRYPT != 0 {
            feats.push("encrypt");
        }
        if self.incompat & INCOMPAT_CASEFOLD != 0 {
            feats.push("casefold");
        }
        format!(
            "{} \"{}\" ({} inodes, {} blocos de {} bytes = {}{}{})",
            self.version(),
            self.label,
            self.inodes_count,
            self.blocks_count,
            self.bs,
            fmt_size(self.blocks_count * self.bs),
            if feats.is_empty() { String::new() } else { format!(", {}", feats.join(" ")) },
            if self.incompat & INCOMPAT_RECOVER != 0 { ", journal pendente" } else { "" }
        )
    }
}
