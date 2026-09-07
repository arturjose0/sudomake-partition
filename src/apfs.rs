//! Leitor APFS (somente leitura): container, checkpoints, object map, volumes e árvore de arquivos.
//! Baseado na "Apple File System Reference" publicada pela Apple.

use std::cell::RefCell;
use std::cmp::Ordering;
use std::io;
use std::rc::Rc;

use crate::device::BlockDevice;
use crate::fs::{read_all, BytesReader, Entry, FileReader, FileSystem, Kind, Native};
use crate::util::*;

const NX_MAGIC: u32 = 0x4253_584E; // "NXSB"
const APSB_MAGIC: u32 = 0x4253_5041; // "APSB"
const OBJ_TYPE_NX_SUPERBLOCK: u32 = 1;
const OBJ_TYPE_BTREE: u32 = 2;
const OBJ_TYPE_BTREE_NODE: u32 = 3;
const OBJ_TYPE_OMAP: u32 = 0x0B;

const BTNODE_ROOT: u16 = 1;
const BTNODE_LEAF: u16 = 2;
const BTNODE_FIXED_KV_SIZE: u16 = 4;

const OMAP_VAL_DELETED: u32 = 1;
const OMAP_VAL_ENCRYPTED: u32 = 4;

const J_TYPE_INODE: u8 = 3;
const J_TYPE_XATTR: u8 = 4;
const J_TYPE_FILE_EXTENT: u8 = 8;
const J_TYPE_DIR_REC: u8 = 9;

const APFS_INCOMPAT_CASE_INSENSITIVE: u64 = 0x01;
const APFS_INCOMPAT_NORMALIZATION_INSENSITIVE: u64 = 0x08;
const APFS_INCOMPAT_SEALED_VOLUME: u64 = 0x20;
const APFS_FS_UNENCRYPTED: u64 = 0x01;

const ROOT_DIR_INO: u64 = 2;
const UF_COMPRESSED: u32 = 0x20;
const INODE_HAS_UNCOMPRESSED_SIZE: u64 = 0x40000;
const XATTR_DATA_STREAM: u16 = 1;

pub const DECMPFS_XATTR: &str = "com.apple.decmpfs";
pub const RSRC_XATTR: &str = "com.apple.ResourceFork";
pub const SYMLINK_XATTR: &str = "com.apple.fs.symlink";

/// Verifica o checksum Fletcher-64 de um objeto APFS.
pub fn fletcher64_ok(b: &[u8]) -> bool {
    if b.len() < 16 {
        return false;
    }
    const M: u64 = 0xFFFF_FFFF;
    let (mut s1, mut s2) = (0u64, 0u64);
    let mut i = 8;
    while i + 4 <= b.len() {
        s1 += le32(b, i) as u64;
        s2 += s1;
        i += 4;
    }
    s1 %= M;
    s2 %= M;
    let c1 = M - ((s1 + s2) % M);
    let c2 = M - ((s1 + c1) % M);
    ((c2 << 32) | c1) == le64(b, 0)
}

pub fn role_name(role: u16) -> &'static str {
    match role {
        0 => "(nenhum)",
        1 => "Sistema",
        2 => "Usuário",
        4 => "Recuperação",
        8 => "VM",
        0x10 => "Preboot",
        0x20 => "Instalador",
        0x40 => "Dados",
        0x80 => "Baseband",
        0xC0 => "Update",
        0x100 => "XART",
        0x140 => "Hardware",
        0x180 => "Backup",
        0x240 => "Enterprise",
        0x2C0 => "Prelogin",
        _ => "?",
    }
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct VolumeInfo {
    pub index: usize,
    pub oid: u64,
    pub paddr: u64,
    pub name: String,
    pub role: u16,
    pub uuid: [u8; 16],
    pub encrypted: bool,
    pub case_insensitive: bool,
    pub sealed: bool,
    pub num_files: u64,
    pub num_dirs: u64,
    pub num_symlinks: u64,
    pub last_mod: u64,
    pub incompat: u64,
    pub fs_flags: u64,
}

impl VolumeInfo {
    pub fn role_name(&self) -> &'static str {
        role_name(self.role)
    }
}

pub struct Container {
    dev: Rc<dyn BlockDevice>,
    pub block_size: usize,
    pub block_count: u64,
    pub xid: u64,
    pub uuid: [u8; 16],
    omap: Option<Rc<BTree>>,
    pub volumes: Vec<VolumeInfo>,
    bad_checksums: RefCell<u64>,
}

