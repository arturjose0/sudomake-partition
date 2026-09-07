//! Leitor HFS+/HFSX (somente leitura), conforme a Technical Note TN1150 da Apple.

use std::cmp::Ordering;
use std::io;
use std::rc::Rc;

use crate::device::BlockDevice;
use crate::fs::{read_all, BytesReader, Entry, FileReader, FileSystem, Kind, Native};
use crate::util::*;

const ROOT_FOLDER: u32 = 2;
const HFS_EPOCH_OFFSET: i64 = 2_082_844_800;
const UF_COMPRESSED: u8 = 0x20;
const FT_HLNK: u32 = 0x686C_6E6B; // 'hlnk'
const FT_FDRP: u32 = 0x6664_7270; // 'fdrp'
const CR_HFSP: u32 = 0x6866_732B; // 'hfs+'
const PRIVATE_DATA_DIR: &str = "\u{0}\u{0}\u{0}\u{0}HFS+ Private Data";
const PRIVATE_DIR_DIR: &str = ".HFS+ Private Directory Data\r";

#[derive(Clone, Debug, Default)]
pub struct ForkData {
    pub size: u64,
    pub total_blocks: u32,
    pub extents: Vec<(u32, u32)>,
}

impl ForkData {
    fn parse(b: &[u8]) -> ForkData {
        let mut extents = Vec::new();
        for i in 0..8 {
            let s = be32(b, 16 + i * 8);
            let c = be32(b, 20 + i * 8);
            if c != 0 {
                extents.push((s, c));
            }
        }
        ForkData { size: be64(b, 0), total_blocks: be32(b, 12), extents }
    }
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct CatRec {
    pub cnid: u32,
    pub parent: u32,
    pub is_dir: bool,
    pub flags: u16,
    pub mode: u16,
    pub admin_flags: u8,
    pub owner_flags: u8,
    pub special: u32,
    pub fd_type: u32,
    pub fd_creator: u32,
    pub crtime: u32,
    pub mtime: u32,
    pub ctime: u32,
    pub data: ForkData,
    pub rsrc: ForkData,
    pub name: String,
}

fn parse_cat_rec(key: &[u8], d: &[u8]) -> Option<CatRec> {
    if key.len() < 6 || d.len() < 88 {
        return None;
    }
    let rt = be16(d, 0);
    let is_dir = match rt {
        1 => true,
        2 => false,
        _ => return None,
    };
    if !is_dir && d.len() < 248 {
        return None;
    }
    let nlen = be16(key, 4) as usize;
    if 6 + nlen * 2 > key.len() {
        return None;
    }
    let name = utf16_to_string(&key[6..6 + nlen * 2], true);
    Some(CatRec {
        cnid: be32(d, 8),
        parent: be32(key, 0),
        is_dir,
        flags: be16(d, 2),
        mode: be16(d, 42),
        admin_flags: d[40],
        owner_flags: d[41],
        special: be32(d, 44),
        fd_type: be32(d, 48),
        fd_creator: be32(d, 52),
        crtime: be32(d, 12),
        mtime: be32(d, 16),
        ctime: be32(d, 20),
        data: if is_dir { ForkData::default() } else { ForkData::parse(&d[88..168]) },
        rsrc: if is_dir { ForkData::default() } else { ForkData::parse(&d[168..248]) },
        name,
    })
}

/// Leitor de um fork (dados ou recursos) a partir da lista de extents.
pub struct ForkReader {
    dev: Rc<dyn BlockDevice>,
    size: u64,
    /// (início lógico em bytes, início físico em bytes, tamanho em bytes)
    runs: Vec<(u64, u64, u64)>,
}

impl FileReader for ForkReader {
    fn size(&self) -> u64 {
        self.size
    }
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> io::Result<usize> {
        if off >= self.size {
            return Ok(0);
        }
        let n = (buf.len() as u64).min(self.size - off) as usize;
        let buf = &mut buf[..n];
        buf.fill(0);
        let end = off + n as u64;
        let start_i = self.runs.partition_point(|r| r.0 + r.2 <= off);
        for r in &self.runs[start_i..] {
            if r.0 >= end {
                break;
            }
            let s = r.0.max(off);
            let t = (r.0 + r.2).min(end);
            if t <= s {
                continue;
            }
            let dst = &mut buf[(s - off) as usize..(t - off) as usize];
            self.dev.read_at(r.1 + (s - r.0), dst)?;
        }
        Ok(n)
    }
}

fn partition_point(n: usize, pred: impl Fn(usize) -> bool) -> usize {
    let (mut lo, mut hi) = (0usize, n);
    while lo < hi {
        let mid = (lo + hi) / 2;
        if pred(mid) {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo
}

struct BTree {
    fork: std::cell::RefCell<ForkReader>,
    node_size: usize,
    root: u32,
    max_key_len: usize,
    variable_index_keys: bool,
    key_compare_type: u8,
}

struct Cursor<'a> {
    tree: &'a BTree,
    node: Vec<u8>,
    idx: usize,
}

impl BTree {
    fn open(fork: ForkReader) -> io::Result<BTree> {
        let mut hdr = vec![0u8; 512];
        let mut f = fork;
        let n = f.read_at(0, &mut hdr)?;
        if n < 120 {
            return Err(err("cabeçalho de árvore B do HFS+ truncado"));
        }
        let node_size = be16(&hdr, 14 + 18) as usize;
        if !(512..=32768).contains(&node_size) || !node_size.is_power_of_two() {
            return Err(err(format!("tamanho de nó inválido na árvore B: {}", node_size)));
        }
        let attributes = be32(&hdr, 14 + 38);
        if attributes & 2 == 0 {
            return Err(err("árvore B sem chaves grandes (HFS clássico não suportado)"));
        }
        Ok(BTree {
            fork: std::cell::RefCell::new(f),
            node_size,
            root: be32(&hdr, 14 + 2),
            max_key_len: be16(&hdr, 14 + 20) as usize,
            variable_index_keys: attributes & 4 != 0,
            key_compare_type: hdr[14 + 37],
        })
    }

    fn node(&self, n: u32) -> io::Result<Vec<u8>> {
        let mut v = vec![0u8; self.node_size];
        let got = self.fork.borrow_mut().read_at(n as u64 * self.node_size as u64, &mut v)?;
        if got < 14 {
            return Err(err(format!("nó {} fora do arquivo da árvore B", n)));
        }
        Ok(v)
    }

    fn record<'b>(&self, node: &'b [u8], i: usize, index_node: bool) -> Option<(&'b [u8], &'b [u8])> {
        let ns = node.len();
        let cnt = be16(node, 10) as usize;
        if i >= cnt || ns < 14 + 2 * (i + 2) {
            return None;
        }
        let off = be16(node, ns - 2 * (i + 1)) as usize;
        let end = be16(node, ns - 2 * (i + 2)) as usize;
        if off + 2 > end || end > ns {
            return None;
        }
        let rec = &node[off..end];
        let klen = be16(rec, 0) as usize;
        let span = if index_node && !self.variable_index_keys { self.max_key_len } else { klen };
        let mut doff = 2 + span;
        if doff & 1 == 1 {
            doff += 1;
        }
        if 2 + klen > rec.len() || doff > rec.len() {
            return None;
        }
        Some((&rec[2..2 + klen], &rec[doff..]))
    }

    /// Posiciona o cursor no primeiro registro cuja chave é ≥ à busca.
    fn seek(&self, cmp: &dyn Fn(&[u8]) -> Ordering) -> io::Result<Cursor<'_>> {
        if self.root == 0 {
            return Ok(Cursor { tree: self, node: vec![0u8; 14], idx: 0 });
        }
        let mut n = self.root;
        for _ in 0..32 {
            let node = self.node(n)?;
            let kind = node[8] as i8;
            let cnt = be16(&node, 10) as usize;
            match kind {
                -1 => {
                    let idx = partition_point(cnt, |i| {
                        self.record(&node, i, false).map(|(k, _)| cmp(k) == Ordering::Less).unwrap_or(true)
                    });
                    return Ok(Cursor { tree: self, node, idx });
                }
                0 => {
                    if cnt == 0 {
                        return Err(err("nó índice vazio"));
                    }
                    let le = partition_point(cnt, |i| {
                        self.record(&node, i, true).map(|(k, _)| cmp(k) != Ordering::Greater).unwrap_or(false)
                    });
                    let child = le.saturating_sub(1);
                    let (_, d) = self.record(&node, child, true).ok_or_else(|| err("registro de índice corrompido"))?;
                    if d.len() < 4 {
                        return Err(err("ponteiro de nó filho inválido"));
                    }
                    n = be32(d, 0);
                }
                _ => return Err(err(format!("tipo de nó inesperado ({}) na árvore B", kind))),
            }
        }
        Err(err("árvore B profunda demais (corrompida?)"))
    }
}

