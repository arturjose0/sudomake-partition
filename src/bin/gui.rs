//! SUDOMAKE Partition — interface gráfica: discos do computador em azulejos com o sistema
//! detectado, abrir no programa ou montar como unidade (várias ao mesmo tempo), cópia com
//! progresso, cópia de disco completo com verificação de espaço, actualização automática dos
//! discos, temas claro/escuro, quatro idiomas. Feito em Angola por José Artur Kassala / SUDOMAKE.
#![windows_subsystem = "windows"]

use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use native_windows_gui as nwg;
use winapi::shared::minwindef::{LPARAM, LRESULT};
use winapi::shared::windef::{HBRUSH, HDC, HFONT, HGDIOBJ, HICON, HWND, RECT};
use winapi::um::wingdi;
use winapi::um::winuser;

use sudomake_partition::apfs;
use sudomake_partition::copy;
use sudomake_partition::device;
use sudomake_partition::dokan;
use sudomake_partition::fs::{self, Entry, FileSystem, Kind};
use sudomake_partition::fsservice::FsService;
use sudomake_partition::i18n::{self, fmt, tr, Lang};
use sudomake_partition::open::{self, Source};
use sudomake_partition::osdetect::{self, Detected};
use sudomake_partition::partition::{self, FsKind};
use sudomake_partition::util::*;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// temporizador periódico (procura mudanças nos discos) e temporizador de "debounce" dos
/// avisos de ligação/remoção de dispositivos
const TIMER_PERIODIC: usize = 1;
const TIMER_DEVICE: usize = 2;
const PERIODIC_MS: u32 = 30_000;
const DEVICE_DEBOUNCE_MS: u32 = 2_500;

// ------------------------------------------------------------------------------------------
// configuração (idioma / tema)

#[derive(Clone, Copy, PartialEq, Eq)]
enum Theme {
    System,
    Light,
    Dark,
}

impl Theme {
    fn code(self) -> &'static str {
        match self {
            Theme::System => "system",
            Theme::Light => "light",
            Theme::Dark => "dark",
        }
    }
    fn from_code(s: &str) -> Theme {
        match s.trim() {
            "light" => Theme::Light,
            "dark" => Theme::Dark,
            _ => Theme::System,
        }
    }
    const ALL: [Theme; 3] = [Theme::System, Theme::Light, Theme::Dark];
}

impl Default for Theme {
    fn default() -> Self {
        Theme::System
    }
}

fn config_path() -> Option<PathBuf> {
    let base = std::env::var_os("APPDATA")?;
    Some(PathBuf::from(base).join("SUDOMAKE").join("Partition").join("config.txt"))
}

fn load_config() -> (Option<Lang>, Theme) {
    let mut lang = None;
    let mut theme = Theme::System;
    if let Some(p) = config_path() {
        if let Ok(t) = std::fs::read_to_string(p) {
            for line in t.lines() {
                if let Some(v) = line.strip_prefix("lang=") {
                    lang = Lang::from_code(v);
                } else if let Some(v) = line.strip_prefix("theme=") {
                    theme = Theme::from_code(v);
                }
            }
        }
    }
    (lang, theme)
}

fn save_config(lang: Option<Lang>, theme: Theme) {
    if let Some(p) = config_path() {
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(p, format!("lang={}\r\ntheme={}\r\n", lang.map(|l| l.code()).unwrap_or("auto"), theme.code()));
    }
}

fn system_lang() -> Lang {
    Lang::from_langid(unsafe { winapi::um::winnls::GetUserDefaultUILanguage() })
}

