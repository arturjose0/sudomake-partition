//! macread-gui — interface gráfica para navegar em discos Mac (APFS/HFS+), Linux (ext2/3/4, LVM)
//! e imagens de disco/DMG no Windows, copiar arquivos e montar volumes como unidade.
//! Desenvolvido por SUDOMAKE - PRESTAÇÃO DE SERVIÇOS, (SU), LDA.
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
use macread::osdetect;
use macread::partition::{self, FsKind};
use macread::util::*;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const TITLE: &str = "macread — Navegador de discos Mac e Linux · SUDOMAKE";
const COMPANY: &str = "SUDOMAKE - PRESTAÇÃO DE SERVIÇOS, (SU), LDA";
const COMPANY_NIF: &str = "5002359936";
const COMPANY_PHONE: &str = "932693623";
const COMPANY_SITE: &str = "https://sudomakes.com";
const REPO: &str = "https://github.com/arturjose0/macread";

#[derive(Clone)]
enum Node {
    Info { title: String, details: String },
    Volume { src: Source, title: String, details: String },
}

/// Descrição de um nó (partição/volume) antes de ser inserido na árvore.
struct Child {
    text: String,
    node: Node,
    os: Option<String>,
    children: Vec<Child>,
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
    title_font: nwg::Font,
    small_font: nwg::Font,
    btn_refresh: nwg::Button,
    btn_image: nwg::Button,
    btn_up: nwg::Button,
    btn_copy_sel: nwg::Button,
    btn_copy_all: nwg::Button,
    btn_verify: nwg::Button,
    btn_mount: nwg::Button,
    btn_cancel: nwg::Button,
    btn_about: nwg::Button,
    path_box: nwg::TextInput,
    tree: nwg::TreeView,
    list: nwg::ListView,
    info_title: nwg::Label,
    info_text: nwg::TextBox,
    btn_open_here: nwg::Button,
    btn_mount_here: nwg::Button,
    status: nwg::Label,
    footer: nwg::Label,
    btn_site: nwg::Button,
    progress: nwg::ProgressBar,
    notice: nwg::Notice,
    folder_dialog: nwg::FileDialog,
    image_dialog: nwg::FileDialog,
    nodes: RefCell<Vec<(isize, Node)>>,
    session: RefCell<Option<Session>>,
    selected: RefCell<Option<(Source, String)>>,
    job: RefCell<Option<Job>>,
    images: RefCell<Vec<String>>,
    admin: bool,
    mount: RefCell<Option<(dokan::Mount, Arc<FsService>, Source)>>,
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

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn open_url(url: &str) {
    use winapi::um::shellapi::ShellExecuteW;
    use winapi::um::winuser::SW_SHOWNORMAL;
    let verb = wide("open");
    let u = wide(url);
    unsafe {
        ShellExecuteW(std::ptr::null_mut(), verb.as_ptr(), u.as_ptr(), std::ptr::null(), std::ptr::null(), SW_SHOWNORMAL);
    }
}

fn company_line() -> String {
    format!("{}  ·  NIF {}  ·  Tel. {}  ·  {}", COMPANY, COMPANY_NIF, COMPANY_PHONE, COMPANY_SITE.trim_start_matches("https://"))
}

fn welcome_text() -> String {
    format!(
        "À esquerda estão os discos ligados a este computador e as imagens abertas, com o sistema \
         encontrado em cada partição (Windows, Linux, macOS, dados...).\r\n\r\n\
         Clique numa partição ou volume marcado com ◄ e escolha:\r\n\
         •  Abrir no programa: navegar pelas pastas e copiar arquivos para o Windows;\r\n\
         •  Montar como unidade: o volume aparece no Explorador como uma letra de disco (somente leitura).\r\n\r\n\
         Discos físicos exigem o programa aberto como Administrador. Nada é gravado no disco de origem.\r\n\r\n\
         macread {}  —  desenvolvido por {}\r\nNIF {}  ·  Contacto {}  ·  {}",
        VERSION, COMPANY, COMPANY_NIF, COMPANY_PHONE, COMPANY_SITE
    )
}

/// Abre a origem e descreve cada partição/volume, detectando o sistema instalado.
fn describe_source(spec: &str) -> Result<Vec<Child>, String> {
    let dev = open::open_source(spec).map_err(|e| e.to_string())?;
    let layout = partition::scan(&dev).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for p in &layout.parts {
        let name = if p.name.is_empty() { String::new() } else { format!(" \"{}\"", p.name) };
        let base_text = format!("Partição {}: {}  {}{}", p.index, p.fs.name(), fmt_size(p.len), name);
        let base = Source { spec: spec.to_string(), part: Some(p.index), vol: None, force: false };
        let common = format!("Partição {} de {}\r\nTipo: {}\r\nTamanho: {}\r\nSistema de arquivos: {}", p.index, spec, p.type_name, fmt_size(p.len), p.fs.name());
        match &p.fs {
            FsKind::Apfs => {
                let mut children = Vec::new();
                let mut oses = Vec::new();
                match apfs::Container::open(open::partition_device(&dev, p)) {
                    Ok(c) => {
                        for v in &c.volumes {
                            let src = Source { vol: Some(v.index + 1), ..base.clone() };
                            let (os, extra) = if v.encrypted {
                                ("criptografado (FileVault)".to_string(), String::new())
                            } else {
                                match open::open(&src, true) {
                                    Ok(o) => (osdetect::detect(o.fs.as_ref()), o.fs.summary()),
                                    Err(e) => (format!("não foi possível abrir: {}", e), String::new()),
                                }
                            };
                            let text = format!("Volume {}: \"{}\" — {}", v.index + 1, v.name, os);
                            let details = format!(
                                "{}\r\n\r\nVolume APFS {} \"{}\"\r\nFunção: {}\r\n{} arquivos, {} pastas{}\r\n{}\r\n\r\n{}",
                                os,
                                v.index + 1,
                                v.name,
                                v.role_name(),
                                v.num_files,
                                v.num_dirs,
                                if v.encrypted { "\r\nCRIPTOGRAFADO: precisa da senha do FileVault (não suportado)" } else { "" },
                                extra,
                                common
                            );
                            if !v.encrypted {
                                oses.push(os.clone());
                            }
                            let node = if v.encrypted { Node::Info { title: text.clone(), details } } else { Node::Volume { src, title: text.clone(), details } };
                            children.push(Child { text, node, os: Some(os), children: Vec::new() });
                        }
                    }
                    Err(e) => children.push(Child {
                        text: format!("(erro: {})", e),
                        node: Node::Info { title: "Erro".into(), details: e.to_string() },
                        os: None,
                        children: Vec::new(),
                    }),
                }
                let text = format!("{}  (container APFS com {} volumes)", base_text, children.len());
                out.push(Child {
                    text: text.clone(),
                    node: Node::Info { title: text, details: format!("Container APFS: escolha um dos volumes abaixo.\r\n\r\n{}", common) },
                    os: oses.first().cloned(),
                    children,
                });
            }
            f if f.is_supported() => match open::open(&base, true) {
                Ok(o) => {
                    let os = osdetect::detect(o.fs.as_ref());
                    let text = format!("{} — {}  {}", base_text, os, if f.is_mac() { "◄ Mac" } else { "◄ Linux" });
                    let details = format!("{}\r\n\r\n{}\r\n\r\n{}", os, o.fs.summary(), common);
                    out.push(Child { text: text.clone(), node: Node::Volume { src: base, title: text, details }, os: Some(os), children: Vec::new() });
                }
                Err(e) => {
                    let text = format!("{} — não foi possível abrir", base_text);
                    out.push(Child { text: text.clone(), node: Node::Info { title: text, details: format!("{}\r\n\r\n{}", e, common) }, os: None, children: Vec::new() });
                }
            },
            _ => {
                let purpose = osdetect::describe_partition(p);
                let text = if purpose.is_empty() { base_text.clone() } else { format!("{} — {}", base_text, purpose) };
                let details = format!(
                    "{}\r\n\r\nEsta partição não pode ser aberta pelo macread{}.\r\n\r\n{}",
                    if purpose.is_empty() { "Partição não legível".to_string() } else { purpose.clone() },
                    match &p.fs {
                        FsKind::Ntfs | FsKind::Fat | FsKind::ExFat => " (o próprio Windows já a lê, se estiver íntegra)",
                        FsKind::Luks => " (criptografada; precisa da senha, ainda não suportado)",
                        _ => "",
                    },
                    common
                );
                let os = match &p.fs {
                    FsKind::Ntfs => Some("Windows".to_string()),
                    FsKind::Luks => Some("Linux criptografado".to_string()),
                    FsKind::Xfs | FsKind::Btrfs | FsKind::F2fs => Some("Linux".to_string()),
                    _ => None,
                };
                out.push(Child { text: text.clone(), node: Node::Info { title: text, details }, os, children: Vec::new() });
            }
        }
    }
    Ok(out)
}

fn short_os(os: &str) -> String {
    let s = os.split(" (").next().unwrap_or(os).split(':').next().unwrap_or(os).trim();
    s.to_string()
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
        let buttons: [(&nwg::Button, i32); 9] = [
            (&self.btn_refresh, 118),
            (&self.btn_image, 150),
            (&self.btn_up, 80),
            (&self.btn_copy_sel, 188),
            (&self.btn_copy_all, 178),
            (&self.btn_verify, 128),
            (&self.btn_mount, 170),
            (&self.btn_cancel, 88),
            (&self.btn_about, 150),
        ];
        for (b, bw) in buttons {
            b.set_position(x, top);
            b.set_size(bw as u32, bh as u32);
            x += bw + 6;
        }
        let y2 = top + bh + 10;
        let bottom_h = 58;
        let tree_w = 430;
        let right_x = tree_w + 16;
        let right_w = (w - right_x - 8).max(120);
        let pane_h = (h - y2 - bottom_h - 8).max(80);
        self.tree.set_position(8, y2);
        self.tree.set_size(tree_w as u32, pane_h as u32);
        // modo navegação
        self.path_box.set_position(right_x, y2);
        self.path_box.set_size(right_w as u32, 26);
        self.list.set_position(right_x, y2 + 34);
        self.list.set_size(right_w as u32, (pane_h - 34).max(60) as u32);
        // modo informações
        self.info_title.set_position(right_x, y2 + 6);
        self.info_title.set_size(right_w as u32, 40);
        self.info_text.set_position(right_x + 2, y2 + 56);
        self.info_text.set_size((right_w - 4) as u32, (pane_h - 130).max(100) as u32);
        let by = y2 + pane_h - 54;
        self.btn_open_here.set_position(right_x, by);
        self.btn_open_here.set_size(250, 44);
        self.btn_mount_here.set_position(right_x + 262, by);
        self.btn_mount_here.set_size(270, 44);
        // rodapé
        let yb = h - bottom_h;
        self.status.set_position(8, yb + 4);
        self.status.set_size((w - 252).max(100) as u32, 22);
        self.progress.set_position(w - 236, yb + 5);
        self.progress.set_size(228, 20);
        self.footer.set_position(8, yb + 32);
        self.footer.set_size((w - 150).max(100) as u32, 22);
        self.btn_site.set_position(w - 136, yb + 30);
        self.btn_site.set_size(128, 24);
    }