impl<'a> Cursor<'a> {
    fn next(&mut self) -> io::Result<Option<(Vec<u8>, Vec<u8>)>> {
        loop {
            let cnt = be16(&self.node, 10) as usize;
            if self.idx < cnt {
                let (k, d) = self.tree.record(&self.node, self.idx, false).ok_or_else(|| err("registro de folha corrompido"))?;
                let out = (k.to_vec(), d.to_vec());
                self.idx += 1;
                return Ok(Some(out));
            }
            let flink = be32(&self.node, 0);
            if flink == 0 {
                return Ok(None);
            }
            self.node = self.tree.node(flink)?;
            self.idx = 0;
        }
    }
}

pub struct HfsPlus {
    dev: Rc<dyn BlockDevice>,
    bs: u64,
    hfsx: bool,
    case_sensitive: bool,
    label: String,
    catalog: BTree,
    extents: BTree,
    attrs: Option<BTree>,
    priv_data: Option<u32>,
    priv_dir: Option<u32>,
    file_count: u32,
    folder_count: u32,
}

fn cat_cmp(k: &[u8], parent: u32) -> Ordering {
    if k.len() < 6 {
        return Ordering::Less;
    }
    be32(k, 0).cmp(&parent).then_with(|| if be16(k, 4) == 0 { Ordering::Equal } else { Ordering::Greater })
}

