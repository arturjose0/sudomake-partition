//! Abertura de origens (disco físico, imagem, DMG), escolha de partição/volume e do leitor adequado.
//! Compartilhado pela linha de comando e pela interface gráfica.

use std::io;
use std::path::Path;
use std::rc::Rc;

use crate::apfs;
use crate::device::{BlockDevice, CachedDevice, FileDevice, MappedDevice, SubDevice};
use crate::ext4;
use crate::fs::FileSystem;
use crate::hfsplus;
use crate::partition::{self, FsKind, Layout, Partition};
use crate::util::*;

/// Identificação serializável de um volume (pode ser enviada a outra thread).
#[derive(Clone, Debug, PartialEq)]
pub struct Source {
    /// "disco:N", "\\.\PhysicalDriveN" ou caminho de imagem/DMG
    pub spec: String,
    /// índice da partição (1-based) ou None para a primeira suportada
    pub part: Option<usize>,
    /// índice do volume APFS (1-based) ou None para o padrão
    pub vol: Option<usize>,
    /// tenta abrir volumes marcados como criptografados
    pub force: bool,
}

pub fn map_dev_err(e: io::Error) -> io::Error {
    if e.kind() == io::ErrorKind::PermissionDenied {
        io::Error::new(
            io::ErrorKind::PermissionDenied,
            "acesso negado ao disco físico: abra o Prompt de Comando/PowerShell como Administrador",
        )
    } else {
        e
    }
}

pub fn open_physical(n: u32) -> io::Result<Rc<dyn BlockDevice>> {
    FileDevice::open_physical(n).map(|d| Rc::new(d) as Rc<dyn BlockDevice>).map_err(map_dev_err)
}

/// Abre a origem: "disco:N", "discoN", "\\.\PhysicalDriveN" ou um arquivo de imagem (bruto ou DMG).
pub fn open_source(spec: &str) -> io::Result<Rc<dyn BlockDevice>> {
    let lower = spec.to_lowercase();
    let prefixes = ["disco:", "disco", "disk:", "disk", "physicaldrive", "pd"];
    for p in prefixes {
        if let Some(rest) = lower.strip_prefix(p) {
            if let Ok(n) = rest.parse::<u32>() {
                return open_physical(n);
            }
        }
    }
    if lower.starts_with(r"\\.\") || lower.starts_with(r"\\?\") {
        return FileDevice::open_device(spec).map(|d| Rc::new(d) as Rc<dyn BlockDevice>).map_err(map_dev_err);
    }
    if !Path::new(spec).exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("origem '{}' não encontrada (use disco:N ou o caminho de uma imagem)", spec),
        ));
    }
    if let Some(d) = crate::dmg::DmgDevice::open(spec)? {
        return Ok(Rc::new(d));
    }
    Ok(Rc::new(FileDevice::open_image(spec)?))
}

pub fn select_partition(layout: &Layout, want: Option<usize>) -> io::Result<Partition> {
    if let Some(n) = want {
        return layout
            .parts
            .iter()
            .find(|p| p.index == n)
            .cloned()
            .ok_or_else(|| err(format!("partição {} não existe", n)));
    }
    if let Some(p) = layout.parts.iter().find(|p| p.fs.is_mac()) {
        return Ok(p.clone());
    }
    if let Some(p) = layout.parts.iter().find(|p| p.fs.is_supported()) {
        return Ok(p.clone());
    }
    let found: Vec<String> = layout.parts.iter().map(|p| format!("{} ({})", p.index, p.fs.name())).collect();
    let mut msg = String::from("nenhuma partição APFS, HFS+ ou ext2/3/4 encontrada");
    if layout.parts.iter().any(|p| p.fs == FsKind::CoreStorage) {
        msg.push_str("; há uma partição Core Storage (FileVault antigo ou Fusion Drive), que não é suportada");
    }
    if layout.parts.iter().any(|p| p.fs == FsKind::Luks) {
        msg.push_str("; há uma partição LUKS (Linux criptografado), que precisa da senha e não é suportada");
    }
    if layout.parts.iter().any(|p| matches!(p.fs, FsKind::Xfs | FsKind::Btrfs | FsKind::F2fs)) {
        msg.push_str("; XFS, Btrfs e F2FS ainda não são suportados");
    }
    if !found.is_empty() {
        msg.push_str(&format!(". Partições: {}", found.join(", ")));
    }
    Err(err(msg))
}

