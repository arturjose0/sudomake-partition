//! macread-gui — interface gráfica para navegar em discos Mac (APFS/HFS+), Linux (ext2/3/4, LVM)
//! e imagens de disco/DMG no Windows, e copiar arquivos.
#![windows_subsystem = "windows"]

use std::cell::RefCell;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use native_windows_gui as nwg;

use macread::apfs;
use macread::copy;
use macread::device;
use macread::dokan;
use macread::fs::{self, Entry, FileSystem, Kind};
use macread::fsservice::FsService;
use macread::open::{self, Source};
use macread::partition::{self, FsKind};
use macread::util::*;

const TITLE: &str = "macread — Navegador de discos Mac e Linux";

#[derive(Clone)]
enum Node {
    Info(String),
    Volume { src: Source, label: String },
}

struct Session {
    src: Source,
    fs: Box<dyn FileSystem>,
    stack: Vec<Entry>,
    entries: Vec<Entry>,
}

#[derive(Default)]
struct Progress {
    files: u64,
    bytes: u64,
    current: String,
    done: bool,
    summary: String,
    errors: Vec<String>,
    log_path: String,
    failed: Option<String>,
}

struct Job {
    progress: Arc<Mutex<Progress>>,
    cancel: Arc<AtomicBool>,
}

#[derive(Default)]
struct App {
    window: nwg::Window,
    btn_refresh: nwg::Button,
    btn_image: nwg::Button,
    btn_up: nwg::Button,
    btn_copy_sel: nwg::Button,
    btn_copy_all: nwg::Button,
    btn_verify: nwg::Button,
    btn_mount: nwg::Button,
    btn_cancel: nwg::Button,
    path_box: nwg::TextInput,
    tree: nwg::TreeView,
    list: nwg::ListView,
    status: nwg::Label,
    progress: nwg::ProgressBar,
    notice: nwg::Notice,
    folder_dialog: nwg::FileDialog,
    image_dialog: nwg::FileDialog,
    nodes: RefCell<Vec<(isize, Node)>>,
    session: RefCell<Option<Session>>,
    job: RefCell<Option<Job>>,
    images: RefCell<Vec<String>>,
    admin: bool,
    mount: RefCell<Option<(dokan::Mount, Arc<FsService>)>>,
}

fn path_string(stack: &[Entry]) -> String {
    let mut s = String::new();
    for e in stack.iter().skip(1) {
        s.push('/');
        s.push_str(&e.name);
    }
    if s.is_empty() {
        s.push('/');
    }
    s
}

impl App {
    fn set_status(&self, text: &str) {
        self.status.set_text(text);
    }

    fn layout(&self) {
        let (w, h) = self.window.size();
        let (w, h) = (w as i32, h as i32);
        let top = 8;
        let bh = 30;
        let mut x = 8;
        let buttons: [(&nwg::Button, i32); 8] = [
            (&self.btn_refresh, 120),
            (&self.btn_image, 150),
            (&self.btn_up, 80),
            (&self.btn_copy_sel, 190),
            (&self.btn_copy_all, 180),
            (&self.btn_verify, 130),
            (&self.btn_mount, 190),
            (&self.btn_cancel, 90),
        ];
        for (b, bw) in buttons {
            b.set_position(x, top);
            b.set_size(bw as u32, bh as u32);
            x += bw + 6;
        }
        let y2 = top + bh + 10;
        let tree_w = 360;
        let right_x = tree_w + 16;
        let right_w = (w - right_x - 8).max(120);
        self.path_box.set_position(right_x, y2);
        self.path_box.set_size(right_w as u32, 26);
        let y3 = y2 + 34;
        let bottom_h = 32;
        let list_h = (h - y3 - bottom_h - 8).max(80);
        self.tree.set_position(8, y2);
        self.tree.set_size(tree_w as u32, (h - y2 - bottom_h - 8).max(80) as u32);
        self.list.set_position(right_x, y3);
        self.list.set_size(right_w as u32, list_h as u32);
        let yb = h - bottom_h;
        self.status.set_position(8, yb + 6);
        self.status.set_size((w - 250).max(100) as u32, 24);
        self.progress.set_position(w - 236, yb + 6);
        self.progress.set_size(228, 20);
    }