impl HfsPlus {
    pub fn open(dev: Rc<dyn BlockDevice>) -> io::Result<HfsPlus> {
        let mut h = vec![0u8; 512];
        dev.read_at(1024, &mut h)?;
        let sig = be16(&h, 0);
        let hfsx = match sig {
            0x482B => false,
            0x4858 => true,
            _ => return Err(err("assinatura HFS+ não encontrada")),
        };
        let bs = be32(&h, 40) as u64;
        if bs < 512 || !bs.is_power_of_two() {
            return Err(err(format!("tamanho de bloco HFS+ inválido: {}", bs)));
        }
        let file_count = be32(&h, 32);
        let folder_count = be32(&h, 36);
        let extents_fd = ForkData::parse(&h[192..272]);
        let catalog_fd = ForkData::parse(&h[272..352]);
        let attrs_fd = ForkData::parse(&h[352..432]);

        let extents = BTree::open(make_reader(&dev, bs, &extents_fd, None, 3, 0)?)?;
        let catalog = BTree::open(make_reader(&dev, bs, &catalog_fd, Some(&extents), 4, 0)?)?;
        let attrs = if attrs_fd.size > 0 && !attrs_fd.extents.is_empty() {
            match make_reader(&dev, bs, &attrs_fd, Some(&extents), 8, 0).and_then(BTree::open) {
                Ok(t) => Some(t),
                Err(e) => {
                    eprintln!("aviso: arquivo de atributos não pôde ser aberto: {}", e);
                    None
                }
            }
        } else {
            None
        };
        let case_sensitive = hfsx && catalog.key_compare_type == 0xBC;

        let mut fs = HfsPlus {
            dev,
            bs,
            hfsx,
            case_sensitive,
            label: String::new(),
            catalog,
            extents,
            attrs,
            priv_data: None,
            priv_dir: None,
            file_count,
            folder_count,
        };
        // Nome do volume = nome da pasta raiz (registro thread do CNID 2)
        let mut cur = fs.catalog.seek(&|k| cat_cmp(k, ROOT_FOLDER))?;
        if let Some((_, d)) = cur.next()? {
            if d.len() >= 10 && be16(&d, 0) == 3 {
                let nlen = be16(&d, 8) as usize;
                if 10 + nlen * 2 <= d.len() {
                    fs.label = utf16_to_string(&d[10..10 + nlen * 2], true);
                }
            }
        }
        drop(cur);
        for r in fs.raw_children(ROOT_FOLDER)? {
            if r.is_dir && r.name == PRIVATE_DATA_DIR {
                fs.priv_data = Some(r.cnid);
            } else if r.is_dir && r.name == PRIVATE_DIR_DIR {
                fs.priv_dir = Some(r.cnid);
            }
        }
        Ok(fs)
    }

    fn reader(&self, cnid: u32, fork_type: u8, fd: &ForkData) -> io::Result<ForkReader> {
        make_reader(&self.dev, self.bs, fd, Some(&self.extents), cnid, fork_type)
    }

    /// Registros de catálogo (arquivos e pastas) filhos de `parent`, sem resolver hard links.
    fn raw_children(&self, parent: u32) -> io::Result<Vec<CatRec>> {
        let mut cur = self.catalog.seek(&|k| cat_cmp(k, parent))?;
        let mut out = Vec::new();
        while let Some((k, d)) = cur.next()? {
            if k.len() < 6 || be32(&k, 0) != parent {
                break;
            }
            if let Some(r) = parse_cat_rec(&k, &d) {
                out.push(r);
            }
        }
        Ok(out)
    }