pub fn partition_device(dev: &Rc<dyn BlockDevice>, p: &Partition) -> Rc<dyn BlockDevice> {
    if let Some(map) = &p.map {
        let mapped: Rc<dyn BlockDevice> = Rc::new(MappedDevice::new(dev.clone(), (**map).clone(), p.len, p.name.clone()));
        return Rc::new(CachedDevice::new(mapped));
    }
    let (start, len) = match &p.fs {
        FsKind::HfsWrapped { off, len } => (p.start + off, *len),
        _ => (p.start, p.len),
    };
    let sub: Rc<dyn BlockDevice> = Rc::new(SubDevice::new(dev.clone(), start, len));
    Rc::new(CachedDevice::new(sub))
}

/// Escolhe o volume APFS: o pedido, ou o volume "Dados", ou o mais povoado.
pub fn choose_volume(c: &apfs::Container, want: Option<usize>, force: bool, quiet: bool) -> io::Result<usize> {
    if c.volumes.is_empty() {
        return Err(err("o container APFS não tem volumes"));
    }
    let idx = match want {
        Some(n) => {
            if n == 0 || n > c.volumes.len() {
                return Err(err(format!("volume {} não existe (há {} volumes)", n, c.volumes.len())));
            }
            n - 1
        }
        None => {
            let usable: Vec<&apfs::VolumeInfo> = c.volumes.iter().filter(|v| !v.encrypted).collect();
            let pick: Option<&apfs::VolumeInfo> = usable
                .iter()
                .find(|v| v.role == 0x40)
                .copied()
                .or_else(|| usable.iter().filter(|v| v.role == 0).max_by_key(|v| v.num_files).copied())
                .or_else(|| usable.iter().max_by_key(|v| v.num_files).copied())
                .or_else(|| c.volumes.first());
            let v = pick.ok_or_else(|| err("nenhum volume utilizável"))?;
            if c.volumes.len() > 1 && !quiet {
                eprintln!(
                    "Volume selecionado: {} \"{}\" ({}). Use -v N para escolher outro.",
                    v.index + 1,
                    v.name,
                    v.role_name()
                );
            }
            v.index
        }
    };
    let v = &c.volumes[idx];
    if v.encrypted && !force {
        return Err(err(format!(
            "o volume \"{}\" está criptografado (FileVault); não é possível ler sem a senha. Use --forcar para tentar mesmo assim",
            v.name
        )));
    }
    Ok(idx)
}

pub fn open_fs(dev: &Rc<dyn BlockDevice>, p: &Partition, vol: Option<usize>, force: bool, quiet: bool) -> io::Result<Box<dyn FileSystem>> {
    let pdev = partition_device(dev, p);
    match &p.fs {
        FsKind::Apfs => {
            let c = apfs::Container::open(pdev)?;
            let idx = choose_volume(&c, vol, force, quiet)?;
            Ok(Box::new(apfs::Volume::open(c, idx)?))
        }
        FsKind::HfsPlus | FsKind::HfsWrapped { .. } => Ok(Box::new(hfsplus::HfsPlus::open(pdev)?)),
        FsKind::Ext => Ok(Box::new(ext4::Ext4::open(pdev)?)),
        FsKind::Hfs => Err(err("HFS clássico (anterior a 1998) não é suportado")),
        FsKind::CoreStorage => Err(err("Core Storage (FileVault 2 antigo / Fusion Drive) não é suportado")),
        FsKind::Luks => Err(err("partição LUKS (Linux criptografado): não é possível ler sem a senha")),
        FsKind::Lvm => Err(err("esta partição é um volume físico LVM; escolha um dos volumes lógicos listados")),
        FsKind::Xfs => Err(err("XFS ainda não é suportado")),
        FsKind::Btrfs => Err(err("Btrfs ainda não é suportado")),
        FsKind::F2fs => Err(err("F2FS ainda não é suportado")),
        FsKind::Swap => Err(err("partição de swap não contém arquivos")),
        other => Err(err(format!("a partição {} não é APFS, HFS+ nem ext2/3/4 ({})", p.index, other.name()))),
    }
}

/// Origem aberta: dispositivo, layout e sistema de arquivos escolhido.
pub struct Opened {
    pub dev: Rc<dyn BlockDevice>,
    pub layout: Layout,
    pub partition: Partition,
    pub fs: Box<dyn FileSystem>,
}

pub fn open(src: &Source, quiet: bool) -> io::Result<Opened> {
    let dev = open_source(&src.spec)?;
    let layout = partition::scan(&dev)?;
    let partition = select_partition(&layout, src.part)?;
    let fs = open_fs(&dev, &partition, src.vol, src.force, quiet)?;
    Ok(Opened { dev, layout, partition, fs })
}