    fn show_browser(&self, on: bool) {
        self.path_box.set_visible(on);
        self.list.set_visible(on);
        self.info_title.set_visible(!on);
        self.info_text.set_visible(!on);
        self.btn_open_here.set_visible(!on);
        self.btn_mount_here.set_visible(!on);
    }

    fn show_info(&self, title: &str, text: &str, actions: bool) {
        self.info_title.set_text(title);
        self.info_text.set_text(text);
        self.btn_open_here.set_enabled(actions && self.job.borrow().is_none());
        self.btn_mount_here.set_enabled(actions && self.job.borrow().is_none());
        self.show_browser(false);
    }

    fn add_node(&self, parent: Option<&nwg::TreeItem>, text: &str, node: Node) -> nwg::TreeItem {
        let item = self.tree.insert_item(text, parent, nwg::TreeInsert::Last);
        self.nodes.borrow_mut().push((item.handle as isize, node));
        item
    }

    fn insert_children(&self, parent: &nwg::TreeItem, children: Vec<Child>) {
        for c in children {
            let item = self.add_node(Some(parent), &c.text, c.node);
            if !c.children.is_empty() {
                self.insert_children(&item, c.children);
                self.tree.set_expand_state(&item, nwg::ExpandState::Expand);
            }
        }
    }