    fn find_child_exact(&self, parent: u32, name: &str) -> io::Result<Option<CatRec>> {
        Ok(self.raw_children(parent)?.into_iter().find(|r| r.name == name))
    }

    /// Resolve hard links de arquivos e de pastas (usados pelo Time Machine).
    fn resolve(&self, mut r: CatRec) -> CatRec {
        if r.is_dir || r.fd_creator != CR_HFSP {
            return r;
        }
        let name = r.name.clone();
        let target = if r.fd_type == FT_HLNK {
            self.priv_data.and_then(|p| self.find_child_exact(p, &format!("iNode{}", r.special)).ok().flatten())
        } else if r.fd_type == FT_FDRP {
            self.priv_dir.and_then(|p| self.find_child_exact(p, &format!("dir_{}", r.special)).ok().flatten())
        } else {
            None
        };
        if let Some(mut t) = target {
            t.name = name;
            t.parent = r.parent;
            r = t;
        }
        r
    }

    fn xattr(&self, cnid: u32, wanted: &str) -> io::Result<Option<Vec<u8>>> {
        let tree = match &self.attrs {
            Some(t) => t,
            None => return Ok(None),
        };
        let cmp = |k: &[u8]| -> Ordering {
            if k.len() < 12 {
                return Ordering::Less;
            }
            be32(k, 2).cmp(&cnid).then_with(|| if be32(k, 6) == 0 && be16(k, 10) == 0 { Ordering::Equal } else { Ordering::Greater })
        };
        let mut cur = tree.seek(&cmp)?;
        let mut fork: Option<ForkData> = None;
        let mut extra: Vec<(u32, u32)> = Vec::new();
        while let Some((k, d)) = cur.next()? {
            if k.len() < 12 || be32(&k, 2) != cnid {
                break;
            }
            let nlen = be16(&k, 10) as usize;
            if 12 + nlen * 2 > k.len() {
                continue;
            }
            let name = utf16_to_string(&k[12..12 + nlen * 2], true);
            if name != wanted || d.len() < 16 {
                continue;
            }
            match be32(&d, 0) {
                0x10 => {
                    let sz = be32(&d, 12) as usize;
                    let end = (16 + sz).min(d.len());
                    return Ok(Some(d[16..end].to_vec()));
                }
                0x20 if d.len() >= 88 => fork = Some(ForkData::parse(&d[8..88])),
                0x30 if d.len() >= 72 => {
                    for i in 0..8 {
                        let s = be32(&d, 8 + i * 8);
                        let c = be32(&d, 12 + i * 8);
                        if c != 0 {
                            extra.push((s, c));
                        }
                    }
                }
                _ => {}
            }
        }
        drop(cur);
        if let Some(mut fd) = fork {
            fd.extents.extend(extra);
            let mut r = make_reader(&self.dev, self.bs, &fd, None, cnid, 0)?;
            return Ok(Some(read_all(&mut r)?));
        }
        Ok(None)
    }

    fn entry_from_rec(&self, r: CatRec) -> Entry {
        let kind = if r.is_dir {
            Kind::Dir
        } else if r.mode & 0xF000 == 0xA000 {
            Kind::Symlink
        } else {
            Kind::File
        };
        let compressed = kind == Kind::File && r.owner_flags & UF_COMPRESSED != 0;
        let mut size = r.data.size;
        if compressed {
            if let Ok(Some(x)) = self.xattr(r.cnid, crate::apfs::DECMPFS_XATTR) {
                if let Some((_, s)) = crate::decmpfs::header(&x) {
                    size = s;
                }
            }
        }
        Entry {
            name: r.name.clone(),
            kind,
            size,
            mtime: r.mtime as i64 - HFS_EPOCH_OFFSET,
            crtime: r.crtime as i64 - HFS_EPOCH_OFFSET,
            mode: r.mode & 0o7777,
            compressed,
            id: r.cnid as u64,
            native: Native::Hfs(Box::new(r)),
        }
    }
}

