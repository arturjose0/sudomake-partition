//! Serviço de sistema de arquivos numa thread própria: permite que threads de terceiros
//! (driver Dokan, servidores) usem os leitores, que não são thread-safe, por meio de mensagens.
//! Também traduz os nomes para nomes válidos no Windows.

use std::collections::HashMap;
use std::io;
use std::sync::mpsc::{channel, Sender};
use std::sync::Mutex;
use std::thread;

use crate::fs::{names_equal, Entry, FileReader, Kind};
use crate::open::{self, Source};
use crate::util::sanitize_name;

#[derive(Clone, Debug)]
pub struct VolumeInfo {
    pub label: String,
    pub fs_type: String,
    pub case_sensitive: bool,
    pub total_bytes: u64,
}

/// Entrada como aparece no Windows (nome já sanitizado).
#[derive(Clone, Debug)]
pub struct Item {
    pub name: String,
    pub entry: Entry,
}

enum Req {
    Resolve(String, Sender<io::Result<Option<Item>>>),
    List(String, Sender<io::Result<Vec<Item>>>),
    Read(String, u64, usize, Sender<io::Result<Vec<u8>>>),
    Stop,
}

pub struct FsService {
    tx: Mutex<Sender<Req>>,
    pub info: VolumeInfo,
}

fn split(path: &str) -> Vec<String> {
    path.split(|c| c == '/' || c == '\\').filter(|c| !c.is_empty() && *c != ".").map(|s| s.to_string()).collect()
}

struct Worker {
    fs: Box<dyn crate::fs::FileSystem>,
    /// caminho normalizado ("/a/b") -> itens
    dirs: HashMap<String, Vec<Item>>,
    dirs_order: Vec<String>,
    readers: Vec<(String, Box<dyn FileReader>)>,
}

impl Worker {
    fn items_of(&mut self, path: &str, dir: &Entry) -> io::Result<Vec<Item>> {
        if let Some(v) = self.dirs.get(path) {
            return Ok(v.clone());
        }
        let entries = self.fs.read_dir(dir)?;
        let mut used: HashMap<String, usize> = HashMap::new();
        let mut items = Vec::with_capacity(entries.len());
        for e in entries {
            if !matches!(e.kind, Kind::File | Kind::Dir) {
                continue;
            }
            let mut name = sanitize_name(&e.name);
            let key = name.to_lowercase();
            let n = used.entry(key).or_insert(0);
            *n += 1;
            if *n > 1 {
                // dois nomes que só diferem em maiúsculas/minúsculas ou em caracteres proibidos
                name = format!("{} ({})", name, n);
            }
            items.push(Item { name, entry: e });
        }
        if self.dirs.len() >= 256 {
            if let Some(old) = self.dirs_order.first().cloned() {
                self.dirs.remove(&old);
                self.dirs_order.remove(0);
            }
        }
        self.dirs.insert(path.to_string(), items.clone());
        self.dirs_order.push(path.to_string());
        Ok(items)
    }

    fn resolve(&mut self, path: &str) -> io::Result<Option<Item>> {
        let comps = split(path);
        let root = self.fs.root()?;
        let mut cur = Item { name: String::new(), entry: root };
        let mut cur_path = String::new();
        let cs = self.fs.case_sensitive();
        for c in comps {
            if cur.entry.kind != Kind::Dir {
                return Ok(None);
            }
            let items = self.items_of(if cur_path.is_empty() { "/" } else { &cur_path }, &cur.entry)?;
            let found = items.into_iter().find(|it| it.name == c || names_equal(&it.name, &c, cs));
            match found {
                Some(it) => {
                    cur_path.push('/');
                    cur_path.push_str(&it.name);
                    cur = it;
                }
                None => return Ok(None),
            }
        }
        Ok(Some(cur))
    }

    fn list(&mut self, path: &str) -> io::Result<Vec<Item>> {
        let dir = match self.resolve(path)? {
            Some(it) if it.entry.kind == Kind::Dir => it,
            Some(_) => return Err(io::Error::new(io::ErrorKind::InvalidInput, "não é uma pasta")),
            None => return Err(io::Error::new(io::ErrorKind::NotFound, "pasta não encontrada")),
        };
        let comps = split(path);
        let norm = if comps.is_empty() { "/".to_string() } else { format!("/{}", comps.join("/")) };
        self.items_of(&norm, &dir.entry)
    }