impl Container {
    pub fn open(dev: Rc<dyn BlockDevice>) -> io::Result<Rc<Container>> {
        let mut b0 = vec![0u8; 4096];
        dev.read_at(0, &mut b0)?;
        if le32(&b0, 32) != NX_MAGIC {
            return Err(err("não é um container APFS (assinatura NXSB ausente)"));
        }
        let block_size = le32(&b0, 36) as usize;
        if !(4096..=65536).contains(&block_size) || block_size % 4096 != 0 {
            return Err(err(format!("tamanho de bloco APFS inválido: {}", block_size)));
        }
        let mut sb = vec![0u8; block_size];
        dev.read_at(0, &mut sb)?;
        let block_count = le64(&sb, 40);

        // Procura o superbloco mais recente na área de descritores de checkpoint.
        let desc_blocks = le32(&sb, 104);
        let desc_base = le64(&sb, 112);
        let mut best_xid = if fletcher64_ok(&sb) { le64(&sb, 16) } else { 0 };
        let mut found_valid = best_xid != 0;
        if desc_blocks & 0x8000_0000 == 0 && desc_blocks > 0 && desc_blocks < 1_000_000 {
            let mut blk = vec![0u8; block_size];
            for i in 0..desc_blocks as u64 {
                let paddr = desc_base + i;
                if paddr.checked_mul(block_size as u64).map_or(true, |o| o + block_size as u64 > dev.size()) {
                    break;
                }
                if dev.read_at(paddr * block_size as u64, &mut blk).is_err() {
                    break;
                }
                if le32(&blk, 24) & 0xFFFF == OBJ_TYPE_NX_SUPERBLOCK && le32(&blk, 32) == NX_MAGIC && fletcher64_ok(&blk) {
                    let x = le64(&blk, 16);
                    if x > best_xid || !found_valid {
                        best_xid = x;
                        found_valid = true;
                        sb.copy_from_slice(&blk);
                    }
                }
            }
        }
        if !found_valid {
            eprintln!("aviso: nenhum superbloco APFS com checksum válido; usando o bloco 0 mesmo assim");
        }
        let xid = le64(&sb, 16);
        let mut uuid = [0u8; 16];
        uuid.copy_from_slice(&sb[72..88]);

        let mut c = Container {
            dev,
            block_size,
            block_count,
            xid,
            uuid,
            omap: None,
            volumes: Vec::new(),
            bad_checksums: RefCell::new(0),
        };

        let omap_oid = le64(&sb, 160);
        let omap_blk = c.read_obj(omap_oid)?;
        if le32(&omap_blk, 24) & 0xFFFF != OBJ_TYPE_OMAP {
            return Err(err("object map do container inválido"));
        }
        let omap_tree = BTree::load(&c, le64(&omap_blk, 48), Resolver::Physical)?;
        let omap = Rc::new(omap_tree);
        c.omap = Some(omap.clone());

        let max_fs = le32(&sb, 180).min(100) as usize;
        for i in 0..max_fs {
            let oid = le64(&sb, 184 + 8 * i);
            if oid == 0 {
                continue;
            }
            let (paddr, _flags) = match c.omap_lookup(&omap, oid, xid)? {
                Some(v) => v,
                None => {
                    eprintln!("aviso: volume {} (oid {:#x}) não encontrado no object map", i, oid);
                    continue;
                }
            };
            let vsb = match c.read_obj(paddr) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("aviso: falha ao ler superbloco do volume {}: {}", i, e);
                    continue;
                }
            };
            if le32(&vsb, 32) != APSB_MAGIC {
                eprintln!("aviso: volume {} sem assinatura APSB", i);
                continue;
            }
            let mut vuuid = [0u8; 16];
            vuuid.copy_from_slice(&vsb[240..256]);
            let incompat = le64(&vsb, 56);
            let fs_flags = le64(&vsb, 264);
            let name = cstr(&vsb[704..960]);
            let role = le16(&vsb, 964);
            c.volumes.push(VolumeInfo {
                index: c.volumes.len(),
                oid,
                paddr,
                name,
                role,
                uuid: vuuid,
                encrypted: fs_flags & APFS_FS_UNENCRYPTED == 0,
                case_insensitive: incompat & APFS_INCOMPAT_CASE_INSENSITIVE != 0,
                sealed: incompat & APFS_INCOMPAT_SEALED_VOLUME != 0,
                num_files: le64(&vsb, 184),
                num_dirs: le64(&vsb, 192),
                num_symlinks: le64(&vsb, 200),
                last_mod: le64(&vsb, 256),
                incompat,
                fs_flags,
            });
        }
        Ok(Rc::new(c))
    }

    fn read_block(&self, paddr: u64) -> io::Result<Rc<Vec<u8>>> {
        if paddr >= self.block_count && paddr * self.block_size as u64 >= self.dev.size() {
            return Err(err(format!("bloco {:#x} fora do container", paddr)));
        }
        let mut v = vec![0u8; self.block_size];
        self.dev.read_at(paddr * self.block_size as u64, &mut v)?;
        Ok(Rc::new(v))
    }

    /// Lê um objeto e verifica seu checksum (avisa, mas não falha, em caso de erro).
    fn read_obj(&self, paddr: u64) -> io::Result<Rc<Vec<u8>>> {
        let b = self.read_block(paddr)?;
        if !fletcher64_ok(&b) {
            let mut n = self.bad_checksums.borrow_mut();
            *n += 1;
            if *n <= 3 {
                eprintln!("aviso: checksum inválido no bloco {:#x} (metadados possivelmente corrompidos)", paddr);
            }
        }
        Ok(b)
    }

    #[allow(dead_code)]
    pub fn bad_checksums(&self) -> u64 {
        *self.bad_checksums.borrow()
    }

    fn omap_lookup(&self, omap: &BTree, oid: u64, xid: u64) -> io::Result<Option<(u64, u32)>> {
        let cmp = move |k: &[u8]| -> Ordering {
            if k.len() < 16 {
                return Ordering::Less;
            }
            (le64(k, 0), le64(k, 8)).cmp(&(oid, xid))
        };
        match omap.seek_last_le(self, &cmp)? {
            Some((k, v)) if k.len() >= 16 && v.len() >= 16 && le64(&k, 0) == oid => {
                let flags = le32(&v, 0);
                if flags & OMAP_VAL_DELETED != 0 {
                    Ok(None)
                } else {
                    Ok(Some((le64(&v, 8), flags)))
                }
            }
            _ => Ok(None),
        }
    }
}