fn make_reader(
    dev: &Rc<dyn BlockDevice>,
    bs: u64,
    fd: &ForkData,
    extents_tree: Option<&BTree>,
    cnid: u32,
    fork_type: u8,
) -> io::Result<ForkReader> {
    let mut ext = fd.extents.clone();
    let mut total: u64 = ext.iter().map(|e| e.1 as u64).sum();
    if let Some(tree) = extents_tree {
        let mut guard = 0;
        while total < fd.total_blocks as u64 && guard < 100_000 {
            guard += 1;
            let start = total as u32;
            let cmp = |k: &[u8]| -> Ordering {
                if k.len() < 10 {
                    return Ordering::Less;
                }
                (k[0], be32(k, 2), be32(k, 6)).cmp(&(fork_type, cnid, start))
            };
            let mut cur = tree.seek(&cmp)?;
            let mut added = 0u64;
            if let Some((k, d)) = cur.next()? {
                if k.len() >= 10 && k[0] == fork_type && be32(&k, 2) == cnid && be32(&k, 6) == start && d.len() >= 64 {
                    for i in 0..8 {
                        let s = be32(&d, i * 8);
                        let c = be32(&d, 4 + i * 8);
                        if c != 0 {
                            ext.push((s, c));
                            added += c as u64;
                        }
                    }
                }
            }
            if added == 0 {
                break;
            }
            total += added;
        }
    }
    let mut runs = Vec::with_capacity(ext.len());
    let mut logical = 0u64;
    for (s, c) in ext {
        let len = c as u64 * bs;
        runs.push((logical, s as u64 * bs, len));
        logical += len;
    }
    Ok(ForkReader { dev: dev.clone(), size: fd.size, runs })
}

impl FileSystem for HfsPlus {
    fn fs_type(&self) -> &'static str {
        if self.hfsx {
            "HFSX"
        } else {
            "HFS+"
        }
    }
    fn label(&self) -> String {
        self.label.clone()
    }
    fn case_sensitive(&self) -> bool {
        self.case_sensitive
    }
    fn root(&self) -> io::Result<Entry> {
        let mut e = Entry {
            name: String::new(),
            kind: Kind::Dir,
            size: 0,
            mtime: 0,
            crtime: 0,
            mode: 0o755,
            compressed: false,
            id: ROOT_FOLDER as u64,
            native: Native::None,
        };
        // procura o registro da raiz (filha da pasta CNID 1)
        if let Some(r) = self.raw_children(1)?.into_iter().find(|r| r.cnid == ROOT_FOLDER) {
            e.mtime = r.mtime as i64 - HFS_EPOCH_OFFSET;
            e.crtime = r.crtime as i64 - HFS_EPOCH_OFFSET;
            e.mode = r.mode & 0o7777;
        }
        Ok(e)
    }
    fn read_dir(&self, dir: &Entry) -> io::Result<Vec<Entry>> {
        let parent = dir.id as u32;
        let mut out = Vec::new();
        for r in self.raw_children(parent)? {
            if parent == ROOT_FOLDER
                && (r.name == PRIVATE_DATA_DIR || r.name == PRIVATE_DIR_DIR || r.name == ".journal" || r.name == ".journal_info_block")
            {
                continue;
            }
            let r = self.resolve(r);
            out.push(self.entry_from_rec(r));
        }
        out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        Ok(out)
    }
    fn open(&self, e: &Entry) -> io::Result<Box<dyn FileReader>> {
        let r = match &e.native {
            Native::Hfs(r) => r,
            _ => return Err(err("entrada sem registro de catálogo")),
        };
        if e.compressed {
            if let Some(hdr) = self.xattr(r.cnid, crate::apfs::DECMPFS_XATTR)? {
                let rsrc: Option<Box<dyn FileReader>> = if r.rsrc.size > 0 {
                    Some(Box::new(self.reader(r.cnid, 0xFF, &r.rsrc)?))
                } else {
                    None
                };
                return crate::decmpfs::open(hdr, rsrc);
            }
        }
        if r.data.size == 0 {
            return Ok(Box::new(BytesReader(Vec::new())));
        }
        Ok(Box::new(self.reader(r.cnid, 0, &r.data)?))
    }
    fn read_link(&self, e: &Entry) -> io::Result<String> {
        let r = match &e.native {
            Native::Hfs(r) => r,
            _ => return Err(err("entrada sem registro de catálogo")),
        };
        let mut rd = self.reader(r.cnid, 0, &r.data)?;
        let b = read_all(&mut rd)?;
        Ok(String::from_utf8_lossy(&b).trim_end_matches('\0').to_string())
    }
    fn summary(&self) -> String {
        format!(
            "{} \"{}\" ({} arquivos, {} pastas, bloco de {} bytes{})",
            self.fs_type(),
            self.label,
            self.file_count,
            self.folder_count,
            self.bs,
            if self.case_sensitive { ", com distinção de maiúsculas" } else { "" }
        )
    }
}
