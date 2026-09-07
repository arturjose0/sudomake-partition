//! Identifica o sistema operacional ou a finalidade de um volume pelo seu conteúdo
//! (Ubuntu 22.04, macOS 12.5, backup do Time Machine, dados...).

use crate::fs::{lookup, read_all, FileSystem, Kind};
use crate::partition::{FsKind, Partition};

/// Resultado da detecção (a tradução fica a cargo de quem mostra).
#[derive(Clone, Debug, PartialEq)]
pub enum Detected {
    /// nome do sistema Linux (PRETTY_NAME do os-release)
    Linux(String),
    /// nome do produto e versão do macOS
    MacOs(String, String),
    /// volume de dados do macOS (nomes dos utilizadores)
    MacData(Vec<String>),
    /// pasta /home do Linux; `system` indica se também há /etc e /usr
    LinuxHome { users: Vec<String>, system: bool },
    LinuxGeneric,
    MacGeneric,
    TimeMachine,
    /// disco de dados com N itens na raiz
    Data(usize),
    Empty,
}

impl Detected {
    /// Nome curto para resumos ("Ubuntu 22.04.4 LTS", "macOS", "Linux"...).
    pub fn short(&self) -> Option<String> {
        match self {
            Detected::Linux(n) => Some(n.clone()),
            Detected::MacOs(n, v) => Some(format!("{} {}", n, v).trim().to_string()),
            Detected::MacData(_) | Detected::MacGeneric => Some("macOS".into()),
            Detected::LinuxHome { .. } | Detected::LinuxGeneric => Some("Linux".into()),
            Detected::TimeMachine => Some("Time Machine".into()),
            Detected::Data(_) | Detected::Empty => None,
        }
    }

    /// Descrição em português (linha de comando).
    pub fn describe_pt(&self) -> String {
        match self {
            Detected::Linux(n) => format!("{} (sistema Linux)", n),
            Detected::MacOs(n, v) => format!("{} {} (sistema)", n, v).trim().to_string(),
            Detected::MacData(u) if u.is_empty() => "macOS: dados do usuário".into(),
            Detected::MacData(u) => format!("macOS: dados do usuário ({})", u.join(", ")),
            Detected::LinuxHome { users, system } => {
                let base = if *system { "Linux (sistema sem os-release)" } else { "Linux: pasta /home" };
                if users.is_empty() { base.to_string() } else { format!("{} ({})", base, users.join(", ")) }
            }
            Detected::LinuxGeneric => "Linux (sistema sem os-release)".into(),
            Detected::MacGeneric => "macOS (sem versão identificada)".into(),
            Detected::TimeMachine => "Backup do Time Machine".into(),
            Detected::Data(n) => format!("disco de dados ({} itens na raiz)", n),
            Detected::Empty => "vazio".into(),
        }
    }
}

fn read_text(fs: &dyn FileSystem, path: &str, max: usize) -> Option<String> {
    let e = lookup(fs, path).ok()?;
    if e.kind != Kind::File || e.size as usize > max {
        return None;
    }
    let mut r = fs.open(&e).ok()?;
    let b = read_all(r.as_mut()).ok()?;
    Some(String::from_utf8_lossy(&b).into_owned())
}

fn exists(fs: &dyn FileSystem, path: &str) -> bool {
    lookup(fs, path).is_ok()
}

fn is_dir(fs: &dyn FileSystem, path: &str) -> bool {
    lookup(fs, path).map(|e| e.kind == Kind::Dir).unwrap_or(false)
}

fn subdirs(fs: &dyn FileSystem, path: &str) -> Vec<String> {
    lookup(fs, path)
        .and_then(|e| fs.read_dir(&e))
        .map(|v| v.into_iter().filter(|e| e.kind == Kind::Dir && !e.name.starts_with('.') && e.name != "Shared").map(|e| e.name).collect())
        .unwrap_or_default()
}

fn os_release_name(text: &str) -> Option<String> {
    for key in ["PRETTY_NAME", "NAME"] {
        for line in text.lines() {
            if let Some(v) = line.strip_prefix(&format!("{}=", key)) {
                let v = v.trim().trim_matches('"').trim_matches('\'');
                if !v.is_empty() {
                    return Some(v.to_string());
                }
            }
        }
    }
    None
}

fn plist_string(text: &str, key: &str) -> Option<String> {
    let k = format!("<key>{}</key>", key);
    let i = text.find(&k)?;
    let rest = &text[i + k.len()..];
    let s = rest.find("<string>")? + 8;
    let e = rest[s..].find("</string>")? + s;
    Some(rest[s..e].trim().to_string())
}