    fn add_node(&self, parent: Option<&nwg::TreeItem>, text: &str, node: Node) -> nwg::TreeItem {
        let item = self.tree.insert_item(text, parent, nwg::TreeInsert::Last);
        self.nodes.borrow_mut().push((item.handle as isize, node));
        item
    }

    /// Acrescenta as partições/volumes de uma origem (disco físico ou imagem) sob um nó da árvore.
    fn add_source_tree(&self, parent: &nwg::TreeItem, spec: &str) {
        let (dev, layout) = match open::open_source(spec).and_then(|dev| partition::scan(&dev).map(|l| (dev, l))) {
            Ok(v) => v,
            Err(e) => {
                self.add_node(Some(parent), &format!("(erro: {})", e), Node::Info(e.to_string()));
                return;
            }
        };
        if layout.parts.is_empty() {
            self.add_node(Some(parent), "(sem partições)", Node::Info("Nenhuma partição encontrada.".into()));
        }
        for p in &layout.parts {
            let name = if p.name.is_empty() { String::new() } else { format!(" \"{}\"", p.name) };
            let text = format!("Partição {}: {}  {}{}", p.index, p.fs.name(), fmt_size(p.len), name);
            let base = Source { spec: spec.to_string(), part: Some(p.index), vol: None, force: false };
            match &p.fs {
                FsKind::Apfs => {
                    let item = self.add_node(Some(parent), &format!("{}  (container APFS)", text), Node::Info(format!("{} — expanda para ver os volumes", text)));
                    match apfs::Container::open(open::partition_device(&dev, p)) {
                        Ok(c) => {
                            for v in &c.volumes {
                                let label = format!(
                                    "Volume {}: \"{}\" ({}, {} arquivos){}",
                                    v.index + 1,
                                    v.name,
                                    v.role_name(),
                                    v.num_files,
                                    if v.encrypted { "  [CRIPTOGRAFADO]" } else { "" }
                                );
                                let src = Source { vol: Some(v.index + 1), ..base.clone() };
                                self.add_node(Some(&item), &label, Node::Volume { src, label: label.clone() });
                            }
                            self.tree.set_expand_state(&item, nwg::ExpandState::Expand);
                        }
                        Err(e) => {
                            self.add_node(Some(&item), &format!("(erro: {})", e), Node::Info(e.to_string()));
                        }
                    }
                }
                f if f.is_supported() => {
                    let label = format!("{}{}", text, if f.is_mac() { "  ◄ Mac" } else { "  ◄ Linux" });
                    self.add_node(Some(parent), &label, Node::Volume { src: base, label: text.clone() });
                }
                FsKind::Luks => {
                    self.add_node(Some(parent), &format!("{}  (criptografado: precisa da senha)", text), Node::Info(format!("{}: partição LUKS; não é possível ler sem a senha.", text)));
                }
                FsKind::Lvm => {
                    self.add_node(Some(parent), &format!("{}  (os volumes lógicos aparecem abaixo)", text), Node::Info(format!("{}: volume físico LVM; use os volumes lógicos listados.", text)));
                }
                FsKind::Xfs | FsKind::Btrfs | FsKind::F2fs => {
                    self.add_node(Some(parent), &format!("{}  (ainda não suportado)", text), Node::Info(format!("{}: este sistema de arquivos ainda não é suportado.", text)));
                }
                FsKind::CoreStorage => {
                    self.add_node(Some(parent), &format!("{}  (não suportado)", text), Node::Info(format!("{}: Core Storage (FileVault antigo/Fusion Drive) não é suportado.", text)));
                }
                _ => {
                    self.add_node(Some(parent), &text, Node::Info(format!("{}: não é uma partição Mac nem Linux legível.", text)));
                }
            }
        }
    }