    fn os_summary(children: &[Child]) -> String {
        let mut names: Vec<String> = Vec::new();
        fn walk(list: &[Child], names: &mut Vec<String>) {
            for c in list {
                if let Some(os) = &c.os {
                    let s = short_os(os);
                    if !s.is_empty() && !names.contains(&s) && !s.starts_with("disco de dados") && s != "vazio" {
                        names.push(s);
                    }
                }
                walk(&c.children, names);
            }
        }
        walk(children, &mut names);
        names.truncate(3);
        names.join(" + ")
    }

    fn add_source(&self, root_text: &str, root_details: &str, spec: &str, expand: bool) {
        match describe_source(spec) {
            Ok(children) => {
                let summary = Self::os_summary(&children);
                let text = if summary.is_empty() { root_text.to_string() } else { format!("{}  —  {}", root_text, summary) };
                let item = self.add_node(None, &text, Node::Info { title: root_text.to_string(), details: format!("{}\r\n\r\nSistemas encontrados: {}\r\n\r\nSelecione uma partição ou volume abaixo deste disco.", root_details, if summary.is_empty() { "nenhum reconhecido" } else { &summary }) });
                self.insert_children(&item, children);
                if expand {
                    self.tree.set_expand_state(&item, nwg::ExpandState::Expand);
                }
            }
            Err(e) => {
                let item = self.add_node(None, root_text, Node::Info { title: root_text.to_string(), details: format!("{}\r\n\r\nErro: {}", root_details, e) });
                self.add_node(Some(&item), &format!("(erro: {})", e), Node::Info { title: "Erro".into(), details: e });
                self.tree.set_expand_state(&item, nwg::ExpandState::Expand);
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
        *self.selected.borrow_mut() = None;
        self.list.clear();
        self.path_box.set_text("");
        self.show_info("Discos deste computador", "Procurando discos e identificando os sistemas instalados...", false);
        self.set_status("Procurando discos...");
        let disks = device::enumerate_disks();
        if disks.is_empty() {
            self.add_node(None, "Nenhum disco físico encontrado", Node::Info { title: "Nenhum disco".into(), details: "Nenhum disco físico foi encontrado.".into() });
        }
        for d in disks {
            let (model, bus, size, sector) = match &d.info {
                Some(i) => (i.model.clone(), i.bus.clone(), fmt_size(i.size), i.sector),
                None => ("?".to_string(), "?".to_string(), "?".to_string(), 0),
            };
            let text = format!("Disco {}: {} [{}]  {}", d.number, model, bus, size);
            let details = format!("Disco físico {} (\\\\.\\PhysicalDrive{})\r\nModelo: {}\r\nBarramento: {}\r\nTamanho: {}\r\nSetor: {} bytes", d.number, d.number, model, bus, size, sector);
            if !d.readable {
                let item = self.add_node(None, &format!("{}  —  (sem permissão de Administrador)", text), Node::Info { title: text.clone(), details: format!("{}\r\n\r\nPara ler discos físicos o programa precisa ser executado como Administrador. Feche e abra de novo aceitando o pedido de permissão.", details) });
                self.tree.set_expand_state(&item, nwg::ExpandState::Expand);
            } else {
                self.add_source(&text, &details, &format!("disco:{}", d.number), true);
            }
        }
        let images = self.images.borrow().clone();
        for img in images {
            let name = Path::new(&img).file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| img.clone());
            self.add_source(&format!("Imagem: {}", name), &format!("Arquivo de imagem\r\n{}", img), &img, true);
        }
        self.show_info("Bem-vindo ao macread", &welcome_text(), false);
        self.set_status("Selecione um disco, partição ou volume à esquerda.");
        self.update_mount_buttons();
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
            Some(Node::Volume { src, title, details }) => {
                *self.selected.borrow_mut() = Some((src.clone(), title.clone()));
                let mounted_here = self.mount.borrow().as_ref().map_or(false, |(_, _, s)| *s == src);
                let extra = if mounted_here { "\r\n\r\nEste volume está montado como unidade do Windows." } else { "" };
                self.show_info(&title, &format!("{}{}\r\n\r\nO que deseja fazer com este volume?", details, extra), true);
                self.update_mount_buttons();
                self.set_status("Escolha \"Abrir no programa\" ou \"Montar como unidade\".");
            }
            Some(Node::Info { title, details }) => {
                *self.selected.borrow_mut() = None;
                self.show_info(&title, &details, false);
                self.set_status(&title);
            }
            None => {}
        }
    }