    fn read(&mut self, path: &str, off: u64, len: usize) -> io::Result<Vec<u8>> {
        let comps = split(path);
        let norm = format!("/{}", comps.join("/"));
        let pos = self.readers.iter().position(|(p, _)| *p == norm);
        let idx = match pos {
            Some(i) => i,
            None => {
                let it = match self.resolve(path)? {
                    Some(it) if it.entry.kind == Kind::File => it,
                    Some(_) => return Err(io::Error::new(io::ErrorKind::InvalidInput, "não é um arquivo")),
                    None => return Err(io::Error::new(io::ErrorKind::NotFound, "arquivo não encontrado")),
                };
                let r = self.fs.open(&it.entry)?;
                if self.readers.len() >= 8 {
                    self.readers.remove(0);
                }
                self.readers.push((norm, r));
                self.readers.len() - 1
            }
        };
        let mut buf = vec![0u8; len];
        let n = self.readers[idx].1.read_at(off, &mut buf)?;
        buf.truncate(n);
        Ok(buf)
    }
}

impl FsService {
    /// Abre a origem numa thread dedicada. Devolve erro se a origem não puder ser aberta.
    pub fn start(src: Source, total_bytes_hint: u64) -> io::Result<FsService> {
        let (tx, rx) = channel::<Req>();
        let (ready_tx, ready_rx) = channel::<io::Result<VolumeInfo>>();
        thread::Builder::new()
            .name("macread-fs".into())
            .spawn(move || {
                let opened = match open::open(&src, true) {
                    Ok(o) => o,
                    Err(e) => {
                        let _ = ready_tx.send(Err(e));
                        return;
                    }
                };
                let info = VolumeInfo {
                    label: opened.fs.label(),
                    fs_type: opened.fs.fs_type().to_string(),
                    case_sensitive: opened.fs.case_sensitive(),
                    total_bytes: if total_bytes_hint > 0 { total_bytes_hint } else { opened.partition.len },
                };
                let _ = ready_tx.send(Ok(info));
                let mut w = Worker { fs: opened.fs, dirs: HashMap::new(), dirs_order: Vec::new(), readers: Vec::new() };
                while let Ok(req) = rx.recv() {
                    match req {
                        Req::Resolve(p, reply) => {
                            let _ = reply.send(w.resolve(&p));
                        }
                        Req::List(p, reply) => {
                            let _ = reply.send(w.list(&p));
                        }
                        Req::Read(p, off, len, reply) => {
                            let _ = reply.send(w.read(&p, off, len));
                        }
                        Req::Stop => break,
                    }
                }
            })
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("não foi possível criar a thread: {}", e)))?;
        let info = ready_rx
            .recv()
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "a thread do sistema de arquivos terminou inesperadamente"))??;
        Ok(FsService { tx: Mutex::new(tx), info })
    }

    fn send(&self, req: Req) {
        if let Ok(tx) = self.tx.lock() {
            let _ = tx.send(req);
        }
    }

    pub fn resolve(&self, path: &str) -> io::Result<Option<Item>> {
        let (rtx, rrx) = channel();
        self.send(Req::Resolve(path.to_string(), rtx));
        rrx.recv().unwrap_or_else(|_| Err(io::Error::new(io::ErrorKind::Other, "serviço encerrado")))
    }

    pub fn list(&self, path: &str) -> io::Result<Vec<Item>> {
        let (rtx, rrx) = channel();
        self.send(Req::List(path.to_string(), rtx));
        rrx.recv().unwrap_or_else(|_| Err(io::Error::new(io::ErrorKind::Other, "serviço encerrado")))
    }

    pub fn read(&self, path: &str, off: u64, len: usize) -> io::Result<Vec<u8>> {
        let (rtx, rrx) = channel();
        self.send(Req::Read(path.to_string(), off, len, rtx));
        rrx.recv().unwrap_or_else(|_| Err(io::Error::new(io::ErrorKind::Other, "serviço encerrado")))
    }

    pub fn stop(&self) {
        self.send(Req::Stop);
    }
}

impl Drop for FsService {
    fn drop(&mut self) {
        self.stop();
    }
}