fn system_dark() -> bool {
    use winapi::um::winreg::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
    let key = wide("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize");
    let val = wide("AppsUseLightTheme");
    let mut data: u32 = 1;
    let mut size: u32 = 4;
    let r = unsafe { RegGetValueW(HKEY_CURRENT_USER, key.as_ptr(), val.as_ptr(), RRF_RT_REG_DWORD, std::ptr::null_mut(), &mut data as *mut u32 as *mut winapi::ctypes::c_void, &mut size) };
    r == 0 && data == 0
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

/// Espaço livre (bytes) na unidade onde fica `path`.
fn disk_free_space(path: &Path) -> Option<u64> {
    use winapi::shared::ntdef::ULARGE_INTEGER;
    use winapi::um::fileapi::GetDiskFreeSpaceExW;
    let mut s = path.to_string_lossy().to_string();
    if !s.ends_with('\\') {
        s.push('\\');
    }
    let w = wide(&s);
    unsafe {
        let mut free: ULARGE_INTEGER = std::mem::zeroed();
        let mut total: ULARGE_INTEGER = std::mem::zeroed();
        let mut total_free: ULARGE_INTEGER = std::mem::zeroed();
        if GetDiskFreeSpaceExW(w.as_ptr(), &mut free, &mut total, &mut total_free) != 0 {
            Some(*free.QuadPart())
        } else {
            None
        }
    }
}

/// Nome de pasta válido no Windows a partir do título de um volume.
fn folder_name_from(title: &str) -> String {
    let mut out: String = title
        .chars()
        .map(|c| if matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') || (c as u32) < 32 { '_' } else { c })
        .collect();
    out = out.trim().trim_end_matches('.').to_string();
    if out.is_empty() {
        out = "volume".to_string();
    }
    out
}

// ------------------------------------------------------------------------------------------
// azulejos (painel desenhado à mão)

#[derive(Clone)]
enum Node {
    Info,
    Volume { src: Source, title: String, used: Option<u64> },
}

#[derive(Clone, Copy, PartialEq)]
enum IconKind {
    Fixed,
    Removable,
    Image,
}

#[derive(Clone)]
struct Tile {
    title: String,
    line2: String,
    line3: String,
    used_frac: Option<f32>,
    icon: IconKind,
    node: Node,
    readable: bool,
    mounted: Option<char>,
    rect: RECT,
}

impl Tile {
    fn new(title: String, line2: String, line3: String, used_frac: Option<f32>, icon: IconKind, node: Node, readable: bool) -> Tile {
        Tile { title, line2, line3, used_frac, icon, node, readable, mounted: None, rect: RECT { left: 0, top: 0, right: 0, bottom: 0 } }
    }
    fn src(&self) -> Option<&Source> {
        match &self.node {
            Node::Volume { src, .. } => Some(src),
            Node::Info => None,
        }
    }
}

#[derive(Clone)]
struct Section {
    title: String,
    tiles: Vec<Tile>,
}

struct TileState {
    sections: Vec<Section>,
    selected: Option<(usize, usize)>,
    scroll: i32,
    content_h: i32,
    dark: bool,
    icons: [HICON; 3],
    logo: HICON,
    signature: String,
}

impl Default for TileState {
    fn default() -> Self {
        TileState { sections: Vec::new(), selected: None, scroll: 0, content_h: 0, dark: false, icons: [std::ptr::null_mut(); 3], logo: std::ptr::null_mut(), signature: String::new() }
    }
}

impl TileState {
    fn selected_tile(&self) -> Option<&Tile> {
        let (s, t) = self.selected?;
        self.sections.get(s)?.tiles.get(t)
    }
    fn find(&self, src: &Source) -> Option<(usize, usize)> {
        for (si, sec) in self.sections.iter().enumerate() {
            for (ti, t) in sec.tiles.iter().enumerate() {
                if t.src() == Some(src) {
                    return Some((si, ti));
                }
            }
        }
        None
    }
}

fn sections_signature(sections: &[Section]) -> String {
    let mut s = String::new();
    for sec in sections {
        s.push_str(&sec.title);
        s.push('\n');
        for t in &sec.tiles {
            s.push_str(&t.title);
            s.push('|');
            s.push_str(&t.line2);
            s.push('|');
            s.push_str(&t.line3);
            s.push('\n');
        }
    }
    s
}

fn rgb(r: u8, g: u8, b: u8) -> u32 {
    (r as u32) | ((g as u32) << 8) | ((b as u32) << 16)
}

struct Palette {
    bg: u32,
    tile: u32,
    tile_sel: u32,
    border: u32,
    border_sel: u32,
    text: u32,
    text2: u32,
    track: u32,
    fill: u32,
    fill_warn: u32,
    header: u32,
    badge: u32,
    badge_text: u32,
}

fn palette(dark: bool) -> Palette {
    if dark {
        Palette {
            bg: rgb(32, 32, 32),
            tile: rgb(45, 45, 45),
            tile_sel: rgb(58, 75, 92),
            border: rgb(60, 60, 60),
            border_sel: rgb(96, 165, 250),
            text: rgb(240, 240, 240),
            text2: rgb(185, 185, 185),
            track: rgb(70, 70, 70),
            fill: rgb(38, 160, 218),
            fill_warn: rgb(217, 83, 79),
            header: rgb(230, 230, 230),
            badge: rgb(38, 160, 218),
            badge_text: rgb(255, 255, 255),
        }
    } else {
        Palette {
            bg: rgb(255, 255, 255),
            tile: rgb(246, 246, 246),
            tile_sel: rgb(204, 228, 247),
            border: rgb(225, 225, 225),
            border_sel: rgb(0, 120, 212),
            text: rgb(26, 26, 26),
            text2: rgb(96, 96, 96),
            track: rgb(224, 224, 224),
            fill: rgb(38, 160, 218),
            fill_warn: rgb(217, 83, 79),
            header: rgb(40, 40, 40),
            badge: rgb(0, 120, 212),
            badge_text: rgb(255, 255, 255),
        }
    }
}

unsafe fn make_font(px: i32, bold: bool) -> HFONT {
    let face = wide("Segoe UI");
    wingdi::CreateFontW(
        -px,
        0,
        0,
        0,
        if bold { wingdi::FW_BOLD } else { wingdi::FW_NORMAL },
        0,
        0,
        0,
        wingdi::DEFAULT_CHARSET,
        wingdi::OUT_DEFAULT_PRECIS,
        wingdi::CLIP_DEFAULT_PRECIS,
        wingdi::CLEARTYPE_QUALITY,
        wingdi::DEFAULT_PITCH | wingdi::FF_DONTCARE,
        face.as_ptr(),
    )
}

unsafe fn draw_text(dc: HDC, text: &str, r: &mut RECT, font: HFONT, color: u32, flags: u32) {
    if text.is_empty() {
        return;
    }
    let old = wingdi::SelectObject(dc, font as HGDIOBJ);
    wingdi::SetTextColor(dc, color);
    let w: Vec<u16> = text.encode_utf16().collect();
    winuser::DrawTextW(dc, w.as_ptr(), w.len() as i32, r, flags | winuser::DT_NOPREFIX);
    wingdi::SelectObject(dc, old);
}

unsafe fn fill(dc: HDC, r: &RECT, color: u32) {
    let b = wingdi::CreateSolidBrush(color);
    winuser::FillRect(dc, r, b);
    wingdi::DeleteObject(b as HGDIOBJ);
}

unsafe fn round_rect(dc: HDC, r: &RECT, fill_color: u32, border_color: u32, radius: i32) {
    let b = wingdi::CreateSolidBrush(fill_color);
    let p = wingdi::CreatePen(wingdi::PS_SOLID as i32, 1, border_color);
    let ob = wingdi::SelectObject(dc, b as HGDIOBJ);
    let op = wingdi::SelectObject(dc, p as HGDIOBJ);
    wingdi::RoundRect(dc, r.left, r.top, r.right, r.bottom, radius, radius);
    wingdi::SelectObject(dc, ob);
    wingdi::SelectObject(dc, op);
    wingdi::DeleteObject(b as HGDIOBJ);
    wingdi::DeleteObject(p as HGDIOBJ);
}

#[link(name = "user32")]
extern "system" {
    fn PrivateExtractIconsW(file: *const u16, index: i32, cx: i32, cy: i32, icons: *mut HICON, ids: *mut u32, n: u32, flags: u32) -> u32;
}

/// Ícone do programa (recurso embutido pelo build.rs, ID 1) no tamanho pedido.
unsafe fn app_icon(size: i32) -> HICON {
    use winapi::um::libloaderapi::GetModuleHandleW;
    winuser::LoadImageW(GetModuleHandleW(std::ptr::null()), 1 as *const u16, winuser::IMAGE_ICON, size, size, 0) as HICON
}

/// Ícone de unidade do próprio Windows (fixo/removível), no tamanho pedido.
unsafe fn stock_icon(siid: u32, size: i32) -> HICON {
    use winapi::um::shellapi::{SHGetStockIconInfo, SHSTOCKICONINFO, SHGSI_ICONLOCATION};
    let mut info: SHSTOCKICONINFO = std::mem::zeroed();
    info.cbSize = std::mem::size_of::<SHSTOCKICONINFO>() as u32;
    if SHGetStockIconInfo(siid, SHGSI_ICONLOCATION, &mut info) != 0 {
        return std::ptr::null_mut();
    }
    let mut icon: HICON = std::ptr::null_mut();
    let mut id: u32 = 0;
    let n = PrivateExtractIconsW(info.szPath.as_ptr(), info.iIcon, size, size, &mut icon, &mut id, 1, 0);
    if n == 0 {
        std::ptr::null_mut()
    } else {
        icon
    }
}

const TILE_W: i32 = 330;
const TILE_H: i32 = 96;
const GAP: i32 = 14;

unsafe fn paint_tiles(hwnd: HWND, st: &mut TileState) {
    let mut ps: winuser::PAINTSTRUCT = std::mem::zeroed();
    let hdc = winuser::BeginPaint(hwnd, &mut ps);
    let mut client: RECT = std::mem::zeroed();
    winuser::GetClientRect(hwnd, &mut client);
    let (w, h) = (client.right - client.left, client.bottom - client.top);
    if w <= 0 || h <= 0 {
        winuser::EndPaint(hwnd, &ps);
        return;
    }
    let dpi = winuser::GetDpiForWindow(hwnd) as f32 / 96.0;
    let s = |v: i32| (v as f32 * dpi).round() as i32;
    let pal = palette(st.dark);

    // duplo buffer
    let mem = wingdi::CreateCompatibleDC(hdc);
    let bmp = wingdi::CreateCompatibleBitmap(hdc, w, h);
    let old_bmp = wingdi::SelectObject(mem, bmp as HGDIOBJ);
    fill(mem, &client, pal.bg);
    wingdi::SetBkMode(mem, wingdi::TRANSPARENT as i32);

    let f_header = make_font(s(19), true);
    let f_title = make_font(s(15), true);
    let f_small = make_font(s(13), false);
    let f_badge = make_font(s(13), true);

    if st.icons[0].is_null() {
        st.icons = [stock_icon(8, s(48)), stock_icon(7, s(48)), stock_icon(8, s(48))];
    }
    if st.logo.is_null() {
        st.logo = app_icon(s(64));
    }
    let logo_w = if st.logo.is_null() { 0 } else { s(64) + s(16) };

    let tile_w = s(TILE_W);
    let tile_h = s(TILE_H);
    let gap = s(GAP);
    let margin = s(16);
    let cols = ((w - margin) / (tile_w + gap)).max(1);
    let mut y = margin - st.scroll;
    for (si, sec) in st.sections.iter_mut().enumerate() {
        let mut hr = RECT { left: margin, top: y, right: w - margin - logo_w, bottom: y + s(30) };
        draw_text(mem, &sec.title, &mut hr, f_header, pal.header, winuser::DT_LEFT | winuser::DT_SINGLELINE | winuser::DT_END_ELLIPSIS);
        y += s(38);
        for (ti, tile) in sec.tiles.iter_mut().enumerate() {
            let col = (ti as i32) % cols;
            let row = (ti as i32) / cols;
            let x = margin + col * (tile_w + gap);
            let ty = y + row * (tile_h + gap);
            let r = RECT { left: x, top: ty, right: x + tile_w, bottom: ty + tile_h };
            tile.rect = r;
            let selected = st.selected == Some((si, ti));
            round_rect(mem, &r, if selected { pal.tile_sel } else { pal.tile }, if selected { pal.border_sel } else { pal.border }, s(8));
            if selected {
                let r2 = RECT { left: r.left + 1, top: r.top + 1, right: r.right - 1, bottom: r.bottom - 1 };
                round_rect(mem, &r2, pal.tile_sel, pal.border_sel, s(8));
            }
            let icon = match tile.icon {
                IconKind::Fixed => st.icons[0],
                IconKind::Removable => st.icons[1],
                IconKind::Image => st.icons[2],
            };
            let isz = s(48);
            if !icon.is_null() {
                winuser::DrawIconEx(mem, x + s(12), ty + (tile_h - isz) / 2, icon, isz, isz, 0, std::ptr::null_mut(), 0x0003);
            } else {
                let ir = RECT { left: x + s(12), top: ty + (tile_h - isz) / 2, right: x + s(12) + isz, bottom: ty + (tile_h + isz) / 2 };
                round_rect(mem, &ir, pal.track, pal.border, s(6));
            }
            let tx = x + s(72);
            let mut tw = tile_w - s(84);
            if let Some(letter) = tile.mounted {
                // etiqueta "Z:" no canto superior direito
                let br = RECT { left: r.right - s(48), top: ty + s(9), right: r.right - s(10), bottom: ty + s(29) };
                round_rect(mem, &br, pal.badge, pal.badge, s(6));
                let mut btr = br;
                draw_text(mem, &format!("{}:", letter), &mut btr, f_badge, pal.badge_text, winuser::DT_CENTER | winuser::DT_VCENTER | winuser::DT_SINGLELINE);
                tw -= s(42);
            }
            let mut r1 = RECT { left: tx, top: ty + s(10), right: tx + tw, bottom: ty + s(30) };
            draw_text(mem, &tile.title, &mut r1, f_title, if tile.readable { pal.text } else { pal.text2 }, winuser::DT_LEFT | winuser::DT_SINGLELINE | winuser::DT_END_ELLIPSIS);
            let tw_full = tile_w - s(84);
            let mut r2 = RECT { left: tx, top: ty + s(32), right: tx + tw_full, bottom: ty + s(50) };
            draw_text(mem, &tile.line2, &mut r2, f_small, pal.text2, winuser::DT_LEFT | winuser::DT_SINGLELINE | winuser::DT_END_ELLIPSIS);
            let mut next_y = ty + s(52);
            if let Some(frac) = tile.used_frac {
                let bar = RECT { left: tx, top: next_y, right: tx + tw_full, bottom: next_y + s(12) };
                fill(mem, &bar, pal.track);
                let fw = ((tw_full as f32) * frac.clamp(0.0, 1.0)) as i32;
                let fr = RECT { left: tx, top: next_y, right: tx + fw, bottom: next_y + s(12) };
                fill(mem, &fr, if frac > 0.9 { pal.fill_warn } else { pal.fill });
                next_y += s(15);
            }
            let mut r3 = RECT { left: tx, top: next_y, right: tx + tw_full, bottom: next_y + s(18) };
            draw_text(mem, &tile.line3, &mut r3, f_small, pal.text2, winuser::DT_LEFT | winuser::DT_SINGLELINE | winuser::DT_END_ELLIPSIS);
        }
        let rows = ((sec.tiles.len() as i32) + cols - 1) / cols;
        y += rows.max(0) * (tile_h + gap) + s(16);
    }
    st.content_h = y + st.scroll;

    // logótipo fixo no canto superior direito
    if !st.logo.is_null() {
        let lsz = s(64);
        let lx = w - margin - lsz;
        let bg = RECT { left: lx - s(8), top: 0, right: w, bottom: s(8) + lsz + s(8) };
        fill(mem, &bg, pal.bg);
        winuser::DrawIconEx(mem, lx, s(8), st.logo, lsz, lsz, 0, std::ptr::null_mut(), 0x0003);
    }

    wingdi::BitBlt(hdc, 0, 0, w, h, mem, 0, 0, wingdi::SRCCOPY);
    wingdi::SelectObject(mem, old_bmp);
    wingdi::DeleteObject(bmp as HGDIOBJ);
    wingdi::DeleteDC(mem);
    for f in [f_header, f_title, f_small, f_badge] {
        wingdi::DeleteObject(f as HGDIOBJ);
    }
    winuser::EndPaint(hwnd, &ps);
}

fn hit_test(st: &TileState, x: i32, y: i32) -> Option<(usize, usize)> {
    for (si, sec) in st.sections.iter().enumerate() {
        for (ti, t) in sec.tiles.iter().enumerate() {
            let r = &t.rect;
            if x >= r.left && x < r.right && y >= r.top && y < r.bottom {
                return Some((si, ti));
            }
        }
    }
    None
}

// ------------------------------------------------------------------------------------------
// leitura dos discos (corre numa thread própria; só usa o idioma, não a janela)

struct Child {
    tile: Tile,
    /// (nome do sistema, é genérico?) para o resumo do disco
    os: Option<(String, bool)>,
    children: Vec<Child>,
}

fn os_text(lang: Lang, d: &Detected) -> String {
    let t = |k: &str| tr(lang, k);
    match d {
        Detected::Linux(n) => fmt(t("os_linux"), &[n]),
        Detected::MacOs(n, v) => fmt(t("os_macos"), &[n, v]).trim().to_string(),
        Detected::MacData(u) => {
            let base = t("os_mac_data").to_string();
            if u.is_empty() { base } else { format!("{} ({})", base, u.join(", ")) }
        }
        Detected::LinuxHome { users, system } => {
            let base = if *system { t("os_linux_generic") } else { t("os_linux_home") }.to_string();
            if users.is_empty() { base } else { format!("{} ({})", base, users.join(", ")) }
        }
        Detected::LinuxGeneric => t("os_linux_generic").to_string(),
        Detected::MacGeneric => t("os_mac_generic").to_string(),
        Detected::TimeMachine => t("os_timemachine").to_string(),
        Detected::Data(n) => fmt(t("os_data"), &[&n.to_string()]),
        Detected::Empty => t("os_empty").to_string(),
    }
}

fn fs_name(lang: Lang, fs: &FsKind) -> String {
    match fs {
        FsKind::Empty => tr(lang, "part_empty").to_string(),
        FsKind::Unknown => tr(lang, "fs_unknown").to_string(),
        FsKind::Hfs => tr(lang, "fs_hfs_classic").to_string(),
        FsKind::Luks => tr(lang, "fs_luks").to_string(),
        other => other.name().to_string(),
    }
}

fn free_text(lang: Lang, cap: Option<(u64, u64)>, size: u64) -> (String, Option<f32>) {
    match cap {
        Some((total, free)) if total > 0 => {
            let used = total.saturating_sub(free);
            (fmt(tr(lang, "free_of"), &[&fmt_size(free), &fmt_size(total)]), Some(used as f32 / total as f32))
        }
        _ => (fmt_size(size), None),
    }
}

fn short_os(d: &Detected) -> Option<(String, bool)> {
    let generic = !matches!(d, Detected::Linux(_) | Detected::MacOs(..) | Detected::TimeMachine);
    d.short().map(|s| (s, generic))
}

/// Descreve as partições/volumes de uma origem.
fn describe_source(lang: Lang, spec: &str, icon: IconKind) -> Result<Vec<Child>, String> {
    let t = |k: &str| tr(lang, k);
    let dev = open::open_source(spec).map_err(|e| e.to_string())?;
    let layout = partition::scan(&dev).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    let part_word = t("partition");
    for p in &layout.parts {
        let base_name = if p.name.is_empty() { format!("{} {}", part_word, p.index) } else { format!("{} {} \"{}\"", part_word, p.index, p.name) };
        let base = Source { spec: spec.to_string(), part: Some(p.index), vol: None, force: false };
        match &p.fs {
            FsKind::Apfs => {
                let mut children = Vec::new();
                let mut first_os = None;
                match apfs::Container::open(open::partition_device(&dev, p)) {
                    Ok(c) => {
                        for v in &c.volumes {
                            let src = Source { vol: Some(v.index + 1), ..base.clone() };
                            let mut det_short: Option<(String, bool)> = None;
                            let mut used = None;
                            let (os, cap) = if v.encrypted {
                                (t("encrypted_volume").to_string(), None)
                            } else {
                                match open::open(&src, true) {
                                    Ok(o) => {
                                        let d = osdetect::detect(o.fs.as_ref());
                                        det_short = short_os(&d);
                                        used = o.fs.used();
                                        (os_text(lang, &d), o.fs.capacity())
                                    }
                                    Err(e) => (format!("{}: {}", t("cannot_open"), e), None),
                                }
                            };
                            let title = format!("{} {}: \"{}\"", t("volume"), v.index + 1, v.name);
                            let (line3, frac) = free_text(lang, cap, p.len);
                            if !v.encrypted && first_os.is_none() {
                                first_os = det_short.clone();
                            }
                            let node = if v.encrypted { Node::Info } else { Node::Volume { src, title: title.clone(), used } };
                            children.push(Child {
                                tile: Tile::new(title, os, format!("APFS · {}", line3), frac, icon, node, !v.encrypted),
                                os: det_short,
                                children: Vec::new(),
                            });
                        }
                    }
                    Err(e) => {
                        let title = format!("{} — {}", base_name, t("cannot_open"));
                        children.push(Child { tile: Tile::new(title, e.to_string(), fmt_size(p.len), None, icon, Node::Info, false), os: None, children: Vec::new() });
                    }
                }
                let title = format!("{} — {}", base_name, fmt(t("container_apfs"), &[&children.len().to_string()]));
                out.push(Child { tile: Tile::new(title, "APFS".into(), fmt_size(p.len), None, icon, Node::Info, false), os: first_os, children });
            }
            f if f.is_supported() => match open::open(&base, true) {
                Ok(o) => {
                    let d = osdetect::detect(o.fs.as_ref());
                    let os = os_text(lang, &d);
                    let (line3, frac) = free_text(lang, o.fs.capacity(), p.len);
                    let node = Node::Volume { src: base, title: base_name.clone(), used: o.fs.used() };
                    out.push(Child {
                        tile: Tile::new(base_name, os, format!("{} · {}", o.fs.fs_type(), line3), frac, icon, node, true),
                        os: short_os(&d),
                        children: Vec::new(),
                    });
                }
                Err(e) => {
                    let title = format!("{} — {}", base_name, t("cannot_open"));
                    out.push(Child { tile: Tile::new(title, e.to_string(), fmt_size(p.len), None, icon, Node::Info, false), os: None, children: Vec::new() });
                }
            },
            _ => {
                let kind = osdetect::classify_partition(p);
                let purpose = t(kind.key()).to_string();
                out.push(Child {
                    tile: Tile::new(base_name, purpose, format!("{} · {}", fs_name(lang, &p.fs), fmt_size(p.len)), None, icon, Node::Info, false),
                    os: kind.short().map(|s| (s.to_string(), true)),
                    children: Vec::new(),
                });
            }
        }
    }
    Ok(out)
}

/// Resumo dos sistemas encontrados num disco ("Windows + Ubuntu 22.04 LTS"): os nomes
/// específicos primeiro; "Linux"/"macOS" genéricos só quando não há nome específico.
fn os_summary(children: &[Child]) -> String {
    fn walk(list: &[Child], out: &mut Vec<(String, bool)>) {
        for c in list {
            if let Some((name, generic)) = &c.os {
                if !out.iter().any(|(n, _)| n == name) {
                    out.push((name.clone(), *generic));
                }
            }
            walk(&c.children, out);
        }
    }
    let mut all = Vec::new();
    walk(children, &mut all);
    let specific_linux = all.iter().any(|(n, g)| !g && !n.starts_with("macOS") && !n.starts_with("Mac OS") && !n.starts_with("OS X") && n != "Time Machine");
    let specific_mac = all.iter().any(|(n, g)| !g && (n.starts_with("macOS") || n.starts_with("Mac OS") || n.starts_with("OS X")));
    let mut names: Vec<String> = Vec::new();
    for (n, generic) in all {
        if generic && n == "Linux" && specific_linux {
            continue;
        }
        if generic && n == "macOS" && specific_mac {
            continue;
        }
        if !names.contains(&n) {
            names.push(n);
        }
    }
    names.truncate(3);
    names.join(" + ")
}

fn flatten(children: Vec<Child>, into: &mut Vec<Tile>) {
    for c in children {
        if c.children.is_empty() {
            into.push(c.tile);
        }
        flatten(c.children, into);
    }
}

fn add_section(lang: Lang, sections: &mut Vec<Section>, title: String, spec: &str, icon: IconKind) {
    match describe_source(lang, spec, icon) {
        Ok(children) => {
            let summary = os_summary(&children);
            let title = if summary.is_empty() { title } else { format!("{}  —  {}", title, summary) };
            let mut tiles = Vec::new();
            flatten(children, &mut tiles);
            sections.push(Section { title, tiles });
        }
        Err(e) => {
            sections.push(Section { title, tiles: vec![Tile::new(tr(lang, "error").to_string(), e, String::new(), None, icon, Node::Info, false)] });
        }
    }
}

/// Lê todos os discos e imagens e devolve as secções prontas a mostrar.
fn scan_sections(lang: Lang, images: &[String]) -> Vec<Section> {
    let t = |k: &str| tr(lang, k);
    let mut sections = Vec::new();
    let disks = device::enumerate_disks();
    if disks.is_empty() && images.is_empty() {
        sections.push(Section { title: t("no_disks").to_string(), tiles: Vec::new() });
    }
    for d in disks {
        let (model, bus, size) = match &d.info {
            Some(i) => (i.model.clone(), i.bus.clone(), fmt_size(i.size)),
            None => ("?".to_string(), "?".to_string(), "?".to_string()),
        };
        let removable = bus == "USB" || bus == "SD" || bus == "MMC" || bus == "FireWire";
        let icon = if removable { IconKind::Removable } else { IconKind::Fixed };
        let title = format!("{} {}: {} [{}]  {}", t("disk"), d.number, model, bus, size);
        if !d.readable {
            let title = format!("{}  —  {}", title, t("no_permission"));
            sections.push(Section { title, tiles: vec![Tile::new(t("no_permission").to_string(), t("no_permission_long").to_string(), size, None, icon, Node::Info, false)] });
        } else {
            add_section(lang, &mut sections, title, &format!("disco:{}", d.number), icon);
        }
    }
    for img in images {
        let name = Path::new(img).file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| img.clone());
        add_section(lang, &mut sections, format!("{}: {}", t("image"), name), img, IconKind::Image);
    }
    sections
}

// ------------------------------------------------------------------------------------------
// aplicação

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

struct MountEntry {
    mount: dokan::Mount,
    svc: Arc<FsService>,
    src: Source,
}

#[derive(Clone, Copy, Default)]
struct LangCell(Option<Lang>);

#[derive(Default)]
struct App {
    window: nwg::Window,
    small_font: nwg::Font,
    // barra de ferramentas (contextual)
    btn_home: nwg::Button,
    btn_image: nwg::Button,
    btn_open: nwg::Button,
    btn_mount: nwg::Button,
    btn_copy_disk: nwg::Button,
    btn_up: nwg::Button,
    btn_copy_sel: nwg::Button,
    btn_copy_all: nwg::Button,
    btn_verify: nwg::Button,
    btn_cancel: nwg::Button,
    btn_about: nwg::Button,
    btn_donate: nwg::Button,
    info: nwg::Label,
    progress: nwg::ProgressBar,
    // área principal
    path_box: nwg::TextInput,
    list: nwg::ListView,
    tiles: nwg::Frame,
    page: nwg::TextBox,
    // rodapé: empresa, contactos, idioma e tema
    footer: nwg::Label,
    btn_whatsapp: nwg::Button,
    btn_email: nwg::Button,
    btn_site: nwg::Button,
    btn_youtube: nwg::Button,
    btn_github: nwg::Button,
    btn_paypal: nwg::Button,
    lbl_lang: nwg::Label,
    combo_lang: nwg::ComboBox<String>,
    lbl_theme: nwg::Label,
    combo_theme: nwg::ComboBox<String>,
    notice: nwg::Notice,
    folder_dialog: nwg::FileDialog,
    image_dialog: nwg::FileDialog,
    // estado
    tile_state: Rc<RefCell<TileState>>,
    session: RefCell<Option<Session>>,
    job: RefCell<Option<Job>>,
    images: RefCell<Vec<String>>,
    admin: bool,
    mounts: RefCell<Vec<MountEntry>>,
    lang: Cell<LangCell>,
    theme: Cell<Theme>,
    dark: Rc<Cell<bool>>,
    /// 0 = azulejos, 1 = navegação, 2 = página de texto (sobre/doar)
    mode: Cell<u8>,
    scan_result: Arc<Mutex<Option<Vec<Section>>>>,
    scanning: Cell<bool>,
    scan_pending: Cell<bool>,
    scan_force: Cell<bool>,
    copy_sel_visible: Cell<bool>,
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
    fn lang_now(&self) -> Lang {
        self.lang.get().0.unwrap_or(Lang::Pt)
    }

    fn t(&self, key: &str) -> &'static str {
        tr(self.lang_now(), key)
    }

    fn tf(&self, key: &str, args: &[&str]) -> String {
        fmt(self.t(key), args)
    }

    fn set_info(&self, text: &str) {
        self.info.set_text(text);
    }

    // ---- discos (leitura em segundo plano) ----------------------------------------------

    /// Pede uma nova leitura dos discos. `force` substitui o ecrã mesmo que nada tenha mudado.
    fn request_scan(&self, force: bool) {
        if force {
            self.scan_force.set(true);
        }
        if self.scanning.get() || self.job.borrow().is_some() {
            self.scan_pending.set(true);
            return;
        }
        self.scanning.set(true);
        self.scan_pending.set(false);
        if self.tile_state.borrow().sections.is_empty() {
            self.set_info(self.t("searching"));
        }
        let lang = self.lang_now();
        let images = self.images.borrow().clone();
        let result = self.scan_result.clone();
        let sender = self.notice.sender();
        thread::spawn(move || {
            let sections = catch_unwind(AssertUnwindSafe(|| scan_sections(lang, &images))).unwrap_or_default();
            if let Ok(mut r) = result.lock() {
                *r = Some(sections);
            }
            sender.notice();
        });
    }

    /// Recebe o resultado da leitura (chamado pelo Notice na thread da janela).
    fn scan_update(&self) {
        let sections = match self.scan_result.lock().ok().and_then(|mut r| r.take()) {
            Some(s) => s,
            None => return,
        };
        self.scanning.set(false);
        let force = self.scan_force.replace(false);
        let signature = sections_signature(&sections);
        let changed = force || signature != self.tile_state.borrow().signature;
        if changed {
            let mut sections = sections;
            // etiquetas das unidades montadas
            for sec in sections.iter_mut() {
                for t in sec.tiles.iter_mut() {
                    t.mounted = t.src().and_then(|s| self.mount_letter(s));
                }
            }
            let selected_src = self.tile_state.borrow().selected_tile().and_then(|t| t.src().cloned());
            {
                let mut st = self.tile_state.borrow_mut();
                st.sections = sections;
                st.signature = signature;
                st.selected = selected_src.as_ref().and_then(|s| st.find(s));
                if st.selected.is_none() {
                    st.scroll = 0;
                }
            }
            self.invalidate_tiles();
            self.drop_lost_mounts();
            if self.mode.get() == 0 {
                self.set_info(if force { self.t("disks_updated") } else { self.t("select_hint") });
            }
            self.update_toolbar();
        } else if self.mode.get() == 0 && force {
            self.set_info(self.t("disks_updated"));
        }
        if self.scan_pending.get() {
            self.request_scan(false);
        }
    }

    /// Desmonta unidades cujo disco já não existe.
    fn drop_lost_mounts(&self) {
        let present: Vec<Source> = {
            let st = self.tile_state.borrow();
            st.sections.iter().flat_map(|s| s.tiles.iter()).filter_map(|t| t.src().cloned()).collect()
        };
        let lost: Vec<Source> = self.mounts.borrow().iter().filter(|m| !present.contains(&m.src)).map(|m| m.src.clone()).collect();
        for src in lost {
            if let Some(l) = self.unmount_source(&src) {
                self.set_info(&self.tf("mount_lost", &[&l.to_string()]));
            }
        }
    }

    fn invalidate_tiles(&self) {
        if let Some(h) = self.tiles.handle.hwnd() {
            unsafe {
                winuser::InvalidateRect(h, std::ptr::null(), 0);
            }
        }
    }

    /// Volume seleccionado no ecrã inicial: (origem, título, bytes usados).
    fn selected_volume(&self) -> Option<(Source, String, Option<u64>)> {
        let st = self.tile_state.borrow();
        match st.selected_tile().map(|t| t.node.clone()) {
            Some(Node::Volume { src, title, used }) => Some((src, title, used)),
            _ => None,
        }
    }

    fn tile_selected(&self) {
        let title = self.tile_state.borrow().selected_tile().map(|t| format!("{}  —  {}", t.title, t.line2));
        match title {
            Some(t) => self.set_info(&t),
            None => self.set_info(self.t("select_hint")),
        }
        self.update_toolbar();
    }

    // ---- modos ---------------------------------------------------------------------------

    /// 0 = azulejos, 1 = navegação, 2 = página de texto
    fn show_mode(&self, mode: u8) {
        self.mode.set(mode);
        self.tiles.set_visible(mode == 0);
        self.page.set_visible(mode == 2);
        self.path_box.set_visible(mode == 1);
        self.list.set_visible(mode == 1);
        if mode == 0 {
            self.tile_selected();
        }
        self.update_toolbar();
    }

    fn show_text_page(&self, text: &str) {
        self.show_mode(2);
        self.page.set_text(text);
        self.set_info("");
    }

    /// Origem a que os botões Montar/Verificar se aplicam no modo actual.
    fn context_source(&self) -> Option<Source> {
        match self.mode.get() {
            0 => self.selected_volume().map(|(s, _, _)| s),
            1 => self.session.borrow().as_ref().map(|s| s.src.clone()),
            _ => None,
        }
    }

    /// Mostra só os botões que fazem sentido no contexto actual e reposiciona tudo.
    fn update_toolbar(&self) {
        let job = self.job.borrow().is_some();
        let mode = self.mode.get();
        let sel = mode == 0 && self.selected_volume().is_some();
        let ctx = self.context_source();
        let mounted = ctx.as_ref().and_then(|s| self.mount_letter(s));
        self.btn_mount.set_text(&match mounted {
            Some(l) => self.tf("btn_unmount", &[&l.to_string()]),
            None => self.t("btn_mount").to_string(),
        });
        let copy_sel = !job && mode == 1 && self.list.selected_count() > 0;
        self.copy_sel_visible.set(copy_sel);
        self.btn_home.set_visible(!job);
        self.btn_image.set_visible(!job && mode == 0);
        self.btn_open.set_visible(!job && sel);
        self.btn_mount.set_visible(!job && ctx.is_some());
        self.btn_copy_disk.set_visible(!job && sel);
        self.btn_up.set_visible(!job && mode == 1);
        self.btn_copy_sel.set_visible(copy_sel);
        self.btn_copy_all.set_visible(!job && mode == 1);
        self.btn_verify.set_visible(!job && (sel || mode == 1));
        self.btn_cancel.set_visible(job);
        self.progress.set_visible(job);
        self.progress.set_marquee(job, 30);
        self.layout();
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
                self.request_scan(true);
            }
        }
    }

    fn open_selected(&self) {
        if let Some((src, label, _)) = self.selected_volume() {
            self.open_volume(src, label);
        }
    }

    fn open_volume(&self, src: Source, label: String) {
        if self.job.borrow().is_some() {
            return;
        }
        let same = self.session.borrow().as_ref().map_or(false, |s| s.src == src);
        if same {
            self.show_mode(1);
            self.list_current();
            return;
        }
        self.set_info(&self.tf("opening", &[&label]));
        match open::open(&src, true) {
            Ok(o) => match o.fs.root() {
                Ok(root) => {
                    *self.session.borrow_mut() = Some(Session { src, fs: o.fs, stack: vec![root], entries: Vec::new() });
                    self.show_mode(1);
                    self.list_current();
                }
                Err(e) => {
                    self.set_info(self.t("err_root"));
                    nwg::modal_error_message(&self.window, self.t("error"), &e.to_string());
                }
            },
            Err(e) => {
                self.set_info(self.t("err_open_volume"));
                nwg::modal_error_message(&self.window, self.t("err_open_volume"), &e.to_string());
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
                nwg::modal_error_message(&self.window, self.t("err_list"), &e.to_string());
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
                    (String::new(), self.t("kind_folder").to_string())
                }
                Kind::File => {
                    nf += 1;
                    total += e.size;
                    (fmt_size(e.size), if e.compressed { self.t("kind_file_compressed").to_string() } else { self.t("kind_file").to_string() })
                }
                Kind::Symlink => (String::new(), self.tf("kind_link", &[&sess.fs.read_link(e).unwrap_or_default()])),
                Kind::Other => (String::new(), self.t("kind_special").to_string()),
            };
            let date = fmt_time(e.mtime);
            let cols = [e.name.as_str(), size.as_str(), date.as_str(), kind.as_str()];
            self.list.insert_items_row(Some(i as i32), &cols);
        }
        let path = path_string(&sess.stack);
        sess.entries = entries;
        let summary = sess.fs.summary();
        drop(guard);
        self.path_box.set_text(&format!("{}      ·      {}", path, self.tf("folder_stats", &[&nd.to_string(), &nf.to_string(), &fmt_size(total)])));
        self.set_info(&summary);
        self.update_toolbar();
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
                let text = self.tf(
                    "file_info",
                    &[
                        &entry.name,
                        &fmt_size(entry.size),
                        &entry.size.to_string(),
                        &fmt_time(entry.mtime),
                        &fmt_time(entry.crtime),
                        &mode_string(entry.mode, '-'),
                        if entry.compressed { self.t("file_compressed_note") } else { "" },
                    ],
                );
                nwg::modal_info_message(&self.window, self.t("file_title"), &text);
            }
            Kind::Symlink => {
                let target = self.session.borrow().as_ref().map(|s| s.fs.read_link(&entry).unwrap_or_default()).unwrap_or_default();
                nwg::modal_info_message(&self.window, self.t("symlink_title"), &format!("{} → {}", entry.name, target));
            }
            Kind::Other => {}
        }
    }

    fn go_up(&self) {
        if self.mode.get() != 1 {
            self.show_mode(0);
            return;
        }
        let popped = match self.session.borrow_mut().as_mut() {
            Some(s) if s.stack.len() > 1 => {
                s.stack.pop();
                true
            }
            _ => false,
        };
        if popped {
            self.list_current();
        } else {
            self.show_mode(0);
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

    fn need_session(&self, title_key: &str) -> Option<(Source, String)> {
        match self.current_source() {
            Some(v) if self.mode.get() == 1 => Some(v),
            _ => {
                nwg::modal_info_message(&self.window, self.t(title_key), self.t("need_volume"));
                None
            }
        }
    }

    fn copy_selected(&self) {
        let (src, path) = match self.need_session("copy_title") {
            Some(v) => v,
            None => return,
        };
        let rows = self.list.selected_items();
        if rows.is_empty() {
            nwg::modal_info_message(&self.window, self.t("copy_title"), self.t("select_items"));
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
        let (src, path) = match self.need_session("copy_title") {
            Some(v) => v,
            None => return,
        };
        if let Some(dest) = self.choose_folder() {
            self.start_job(src, path, None, Some(dest));
        }
    }

    /// Copia todo o conteúdo do volume seleccionado para outro disco, depois de confirmar
    /// que há espaço suficiente no destino.
    fn copy_disk(&self) {
        let (src, title, mut used) = match self.selected_volume() {
            Some(v) => v,
            None => return,
        };
        if self.job.borrow().is_some() {
            return;
        }
        if used.is_none() {
            if let Ok(o) = open::open(&src, true) {
                used = o.fs.used();
            }
        }
        let dest = match self.choose_folder() {
            Some(d) => d,
            None => return,
        };
        let free = disk_free_space(&dest);
        let dest_disp = dest.to_string_lossy().to_string();
        match (used, free) {
            (Some(need), Some(free)) if free < need => {
                nwg::modal_error_message(&self.window, self.t("copy_disk_title"), &self.tf("space_insufficient", &[&fmt_size(need), &dest_disp, &fmt_size(free)]));
                return;
            }
            (Some(need), Some(free)) => {
                let p = nwg::MessageParams {
                    title: self.t("copy_disk_title"),
                    content: &self.tf("copy_disk_confirm", &[&title, &fmt_size(need), &dest_disp, &fmt_size(free)]),
                    buttons: nwg::MessageButtons::YesNo,
                    icons: nwg::MessageIcons::Question,
                };
                if nwg::modal_message(&self.window, &p) != nwg::MessageChoice::Yes {
                    return;
                }
            }
            _ => {
                let p = nwg::MessageParams { title: self.t("copy_disk_title"), content: self.t("space_unknown"), buttons: nwg::MessageButtons::YesNo, icons: nwg::MessageIcons::Warning };
                if nwg::modal_message(&self.window, &p) != nwg::MessageChoice::Yes {
                    return;
                }
            }
        }
        let target = dest.join(folder_name_from(&title));
        self.start_job(src, "/".to_string(), None, Some(target));
    }

    fn verify(&self) {
        if self.mode.get() == 0 {
            if let Some((src, _, _)) = self.selected_volume() {
                self.start_job(src, "/".to_string(), None, None);
            }
            return;
        }
        let (src, path) = match self.need_session("verify_title") {
            Some(v) => v,
            None => return,
        };
        self.start_job(src, path, None, None);
    }

    // ---- montagem (várias unidades ao mesmo tempo) ----------------------------------------

    fn mount_letter(&self, src: &Source) -> Option<char> {
        self.mounts.borrow().iter().find(|m| m.src == *src).map(|m| m.mount.letter)
    }

    fn unmount_source(&self, src: &Source) -> Option<char> {
        let pos = self.mounts.borrow().iter().position(|m| m.src == *src)?;
        let entry = self.mounts.borrow_mut().remove(pos);
        let letter = entry.mount.letter;
        entry.mount.unmount();
        entry.svc.stop();
        self.set_tile_mounted(src, None);
        Some(letter)
    }

    fn unmount_all(&self) {
        let all: Vec<MountEntry> = self.mounts.borrow_mut().drain(..).collect();
        for m in all {
            m.mount.unmount();
            m.svc.stop();
        }
    }

    fn set_tile_mounted(&self, src: &Source, letter: Option<char>) {
        let mut st = self.tile_state.borrow_mut();
        if let Some((si, ti)) = st.find(src) {
            st.sections[si].tiles[ti].mounted = letter;
        }
        drop(st);
        self.invalidate_tiles();
    }

    fn toggle_mount(&self) {
        if self.job.borrow().is_some() {
            return;
        }
        let src = match self.context_source() {
            Some(s) => s,
            None => {
                nwg::modal_info_message(&self.window, self.t("mount_title"), self.t("select_to_mount"));
                return;
            }
        };
        if self.mount_letter(&src).is_some() {
            if let Some(l) = self.unmount_source(&src) {
                self.set_info(&self.tf("unmounted_status", &[&l.to_string()]));
            }
        } else {
            self.mount_source(src);
        }
        self.update_toolbar();
    }

    fn mount_source(&self, src: Source) {
        if let Err(e) = dokan::available() {
            nwg::modal_error_message(&self.window, self.t("dokan_needed_title"), &self.tf("dokan_needed_text", &[&e]));
            return;
        }
        let letter = match dokan::free_letters().last() {
            Some(l) => *l,
            None => {
                nwg::modal_error_message(&self.window, self.t("mount_title"), self.t("no_free_letter"));
                return;
            }
        };
        self.set_info(&self.tf("mounting", &[&letter.to_string()]));
        let svc = match FsService::start(src.clone(), 0) {
            Ok(s) => Arc::new(s),
            Err(e) => {
                nwg::modal_error_message(&self.window, self.t("mount_title"), &e.to_string());
                return;
            }
        };
        match dokan::mount(svc.clone(), letter, self.admin) {
            Ok(m) => {
                self.mounts.borrow_mut().push(MountEntry { mount: m, svc, src: src.clone() });
                self.set_tile_mounted(&src, Some(letter));
                self.set_info(&self.tf("mounted_status", &[&letter.to_string()]));
                let _ = std::process::Command::new("explorer.exe").arg(format!("{}:\\", letter)).spawn();
            }
            Err(e) => {
                svc.stop();
                self.set_info(self.t("mount_failed"));
                nwg::modal_error_message(&self.window, self.t("mount_failed_title"), &e);
            }
        }
    }

    // ---- páginas de texto ------------------------------------------------------------------

    fn about(&self) {
        let text = self.tf(
            "about_text",
            &[VERSION, i18n::AUTHOR, i18n::COMPANY, i18n::COMPANY_NIF, i18n::PHONE_DISPLAY, i18n::EMAIL, i18n::SITE, i18n::YOUTUBE, i18n::GITHUB_USER, i18n::REPO, i18n::PHONE_LOCAL],
        );
        self.show_text_page(&format!("{}\r\n\r\n{}", self.t("about_title"), text));
    }

    fn donate(&self) {
        let text = self.tf("donate_text", &[i18n::EMAIL, i18n::PHONE_LOCAL, i18n::PHONE_DISPLAY, i18n::YOUTUBE, i18n::GITHUB_USER]);
        self.show_text_page(&format!("{}\r\n\r\n{}", self.t("donate_title"), text));
    }

    // ---- textos, tema e disposição ------------------------------------------------------

    fn apply_texts(&self) {
        self.window.set_text(&format!("{} {} · {}", i18n::APP_NAME, VERSION, self.t("made_in_angola")));
        self.btn_home.set_text(self.t("btn_home"));
        self.btn_image.set_text(self.t("btn_image"));
        self.btn_open.set_text(self.t("btn_open_here"));
        self.btn_copy_disk.set_text(self.t("btn_copy_disk"));
        self.btn_up.set_text(self.t("btn_up"));
        self.btn_copy_sel.set_text(self.t("btn_copy_sel"));
        self.btn_copy_all.set_text(self.t("btn_copy_all"));
        self.btn_verify.set_text(self.t("btn_verify"));
        self.btn_cancel.set_text(self.t("btn_cancel"));
        self.btn_about.set_text(self.t("btn_about"));
        self.btn_donate.set_text(self.t("btn_donate"));
        self.btn_whatsapp.set_text(self.t("btn_whatsapp"));
        self.btn_email.set_text(self.t("btn_email"));
        self.btn_site.set_text(self.t("btn_site"));
        self.btn_youtube.set_text(self.t("btn_youtube"));
        self.btn_github.set_text(self.t("btn_github"));
        self.btn_paypal.set_text(self.t("btn_paypal"));
        self.lbl_lang.set_text(self.t("lbl_lang"));
        self.lbl_theme.set_text(self.t("lbl_theme"));
        self.footer.set_text(&format!("{} · {} · {} · NIF {}", self.t("made_in_angola"), i18n::AUTHOR, i18n::COMPANY, i18n::COMPANY_NIF));
        let themes: Vec<String> = Theme::ALL.iter().map(|t| self.t(match t { Theme::System => "theme_system", Theme::Light => "theme_light", Theme::Dark => "theme_dark" }).to_string()).collect();
        let sel = Theme::ALL.iter().position(|t| *t == self.theme.get());
        self.combo_theme.set_collection(themes);
        self.combo_theme.set_selection(sel);
        for (i, key) in ["col_name", "col_size", "col_modified", "col_type"].iter().enumerate() {
            self.list.update_column(i, nwg::InsertListViewColumn { index: Some(i as i32), fmt: None, width: None, text: Some(self.t(key).to_string()) });
        }
        self.update_toolbar();
    }

    fn is_dark(&self) -> bool {
        match self.theme.get() {
            Theme::Dark => true,
            Theme::Light => false,
            Theme::System => system_dark(),
        }
    }

    fn apply_theme(&self) {
        use winapi::um::dwmapi::DwmSetWindowAttribute;
        use winapi::um::uxtheme::SetWindowTheme;
        let dark = self.is_dark();
        self.dark.set(dark);
        self.tile_state.borrow_mut().dark = dark;
        let theme_name = wide(if dark { "DarkMode_Explorer" } else { "Explorer" });
        let combo_theme = wide(if dark { "DarkMode_CFD" } else { "CFD" });
        unsafe {
            if let Some(h) = self.window.handle.hwnd() {
                let v: i32 = dark as i32;
                DwmSetWindowAttribute(h, 20, &v as *const i32 as *const winapi::ctypes::c_void, 4);
            }
            let handles = [
                self.list.handle.hwnd(),
                self.path_box.handle.hwnd(),
                self.page.handle.hwnd(),
                self.btn_home.handle.hwnd(),
                self.btn_image.handle.hwnd(),
                self.btn_open.handle.hwnd(),
                self.btn_mount.handle.hwnd(),
                self.btn_copy_disk.handle.hwnd(),
                self.btn_up.handle.hwnd(),
                self.btn_copy_sel.handle.hwnd(),
                self.btn_copy_all.handle.hwnd(),
                self.btn_verify.handle.hwnd(),
                self.btn_cancel.handle.hwnd(),
                self.btn_about.handle.hwnd(),
                self.btn_donate.handle.hwnd(),
                self.btn_whatsapp.handle.hwnd(),
                self.btn_email.handle.hwnd(),
                self.btn_site.handle.hwnd(),
                self.btn_youtube.handle.hwnd(),
                self.btn_github.handle.hwnd(),
                self.btn_paypal.handle.hwnd(),
            ];
            for h in handles.into_iter().flatten() {
                SetWindowTheme(h, theme_name.as_ptr(), std::ptr::null());
            }
            for h in [self.combo_lang.handle.hwnd(), self.combo_theme.handle.hwnd()].into_iter().flatten() {
                SetWindowTheme(h, combo_theme.as_ptr(), std::ptr::null());
            }
            if let Some(h) = self.list.handle.hwnd() {
                let (bg, fg) = if dark { (rgb(32, 32, 32), rgb(240, 240, 240)) } else { (rgb(255, 255, 255), rgb(0, 0, 0)) };
                winuser::SendMessageW(h, winapi::um::commctrl::LVM_SETBKCOLOR, 0, bg as LPARAM);
                winuser::SendMessageW(h, winapi::um::commctrl::LVM_SETTEXTBKCOLOR, 0, bg as LPARAM);
                winuser::SendMessageW(h, winapi::um::commctrl::LVM_SETTEXTCOLOR, 0, fg as LPARAM);
            }
            if let Some(h) = self.window.handle.hwnd() {
                winuser::RedrawWindow(h, std::ptr::null(), std::ptr::null_mut(), winuser::RDW_INVALIDATE | winuser::RDW_ERASE | winuser::RDW_ALLCHILDREN | winuser::RDW_FRAME);
            }
        }
        self.invalidate_tiles();
    }

    /// Largura necessária para o texto de um botão (com a fonte dele) mais margem.
    fn text_width(b: &nwg::Button, min: i32) -> i32 {
        let hwnd = match b.handle.hwnd() {
            Some(h) => h,
            None => return min,
        };
        let text = b.text();
        if text.is_empty() {
            return min;
        }
        let w: Vec<u16> = text.encode_utf16().collect();
        unsafe {
            let dc = winuser::GetDC(hwnd);
            let font = winuser::SendMessageW(hwnd, winuser::WM_GETFONT, 0, 0) as HFONT;
            let old = wingdi::SelectObject(dc, font as HGDIOBJ);
            let mut r: RECT = std::mem::zeroed();
            winuser::DrawTextW(dc, w.as_ptr(), w.len() as i32, &mut r, winuser::DT_CALCRECT | winuser::DT_SINGLELINE | winuser::DT_NOPREFIX);
            wingdi::SelectObject(dc, old);
            winuser::ReleaseDC(hwnd, dc);
            (r.right - r.left + 26).max(min)
        }
    }

    fn layout(&self) {
        let (w, h) = self.window.size();
        let (w, h) = (w as i32, h as i32);
        let top = 8;
        let bh = 30;
        // botões da esquerda (só os visíveis)
        let mut x = 8;
        let left: [&nwg::Button; 10] = [
            &self.btn_home,
            &self.btn_image,
            &self.btn_open,
            &self.btn_mount,
            &self.btn_copy_disk,
            &self.btn_up,
            &self.btn_copy_sel,
            &self.btn_copy_all,
            &self.btn_verify,
            &self.btn_cancel,
        ];
        for b in left {
            if !b.visible() {
                continue;
            }
            let bw = Self::text_width(b, 70);
            b.set_position(x, top);
            b.set_size(bw as u32, bh as u32);
            x += bw + 6;
        }
        // botões da direita
        let mut rx = w - 8;
        for b in [&self.btn_donate, &self.btn_about] {
            let bw = Self::text_width(b, 70);
            rx -= bw;
            b.set_position(rx, top);
            b.set_size(bw as u32, bh as u32);
            rx -= 6;
        }
        // informação e progresso entre os dois grupos
        if self.progress.visible() {
            rx -= 200;
            self.progress.set_position(rx, top + 5);
            self.progress.set_size(190, 20);
            rx -= 6;
        }
        self.info.set_position(x + 8, top + 6);
        self.info.set_size((rx - x - 16).max(40) as u32, 20);

        let y2 = top + bh + 10;
        let bottom_h = 62;
        let main_h = (h - y2 - bottom_h - 6).max(120);
        match self.mode.get() {
            0 => {
                self.tiles.set_position(8, y2);
                self.tiles.set_size((w - 16).max(100) as u32, main_h as u32);
            }
            1 => {
                self.path_box.set_position(8, y2);
                self.path_box.set_size((w - 16).max(100) as u32, 26);
                self.list.set_position(8, y2 + 34);
                self.list.set_size((w - 16).max(100) as u32, (main_h - 34).max(60) as u32);
            }
            _ => {
                self.page.set_position(8, y2);
                self.page.set_size((w - 16).max(100) as u32, main_h as u32);
            }
        }
        // rodapé: contactos + idioma/tema, e a linha da empresa
        let ly = h - bottom_h + 4;
        let mut lx = 8;
        for b in [&self.btn_whatsapp, &self.btn_email, &self.btn_site, &self.btn_youtube, &self.btn_github, &self.btn_paypal] {
            let bw = Self::text_width(b, 70);
            b.set_position(lx, ly);
            b.set_size(bw as u32, 26);
            lx += bw + 6;
        }
        let cx = w - 8 - 110;
        self.combo_theme.set_position(cx, ly);
        self.combo_theme.set_size(110, 26);
        self.lbl_theme.set_position(cx - 62, ly + 4);
        self.lbl_theme.set_size(58, 22);
        let lx2 = cx - 62 - 12 - 170;
        self.combo_lang.set_position(lx2, ly);
        self.combo_lang.set_size(170, 26);
        self.lbl_lang.set_position(lx2 - 66, ly + 4);
        self.lbl_lang.set_size(62, 22);
        self.footer.set_position(8, h - 24);
        self.footer.set_size((w - 16).max(100) as u32, 20);
    }

    // ---- trabalhos de cópia / verificação ---------------------------------------------------

    fn start_job(&self, src: Source, dir_path: String, names: Option<Vec<String>>, dest: Option<PathBuf>) {
        if self.job.borrow().is_some() {
            return;
        }
        let progress = Arc::new(Mutex::new(Progress::default()));
        let cancel = Arc::new(AtomicBool::new(false));
        let sender = self.notice.sender();
        let (p2, c2) = (progress.clone(), cancel.clone());
        let is_copy = dest.is_some();
        let lang = self.lang_now();
        thread::spawn(move || {
            let r = catch_unwind(AssertUnwindSafe(|| run_job(lang, &src, &dir_path, names.as_deref(), dest.as_deref(), &p2, &c2, &sender)));
            let mut p = p2.lock().unwrap();
            match r {
                Ok(Ok(())) => {}
                Ok(Err(e)) => p.failed = Some(e),
                Err(_) => p.failed = Some(tr(lang, "internal_error").to_string()),
            }
            p.done = true;
            drop(p);
            sender.notice();
        });
        *self.job.borrow_mut() = Some(Job { progress, cancel });
        self.set_info(self.t(if is_copy { "copying" } else { "verifying" }));
        self.update_toolbar();
    }

    fn cancel_job(&self) {
        if let Some(j) = self.job.borrow().as_ref() {
            j.cancel.store(true, Ordering::Relaxed);
            self.set_info(self.t("cancelling"));
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
            self.set_info(&self.tf("progress", &[&files.to_string(), &fmt_size(bytes), &current]));
            return;
        }
        *self.job.borrow_mut() = None;
        self.update_toolbar();
        match failed {
            Some(f) => {
                self.set_info(self.t("failed"));
                nwg::modal_error_message(&self.window, self.t("failure"), &f);
            }
            None => {
                self.set_info(&summary);
                let mut text = summary;
                if !errors.is_empty() {
                    text.push_str(self.t("first_errors"));
                    for e in errors.iter().take(12) {
                        text.push_str(e);
                        text.push('\n');
                    }
                    if !log_path.is_empty() {
                        text.push_str(&self.tf("log_full", &[&log_path]));
                    }
                    nwg::modal_error_message(&self.window, self.t("done_errors"), &text);
                } else {
                    nwg::modal_info_message(&self.window, self.t("done"), &text);
                }
            }
        }
        if self.scan_pending.get() {
            self.request_scan(false);
        }
    }
}

fn run_job(
    lang: Lang,
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
        std::fs::create_dir_all(&dest_long).map_err(|e| fmt(tr(lang, "dest_create_fail"), &[&e.to_string()]))?;
    }
    let log_path = dest_long.join("sudomake-partition-log.txt");
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
    let summary = fmt(
        tr(lang, "summary"),
        &[
            if cancelled { tr(lang, "cancelled_prefix") } else { "" },
            tr(lang, if verify { "summary_verify" } else { "summary_copy" }),
            &st.files.to_string(),
            &fmt_size(st.bytes),
            &st.dirs.to_string(),
            &format!("{:.1}", secs),
            &fmt_size((st.bytes as f64 / secs) as u64),
            &st.skipped.to_string(),
            &st.symlinks.to_string(),
            &st.errors.to_string(),
        ],
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

fn build(lang: Lang, theme: Theme) -> Result<Rc<App>, nwg::NwgError> {
    let mut app = App::default();
    app.admin = is_admin();
    app.lang.set(LangCell(Some(lang)));
    app.theme.set(theme);

    let _ = nwg::Font::set_global_family("Segoe UI");
    nwg::Font::builder().family("Segoe UI").size(14).build(&mut app.small_font)?;
    nwg::Window::builder()
        .size((1400, 800))
        .position((80, 40))
        .title(i18n::APP_NAME)
        .flags(nwg::WindowFlags::MAIN_WINDOW | nwg::WindowFlags::VISIBLE)
        .build(&mut app.window)?;
    if let Some(h) = app.window.handle.hwnd() {
        unsafe {
            let small = app_icon(16);
            let big = app_icon(32);
            if !small.is_null() {
                winuser::SendMessageW(h, winuser::WM_SETICON, winuser::ICON_SMALL as usize, small as LPARAM);
            }
            if !big.is_null() {
                winuser::SendMessageW(h, winuser::WM_SETICON, winuser::ICON_BIG as usize, big as LPARAM);
            }
        }
    }

    for b in [
        &mut app.btn_home,
        &mut app.btn_image,
        &mut app.btn_open,
        &mut app.btn_mount,
        &mut app.btn_copy_disk,
        &mut app.btn_up,
        &mut app.btn_copy_sel,
        &mut app.btn_copy_all,
        &mut app.btn_verify,
        &mut app.btn_cancel,
        &mut app.btn_about,
        &mut app.btn_donate,
    ] {
        nwg::Button::builder().text("").parent(&app.window).build(b)?;
    }
    for b in [&mut app.btn_whatsapp, &mut app.btn_email, &mut app.btn_site, &mut app.btn_youtube, &mut app.btn_github, &mut app.btn_paypal] {
        nwg::Button::builder().text("").parent(&app.window).font(Some(&app.small_font)).build(b)?;
    }
    nwg::Label::builder().parent(&app.window).text("").font(Some(&app.small_font)).build(&mut app.info)?;
    nwg::ProgressBar::builder().parent(&app.window).flags(nwg::ProgressBarFlags::VISIBLE | nwg::ProgressBarFlags::MARQUEE).build(&mut app.progress)?;
    app.progress.set_visible(false);

    nwg::TextInput::builder().parent(&app.window).readonly(true).text("").build(&mut app.path_box)?;
    nwg::ListView::builder()
        .parent(&app.window)
        .ex_flags(nwg::ListViewExFlags::FULL_ROW_SELECT | nwg::ListViewExFlags::GRID)
        .build(&mut app.list)?;
    app.list.set_list_style(nwg::ListViewStyle::Detailed);
    app.list.set_headers_enabled(true);
    for (i, (key, width, right)) in [("col_name", 460, false), ("col_size", 110, true), ("col_modified", 150, false), ("col_type", 300, false)].iter().enumerate() {
        app.list.insert_column(nwg::InsertListViewColumn {
            index: Some(i as i32),
            fmt: Some(if *right { nwg::ListViewColumnFlags::RIGHT } else { nwg::ListViewColumnFlags::LEFT }),
            width: Some(*width),
            text: Some(tr(lang, key).to_string()),
        });
    }
    nwg::Frame::builder().parent(&app.window).flags(nwg::FrameFlags::VISIBLE).build(&mut app.tiles)?;
    nwg::TextBox::builder()
        .parent(&app.window)
        .text("")
        .readonly(true)
        .flags(nwg::TextBoxFlags::VISIBLE | nwg::TextBoxFlags::VSCROLL | nwg::TextBoxFlags::AUTOVSCROLL | nwg::TextBoxFlags::TAB_STOP)
        .build(&mut app.page)?;

    nwg::Label::builder().parent(&app.window).text("").font(Some(&app.small_font)).build(&mut app.footer)?;
    nwg::Label::builder().parent(&app.window).text("").font(Some(&app.small_font)).build(&mut app.lbl_lang)?;
    nwg::Label::builder().parent(&app.window).text("").font(Some(&app.small_font)).build(&mut app.lbl_theme)?;
    let langs: Vec<String> = Lang::ALL.iter().map(|l| l.name().to_string()).collect();
    let lang_idx = Lang::ALL.iter().position(|l| *l == lang);
    nwg::ComboBox::builder().parent(&app.window).collection(langs).selected_index(lang_idx).font(Some(&app.small_font)).build(&mut app.combo_lang)?;
    nwg::ComboBox::builder().parent(&app.window).collection(vec![String::new(), String::new(), String::new()]).selected_index(Some(0)).font(Some(&app.small_font)).build(&mut app.combo_theme)?;
    nwg::Notice::builder().parent(&app.window).build(&mut app.notice)?;
    nwg::FileDialog::builder().title(tr(lang, "folder_dialog")).action(nwg::FileDialogAction::OpenDirectory).build(&mut app.folder_dialog)?;
    nwg::FileDialog::builder().title(tr(lang, "image_dialog")).action(nwg::FileDialogAction::Open).filters(tr(lang, "image_filter")).build(&mut app.image_dialog)?;

    let app = Rc::new(app);
    app.apply_texts();
    app.show_mode(0);

    // painel de azulejos: pintura e rato
    let st = app.tile_state.clone();
    let a_tiles = app.clone();
    let _tiles_handler = nwg::bind_raw_event_handler(&app.tiles.handle, 0x10010, move |hwnd, msg, w, l| {
        match msg {
            winuser::WM_PAINT => {
                let mut s = st.borrow_mut();
                unsafe { paint_tiles(hwnd, &mut s) };
                Some(0)
            }
            winuser::WM_ERASEBKGND => Some(1),
            winuser::WM_LBUTTONDOWN | winuser::WM_LBUTTONDBLCLK => {
                let x = (l & 0xFFFF) as i16 as i32;
                let y = ((l >> 16) & 0xFFFF) as i16 as i32;
                let hit = hit_test(&st.borrow(), x, y);
                st.borrow_mut().selected = hit;
                unsafe { winuser::InvalidateRect(hwnd, std::ptr::null(), 0) };
                a_tiles.tile_selected();
                if msg == winuser::WM_LBUTTONDBLCLK && hit.is_some() {
                    a_tiles.open_selected();
                }
                Some(0)
            }
            winuser::WM_MOUSEWHEEL => {
                let delta = ((w >> 16) & 0xFFFF) as i16 as i32;
                let mut s = st.borrow_mut();
                let mut client: RECT = unsafe { std::mem::zeroed() };
                unsafe { winuser::GetClientRect(hwnd, &mut client) };
                let max = (s.content_h - (client.bottom - client.top)).max(0);
                s.scroll = (s.scroll - delta / 120 * 60).clamp(0, max);
                unsafe { winuser::InvalidateRect(hwnd, std::ptr::null(), 0) };
                Some(0)
            }
            winuser::WM_SIZE => {
                unsafe { winuser::InvalidateRect(hwnd, std::ptr::null(), 0) };
                None
            }
            _ => None,
        }
    });

    // janela: fundo do tema, temporizadores e avisos de dispositivos ligados/removidos
    let dark_flag = app.dark.clone();
    let a_win = app.clone();
    let brush_dark: HBRUSH = unsafe { wingdi::CreateSolidBrush(rgb(32, 32, 32)) };
    let brush_dark_edit: HBRUSH = unsafe { wingdi::CreateSolidBrush(rgb(43, 43, 43)) };
    let brush_light: HBRUSH = unsafe { wingdi::CreateSolidBrush(rgb(255, 255, 255)) };
    let _win_handler = nwg::bind_raw_event_handler(&app.window.handle, 0x10020, move |hwnd, msg, w, l| {
        match msg {
            winuser::WM_TIMER => {
                if w == TIMER_DEVICE {
                    unsafe { winuser::KillTimer(hwnd, TIMER_DEVICE) };
                    a_win.request_scan(false);
                } else if w == TIMER_PERIODIC && a_win.mode.get() == 0 {
                    a_win.request_scan(false);
                }
                return Some(0);
            }
            winuser::WM_DEVICECHANGE => {
                // 0x8000 chegada, 0x8004 remoção, 0x0007 nós de dispositivos alterados
                if w == 0x8000 || w == 0x8004 || w == 0x0007 {
                    unsafe { winuser::SetTimer(hwnd, TIMER_DEVICE, DEVICE_DEBOUNCE_MS, None) };
                }
                return None;
            }
            _ => {}
        }
        let dark = dark_flag.get();
        match msg {
            winuser::WM_ERASEBKGND => {
                let mut r: RECT = unsafe { std::mem::zeroed() };
                unsafe {
                    winuser::GetClientRect(hwnd, &mut r);
                    winuser::FillRect(w as HDC, &r, if dark { brush_dark } else { brush_light });
                }
                Some(1)
            }
            winuser::WM_CTLCOLORSTATIC | winuser::WM_CTLCOLOREDIT | winuser::WM_CTLCOLORLISTBOX | winuser::WM_CTLCOLORBTN => {
                let dc = w as HDC;
                let is_edit = msg == winuser::WM_CTLCOLOREDIT || msg == winuser::WM_CTLCOLORLISTBOX || {
                    let mut cls = [0u16; 32];
                    let n = unsafe { winuser::GetClassNameW(l as HWND, cls.as_mut_ptr(), 32) };
                    String::from_utf16_lossy(&cls[..n.max(0) as usize]).eq_ignore_ascii_case("Edit")
                };
                unsafe {
                    if dark {
                        wingdi::SetTextColor(dc, rgb(240, 240, 240));
                        wingdi::SetBkColor(dc, if is_edit { rgb(43, 43, 43) } else { rgb(32, 32, 32) });
                    } else {
                        wingdi::SetTextColor(dc, rgb(26, 26, 26));
                        wingdi::SetBkColor(dc, rgb(255, 255, 255));
                    }
                }
                Some((if dark { if is_edit { brush_dark_edit } else { brush_dark } } else { brush_light }) as LRESULT)
            }
            _ => None,
        }
    });

    // etiquetas: a área não-cliente (centragem vertical do nwg) pintada com a cor do tema
    let mut label_handlers = Vec::new();
    for (i, lbl) in [&app.info, &app.footer, &app.lbl_lang, &app.lbl_theme].into_iter().enumerate() {
        let dark_flag = app.dark.clone();
        if let Ok(h) = nwg::bind_raw_event_handler(&lbl.handle, 0x10030 + i, move |hwnd, msg, _w, _l| {
            if msg == winuser::WM_NCPAINT {
                unsafe {
                    let dc = winuser::GetWindowDC(hwnd);
                    let mut r: RECT = std::mem::zeroed();
                    winuser::GetWindowRect(hwnd, &mut r);
                    let r = RECT { left: 0, top: 0, right: r.right - r.left, bottom: r.bottom - r.top };
                    winuser::FillRect(dc, &r, if dark_flag.get() { brush_dark } else { brush_light });
                    winuser::ReleaseDC(hwnd, dc);
                }
                return Some(0);
            }
            None
        }) {
            label_handlers.push(h);
        }
    }

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
                        let p = nwg::MessageParams { title: a.t("exit_title"), content: a.t("exit_text"), buttons: nwg::MessageButtons::YesNo, icons: nwg::MessageIcons::Question };
                        if nwg::modal_message(&a.window, &p) != nwg::MessageChoice::Yes {
                            return;
                        }
                        a.cancel_job();
                    }
                    a.unmount_all();
                    nwg::stop_thread_dispatch();
                }
            }
            E::OnButtonClick => {
                if handle == a.btn_home.handle {
                    if a.mode.get() == 0 {
                        a.request_scan(true);
                    } else {
                        a.show_mode(0);
                    }
                } else if handle == a.btn_image.handle {
                    a.open_image();
                } else if handle == a.btn_open.handle {
                    a.open_selected();
                } else if handle == a.btn_mount.handle {
                    a.toggle_mount();
                } else if handle == a.btn_copy_disk.handle {
                    a.copy_disk();
                } else if handle == a.btn_up.handle {
                    a.go_up();
                } else if handle == a.btn_copy_sel.handle {
                    a.copy_selected();
                } else if handle == a.btn_copy_all.handle {
                    a.copy_all();
                } else if handle == a.btn_verify.handle {
                    a.verify();
                } else if handle == a.btn_cancel.handle {
                    a.cancel_job();
                } else if handle == a.btn_about.handle {
                    a.about();
                } else if handle == a.btn_donate.handle {
                    a.donate();
                } else if handle == a.btn_whatsapp.handle {
                    open_url(i18n::WHATSAPP_URL);
                } else if handle == a.btn_email.handle {
                    open_url(i18n::EMAIL_URL);
                } else if handle == a.btn_site.handle {
                    open_url(i18n::SITE);
                } else if handle == a.btn_youtube.handle {
                    open_url(i18n::YOUTUBE);
                } else if handle == a.btn_github.handle {
                    open_url(i18n::GITHUB_USER);
                } else if handle == a.btn_paypal.handle {
                    open_url(i18n::PAYPAL_URL);
                }
            }
            E::OnComboxBoxSelection => {
                if handle == a.combo_lang.handle {
                    if let Some(i) = a.combo_lang.selection() {
                        let l = Lang::ALL[i.min(3)];
                        if l != a.lang_now() {
                            a.lang.set(LangCell(Some(l)));
                            save_config(Some(l), a.theme.get());
                            a.apply_texts();
                            a.request_scan(true);
                        }
                    }
                } else if handle == a.combo_theme.handle {
                    if let Some(i) = a.combo_theme.selection() {
                        let t = Theme::ALL[i.min(2)];
                        a.theme.set(t);
                        save_config(Some(a.lang_now()), t);
                        a.apply_theme();
                    }
                }
            }
            E::OnListViewItemActivated => {
                if handle == a.list.handle {
                    if let nwg::EventData::OnListViewItemIndex { row_index, .. } = data {
                        a.activate_row(row_index);
                    }
                }
            }
            E::OnListViewItemChanged | E::OnListViewClick => {
                if handle == a.list.handle && a.mode.get() == 1 {
                    let has = a.job.borrow().is_none() && a.list.selected_count() > 0;
                    if has != a.copy_sel_visible.get() {
                        a.update_toolbar();
                    }
                }
            }
            E::OnNotice => {
                if handle == a.notice.handle {
                    a.job_update();
                    a.scan_update();
                }
            }
            _ => {}
        }
    });
    app.layout();
    app.apply_theme();
    if let Some(h) = app.window.handle.hwnd() {
        unsafe { winuser::SetTimer(h, TIMER_PERIODIC, PERIODIC_MS, None) };
    }
    Ok(app)
}

fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let msg = format!("{}", info);
        let path = std::env::temp_dir().join("sudomake-partition-panic.txt");
        let _ = std::fs::write(&path, &msg);
        nwg::simple_message(i18n::APP_NAME, &format!("Erro interno / internal error:\n{}\n\n{}", msg, path.display()));
    }));
}

fn main() {
    install_panic_hook();
    let elevated_arg = std::env::args().any(|a| a == "--elevado" || a == "--sem-admin");
    if !elevated_arg && !is_admin() && relaunch_as_admin() {
        return;
    }
    unsafe {
        winuser::SetProcessDPIAware();
    }
    if let Err(e) = nwg::init() {
        nwg::simple_message(i18n::APP_NAME, &format!("Falha ao iniciar a interface: {}", e));
        return;
    }
    let (cfg_lang, theme) = load_config();
    let mut lang = cfg_lang.unwrap_or_else(system_lang);
    for a in std::env::args() {
        if let Some(v) = a.strip_prefix("--lang=") {
            if let Some(l) = Lang::from_code(v) {
                lang = l;
            }
        }
    }
    let app = match build(lang, theme) {
        Ok(a) => a,
        Err(e) => {
            nwg::simple_message(i18n::APP_NAME, &format!("Falha ao criar a janela: {}", e));
            return;
        }
    };
    for arg in std::env::args().skip(1) {
        if !arg.starts_with("--") && Path::new(&arg).is_file() {
            app.images.borrow_mut().push(arg);
        }
    }
    app.request_scan(true);
    let quiet = std::env::args().any(|a| a == "--sem-admin");
    if !app.admin && !quiet {
        nwg::modal_info_message(&app.window, app.t("no_admin_title"), app.t("no_admin_text"));
    }
    nwg::dispatch_thread_events();
}
