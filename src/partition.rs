//! Tabelas de partição: GPT, MBR e Apple Partition Map, com detecção do sistema de arquivos.

use std::io;
use std::rc::Rc;

use crate::device::BlockDevice;
use crate::util::*;

#[derive(Clone, Debug, PartialEq)]
pub enum FsKind {
    Apfs,
    HfsPlus,
    /// HFS+ embutido num volume HFS clássico (wrapper); offset/tamanho internos.
    HfsWrapped { off: u64, len: u64 },
    Hfs,
    Ntfs,
    Fat,
    ExFat,
    Ext,
    Xfs,
    Btrfs,
    F2fs,
    Luks,
    Lvm,
    Swap,
    CoreStorage,
    Empty,
    Unknown,
}

impl FsKind {
    pub fn name(&self) -> &'static str {
        match self {
            FsKind::Apfs => "APFS",
            FsKind::HfsPlus => "HFS+",
            FsKind::HfsWrapped { .. } => "HFS+ (wrapper HFS)",
            FsKind::Hfs => "HFS clássico",
            FsKind::Ntfs => "NTFS",
            FsKind::Fat => "FAT",
            FsKind::ExFat => "exFAT",
            FsKind::Ext => "ext2/3/4",
            FsKind::Xfs => "XFS",
            FsKind::Btrfs => "Btrfs",
            FsKind::F2fs => "F2FS",
            FsKind::Luks => "LUKS (cript.)",
            FsKind::Lvm => "LVM (PV)",
            FsKind::Swap => "Linux swap",
            FsKind::CoreStorage => "Core Storage",
            FsKind::Empty => "vazio",
            FsKind::Unknown => "desconhecido",
        }
    }
    pub fn is_mac(&self) -> bool {
        matches!(self, FsKind::Apfs | FsKind::HfsPlus | FsKind::HfsWrapped { .. })
    }
    pub fn is_linux(&self) -> bool {
        matches!(self, FsKind::Ext | FsKind::Xfs | FsKind::Btrfs | FsKind::F2fs | FsKind::Luks | FsKind::Lvm | FsKind::Swap)
    }
    /// Sistemas de arquivos que o macread consegue ler.
    pub fn is_supported(&self) -> bool {
        self.is_mac() || matches!(self, FsKind::Ext)
    }
    pub fn marker(&self) -> &'static str {
        if self.is_mac() {
            "  ◄ Mac"
        } else if matches!(self, FsKind::Ext) {
            "  ◄ Linux"
        } else if self.is_linux() {
            "  (Linux, não suportado)"
        } else {
            ""
        }
    }
}

#[derive(Clone, Debug)]
pub struct Partition {
    pub index: usize,
    pub start: u64,
    pub len: u64,
    pub type_name: String,
    pub name: String,
    pub fs: FsKind,
    /// Volume lógico LVM: trechos (offset no volume, offset no disco, tamanho)
    pub map: Option<Rc<Vec<(u64, u64, u64)>>>,
}

pub struct Layout {
    pub scheme: &'static str,
    pub parts: Vec<Partition>,
}