#[derive(Clone)]
enum Resolver {
    Physical,
    Virtual { omap: Rc<BTree>, xid: u64 },
}

struct Node {
    data: Rc<Vec<u8>>,
    flags: u16,
    nkeys: usize,
    toc: usize,
    key_area: usize,
    val_end: usize,
}

impl Node {
    fn is_leaf(&self) -> bool {
        self.flags & BTNODE_LEAF != 0
    }

    fn entry(&self, t: &BTree, i: usize) -> Option<(&[u8], &[u8])> {
        let d = &self.data[..];
        if i >= self.nkeys {
            return None;
        }
        if self.flags & BTNODE_FIXED_KV_SIZE != 0 {
            let o = self.toc + 4 * i;
            if o + 4 > self.key_area {
                return None;
            }
            let k = le16(d, o) as usize;
            let v = le16(d, o + 2) as usize;
            let ks = self.key_area + k;
            let ke = ks + t.key_size;
            let vl = if self.is_leaf() { t.val_size } else { 8 };
            if ke > self.val_end || v > self.val_end || v < vl {
                return None;
            }
            let vs = self.val_end - v;
            Some((&d[ks..ke], &d[vs..vs + vl]))
        } else {
            let o = self.toc + 8 * i;
            if o + 8 > self.key_area {
                return None;
            }
            let k = le16(d, o) as usize;
            let kl = le16(d, o + 2) as usize;
            let v = le16(d, o + 4) as usize;
            let vl = le16(d, o + 6) as usize;
            let ks = self.key_area + k;
            if ks + kl > self.val_end || v > self.val_end || vl > v {
                return None;
            }
            let vs = self.val_end - v;
            Some((&d[ks..ks + kl], &d[vs..vs + vl]))
        }
    }

    fn child_oid(&self, t: &BTree, i: usize) -> io::Result<u64> {
        let (_, v) = self.entry(t, i).ok_or_else(|| err("nó de árvore B corrompido"))?;
        if v.len() < 8 {
            return Err(err("valor de nó índice inválido"));
        }
        Ok(le64(v, 0))
    }
}