    fn refresh_disks(&self) {
        if self.job.borrow().is_some() {
            return;
        }
        self.tree.clear();
        self.nodes.borrow_mut().clear();
        *self.session.borrow_mut() = None;
        self.list.clear();
        self.path_box.set_text("");
        self.set_status("Procurando discos...");
        let disks = device::enumerate_disks();
        if disks.is_empty() {
            self.add_node(None, "Nenhum disco físico encontrado", Node::Info("Nenhum disco físico encontrado.".into()));
        }
        for d in disks {
            let (model, size) = match &d.info {
                Some(i) => (format!("{} [{}]", i.model, i.bus), fmt_size(i.size)),
                None => ("?".to_string(), "?".to_string()),
            };
            let text = format!("Disco {}: {}  {}", d.number, model, size);
            let item = self.add_node(None, &text, Node::Info(text.clone()));
            if !d.readable {
                self.add_node(
                    Some(&item),
                    "(sem permissão: feche e execute como Administrador)",
                    Node::Info("Para ler discos físicos o programa precisa ser executado como Administrador.".into()),
                );
            } else {
                self.add_source_tree(&item, &format!("disco:{}", d.number));
            }
            self.tree.set_expand_state(&item, nwg::ExpandState::Expand);
        }
        let images = self.images.borrow().clone();
        for img in images {
            let name = Path::new(&img).file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| img.clone());
            let item = self.add_node(None, &format!("Imagem: {}", name), Node::Info(img.clone()));
            self.add_source_tree(&item, &img);
            self.tree.set_expand_state(&item, nwg::ExpandState::Expand);
        }
        self.set_status("Selecione uma partição ou volume à esquerda.");
    }

    fn open_image(&self) {
        if self.job.borrow().is_some() {
            return;
        }
        if self.image_dialog.run(Some(&self.window)) {
            if let Ok(p) = self.image_dialog.get_selected_item() {
                let p = p.to_string_lossy().to_string();
                let mut imgs = self.images.borrow_mut();
                if !imgs.contains(&p) {
                    imgs.push(p);
                }
                drop(imgs);
                self.refresh_disks();
            }
        }
    }

    fn tree_selected(&self) {
        let item = match self.tree.selected_item() {
            Some(i) => i,
            None => return,
        };
        let h = item.handle as isize;
        let node = self.nodes.borrow().iter().find(|(k, _)| *k == h).map(|(_, n)| n.clone());
        match node {
            Some(Node::Volume { src, label }) => self.open_volume(src, label),
            Some(Node::Info(s)) => self.set_status(&s),
            None => {}
        }
    }

    fn open_volume(&self, src: Source, label: String) {
        if self.job.borrow().is_some() {
            return;
        }
        if let Some(s) = self.session.borrow().as_ref() {
            if s.src == src {
                return;
            }
        }
        self.set_status(&format!("Abrindo {}...", label));
        match open::open(&src, true) {
            Ok(o) => match o.fs.root() {
                Ok(root) => {
                    *self.session.borrow_mut() = Some(Session { src, fs: o.fs, stack: vec![root], entries: Vec::new() });
                    self.list_current();
                }
                Err(e) => {
                    self.set_status("Erro ao abrir a pasta raiz.");
                    nwg::modal_error_message(&self.window, "Erro", &e.to_string());
                }
            },
            Err(e) => {
                self.set_status("Não foi possível abrir o volume.");
                nwg::modal_error_message(&self.window, "Não foi possível abrir", &e.to_string());
            }
        }
    }

    fn list_current(&self) {
        let mut guard = self.session.borrow_mut();
        let sess = match guard.as_mut() {
            Some(s) => s,
            None => return,
        };
        let dir = sess.stack.last().cloned().unwrap();
        let mut entries = match sess.fs.read_dir(&dir) {
            Ok(e) => e,
            Err(e) => {
                drop(guard);
                nwg::modal_error_message(&self.window, "Erro ao listar a pasta", &e.to_string());
                return;
            }
        };
        entries.sort_by(|a, b| (a.kind != Kind::Dir).cmp(&(b.kind != Kind::Dir)).then(a.name.to_lowercase().cmp(&b.name.to_lowercase())));
        self.list.clear();
        let (mut nd, mut nf, mut total) = (0u64, 0u64, 0u64);
        for (i, e) in entries.iter().enumerate() {
            let (size, kind) = match e.kind {
                Kind::Dir => {
                    nd += 1;
                    (String::new(), "Pasta".to_string())
                }
                Kind::File => {
                    nf += 1;
                    total += e.size;
                    (fmt_size(e.size), if e.compressed { "Arquivo (comprimido)".to_string() } else { "Arquivo".to_string() })
                }
                Kind::Symlink => (String::new(), format!("Link → {}", sess.fs.read_link(e).unwrap_or_default())),
                Kind::Other => (String::new(), "Especial".to_string()),
            };
            let date = fmt_time(e.mtime);
            let cols = [e.name.as_str(), size.as_str(), date.as_str(), kind.as_str()];
            self.list.insert_items_row(Some(i as i32), &cols);
        }
        self.path_box.set_text(&path_string(&sess.stack));
        sess.entries = entries;
        let summary = sess.fs.summary();
        drop(guard);
        self.set_status(&format!("{} pastas, {} arquivos ({})  —  {}", nd, nf, fmt_size(total), summary));
    }

    fn activate_row(&self, row: usize) {
        let entry = {
            let guard = self.session.borrow();
            match guard.as_ref().and_then(|s| s.entries.get(row).cloned()) {
                Some(e) => e,
                None => return,
            }
        };
        match entry.kind {
            Kind::Dir => {
                if let Some(s) = self.session.borrow_mut().as_mut() {
                    s.stack.push(entry);
                }
                self.list_current();
            }
            Kind::File => {
                let text = format!(
                    "{}\n\nTamanho: {} ({} bytes)\nModificado: {}\nCriado: {}\nPermissões: {}{}\n\nSelecione o arquivo e use \"Copiar selecionados...\" para copiá-lo.",
                    entry.name,
                    fmt_size(entry.size),
                    entry.size,
                    fmt_time(entry.mtime),
                    fmt_time(entry.crtime),
                    mode_string(entry.mode, '-'),
                    if entry.compressed { "\nComprimido pelo macOS (será descomprimido na cópia)" } else { "" }
                );
                nwg::modal_info_message(&self.window, "Arquivo", &text);
            }
            Kind::Symlink => {
                let target = self.session.borrow().as_ref().map(|s| s.fs.read_link(&entry).unwrap_or_default()).unwrap_or_default();
                nwg::modal_info_message(&self.window, "Link simbólico", &format!("{} → {}", entry.name, target));
            }
            Kind::Other => {}
        }
    }

    fn go_up(&self) {
        let popped = match self.session.borrow_mut().as_mut() {
            Some(s) if s.stack.len() > 1 => {
                s.stack.pop();
                true
            }
            _ => false,
        };
        if popped {
            self.list_current();
        }
    }

    fn choose_folder(&self) -> Option<PathBuf> {
        if self.folder_dialog.run(Some(&self.window)) {
            self.folder_dialog.get_selected_item().ok().map(PathBuf::from)
        } else {
            None
        }
    }

    fn current_source(&self) -> Option<(Source, String)> {
        self.session.borrow().as_ref().map(|s| (s.src.clone(), path_string(&s.stack)))
    }

    fn copy_selected(&self) {
        let (src, path) = match self.current_source() {
            Some(v) => v,
            None => {
                nwg::modal_info_message(&self.window, "Copiar", "Abra primeiro um volume à esquerda.");
                return;
            }
        };
        let rows = self.list.selected_items();
        if rows.is_empty() {
            nwg::modal_info_message(&self.window, "Copiar", "Selecione um ou mais itens na lista (Ctrl/Shift para vários).");
            return;
        }
        let names: Vec<String> = {
            let guard = self.session.borrow();
            let s = guard.as_ref().unwrap();
            rows.iter().filter_map(|r| s.entries.get(*r).map(|e| e.name.clone())).collect()
        };
        if let Some(dest) = self.choose_folder() {
            self.start_job(src, path, Some(names), Some(dest));
        }
    }

    fn copy_all(&self) {
        let (src, path) = match self.current_source() {
            Some(v) => v,
            None => {
                nwg::modal_info_message(&self.window, "Copiar", "Abra primeiro um volume à esquerda.");
                return;
            }
        };
        if let Some(dest) = self.choose_folder() {
            self.start_job(src, path, None, Some(dest));
        }
    }

    fn verify(&self) {
        let (src, path) = match self.current_source() {
            Some(v) => v,
            None => {
                nwg::modal_info_message(&self.window, "Verificar", "Abra primeiro um volume à esquerda.");
                return;
            }
        };
        self.start_job(src, path, None, None);
    }

    fn toggle_mount(&self) {
        if self.job.borrow().is_some() {
            return;
        }
        let current = self.mount.borrow_mut().take();
        if let Some((m, svc)) = current {
            let letter = m.letter;
            m.unmount();
            svc.stop();
            self.btn_mount.set_text("Montar como unidade");
            self.set_status(&format!("Unidade {}: desmontada.", letter));
            return;
        }
        let src = match self.session.borrow().as_ref() {
            Some(s) => s.src.clone(),
            None => {
                nwg::modal_info_message(&self.window, "Montar", "Abra primeiro um volume à esquerda; ele será montado como uma unidade do Windows.");
                return;
            }
        };
        if let Err(e) = dokan::available() {
            nwg::modal_error_message(
                &self.window,
                "Driver Dokan necessário",
                &format!(
                    "Para montar o volume como uma unidade do Windows é preciso o driver Dokan 2 (gratuito, código aberto).\n\n{}\n\nBaixe o instalador DokanSetup.exe em https://github.com/dokan-dev/dokany/releases, instale e tente de novo. As outras funções (navegar e copiar) não precisam dele.",
                    e
                ),
            );
            return;
        }
        let letter = match dokan::free_letters().last() {
            Some(l) => *l,
            None => {
                nwg::modal_error_message(&self.window, "Montar", "Não há letra de unidade livre.");
                return;
            }
        };
        self.set_status(&format!("Montando como {}:...", letter));
        let svc = match FsService::start(src, 0) {
            Ok(s) => Arc::new(s),
            Err(e) => {
                nwg::modal_error_message(&self.window, "Montar", &e.to_string());
                return;
            }
        };
        match dokan::mount(svc.clone(), letter, self.admin) {
            Ok(m) => {
                *self.mount.borrow_mut() = Some((m, svc));
                self.btn_mount.set_text(&format!("Desmontar {}:", letter));
                self.set_status(&format!("Unidade {}: montada (somente leitura). Clique em \"Desmontar\" antes de remover o disco.", letter));
                let _ = std::process::Command::new("explorer.exe").arg(format!("{}:\\", letter)).spawn();
            }
            Err(e) => {
                svc.stop();
                self.set_status("Falha ao montar.");
                nwg::modal_error_message(&self.window, "Não foi possível montar", &e);
            }
        }
    }

    fn set_busy(&self, busy: bool) {
        for b in [&self.btn_refresh, &self.btn_image, &self.btn_up, &self.btn_copy_sel, &self.btn_copy_all, &self.btn_verify, &self.btn_mount] {
            b.set_enabled(!busy);
        }
        self.btn_cancel.set_enabled(busy);
        self.progress.set_visible(busy);
        self.progress.set_marquee(busy, 30);
    }

    fn start_job(&self, src: Source, dir_path: String, names: Option<Vec<String>>, dest: Option<PathBuf>) {
        if self.job.borrow().is_some() {
            return;
        }
        let progress = Arc::new(Mutex::new(Progress::default()));
        let cancel = Arc::new(AtomicBool::new(false));
        let sender = self.notice.sender();
        let (p2, c2) = (progress.clone(), cancel.clone());
        let is_copy = dest.is_some();
        thread::spawn(move || {
            let r = catch_unwind(AssertUnwindSafe(|| run_job(&src, &dir_path, names.as_deref(), dest.as_deref(), &p2, &c2, &sender)));
            let mut p = p2.lock().unwrap();
            match r {
                Ok(Ok(())) => {}
                Ok(Err(e)) => p.failed = Some(e),
                Err(_) => p.failed = Some("erro interno (metadados corrompidos?)".into()),
            }
            p.done = true;
            drop(p);
            sender.notice();
        });
        *self.job.borrow_mut() = Some(Job { progress, cancel });
        self.set_busy(true);
        self.set_status(if is_copy { "Copiando..." } else { "Verificando a leitura..." });
    }

    fn cancel_job(&self) {
        if let Some(j) = self.job.borrow().as_ref() {
            j.cancel.store(true, Ordering::Relaxed);
            self.set_status("Cancelando...");
        }
    }

    fn job_update(&self) {
        let snapshot = {
            let guard = self.job.borrow();
            let job = match guard.as_ref() {
                Some(j) => j,
                None => return,
            };
            let p = job.progress.lock().unwrap();
            (p.files, p.bytes, p.current.clone(), p.done, p.summary.clone(), p.errors.clone(), p.log_path.clone(), p.failed.clone())
        };
        let (files, bytes, current, done, summary, errors, log_path, failed) = snapshot;
        if !done {
            self.set_status(&format!("{} arquivos, {}  —  {}", files, fmt_size(bytes), current));
            return;
        }
        *self.job.borrow_mut() = None;
        self.set_busy(false);
        match failed {
            Some(f) => {
                self.set_status("Falhou.");
                nwg::modal_error_message(&self.window, "Falha", &f);
            }
            None => {
                self.set_status(&summary);
                let mut text = summary;
                if !errors.is_empty() {
                    text.push_str("\n\nPrimeiros erros:\n");
                    for e in errors.iter().take(12) {
                        text.push_str(e);
                        text.push('\n');
                    }
                    if !log_path.is_empty() {
                        text.push_str(&format!("\nLog completo: {}", log_path));
                    }
                    nwg::modal_error_message(&self.window, "Concluído com erros", &text);
                } else {
                    nwg::modal_info_message(&self.window, "Concluído", &text);
                }
            }
        }
    }
}