fn gpt_type_name(guid: &str) -> &'static str {
    match guid {
        "48465300-0000-11AA-AA11-00306543ECAC" => "Apple HFS+",
        "7C3457EF-0000-11AA-AA11-00306543ECAC" => "Apple APFS",
        "53746F72-6167-11AA-AA11-00306543ECAC" => "Apple Core Storage",
        "426F6F74-0000-11AA-AA11-00306543ECAC" => "Apple Boot (Recovery HD)",
        "55465300-0000-11AA-AA11-00306543ECAC" => "Apple UFS",
        "52414944-0000-11AA-AA11-00306543ECAC" => "Apple RAID",
        "52414944-5F4F-11AA-AA11-00306543ECAC" => "Apple RAID offline",
        "4C616265-6C00-11AA-AA11-00306543ECAC" => "Apple Label",
        "5265636F-7665-11AA-AA11-00306543ECAC" => "Apple TV Recovery",
        "69646961-6700-11AA-AA11-00306543ECAC" => "Apple APFS Preboot",
        "52637672-7900-11AA-AA11-00306543ECAC" => "Apple APFS Recovery",
        "C12A7328-F81F-11D2-BA4B-00A0C93EC93B" => "EFI System",
        "EBD0A0A2-B9E5-4433-87C0-68B6B72699C7" => "Microsoft Basic Data",
        "E3C9E316-0B5C-4DB8-817D-F92DF00215AE" => "Microsoft Reserved",
        "DE94BBA4-06D1-4D40-A16A-BFD50179D6AC" => "Windows Recovery",
        "0FC63DAF-8483-4772-8E79-3D69D8477DE4" => "Linux filesystem",
        "0657FD6D-A4AB-43C4-84E5-0933C84B4F4F" => "Linux swap",
        "E6D6D379-F507-44C2-A23C-238F2A3DF928" => "Linux LVM",
        "933AC7E1-2EB4-4F13-B844-B6D5E1A9B6D8" => "Linux /home",
        "4F68BCE3-E8CD-4DB1-96E7-FBCAF984B709" => "Linux root (x86-64)",
        "44479540-F297-41B2-9AF7-D131D5F0458A" => "Linux root (x86)",
        "B921B045-1DF0-41C3-AF44-4C6F280D3FAE" => "Linux root (ARM64)",
        "4D21B016-B534-45C2-A9FB-5C16E091FD2D" => "Linux /var",
        "3B8F8425-20E0-4F3B-907F-1A25A76F98E8" => "Linux /srv",
        "BC13C2FF-59E6-4262-A352-B275FD6F7172" => "Linux boot",
        "CA7D7CCB-63ED-4C53-861C-1742536059CC" => "Linux LUKS",
        "A19D880F-05FC-4D3B-A006-743F0F84911E" => "Linux RAID",
        "8DA63339-0007-60C0-C436-083AC8230908" => "Linux reservado",
        "21686148-6449-6E6F-744E-656564454649" => "BIOS boot",
        "00000000-0000-0000-0000-000000000000" => "(vazio)",
        _ => "(outro)",
    }
}

fn mbr_type_name(t: u8) -> &'static str {
    match t {
        0x00 => "(vazio)",
        0x01 | 0x04 | 0x06 | 0x0E => "FAT",
        0x05 | 0x0F => "Estendida",
        0x07 => "NTFS/exFAT",
        0x0B | 0x0C => "FAT32",
        0x82 => "Linux swap",
        0x83 => "Linux",
        0x8E => "Linux LVM",
        0xFD => "Linux RAID",
        0xA8 => "Apple UFS",
        0xAB => "Apple Boot",
        0xAF => "Apple HFS+",
        0xEE => "GPT protetiva",
        0xEF => "EFI",
        _ => "(outro)",
    }
}

/// Identifica o sistema de arquivos no início de uma região.
pub fn probe_fs(dev: &dyn BlockDevice, start: u64, len: u64) -> FsKind {
    if len < 4096 || start + 4096 > dev.size() {
        return FsKind::Unknown;
    }
    let mut b = vec![0u8; 4096];
    if dev.read_at(start, &mut b).is_err() {
        return FsKind::Unknown;
    }
    if le32(&b, 32) == 0x4253584E {
        return FsKind::Apfs;
    }
    let sig = be16(&b, 1024);
    if sig == 0x482B || sig == 0x4858 {
        return FsKind::HfsPlus;
    }
    if sig == 0x4244 {
        // HFS clássico; pode conter um HFS+ embutido
        if be16(&b, 1024 + 0x7C) == 0x482B {
            let blk = be32(&b, 1024 + 0x14) as u64;
            let al_bl_st = be16(&b, 1024 + 0x1C) as u64;
            let e_start = be16(&b, 1024 + 0x7E) as u64;
            let e_count = be16(&b, 1024 + 0x80) as u64;
            let off = al_bl_st * 512 + e_start * blk;
            let ilen = e_count * blk;
            if ilen > 0 && off + 2048 <= len {
                let mut s = [0u8; 2];
                if dev.read_at(start + off + 1024, &mut s).is_ok()
                    && (be16(&s, 0) == 0x482B || be16(&s, 0) == 0x4858)
                {
                    return FsKind::HfsWrapped { off, len: ilen };
                }
            }
        }
        return FsKind::Hfs;
    }
    if &b[3..7] == b"NTFS" {
        return FsKind::Ntfs;
    }
    if &b[3..11] == b"EXFAT   " {
        return FsKind::ExFat;
    }
    if &b[54..57] == b"FAT" || &b[82..87] == b"FAT32" {
        return FsKind::Fat;
    }
    if le16(&b, 1024 + 56) == 0xEF53 {
        return FsKind::Ext;
    }
    if &b[0..4] == b"XFSB" {
        return FsKind::Xfs;
    }
    if &b[0..6] == b"LUKS\xba\xbe" {
        return FsKind::Luks;
    }
    if &b[512..520] == b"LABELONE" {
        return FsKind::Lvm;
    }
    if le32(&b, 1024) == 0xF2F5_2010 {
        return FsKind::F2fs;
    }
    if len >= 0x10048 {
        let mut m = [0u8; 8];
        if dev.read_at(start + 0x10040, &mut m).is_ok() && &m == b"_BHRfS_M" {
            return FsKind::Btrfs;
        }
    }
    if &b[4096 - 10..4096] == b"SWAPSPACE2" || &b[4096 - 10..4096] == b"SWAP-SPACE" {
        return FsKind::Swap;
    }
    if b.iter().all(|&x| x == 0) {
        return FsKind::Empty;
    }
    FsKind::Unknown
}

