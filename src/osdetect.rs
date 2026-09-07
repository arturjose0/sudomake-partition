//! Identifica o sistema operacional ou a finalidade de um volume pelo seu conteúdo
//! (Ubuntu 22.04, macOS 12.5, backup do Time Machine, dados...).

use crate::fs::{lookup, read_all, FileSystem, Kind};
use crate::partition::{FsKind, Partition};

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

/// Nome do sistema encontrado no volume, ou uma descrição da sua finalidade.
pub fn detect(fs: &dyn FileSystem) -> String {
    // Linux
    for p in ["/etc/os-release", "/usr/lib/os-release"] {
        if let Some(t) = read_text(fs, p, 64 * 1024) {
            if let Some(n) = os_release_name(&t) {
                return format!("{} (sistema Linux)", n);
            }
        }
    }
    // macOS (volume de sistema, ou volume único em versões antigas)
    for p in ["/System/Library/CoreServices/SystemVersion.plist", "/System/Library/CoreServices/ServerVersion.plist"] {
        if let Some(t) = read_text(fs, p, 64 * 1024) {
            let name = plist_string(&t, "ProductName").unwrap_or_else(|| "macOS".into());
            let ver = plist_string(&t, "ProductVersion").unwrap_or_default();
            return format!("{} {} (sistema)", name, ver).trim().to_string();
        }
    }
    if is_dir(fs, "/Backups.backupdb") {
        return "Backup do Time Machine".into();
    }
    if is_dir(fs, "/Users") {
        let users: Vec<String> = fs
            .read_dir(&lookup(fs, "/Users").unwrap())
            .map(|v| v.into_iter().filter(|e| e.kind == Kind::Dir && !e.name.starts_with('.') && e.name != "Shared").map(|e| e.name).collect())
            .unwrap_or_default();
        return if users.is_empty() {
            "macOS: dados do usuário".into()
        } else {
            format!("macOS: dados do usuário ({})", users.join(", "))
        };
    }
    if is_dir(fs, "/home") {
        let users: Vec<String> = fs
            .read_dir(&lookup(fs, "/home").unwrap())
            .map(|v| v.into_iter().filter(|e| e.kind == Kind::Dir && !e.name.starts_with('.')).map(|e| e.name).collect())
            .unwrap_or_default();
        let base = if exists(fs, "/etc") && exists(fs, "/usr") { "Linux (sistema sem os-release)" } else { "Linux: pasta /home" };
        return if users.is_empty() { base.to_string() } else { format!("{} ({})", base, users.join(", ")) };
    }
    if exists(fs, "/Applications") && exists(fs, "/Library") {
        return "macOS (sem versão identificada)".into();
    }
    if exists(fs, "/etc") && exists(fs, "/usr") {
        return "Linux (sistema sem os-release)".into();
    }
    match fs.root().and_then(|r| fs.read_dir(&r)) {
        Ok(v) if v.is_empty() => "vazio".into(),
        Ok(v) => format!("disco de dados ({} itens na raiz)", v.len()),
        Err(_) => "disco de dados".into(),
    }
}

/// Descrição curta de partições que o macread não lê (Windows, EFI, swap...).
pub fn describe_partition(p: &Partition) -> String {
    let t = p.type_name.to_lowercase();
    match &p.fs {
        FsKind::Ntfs => {
            if t.contains("recovery") || p.len < 2_000_000_000 {
                "Windows: recuperação/sistema".into()
            } else {
                "Windows (NTFS)".into()
            }
        }
        FsKind::ExFat => "dados (exFAT)".into(),
        FsKind::Fat => {
            if t.contains("efi") {
                "EFI (inicialização)".into()
            } else {
                "dados (FAT)".into()
            }
        }
        FsKind::Swap => "Linux swap".into(),
        FsKind::Luks => "Linux criptografado (LUKS)".into(),
        FsKind::Lvm => "LVM (os volumes lógicos aparecem em separado)".into(),
        FsKind::Xfs => "Linux (XFS, ainda não legível)".into(),
        FsKind::Btrfs => "Linux (Btrfs, ainda não legível)".into(),
        FsKind::F2fs => "Linux (F2FS, ainda não legível)".into(),
        FsKind::CoreStorage => "macOS Core Storage (FileVault antigo/Fusion, não legível)".into(),
        FsKind::Hfs => "HFS clássico (não legível)".into(),
        FsKind::Empty => {
            if t.contains("reserved") {
                "reservado (Windows)".into()
            } else {
                "vazio".into()
            }
        }
        FsKind::Unknown => {
            if t.contains("recovery") {
                "recuperação do Windows".into()
            } else if t.contains("boot") {
                "inicialização".into()
            } else {
                "desconhecido".into()
            }
        }
        _ => String::new(),
    }
}