fn run_job(
    src: &Source,
    dir_path: &str,
    names: Option<&[String]>,
    dest: Option<&Path>,
    progress: &Arc<Mutex<Progress>>,
    cancel: &Arc<AtomicBool>,
    sender: &nwg::NoticeSender,
) -> Result<(), String> {
    let opened = open::open(src, true).map_err(|e| e.to_string())?;
    let fsys = opened.fs.as_ref();
    let base = fs::lookup(fsys, dir_path).map_err(|e| e.to_string())?;
    let items: Vec<Entry> = match names {
        Some(ns) => fsys.read_dir(&base).map_err(|e| e.to_string())?.into_iter().filter(|e| ns.contains(&e.name)).collect(),
        None => vec![base.clone()],
    };
    let verify = dest.is_none();
    let dest_long = dest.map(copy::long_path).unwrap_or_else(|| PathBuf::from("."));
    if !verify {
        std::fs::create_dir_all(&dest_long).map_err(|e| format!("não foi possível criar a pasta de destino: {}", e))?;
    }
    let log_path = dest_long.join("macread-log.txt");
    let mut st = copy::Stats::new(if verify { None } else { Some(&log_path) });
    st.quiet = true;
    st.cancel = Some(cancel.clone());
    let (p, s) = (progress.clone(), sender.clone());
    st.hook = Some(Box::new(move |files, bytes, cur| {
        if let Ok(mut g) = p.lock() {
            g.files = files;
            g.bytes = bytes;
            g.current = cur.to_string();
        }
        s.notice();
    }));
    let opts = copy::Options { overwrite: false, symlinks: true, verbose: false, dry_run: false, verify };
    for e in &items {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let sp = if names.is_some() { format!("{}/{}", dir_path.trim_end_matches('/'), e.name) } else { dir_path.to_string() };
        copy::copy_entry(fsys, e, &dest_long, &sp, &opts, &mut st);
    }
    st.finish();
    let secs = st.elapsed().as_secs_f64().max(0.001);
    let cancelled = cancel.load(Ordering::Relaxed);
    let summary = format!(
        "{}{}: {} arquivos ({}) e {} pastas em {:.1} s ({}/s). {} já existiam no destino, {} links, {} erros.",
        if cancelled { "Cancelado. " } else { "" },
        if verify { "Verificação" } else { "Cópia" },
        st.files,
        fmt_size(st.bytes),
        st.dirs,
        secs,
        fmt_size((st.bytes as f64 / secs) as u64),
        st.skipped,
        st.symlinks,
        st.errors
    );
    let mut g = progress.lock().map_err(|_| "erro interno".to_string())?;
    g.summary = summary;
    g.errors = st.error_list.clone();
    g.log_path = if verify { String::new() } else { copy::display_path(&log_path) };
    g.files = st.files;
    g.bytes = st.bytes;
    Ok(())
}