fn read_gpt(dev: &dyn BlockDevice, ss: u64, header_lba: u64) -> io::Result<Option<Vec<Partition>>> {
    let mut h = vec![0u8; ss as usize];
    if header_lba * ss + ss > dev.size() {
        return Ok(None);
    }
    dev.read_at(header_lba * ss, &mut h)?;
    if &h[0..8] != b"EFI PART" {
        return Ok(None);
    }
    let entries_lba = le64(&h, 72);
    let num = le32(&h, 80) as u64;
    let esz = le32(&h, 84) as u64;
    if esz < 128 || num == 0 || num > 1024 {
        return Ok(None);
    }
    let total = (num * esz) as usize;
    let mut e = vec![0u8; total];
    if entries_lba * ss + total as u64 > dev.size() {
        return Ok(None);
    }
    dev.read_at(entries_lba * ss, &mut e)?;
    let mut parts = Vec::new();
    for i in 0..num as usize {
        let ent = &e[i * esz as usize..(i + 1) * esz as usize];
        let guid = guid_string(&ent[0..16]);
        if guid == "00000000-0000-0000-0000-000000000000" {
            continue;
        }
        let first = le64(ent, 32);
        let last = le64(ent, 40);
        if last < first {
            continue;
        }
        let name = utf16_to_string(&ent[56..128], false).trim_end_matches('\0').to_string();
        let tname = gpt_type_name(&guid);
        let start = first * ss;
        let len = (last - first + 1) * ss;
        let mut fs = probe_fs(dev, start, len);
        if fs == FsKind::Unknown && guid == "53746F72-6167-11AA-AA11-00306543ECAC" {
            fs = FsKind::CoreStorage;
        }
        parts.push(Partition { index: i + 1, start, len, type_name: tname.to_string(), name, fs, map: None });
    }
    Ok(Some(parts))
}

fn read_apm(dev: &dyn BlockDevice, ss: u64) -> io::Result<Option<Vec<Partition>>> {
    let mut e = vec![0u8; ss as usize];
    if ss * 2 > dev.size() {
        return Ok(None);
    }
    dev.read_at(ss, &mut e)?;
    if be16(&e, 0) != 0x504D {
        return Ok(None);
    }
    let count = be32(&e, 4) as u64;
    if count == 0 || count > 256 {
        return Ok(None);
    }
    let mut parts = Vec::new();
    for i in 0..count {
        let mut ent = vec![0u8; ss as usize];
        if (i + 1) * ss + ss > dev.size() {
            break;
        }
        dev.read_at((i + 1) * ss, &mut ent)?;
        if be16(&ent, 0) != 0x504D {
            break;
        }
        let start = be32(&ent, 8) as u64 * ss;
        let len = be32(&ent, 12) as u64 * ss;
        let name = cstr(&ent[16..48]);
        let ptype = cstr(&ent[48..80]);
        if ptype == "Apple_partition_map" || ptype == "Apple_Free" {
            continue;
        }
        let fs = probe_fs(dev, start, len);
        parts.push(Partition { index: (i + 1) as usize, start, len, type_name: ptype, name, fs, map: None });
    }
    Ok(Some(parts))
}

/// Um setor de boot de FAT/NTFS também termina em 55AA; evita confundir com MBR.
fn looks_like_boot_sector(s: &[u8]) -> bool {
    &s[3..7] == b"NTFS" || &s[3..11] == b"EXFAT   " || &s[54..57] == b"FAT" || &s[82..87] == b"FAT32"
}

