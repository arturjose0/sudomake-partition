//! Embute o ícone (installer/sudomake-partition.ico) nos executáveis sem precisar de
//! `rc.exe` nem `windres`: gera o recurso em Rust puro — um ficheiro `.res` para o linker
//! MSVC ou um objecto COFF com a secção `.rsrc` para o linker GNU — e pede ao cargo para o
//! ligar em todos os binários.

use std::env;
use std::fs;
use std::path::PathBuf;

struct IcoEntry {
    w: u8,
    h: u8,
    colors: u8,
    planes: u16,
    bpp: u16,
    data: Vec<u8>,
}

fn u16le(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([b[o], b[o + 1]])
}

fn u32le(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

fn read_ico(bytes: &[u8]) -> Vec<IcoEntry> {
    if bytes.len() < 6 || u16le(bytes, 2) != 1 {
        return Vec::new();
    }
    let count = u16le(bytes, 4) as usize;
    let mut out = Vec::new();
    for i in 0..count {
        let o = 6 + i * 16;
        if o + 16 > bytes.len() {
            break;
        }
        let size = u32le(bytes, o + 8) as usize;
        let off = u32le(bytes, o + 12) as usize;
        if off + size > bytes.len() {
            break;
        }
        out.push(IcoEntry { w: bytes[o], h: bytes[o + 1], colors: bytes[o + 2], planes: u16le(bytes, o + 4), bpp: u16le(bytes, o + 6), data: bytes[off..off + size].to_vec() });
    }
    out
}

/// RT_GROUP_ICON: cabeçalho igual ao do .ico, mas cada entrada aponta para um ID de RT_ICON.
fn group_icon(entries: &[IcoEntry]) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&0u16.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    for (i, e) in entries.iter().enumerate() {
        v.push(e.w);
        v.push(e.h);
        v.push(e.colors);
        v.push(0);
        v.extend_from_slice(&e.planes.to_le_bytes());
        v.extend_from_slice(&e.bpp.to_le_bytes());
        v.extend_from_slice(&(e.data.len() as u32).to_le_bytes());
        v.extend_from_slice(&((i + 1) as u16).to_le_bytes());
    }
    v
}

const RT_ICON: u16 = 3;
const RT_GROUP_ICON: u16 = 14;
const LANG: u16 = 0x0409;

// ---- formato .res (aceite directamente pelo link.exe da Microsoft) ------------------------