/// Detecta o sistema instalado no volume.
pub fn detect(fs: &dyn FileSystem) -> Detected {
    for p in ["/etc/os-release", "/usr/lib/os-release"] {
        if let Some(t) = read_text(fs, p, 64 * 1024) {
            if let Some(n) = os_release_name(&t) {
                return Detected::Linux(n);
            }
        }
    }
    for p in ["/System/Library/CoreServices/SystemVersion.plist", "/System/Library/CoreServices/ServerVersion.plist"] {
        if let Some(t) = read_text(fs, p, 64 * 1024) {
            let name = plist_string(&t, "ProductName").unwrap_or_else(|| "macOS".into());
            let ver = plist_string(&t, "ProductVersion").unwrap_or_default();
            return Detected::MacOs(name, ver);
        }
    }
    if is_dir(fs, "/Backups.backupdb") {
        return Detected::TimeMachine;
    }
    if is_dir(fs, "/Users") {
        return Detected::MacData(subdirs(fs, "/Users"));
    }
    if is_dir(fs, "/home") {
        return Detected::LinuxHome { users: subdirs(fs, "/home"), system: exists(fs, "/etc") && exists(fs, "/usr") };
    }
    if exists(fs, "/Applications") && exists(fs, "/Library") {
        return Detected::MacGeneric;
    }
    if exists(fs, "/etc") && exists(fs, "/usr") {
        return Detected::LinuxGeneric;
    }
    match fs.root().and_then(|r| fs.read_dir(&r)) {
        Ok(v) if v.is_empty() => Detected::Empty,
        Ok(v) => Detected::Data(v.len()),
        Err(_) => Detected::Data(0),
    }
}

/// Finalidade de uma partição que o programa não lê (Windows, EFI, swap...).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartKind {
    Windows,
    WindowsRecovery,
    Efi,
    Fat,
    ExFat,
    Swap,
    Luks,
    Lvm,
    Xfs,
    Btrfs,
    F2fs,
    CoreStorage,
    HfsClassic,
    Reserved,
    Empty,
    Boot,
    Recovery,
    Unknown,
    /// partição legível (APFS, HFS+, ext)
    Readable,
}

pub fn classify_partition(p: &Partition) -> PartKind {
    let t = p.type_name.to_lowercase();
    match &p.fs {
        FsKind::Ntfs => {
            if t.contains("recovery") || p.len < 2_000_000_000 {
                PartKind::WindowsRecovery
            } else {
                PartKind::Windows
            }
        }
        FsKind::ExFat => PartKind::ExFat,
        FsKind::Fat => {
            if t.contains("efi") {
                PartKind::Efi
            } else {
                PartKind::Fat
            }
        }
        FsKind::Swap => PartKind::Swap,
        FsKind::Luks => PartKind::Luks,
        FsKind::Lvm => PartKind::Lvm,
        FsKind::Xfs => PartKind::Xfs,
        FsKind::Btrfs => PartKind::Btrfs,
        FsKind::F2fs => PartKind::F2fs,
        FsKind::CoreStorage => PartKind::CoreStorage,
        FsKind::Hfs => PartKind::HfsClassic,
        FsKind::Empty => {
            if t.contains("reserved") {
                PartKind::Reserved
            } else {
                PartKind::Empty
            }
        }
        FsKind::Unknown => {
            if t.contains("recovery") {
                PartKind::Recovery
            } else if t.contains("boot") {
                PartKind::Boot
            } else {
                PartKind::Unknown
            }
        }
        _ => PartKind::Readable,
    }
}

impl PartKind {
    /// Chave de tradução (`i18n`) correspondente.
    pub fn key(self) -> &'static str {
        match self {
            PartKind::Windows => "part_windows",
            PartKind::WindowsRecovery => "part_windows_recovery",
            PartKind::Efi => "part_efi",
            PartKind::Fat => "part_fat",
            PartKind::ExFat => "part_exfat",
            PartKind::Swap => "part_swap",
            PartKind::Luks => "part_luks",
            PartKind::Lvm => "part_lvm",
            PartKind::Xfs => "part_xfs",
            PartKind::Btrfs => "part_btrfs",
            PartKind::F2fs => "part_f2fs",
            PartKind::CoreStorage => "part_corestorage",
            PartKind::HfsClassic => "part_hfs_classic",
            PartKind::Reserved => "part_reserved",
            PartKind::Empty => "part_empty",
            PartKind::Boot => "part_boot",
            PartKind::Recovery => "part_recovery",
            PartKind::Unknown => "part_unknown",
            PartKind::Readable => "",
        }
    }

    /// Nome curto para o resumo do disco.
    pub fn short(self) -> Option<&'static str> {
        match self {
            PartKind::Windows | PartKind::WindowsRecovery => Some("Windows"),
            PartKind::Luks | PartKind::Xfs | PartKind::Btrfs | PartKind::F2fs | PartKind::Swap => Some("Linux"),
            PartKind::CoreStorage => Some("macOS"),
            _ => None,
        }
    }
}

/// Descrição em português (linha de comando).
pub fn describe_partition(p: &Partition) -> String {
    crate::i18n::tr(crate::i18n::Lang::Pt, classify_partition(p).key()).to_string()
}