/// Acrescenta os volumes lógicos LVM encontrados nas partições do tipo PV.
fn expand_lvm(dev: &Rc<dyn BlockDevice>, parts: &mut Vec<Partition>) {
    let pvs: Vec<(u64, u64)> = parts.iter().filter(|p| p.fs == FsKind::Lvm).map(|p| (p.start, p.len)).collect();
    let mut next = parts.iter().map(|p| p.index).max().unwrap_or(0) + 1;
    for (start, len) in pvs {
        let lvs = match crate::lvm::scan(dev.as_ref(), start, len) {
            Ok(Some(l)) => l,
            Ok(None) => continue,
            Err(e) => {
                eprintln!("aviso: falha ao ler metadados LVM: {}", e);
                continue;
            }
        };
        for lv in lvs {
            let runs = Rc::new(lv.runs.clone());
            let mapped = crate::device::MappedDevice::new(dev.clone(), lv.runs.clone(), lv.size, format!("{}/{}", lv.vg, lv.name));
            let fs = if lv.complete && !lv.runs.is_empty() { probe_fs(&mapped, 0, lv.size) } else { FsKind::Unknown };
            let type_name = if lv.complete {
                format!("Volume lógico LVM {}/{}", lv.vg, lv.name)
            } else {
                format!("Volume lógico LVM {}/{} (incompleto: {})", lv.vg, lv.name, lv.segtype)
            };
            parts.push(Partition { index: next, start: 0, len: lv.size, type_name, name: lv.name.clone(), fs, map: Some(runs) });
            next += 1;
        }
    }
}

pub fn scan(dev: &Rc<dyn BlockDevice>) -> io::Result<Layout> {
    let mut layout = scan_tables(dev)?;
    expand_lvm(dev, &mut layout.parts);
    Ok(layout)
}

fn scan_tables(dev: &Rc<dyn BlockDevice>) -> io::Result<Layout> {
    let d: &dyn BlockDevice = dev.as_ref();
    if d.size() < 4096 {
        return Err(err("dispositivo pequeno demais"));
    }
    let mut s = vec![0u8; 4096];
    d.read_at(0, &mut s)?;

    // Apple Partition Map
    if be16(&s, 0) == 0x4552 || be16(&s, 512) == 0x504D {
        let ss = if be16(&s, 0) == 0x4552 { be16(&s, 2) as u64 } else { 512 };
        let ss = if ss == 0 || ss > 4096 { 512 } else { ss };
        if let Some(parts) = read_apm(d, ss)? {
            return Ok(Layout { scheme: "Apple Partition Map", parts });
        }
    }

    // GPT (tenta setores de 512 e 4096; cabeçalho primário e de backup)
    let mbr_ok = le16(&s, 510) == 0xAA55;
    let has_protective = mbr_ok && (0..4).any(|i| s[446 + i * 16 + 4] == 0xEE);
    for &ss in &[512u64, 4096u64] {
        if let Some(parts) = read_gpt(d, ss, 1)? {
            return Ok(Layout { scheme: if ss == 512 { "GPT" } else { "GPT (setor 4K)" }, parts });
        }
    }
    if has_protective {
        for &ss in &[512u64, 4096u64] {
            let last = d.size() / ss;
            if last > 2 {
                if let Some(parts) = read_gpt(d, ss, last - 1)? {
                    return Ok(Layout { scheme: "GPT (cabeçalho de backup)", parts });
                }
            }
        }
    }

    // MBR
    if mbr_ok && !looks_like_boot_sector(&s) {
        let mut parts = Vec::new();
        for i in 0..4 {
            let e = &s[446 + i * 16..446 + (i + 1) * 16];
            let t = e[4];
            let lba = le32(e, 8) as u64;
            let cnt = le32(e, 12) as u64;
            if t == 0 || cnt == 0 {
                continue;
            }
            let start = lba * 512;
            let len = cnt * 512;
            let fs = if t == 0x05 || t == 0x0F { FsKind::Unknown } else { probe_fs(d, start, len) };
            parts.push(Partition {
                index: i + 1,
                start,
                len,
                type_name: format!("{} (0x{:02X})", mbr_type_name(t), t),
                name: String::new(),
                fs,
                map: None,
            });
        }
        if !parts.is_empty() {
            return Ok(Layout { scheme: "MBR", parts });
        }
    }

    // Sem tabela de partição: o dispositivo inteiro é um volume
    let fs = probe_fs(d, 0, d.size());
    Ok(Layout {
        scheme: "sem tabela de partição",
        parts: vec![Partition {
            index: 1,
            start: 0,
            len: d.size(),
            type_name: "volume inteiro".into(),
            name: String::new(),
            fs,
            map: None,
        }],
    })
}