fn res_entry(out: &mut Vec<u8>, rtype: u16, id: u16, data: &[u8], null_entry: bool) {
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(&32u32.to_le_bytes());
    out.extend_from_slice(&0xFFFFu16.to_le_bytes());
    out.extend_from_slice(&rtype.to_le_bytes());
    out.extend_from_slice(&0xFFFFu16.to_le_bytes());
    out.extend_from_slice(&id.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // DataVersion
    out.extend_from_slice(&(if null_entry { 0u16 } else { 0x1010u16 }).to_le_bytes()); // MOVEABLE | DISCARDABLE
    out.extend_from_slice(&(if null_entry { 0u16 } else { LANG }).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // Version
    out.extend_from_slice(&0u32.to_le_bytes()); // Characteristics
    out.extend_from_slice(data);
    while out.len() % 4 != 0 {
        out.push(0);
    }
}

fn write_res(entries: &[IcoEntry], group: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    res_entry(&mut out, 0, 0, &[], true);
    for (i, e) in entries.iter().enumerate() {
        res_entry(&mut out, RT_ICON, (i + 1) as u16, &e.data, false);
    }
    res_entry(&mut out, RT_GROUP_ICON, 1, group, false);
    out
}

// ---- objecto COFF com secção .rsrc (para o linker GNU / lld) ----------------------------

fn put_u16(buf: &mut [u8], off: usize, v: u16) {
    buf[off..off + 2].copy_from_slice(&v.to_le_bytes());
}

fn put_u32(buf: &mut [u8], off: usize, v: u32) {
    buf[off..off + 4].copy_from_slice(&v.to_le_bytes());
}

fn put_dir(buf: &mut [u8], off: usize, id_entries: usize) {
    // IMAGE_RESOURCE_DIRECTORY: só o número de entradas por ID interessa
    put_u16(buf, off + 14, id_entries as u16);
}

fn put_entry(buf: &mut [u8], off: usize, id: u32, target: u32) {
    put_u32(buf, off, id);
    put_u32(buf, off + 4, target);
}

fn write_coff(entries: &[IcoEntry], group: &[u8]) -> Vec<u8> {
    let mut res: Vec<(u32, &[u8])> = entries.iter().enumerate().map(|(i, e)| ((i + 1) as u32, e.data.as_slice())).collect();
    let n = res.len();
    res.push((1, group)); // RT_GROUP_ICON id 1
    let total = n + 1;

    let root_off = 0usize;
    let dir_icon_off = root_off + 16 + 2 * 8;
    let dir_group_off = dir_icon_off + 16 + n * 8;
    let lang_off = dir_group_off + 16 + 8;
    let data_entries_off = lang_off + total * 24;
    let data_off = data_entries_off + total * 16;

    let mut sec = vec![0u8; data_off];
    put_dir(&mut sec, root_off, 2);
    put_entry(&mut sec, root_off + 16, RT_ICON as u32, dir_icon_off as u32 | 0x8000_0000);
    put_entry(&mut sec, root_off + 24, RT_GROUP_ICON as u32, dir_group_off as u32 | 0x8000_0000);
    put_dir(&mut sec, dir_icon_off, n);
    for i in 0..n {
        put_entry(&mut sec, dir_icon_off + 16 + i * 8, (i + 1) as u32, (lang_off + i * 24) as u32 | 0x8000_0000);
    }
    put_dir(&mut sec, dir_group_off, 1);
    put_entry(&mut sec, dir_group_off + 16, 1, (lang_off + n * 24) as u32 | 0x8000_0000);

    let mut relocs: Vec<u32> = Vec::new();
    let mut blob: Vec<u8> = Vec::new();
    for (k, (_id, data)) in res.iter().enumerate() {
        let ld = lang_off + k * 24;
        let de = data_entries_off + k * 16;
        put_dir(&mut sec, ld, 1);
        put_entry(&mut sec, ld + 16, LANG as u32, de as u32);
        let start = data_off + blob.len();
        put_u32(&mut sec, de, start as u32); // RVA: corrigido pelo linker (relocação ADDR32NB)
        put_u32(&mut sec, de + 4, data.len() as u32);
        relocs.push(de as u32);
        blob.extend_from_slice(data);
        while blob.len() % 8 != 0 {
            blob.push(0);
        }
    }
    sec.extend_from_slice(&blob);
    let sec_len = sec.len();

    let ptr_raw = 20u32 + 40;
    let ptr_reloc = ptr_raw + sec_len as u32;
    let ptr_sym = ptr_reloc + 10 * relocs.len() as u32;

    let mut f = Vec::new();
    // IMAGE_FILE_HEADER
    f.extend_from_slice(&0x8664u16.to_le_bytes());
    f.extend_from_slice(&1u16.to_le_bytes());
    f.extend_from_slice(&0u32.to_le_bytes());
    f.extend_from_slice(&ptr_sym.to_le_bytes());
    f.extend_from_slice(&2u32.to_le_bytes());
    f.extend_from_slice(&0u16.to_le_bytes());
    f.extend_from_slice(&0u16.to_le_bytes());
    // IMAGE_SECTION_HEADER .rsrc
    f.extend_from_slice(b".rsrc\0\0\0");
    f.extend_from_slice(&0u32.to_le_bytes());
    f.extend_from_slice(&0u32.to_le_bytes());
    f.extend_from_slice(&(sec_len as u32).to_le_bytes());
    f.extend_from_slice(&ptr_raw.to_le_bytes());
    f.extend_from_slice(&ptr_reloc.to_le_bytes());
    f.extend_from_slice(&0u32.to_le_bytes());
    f.extend_from_slice(&(relocs.len() as u16).to_le_bytes());
    f.extend_from_slice(&0u16.to_le_bytes());
    f.extend_from_slice(&0xC030_0040u32.to_le_bytes()); // INITIALIZED_DATA | ALIGN_4 | READ | WRITE
    f.extend_from_slice(&sec);
    for r in &relocs {
        f.extend_from_slice(&r.to_le_bytes());
        f.extend_from_slice(&0u32.to_le_bytes()); // símbolo 0 = secção .rsrc
        f.extend_from_slice(&3u16.to_le_bytes()); // IMAGE_REL_AMD64_ADDR32NB
    }
    // símbolo da secção + registo auxiliar
    f.extend_from_slice(b".rsrc\0\0\0");
    f.extend_from_slice(&0u32.to_le_bytes());
    f.extend_from_slice(&1i16.to_le_bytes());
    f.extend_from_slice(&0u16.to_le_bytes());
    f.push(3); // IMAGE_SYM_CLASS_STATIC
    f.push(1);
    f.extend_from_slice(&(sec_len as u32).to_le_bytes());
    f.extend_from_slice(&(relocs.len() as u16).to_le_bytes());
    f.extend_from_slice(&0u16.to_le_bytes());
    f.extend_from_slice(&0u32.to_le_bytes());
    f.extend_from_slice(&0u16.to_le_bytes());
    f.push(0);
    f.extend_from_slice(&[0, 0, 0]);
    // tabela de strings vazia
    f.extend_from_slice(&4u32.to_le_bytes());
    f
}

fn main() {
    let ico_path = PathBuf::from("installer/sudomake-partition.ico");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", ico_path.display());
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let bytes = match fs::read(&ico_path) {
        Ok(b) => b,
        Err(_) => return,
    };
    let entries = read_ico(&bytes);
    if entries.is_empty() {
        println!("cargo:warning=ícone inválido: {}", ico_path.display());
        return;
    }
    let group = group_icon(&entries);
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let msvc = env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc");
    let out = if msvc { out_dir.join("app-icon.res") } else { out_dir.join("app-icon.o") };
    let data = if msvc { write_res(&entries, &group) } else { write_coff(&entries, &group) };
    fs::write(&out, data).expect("gravar recurso do ícone");
    println!("cargo:rustc-link-arg-bins={}", out.display());
}
