//! Abstração comum de sistema de arquivos (APFS e HFS+).

use std::io;
use std::rc::Rc;
use unicode_normalization::UnicodeNormalization;

use crate::device::BlockDevice;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    File,
    Dir,
    Symlink,
    Other,
}

impl Kind {
    pub fn letter(self) -> char {
        match self {
            Kind::File => '-',
            Kind::Dir => 'd',
            Kind::Symlink => 'l',
            Kind::Other => '?',
        }
    }
}

/// Dados internos específicos de cada sistema de arquivos.
#[derive(Clone, Debug)]
pub enum Native {
    None,
    Apfs(crate::apfs::InodeRec),
    Hfs(Box<crate::hfsplus::CatRec>),
    Ext(crate::ext4::ExtInode),
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub name: String,
    pub kind: Kind,
    pub size: u64,
    /// Modificação (segundos desde 1970, UTC)
    pub mtime: i64,
    pub crtime: i64,
    pub mode: u16,
    pub compressed: bool,
    pub id: u64,
    pub native: Native,
}

/// Leitura posicional de um arquivo.
pub trait FileReader {
    fn size(&self) -> u64;
    /// Lê em `off`; devolve o número de bytes lidos (0 no fim do arquivo).
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> io::Result<usize>;
}

pub struct BytesReader(pub Vec<u8>);

impl FileReader for BytesReader {
    fn size(&self) -> u64 {
        self.0.len() as u64
    }
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> io::Result<usize> {
        if off >= self.0.len() as u64 {
            return Ok(0);
        }
        let start = off as usize;
        let n = buf.len().min(self.0.len() - start);
        buf[..n].copy_from_slice(&self.0[start..start + n]);
        Ok(n)
    }
}

/// Leitor genérico a partir de trechos (lógico, físico, tamanho) em bytes; buracos leem como zeros.
pub struct RunReader {
    pub dev: Rc<dyn BlockDevice>,
    pub size: u64,
    pub runs: Vec<(u64, u64, u64)>,
}

impl FileReader for RunReader {
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

pub fn read_all(r: &mut dyn FileReader) -> io::Result<Vec<u8>> {
    let size = r.size();
    if size > 512 * 1024 * 1024 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "arquivo grande demais para carregar na memória"));
    }
    let mut out = vec![0u8; size as usize];
    let mut off = 0usize;
    while off < out.len() {
        let n = r.read_at(off as u64, &mut out[off..])?;
        if n == 0 {
            break;
        }
        off += n;
    }
    out.truncate(off);
    Ok(out)
}

#[allow(dead_code)]
pub trait FileSystem {
    fn fs_type(&self) -> &'static str;
    fn label(&self) -> String;
    fn case_sensitive(&self) -> bool;
    fn root(&self) -> io::Result<Entry>;
    fn read_dir(&self, dir: &Entry) -> io::Result<Vec<Entry>>;
    fn open(&self, e: &Entry) -> io::Result<Box<dyn FileReader>>;
    fn read_link(&self, e: &Entry) -> io::Result<String>;
    fn summary(&self) -> String;
    /// (total, livre) em bytes, quando o sistema de arquivos informa.
    fn capacity(&self) -> Option<(u64, u64)> {
        None
    }
}

pub fn names_equal(a: &str, b: &str, case_sensitive: bool) -> bool {
    if a == b {
        return true;
    }
    let na: String = a.nfd().collect();
    let nb: String = b.nfd().collect();
    if case_sensitive {
        na == nb
    } else {
        na.to_lowercase() == nb.to_lowercase()
    }
}

/// Resolve um caminho estilo "/Users/nome/Documents" a partir da raiz.
pub fn lookup(fs: &dyn FileSystem, path: &str) -> io::Result<Entry> {
    let mut cur = fs.root()?;
    for comp in path.split(|c| c == '/' || c == '\\').filter(|c| !c.is_empty() && *c != ".") {
        if cur.kind != Kind::Dir {
            return Err(io::Error::new(io::ErrorKind::NotFound, format!("'{}' não é uma pasta", cur.name)));
        }
        let children = fs.read_dir(&cur)?;
        let found = children.into_iter().find(|e| names_equal(&e.name, comp, fs.case_sensitive()));
        match found {
            Some(e) => cur = e,
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("'{}' não encontrado em '/{}'", comp, cur.name),
                ))
            }
        }
    }
    Ok(cur)
}