    fn open_selected(&self) {
        let (src, label) = match self.selected.borrow().clone() {
            Some(v) => v,
            None => return,
        };
        self.open_volume(src, label);
    }

    fn open_volume(&self, src: Source, label: String) {
        if self.job.borrow().is_some() {
            return;
        }
        let same = self.session.borrow().as_ref().map_or(false, |s| s.src == src);
        if same {
            self.show_browser(true);
            self.list_current();
            return;
        }
        self.set_status(&format!("Abrindo {}...", label));
        match open::open(&src, true) {
            Ok(o) => match o.fs.root() {
                Ok(root) => {
                    *self.session.borrow_mut() = Some(Session { src, fs: o.fs, stack: vec![root], entries: Vec::new() });
                    self.show_browser(true);
                    self.list_current();
                    self.update_mount_buttons();
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
                    "{}\n\nTamanho: {} ({} bytes)\nModificado: {}\nCriado: {}\nPermissões: {}{}\n\nSelecione o arquivo e use \"Copiar selecionados...\" para copiá-lo, ou monte o volume como unidade para abri-lo diretamente.",
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
        } else if self.session.borrow().is_some() {
            self.show_browser(true);
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

    fn need_session(&self, what: &str) -> Option<(Source, String)> {
        match self.current_source() {
            Some(v) => Some(v),
            None => {
                nwg::modal_info_message(&self.window, what, "Abra primeiro um volume: selecione-o à esquerda e clique em \"Abrir no programa\".");
                None
            }
        }
    }

    fn copy_selected(&self) {
        let (src, path) = match self.need_session("Copiar") {
            Some(v) => v,
            None => return,
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
        let (src, path) = match self.need_session("Copiar") {
            Some(v) => v,
            None => return,
        };
        if let Some(dest) = self.choose_folder() {
            self.start_job(src, path, None, Some(dest));
        }
    }

    fn verify(&self) {
        let (src, path) = match self.need_session("Verificar") {
            Some(v) => v,
            None => return,
        };
        self.start_job(src, path, None, None);
    }

    fn update_mount_buttons(&self) {
        let mounted = self.mount.borrow().as_ref().map(|(m, _, _)| m.letter);
        let text = match mounted {
            Some(l) => format!("Desmontar {}:", l),
            None => "Montar como unidade".to_string(),
        };
        self.btn_mount.set_text(&text);
        self.btn_mount_here.set_text(&text);
    }

    fn unmount_current(&self) -> Option<char> {
        let current = self.mount.borrow_mut().take();
        if let Some((m, svc, _)) = current {
            let letter = m.letter;
            m.unmount();
            svc.stop();
            self.update_mount_buttons();
            return Some(letter);
        }
        None
    }

    /// Botão da barra: age sobre o volume aberto (ou, se nenhum, sobre o selecionado).
    fn toggle_mount(&self) {
        if self.job.borrow().is_some() {
            return;
        }
        if self.mount.borrow().is_some() {
            if let Some(l) = self.unmount_current() {
                self.set_status(&format!("Unidade {}: desmontada.", l));
            }
            return;
        }
        let src = self.current_source().map(|(s, _)| s).or_else(|| self.selected.borrow().clone().map(|(s, _)| s));
        match src {
            Some(s) => self.mount_source(s),
            None => {
                nwg::modal_info_message(&self.window, "Montar", "Selecione à esquerda o volume que deseja montar como unidade.");
            }
        }
    }

    /// Botão do painel: age sobre o volume selecionado na árvore.
    fn mount_selected(&self) {
        if self.job.borrow().is_some() {
            return;
        }
        if self.mount.borrow().is_some() {
            if let Some(l) = self.unmount_current() {
                self.set_status(&format!("Unidade {}: desmontada.", l));
            }
            return;
        }
        let src = match self.selected.borrow().clone() {
            Some((s, _)) => s,
            None => return,
        };
        self.mount_source(src);
    }

    fn mount_source(&self, src: Source) {
        if let Err(e) = dokan::available() {
            nwg::modal_error_message(
                &self.window,
                "Driver Dokan necessário",
                &format!(
                    "Para montar o volume como uma unidade do Windows é preciso o driver Dokan 2 (gratuito, código aberto).\n\n{}\n\nO instalador do macread oferece a instalação do Dokan; também pode baixá-lo em https://github.com/dokan-dev/dokany/releases (DokanSetup.exe). As outras funções (navegar e copiar) não precisam dele.",
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
        let svc = match FsService::start(src.clone(), 0) {
            Ok(s) => Arc::new(s),
            Err(e) => {
                nwg::modal_error_message(&self.window, "Montar", &e.to_string());
                return;
            }
        };
        match dokan::mount(svc.clone(), letter, self.admin) {
            Ok(m) => {
                *self.mount.borrow_mut() = Some((m, svc, src));
                self.update_mount_buttons();
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

    fn about(&self) {
        let text = format!(
            "macread {}\nNavegador de discos Mac (APFS, HFS+) e Linux (ext2/3/4, LVM) para Windows.\n\n\
             Desenvolvido por\n{}\nNIF: {}\nContacto: {}\nSite: {}\n\n\
             Código aberto (licença MIT):\n{}\n\n\
             A montagem como unidade usa o driver Dokan 2 (https://github.com/dokan-dev/dokany).",
            VERSION, COMPANY, COMPANY_NIF, COMPANY_PHONE, COMPANY_SITE, REPO
        );
        nwg::modal_info_message(&self.window, "Sobre o macread", &text);
    }

    fn set_busy(&self, busy: bool) {
        for b in [&self.btn_refresh, &self.btn_image, &self.btn_up, &self.btn_copy_sel, &self.btn_copy_all, &self.btn_verify, &self.btn_mount, &self.btn_open_here, &self.btn_mount_here] {
            b.set_enabled(!busy);
        }
        self.btn_cancel.set_enabled(busy);
        self.progress.set_visible(busy);
        self.progress.set_marquee(busy, 30);
        if !busy && self.selected.borrow().is_none() {
            self.btn_open_here.set_enabled(false);
            self.btn_mount_here.set_enabled(false);
        }
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
    nwg::Font::builder().family("Segoe UI").size(28).weight(700).build(&mut app.title_font)?;
    nwg::Font::builder().family("Segoe UI").size(14).build(&mut app.small_font)?;
    nwg::Window::builder()
        .size((1320, 740))
        .position((100, 60))
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
    nwg::Button::builder().text("Sobre / SUDOMAKE").parent(&app.window).build(&mut app.btn_about)?;

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

    nwg::Label::builder().parent(&app.window).text("Bem-vindo ao macread").font(Some(&app.title_font)).v_align(nwg::VTextAlign::Top).build(&mut app.info_title)?;
    nwg::TextBox::builder()
        .parent(&app.window)
        .text("")
        .readonly(true)
        .flags(nwg::TextBoxFlags::VISIBLE | nwg::TextBoxFlags::VSCROLL | nwg::TextBoxFlags::AUTOVSCROLL | nwg::TextBoxFlags::TAB_STOP)
        .build(&mut app.info_text)?;
    nwg::Button::builder().text("Abrir no programa").parent(&app.window).enabled(false).build(&mut app.btn_open_here)?;
    nwg::Button::builder().text("Montar como unidade").parent(&app.window).enabled(false).build(&mut app.btn_mount_here)?;

    nwg::Label::builder().parent(&app.window).text("Pronto.").build(&mut app.status)?;
    nwg::Label::builder().parent(&app.window).text(&company_line()).font(Some(&app.small_font)).build(&mut app.footer)?;
    nwg::Button::builder().text("sudomakes.com").parent(&app.window).font(Some(&app.small_font)).build(&mut app.btn_site)?;
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
                    a.unmount_current();
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
                } else if handle == a.btn_about.handle {
                    a.about();
                } else if handle == a.btn_open_here.handle {
                    a.open_selected();
                } else if handle == a.btn_mount_here.handle {
                    a.mount_selected();
                } else if handle == a.btn_site.handle {
                    open_url(COMPANY_SITE);
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
    app.show_browser(false);
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