fn is_admin() -> bool {
    match device::FileDevice::open_physical(0) {
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => false,
        _ => true,
    }
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Reinicia o programa pedindo privilégios de Administrador (UAC). Devolve true se conseguiu.
fn relaunch_as_admin() -> bool {
    use winapi::um::shellapi::ShellExecuteW;
    use winapi::um::winuser::SW_SHOWNORMAL;
    let exe = match std::env::current_exe() {
        Ok(p) => p.to_string_lossy().to_string(),
        Err(_) => return false,
    };
    let verb = wide("runas");
    let file = wide(&exe);
    let args = wide("--elevado");
    let r = unsafe { ShellExecuteW(std::ptr::null_mut(), verb.as_ptr(), file.as_ptr(), args.as_ptr(), std::ptr::null(), SW_SHOWNORMAL) };
    (r as isize) > 32
}

fn build() -> Result<Rc<App>, nwg::NwgError> {
    let mut app = App::default();
    app.admin = is_admin();

    let _ = nwg::Font::set_global_family("Segoe UI");
    nwg::Window::builder()
        .size((1300, 720))
        .position((120, 80))
        .title(TITLE)
        .flags(nwg::WindowFlags::MAIN_WINDOW | nwg::WindowFlags::VISIBLE)
        .build(&mut app.window)?;

    nwg::Button::builder().text("Atualizar discos").parent(&app.window).build(&mut app.btn_refresh)?;
    nwg::Button::builder().text("Abrir imagem/DMG...").parent(&app.window).build(&mut app.btn_image)?;
    nwg::Button::builder().text("▲ Acima").parent(&app.window).build(&mut app.btn_up)?;
    nwg::Button::builder().text("Copiar selecionados para...").parent(&app.window).build(&mut app.btn_copy_sel)?;
    nwg::Button::builder().text("Copiar pasta atual para...").parent(&app.window).build(&mut app.btn_copy_all)?;
    nwg::Button::builder().text("Verificar leitura").parent(&app.window).build(&mut app.btn_verify)?;
    nwg::Button::builder().text("Montar como unidade").parent(&app.window).build(&mut app.btn_mount)?;
    nwg::Button::builder().text("Cancelar").parent(&app.window).enabled(false).build(&mut app.btn_cancel)?;

    nwg::TextInput::builder().parent(&app.window).readonly(true).text("").build(&mut app.path_box)?;
    nwg::TreeView::builder().parent(&app.window).build(&mut app.tree)?;
    nwg::ListView::builder()
        .parent(&app.window)
        .ex_flags(nwg::ListViewExFlags::FULL_ROW_SELECT | nwg::ListViewExFlags::GRID)
        .build(&mut app.list)?;
    app.list.set_list_style(nwg::ListViewStyle::Detailed);
    app.list.set_headers_enabled(true);
    for (i, (name, width, right)) in [("Nome", 420, false), ("Tamanho", 110, true), ("Modificado", 150, false), ("Tipo", 260, false)].iter().enumerate() {
        app.list.insert_column(nwg::InsertListViewColumn {
            index: Some(i as i32),
            fmt: Some(if *right { nwg::ListViewColumnFlags::RIGHT } else { nwg::ListViewColumnFlags::LEFT }),
            width: Some(*width),
            text: Some(name.to_string()),
        });
    }

    nwg::Label::builder().parent(&app.window).text("Pronto.").build(&mut app.status)?;
    nwg::ProgressBar::builder()
        .parent(&app.window)
        .flags(nwg::ProgressBarFlags::VISIBLE | nwg::ProgressBarFlags::MARQUEE)
        .build(&mut app.progress)?;
    app.progress.set_visible(false);
    nwg::Notice::builder().parent(&app.window).build(&mut app.notice)?;
    nwg::FileDialog::builder()
        .title("Escolha a pasta de destino no Windows")
        .action(nwg::FileDialogAction::OpenDirectory)
        .build(&mut app.folder_dialog)?;
    nwg::FileDialog::builder()
        .title("Abrir imagem de disco ou DMG")
        .action(nwg::FileDialogAction::Open)
        .filters("Imagens de disco(*.img;*.raw;*.dd;*.dmg;*.bin;*.iso;*.hfs;*.apfs)|Todos os arquivos(*.*)")
        .build(&mut app.image_dialog)?;

    let app = Rc::new(app);
    let a = app.clone();
    let _handler = nwg::full_bind_event_handler(&app.window.handle, move |evt, data, handle| {
        use nwg::Event as E;
        match evt {
            E::OnResize | E::OnWindowMaximize => {
                if handle == a.window.handle {
                    a.layout();
                }
            }
            E::OnWindowClose => {
                if handle == a.window.handle {
                    if a.job.borrow().is_some() {
                        let p = nwg::MessageParams {
                            title: "Sair",
                            content: "Há uma cópia em andamento. Deseja cancelar e sair?",
                            buttons: nwg::MessageButtons::YesNo,
                            icons: nwg::MessageIcons::Question,
                        };
                        if nwg::modal_message(&a.window, &p) != nwg::MessageChoice::Yes {
                            return;
                        }
                        a.cancel_job();
                    }
                    if let Some((m, svc)) = a.mount.borrow_mut().take() {
                        m.unmount();
                        svc.stop();
                    }
                    nwg::stop_thread_dispatch();
                }
            }
            E::OnButtonClick => {
                if handle == a.btn_refresh.handle {
                    a.refresh_disks();
                } else if handle == a.btn_image.handle {
                    a.open_image();
                } else if handle == a.btn_up.handle {
                    a.go_up();
                } else if handle == a.btn_copy_sel.handle {
                    a.copy_selected();
                } else if handle == a.btn_copy_all.handle {
                    a.copy_all();
                } else if handle == a.btn_verify.handle {
                    a.verify();
                } else if handle == a.btn_mount.handle {
                    a.toggle_mount();
                } else if handle == a.btn_cancel.handle {
                    a.cancel_job();
                }
            }
            E::OnTreeItemSelectionChanged => {
                if handle == a.tree.handle {
                    a.tree_selected();
                }
            }
            E::OnListViewItemActivated => {
                if handle == a.list.handle {
                    if let nwg::EventData::OnListViewItemIndex { row_index, .. } = data {
                        a.activate_row(row_index);
                    }
                }
            }
            E::OnNotice => {
                if handle == a.notice.handle {
                    a.job_update();
                }
            }
            _ => {}
        }
    });
    app.layout();
    Ok(app)
}

fn main() {
    let elevated_arg = std::env::args().any(|a| a == "--elevado" || a == "--sem-admin");
    if !elevated_arg && !is_admin() && relaunch_as_admin() {
        return;
    }
    unsafe {
        winapi::um::winuser::SetProcessDPIAware();
    }
    if let Err(e) = nwg::init() {
        nwg::simple_message("macread", &format!("Falha ao iniciar a interface: {}", e));
        return;
    }
    let app = match build() {
        Ok(a) => a,
        Err(e) => {
            nwg::simple_message("macread", &format!("Falha ao criar a janela: {}", e));
            return;
        }
    };
    // imagens passadas na linha de comando (ou arrastadas sobre o executável)
    for arg in std::env::args().skip(1) {
        if !arg.starts_with("--") && Path::new(&arg).is_file() {
            app.images.borrow_mut().push(arg);
        }
    }
    app.refresh_disks();
    let quiet = std::env::args().any(|a| a == "--sem-admin");
    if !app.admin && !quiet {
        nwg::modal_info_message(
            &app.window,
            "Sem privilégios de Administrador",
            "O programa não está rodando como Administrador, então os discos físicos não podem ser lidos.\nImagens de disco e DMG continuam funcionando.\n\nFeche e execute de novo aceitando o pedido de permissão para acessar os discos.",
        );
    }
    nwg::dispatch_thread_events();
}