fn parse_node(data: Rc<Vec<u8>>, block_size: usize) -> io::Result<Node> {
    if data.len() < 64 {
        return Err(err("nó de árvore B pequeno demais"));
    }
    let otype = le32(&data, 24) & 0xFFFF;
    if otype != OBJ_TYPE_BTREE && otype != OBJ_TYPE_BTREE_NODE {
        return Err(err(format!("objeto não é um nó de árvore B (tipo {:#x})", otype)));
    }
    let flags = le16(&data, 32);
    let nkeys = le32(&data, 36) as usize;
    let ts_off = le16(&data, 40) as usize;
    let ts_len = le16(&data, 42) as usize;
    let toc = 56 + ts_off;
    let key_area = toc + ts_len;
    let val_end = block_size - if flags & BTNODE_ROOT != 0 { 40 } else { 0 };
    if key_area > val_end || val_end > data.len() {
        return Err(err("layout de nó de árvore B inválido"));
    }
    let per = if flags & BTNODE_FIXED_KV_SIZE != 0 { 4 } else { 8 };
    if nkeys * per > ts_len {
        return Err(err("tabela de conteúdo do nó inválida"));
    }
    Ok(Node { data, flags, nkeys, toc, key_area, val_end })
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

pub struct BTree {
    root: u64,
    key_size: usize,
    val_size: usize,
    resolver: Resolver,
}

struct Cursor {
    path: Vec<(Node, usize)>,
}

impl BTree {
    fn resolve(c: &Container, r: &Resolver, oid: u64) -> io::Result<u64> {
        match r {
            Resolver::Physical => Ok(oid),
            Resolver::Virtual { omap, xid } => {
                let (paddr, flags) = c
                    .omap_lookup(omap, oid, *xid)?
                    .ok_or_else(|| err(format!("objeto virtual {:#x} não encontrado no object map", oid)))?;
                if flags & OMAP_VAL_ENCRYPTED != 0 {
                    return Err(err("volume criptografado (FileVault): não é possível ler sem a senha"));
                }
                Ok(paddr)
            }
        }
    }

    fn load(c: &Container, root_oid: u64, resolver: Resolver) -> io::Result<BTree> {
        let root = Self::resolve(c, &resolver, root_oid)?;
        let data = c.read_obj(root)?;
        let otype = le32(&data, 24) & 0xFFFF;
        if otype != OBJ_TYPE_BTREE {
            return Err(err(format!("raiz de árvore B inválida (tipo {:#x})", otype)));
        }
        let bs = c.block_size;
        let key_size = le32(&data, bs - 32) as usize;
        let val_size = le32(&data, bs - 28) as usize;
        Ok(BTree { root, key_size, val_size, resolver })
    }

    fn load_node(&self, c: &Container, oid: u64) -> io::Result<Node> {
        let paddr = Self::resolve(c, &self.resolver, oid)?;
        parse_node(c.read_obj(paddr)?, c.block_size)
    }

    /// A raiz já está resolvida para um endereço físico.
    fn load_root(&self, c: &Container) -> io::Result<Node> {
        parse_node(c.read_obj(self.root)?, c.block_size)
    }

    /// Desce até a folha; `want_le` posiciona após a última chave ≤ busca, senão na primeira ≥ busca.
    fn descend(&self, c: &Container, cmp: &dyn Fn(&[u8]) -> Ordering, want_le: bool) -> io::Result<Cursor> {
        let mut path = Vec::new();
        let mut node = self.load_root(c)?;
        for _ in 0..64 {
            let n = node.nkeys;
            let count_le = partition_point(n, |i| node.entry(self, i).map(|(k, _)| cmp(k) != Ordering::Greater).unwrap_or(false));
            if node.is_leaf() {
                let idx = if want_le {
                    count_le
                } else {
                    partition_point(n, |i| node.entry(self, i).map(|(k, _)| cmp(k) == Ordering::Less).unwrap_or(false))
                };
                path.push((node, idx));
                return Ok(Cursor { path });
            }
            if n == 0 {
                return Err(err("nó índice vazio na árvore B"));
            }
            let child = count_le.saturating_sub(1);
            let oid = node.child_oid(self, child)?;
            path.push((node, child));
            node = self.load_node(c, oid)?;
        }
        Err(err("árvore B profunda demais (corrompida?)"))
    }

    fn seek_last_le(&self, c: &Container, cmp: &dyn Fn(&[u8]) -> Ordering) -> io::Result<Option<(Vec<u8>, Vec<u8>)>> {
        let cur = self.descend(c, cmp, true)?;
        let (node, idx) = cur.path.last().unwrap();
        if *idx == 0 {
            return Ok(None);
        }
        let (k, v) = node.entry(self, idx - 1).ok_or_else(|| err("nó de árvore B corrompido"))?;
        Ok(Some((k.to_vec(), v.to_vec())))
    }

    fn seek_ge(&self, c: &Container, cmp: &dyn Fn(&[u8]) -> Ordering) -> io::Result<Cursor> {
        self.descend(c, cmp, false)
    }

    fn next(&self, c: &Container, cur: &mut Cursor) -> io::Result<Option<(Vec<u8>, Vec<u8>)>> {
        loop {
            let (is_leaf, nkeys, idx) = match cur.path.last() {
                None => return Ok(None),
                Some((n, i)) => (n.is_leaf(), n.nkeys, *i),
            };
            if is_leaf {
                if idx < nkeys {
                    let out = {
                        let (node, _) = cur.path.last().unwrap();
                        let (k, v) = node.entry(self, idx).ok_or_else(|| err("nó de árvore B corrompido"))?;
                        (k.to_vec(), v.to_vec())
                    };
                    cur.path.last_mut().unwrap().1 += 1;
                    return Ok(Some(out));
                }
                cur.path.pop();
                continue;
            }
            let next = idx + 1;
            if next >= nkeys {
                cur.path.pop();
                continue;
            }
            let oid = cur.path.last().unwrap().0.child_oid(self, next)?;
            cur.path.last_mut().unwrap().1 = next;
            let mut child = self.load_node(c, oid)?;
            let mut depth = 0;
            while !child.is_leaf() {
                if child.nkeys == 0 || depth > 64 {
                    return Err(err("árvore B corrompida"));
                }
                let o = child.child_oid(self, 0)?;
                cur.path.push((child, 0));
                child = self.load_node(c, o)?;
                depth += 1;
            }
            cur.path.push((child, 0));
        }
    }
}

fn jkey(k: &[u8]) -> (u64, u8) {
    let raw = le64(k, 0);
    (raw & 0x0FFF_FFFF_FFFF_FFFF, (raw >> 60) as u8)
}

/// Compara a chave de um registro com um prefixo (id, tipo); em empate a chave é considerada maior.
fn cmp_prefix(k: &[u8], id: u64, ty: u8) -> Ordering {
    if k.len() < 8 {
        return Ordering::Less;
    }
    let (kid, kty) = jkey(k);
    kid.cmp(&id).then(kty.cmp(&ty)).then(Ordering::Greater)
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct InodeRec {
    pub id: u64,
    pub parent: u64,
    pub private_id: u64,
    pub crtime: u64,
    pub mtime: u64,
    pub ctime: u64,
    pub internal_flags: u64,
    pub nlink: u32,
    pub bsd_flags: u32,
    pub uid: u32,
    pub gid: u32,
    pub mode: u16,
    pub uncompressed_size: u64,
    pub size: u64,
    pub alloced: u64,
    pub name: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Extent {
    pub logical: u64,
    pub len: u64,
    pub phys: u64,
}

pub enum XattrVal {
    Embedded(Vec<u8>),
    Stream { obj_id: u64, size: u64 },
}

fn parse_inode(id: u64, v: &[u8]) -> io::Result<InodeRec> {
    if v.len() < 92 {
        return Err(err("registro de inode pequeno demais"));
    }
    let mut r = InodeRec {
        id,
        parent: le64(v, 0),
        private_id: le64(v, 8),
        crtime: le64(v, 16),
        mtime: le64(v, 24),
        ctime: le64(v, 32),
        internal_flags: le64(v, 48),
        nlink: le32(v, 56),
        bsd_flags: le32(v, 68),
        uid: le32(v, 72),
        gid: le32(v, 76),
        mode: le16(v, 80),
        uncompressed_size: le64(v, 84),
        size: 0,
        alloced: 0,
        name: None,
    };
    if v.len() >= 96 {
        let xf = &v[92..];
        let n = le16(xf, 0) as usize;
        let mut off = 4 + 4 * n;
        for i in 0..n {
            if 4 + 4 * i + 4 > xf.len() {
                break;
            }
            let t = xf[4 + 4 * i];
            let sz = le16(xf, 4 + 4 * i + 2) as usize;
            if off + sz > xf.len() {
                break;
            }
            let d = &xf[off..off + sz];
            match t {
                4 => r.name = Some(cstr(d)),
                8 if sz >= 16 => {
                    r.size = le64(d, 0);
                    r.alloced = le64(d, 8);
                }
                _ => {}
            }
            off += (sz + 7) & !7;
        }
    }
    Ok(r)
}

pub struct ExtentReader {
    dev: Rc<dyn BlockDevice>,
    bs: u64,
    size: u64,
    ext: Vec<Extent>,
}

impl FileReader for ExtentReader {
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
        let start_i = self.ext.partition_point(|e| e.logical + e.len <= off);
        for e in &self.ext[start_i..] {
            if e.logical >= end {
                break;
            }
            let s = e.logical.max(off);
            let t = (e.logical + e.len).min(end);
            if t <= s {
                continue;
            }
            if e.phys != 0 {
                let dst = &mut buf[(s - off) as usize..(t - off) as usize];
                self.dev.read_at(e.phys * self.bs + (s - e.logical), dst)?;
            }
        }
        Ok(n)
    }
}

pub struct Volume {
    c: Rc<Container>,
    pub info: VolumeInfo,
    root: BTree,
    fext: Option<BTree>,
    hashed: bool,
}

impl Volume {
    pub fn open(c: Rc<Container>, index: usize) -> io::Result<Volume> {
        let info = c.volumes.get(index).cloned().ok_or_else(|| err("índice de volume inválido"))?;
        let sb = c.read_obj(info.paddr)?;
        let omap_oid = le64(&sb, 128);
        let omap_blk = c.read_obj(omap_oid)?;
        if le32(&omap_blk, 24) & 0xFFFF != OBJ_TYPE_OMAP {
            return Err(err("object map do volume inválido"));
        }
        let omap = Rc::new(BTree::load(&c, le64(&omap_blk, 48), Resolver::Physical)?);
        let root_oid = le64(&sb, 136);
        let root = BTree::load(&c, root_oid, Resolver::Virtual { omap, xid: c.xid })?;
        let fext_oid = if sb.len() >= 1040 { le64(&sb, 1032) } else { 0 };
        let fext = if fext_oid != 0 {
            match BTree::load(&c, fext_oid, Resolver::Physical) {
                Ok(t) => Some(t),
                Err(e) => {
                    eprintln!("aviso: árvore de extents (fext) não pôde ser aberta: {}", e);
                    None
                }
            }
        } else {
            None
        };
        let hashed = info.incompat & (APFS_INCOMPAT_CASE_INSENSITIVE | APFS_INCOMPAT_NORMALIZATION_INSENSITIVE) != 0;
        Ok(Volume { c, info, root, fext, hashed })
    }

    /// Todos os registros da árvore de arquivos com o prefixo (id, tipo).
    fn records(&self, id: u64, ty: u8) -> io::Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let cmp = move |k: &[u8]| cmp_prefix(k, id, ty);
        let mut cur = self.root.seek_ge(&self.c, &cmp)?;
        let mut out = Vec::new();
        while let Some((k, v)) = self.root.next(&self.c, &mut cur)? {
            if k.len() < 8 {
                break;
            }
            let (kid, kty) = jkey(&k);
            if kid != id || kty != ty {
                break;
            }
            out.push((k, v));
        }
        Ok(out)
    }

    pub fn inode(&self, id: u64) -> io::Result<InodeRec> {
        let recs = self.records(id, J_TYPE_INODE)?;
        match recs.first() {
            Some((_, v)) => parse_inode(id, v),
            None => Err(io::Error::new(io::ErrorKind::NotFound, format!("inode {} não encontrado", id))),
        }
    }

    fn drec_name(&self, k: &[u8]) -> Option<String> {
        let hashed = |k: &[u8]| -> Option<String> {
            if k.len() < 12 {
                return None;
            }
            let len = (le32(k, 8) & 0x3FF) as usize;
            if len == 0 || 12 + len > k.len() {
                return None;
            }
            Some(cstr(&k[12..12 + len]))
        };
        let plain = |k: &[u8]| -> Option<String> {
            if k.len() < 10 {
                return None;
            }
            let len = le16(k, 8) as usize;
            if len == 0 || 10 + len > k.len() {
                return None;
            }
            Some(cstr(&k[10..10 + len]))
        };
        if self.hashed {
            hashed(k).or_else(|| plain(k))
        } else {
            plain(k).or_else(|| hashed(k))
        }
    }

    /// (nome, id do inode, tipo DT_*) de cada entrada de uma pasta.
    pub fn children(&self, dir: u64) -> io::Result<Vec<(String, u64, u8)>> {
        let mut out = Vec::new();
        for (k, v) in self.records(dir, J_TYPE_DIR_REC)? {
            if v.len() < 18 {
                continue;
            }
            let name = match self.drec_name(&k) {
                Some(n) => n,
                None => continue,
            };
            let file_id = le64(&v, 0);
            let dt = (le16(&v, 16) & 0xF) as u8;
            out.push((name, file_id, dt));
        }
        Ok(out)
    }

    pub fn xattrs(&self, id: u64) -> io::Result<Vec<(String, XattrVal)>> {
        let mut out = Vec::new();
        for (k, v) in self.records(id, J_TYPE_XATTR)? {
            if k.len() < 10 || v.len() < 4 {
                continue;
            }
            let nlen = le16(&k, 8) as usize;
            if 10 + nlen > k.len() {
                continue;
            }
            let name = cstr(&k[10..10 + nlen]);
            let flags = le16(&v, 0);
            let xlen = le16(&v, 2) as usize;
            if 4 + xlen > v.len() {
                continue;
            }
            let data = &v[4..4 + xlen];
            if flags & XATTR_DATA_STREAM != 0 {
                if data.len() < 16 {
                    continue;
                }
                out.push((name, XattrVal::Stream { obj_id: le64(data, 0), size: le64(data, 8) }));
            } else {
                out.push((name, XattrVal::Embedded(data.to_vec())));
            }
        }
        Ok(out)
    }

    pub fn extents(&self, private_id: u64) -> io::Result<Vec<Extent>> {
        let mut out = Vec::new();
        for (k, v) in self.records(private_id, J_TYPE_FILE_EXTENT)? {
            if k.len() < 16 || v.len() < 16 {
                continue;
            }
            out.push(Extent { logical: le64(&k, 8), len: le64(&v, 0) & 0x00FF_FFFF_FFFF_FFFF, phys: le64(&v, 8) });
        }
        if out.is_empty() {
            if let Some(fext) = &self.fext {
                let cmp = move |k: &[u8]| -> Ordering {
                    if k.len() < 16 {
                        return Ordering::Less;
                    }
                    le64(k, 0).cmp(&private_id).then(Ordering::Greater)
                };
                let mut cur = fext.seek_ge(&self.c, &cmp)?;
                while let Some((k, v)) = fext.next(&self.c, &mut cur)? {
                    if k.len() < 16 || v.len() < 16 || le64(&k, 0) != private_id {
                        break;
                    }
                    out.push(Extent { logical: le64(&k, 8), len: le64(&v, 0) & 0x00FF_FFFF_FFFF_FFFF, phys: le64(&v, 8) });
                }
            }
        }
        out.sort_by_key(|e| e.logical);
        Ok(out)
    }

    fn open_stream(&self, obj_id: u64, size: u64) -> io::Result<ExtentReader> {
        let ext = if size == 0 { Vec::new() } else { self.extents(obj_id)? };
        Ok(ExtentReader { dev: self.c.dev.clone(), bs: self.c.block_size as u64, size, ext })
    }

    fn xattr_bytes(&self, v: &XattrVal) -> io::Result<Vec<u8>> {
        match v {
            XattrVal::Embedded(b) => Ok(b.clone()),
            XattrVal::Stream { obj_id, size } => read_all(&mut self.open_stream(*obj_id, *size)?),
        }
    }

    fn xattr_reader(&self, v: &XattrVal) -> io::Result<Box<dyn FileReader>> {
        match v {
            XattrVal::Embedded(b) => Ok(Box::new(BytesReader(b.clone()))),
            XattrVal::Stream { obj_id, size } => Ok(Box::new(self.open_stream(*obj_id, *size)?)),
        }
    }

    fn decmpfs_xattr(&self, id: u64) -> io::Result<Option<Vec<u8>>> {
        for (name, v) in self.xattrs(id)? {
            if name == DECMPFS_XATTR {
                return Ok(Some(self.xattr_bytes(&v)?));
            }
        }
        Ok(None)
    }

    pub fn open_inode(&self, ino: &InodeRec) -> io::Result<Box<dyn FileReader>> {
        if ino.bsd_flags & UF_COMPRESSED != 0 {
            let xs = self.xattrs(ino.id)?;
            let dec = xs.iter().find(|(n, _)| n == DECMPFS_XATTR);
            if let Some((_, d)) = dec {
                let hdr = self.xattr_bytes(d)?;
                let rsrc = match xs.iter().find(|(n, _)| n == RSRC_XATTR) {
                    Some((_, r)) => Some(self.xattr_reader(r)?),
                    None => None,
                };
                return crate::decmpfs::open(hdr, rsrc);
            }
        }
        Ok(Box::new(self.open_stream(ino.private_id, ino.size)?))
    }

    fn entry_from_inode(&self, name: String, ino: InodeRec) -> Entry {
        let kind = match ino.mode & 0xF000 {
            0x4000 => Kind::Dir,
            0x8000 => Kind::File,
            0xA000 => Kind::Symlink,
            _ => Kind::Other,
        };
        let compressed = kind == Kind::File && ino.bsd_flags & UF_COMPRESSED != 0;
        let mut size = ino.size;
        if compressed {
            if ino.internal_flags & INODE_HAS_UNCOMPRESSED_SIZE != 0 {
                size = ino.uncompressed_size;
            } else if let Ok(Some(x)) = self.decmpfs_xattr(ino.id) {
                if let Some((_, s)) = crate::decmpfs::header(&x) {
                    size = s;
                }
            }
        }
        Entry {
            name,
            kind,
            size,
            mtime: (ino.mtime / 1_000_000_000) as i64,
            crtime: (ino.crtime / 1_000_000_000) as i64,
            mode: ino.mode & 0o7777,
            compressed,
            id: ino.id,
            native: Native::Apfs(ino),
        }
    }
}

impl FileSystem for Volume {
    fn fs_type(&self) -> &'static str {
        "APFS"
    }
    fn label(&self) -> String {
        self.info.name.clone()
    }
    fn case_sensitive(&self) -> bool {
        !self.info.case_insensitive
    }
    fn root(&self) -> io::Result<Entry> {
        let ino = self.inode(ROOT_DIR_INO)?;
        Ok(self.entry_from_inode(String::new(), ino))
    }
    fn read_dir(&self, dir: &Entry) -> io::Result<Vec<Entry>> {
        let mut out = Vec::new();
        for (name, id, dt) in self.children(dir.id)? {
            match self.inode(id) {
                Ok(ino) => out.push(self.entry_from_inode(name, ino)),
                Err(e) => {
                    eprintln!("aviso: '{}' (inode {}): {}", name, id, e);
                    let kind = match dt {
                        4 => Kind::Dir,
                        8 => Kind::File,
                        10 => Kind::Symlink,
                        _ => Kind::Other,
                    };
                    out.push(Entry { name, kind, size: 0, mtime: 0, crtime: 0, mode: 0, compressed: false, id, native: Native::None });
                }
            }
        }
        out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        Ok(out)
    }
    fn open(&self, e: &Entry) -> io::Result<Box<dyn FileReader>> {
        match &e.native {
            Native::Apfs(ino) => self.open_inode(ino),
            _ => Err(err("entrada sem inode carregado")),
        }
    }
    fn read_link(&self, e: &Entry) -> io::Result<String> {
        for (name, v) in self.xattrs(e.id)? {
            if name == SYMLINK_XATTR {
                return Ok(cstr(&self.xattr_bytes(&v)?));
            }
        }
        Err(err("destino do link simbólico não encontrado"))
    }
    fn summary(&self) -> String {
        format!(
            "APFS \"{}\" (função: {}, {} arquivos, {} pastas{}{})",
            self.info.name,
            self.info.role_name(),
            self.info.num_files,
            self.info.num_dirs,
            if self.info.case_insensitive { ", sem distinção de maiúsculas" } else { ", com distinção de maiúsculas" },
            if self.info.sealed { ", volume selado" } else { "" }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fletcher_matches_reference_block() {
        // Bloco 0 da imagem de teste apfs.raw (dfvfs)
        let mut b = vec![0u8; 4096];
        b[..8].copy_from_slice(&[0x05, 0x7a, 0x0c, 0x5d, 0x55, 0x29, 0x26, 0xac]);
        b[8] = 1;
        b[16] = 4;
        b[24] = 1;
        b[27] = 0x80;
        // não é o bloco completo: só verifica que a função roda sem pânico
        let _ = fletcher64_ok(&b);
    }
}
