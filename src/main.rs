//! macread — lê e copia arquivos de discos de Mac (APFS / HFS+) no Windows.

use std::io::{self, Write};
use std::path::Path;
use std::rc::Rc;

use macread::device::BlockDevice;
use macread::fs::{self, Entry, FileSystem, Kind};
use macread::open::{open_fs, open_physical, open_source, partition_device, select_partition};
use macread::partition::{self, FsKind, Layout};
use macread::util::*;
use macread::{apfs, copy, device, dokan, ext4, fsservice, hfsplus};
use macread::open::Source;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn usage() {
    println!(
        r#"macread {v} — lê e copia arquivos de discos de Mac (APFS, HFS+) e Linux (ext2/3/4, LVM) no Windows

USO:
  macread discos                                   lista os discos físicos e suas partições
  macread info    <origem>                         mostra partições e volumes da origem
  macread ls      <origem> [caminho] [-l]          lista uma pasta
  macread arvore  <origem> [caminho] [--nivel N]   mostra a árvore de pastas
  macread cat     <origem> <arquivo>               envia um arquivo para a saída padrão
  macread copiar  <origem> <caminho> <destino>     copia um arquivo ou pasta (recursivo)
  macread verificar <origem> [caminho]             lê todos os arquivos sem gravar (testa a leitura)
  macread hex     <origem> [offset] [tamanho]      mostra bytes brutos do disco (diagnóstico)
  macread montar  <origem> [letra]                 monta como unidade do Windows (precisa do Dokan)
  macread desmontar <letra>                        desmonta uma unidade montada pelo macread

ORIGEM:
  disco:1            disco físico nº 1 (veja "macread discos"; precisa de Administrador)
  \\.\PhysicalDrive1 idem, no formato do Windows
  C:\imagem.raw      imagem bruta de disco ou de partição (dd, .img, .raw)
  C:\backup.dmg      imagem DMG (UDIF) criada pelo Utilitário de Disco / hdiutil

OPÇÕES:
  -p, --particao N   escolhe a partição (padrão: a primeira partição Mac ou Linux encontrada)
  -v, --volume N     escolhe o volume APFS (padrão: o volume "Dados" ou o único existente)
  -l                 listagem detalhada (permissões, tamanho, data)
  --sobrescrever     sobrescreve arquivos existentes no destino (padrão: pula se o tamanho bate)
  --links            grava um .symlink.txt para cada link simbólico
  --verboso          mostra cada arquivo copiado
  --simular          só conta, não grava nada
  --forcar           tenta abrir mesmo volumes marcados como criptografados

EXEMPLOS:
  macread discos
  macread info disco:1
  macread ls disco:1 /Users
  macread copiar disco:1 /Users/joao/Documents D:\Recuperado
  macread copiar disco:1 / D:\Recuperado\Tudo -v 2
"#,
        v = VERSION
    );
}

struct Args {
    cmd: String,
    pos: Vec<String>,
    part: Option<usize>,
    vol: Option<usize>,
    long: bool,
    overwrite: bool,
    symlinks: bool,
    verbose: bool,
    dry_run: bool,
    force: bool,
    depth: usize,
}

fn parse_args() -> Result<Args, String> {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let mut a = Args {
        cmd: String::new(),
        pos: Vec::new(),
        part: None,
        vol: None,
        long: false,
        overwrite: false,
        symlinks: false,
        verbose: false,
        dry_run: false,
        force: false,
        depth: 3,
    };
    let mut i = 0;
    let num = |raw: &Vec<String>, i: &mut usize, name: &str| -> Result<usize, String> {
        *i += 1;
        raw.get(*i)
            .and_then(|s| s.parse::<usize>().ok())
            .ok_or_else(|| format!("a opção {} precisa de um número", name))
    };
    while i < raw.len() {
        let s = raw[i].as_str();
        match s {
            "-p" | "--particao" | "--partition" => a.part = Some(num(&raw, &mut i, s)?),
            "-v" | "--volume" => a.vol = Some(num(&raw, &mut i, s)?),
            "--nivel" | "--depth" => a.depth = num(&raw, &mut i, s)?,
            "-l" | "--longo" => a.long = true,
            "--sobrescrever" | "--overwrite" => a.overwrite = true,
            "--links" => a.symlinks = true,
            "--verboso" | "--verbose" => a.verbose = true,
            "--simular" | "--dry-run" => a.dry_run = true,
            "--forcar" | "--force" => a.force = true,
            "-h" | "--help" | "-?" | "/?" | "ajuda" | "help" => {
                a.cmd = "ajuda".into();
            }
            _ if s.starts_with('-') && s.len() > 1 && !Path::new(s).exists() => {
                return Err(format!("opção desconhecida: {}", s));
            }
            _ => {
                if a.cmd.is_empty() {
                    a.cmd = s.to_lowercase();
                } else {
                    a.pos.push(s.to_string());
                }
            }
        }
        i += 1;
    }
    Ok(a)
}

fn print_layout(layout: &Layout, indent: &str) {
    println!("{}Esquema: {}", indent, layout.scheme);
    for p in &layout.parts {
        let mark = p.fs.marker();
        let name = if p.name.is_empty() { String::new() } else { format!(" \"{}\"", p.name) };
        println!(
            "{}  Partição {:>2}: {:<10} {:>10}  {}{}{}",
            indent,
            p.index,
            p.fs.name(),
            fmt_size(p.len),
            p.type_name,
            name,
            mark
        );
    }
}

fn cmd_discos() -> io::Result<()> {
    let disks = device::enumerate_disks();
    if disks.is_empty() {
        println!("Nenhum disco físico encontrado.");
        return Ok(());
    }
    let mut any_denied = false;
    for d in &disks {
        let (model, bus, size, sector) = match &d.info {
            Some(i) => (i.model.clone(), i.bus.clone(), fmt_size(i.size), i.sector),
            None => ("?".into(), "?".into(), "?".into(), 0),
        };
        println!("Disco {}: {}  [{}]  {}  (setor {} bytes)", d.number, model, bus, size, sector);
        if !d.readable {
            any_denied = true;
            println!("  (sem permissão para ler; execute como Administrador para ver as partições)");
            continue;
        }
        match open_physical(d.number).and_then(|dev| partition::scan(&dev)) {
            Ok(layout) => print_layout(&layout, "  "),
            Err(e) => println!("  erro ao ler a tabela de partições: {}", e),
        }
    }
    if any_denied {
        println!("\nDica: clique com o botão direito no PowerShell/Prompt e escolha \"Executar como administrador\".");
    }
    Ok(())
}

fn cmd_info(a: &Args) -> io::Result<()> {
    let spec = a.pos.first().ok_or_else(|| err("informe a origem (ex.: disco:1 ou imagem.raw)"))?;
    let dev = open_source(spec)?;
    println!("Origem: {}  ({})", spec, fmt_size(dev.size()));
    let layout = partition::scan(&dev)?;
    print_layout(&layout, "");
    for p in layout.parts.iter().filter(|p| p.fs.is_supported()) {
        println!("\nPartição {} — {}:", p.index, p.fs.name());
        let pdev = partition_device(&dev, p);
        match &p.fs {
            FsKind::Ext => match ext4::Ext4::open(pdev) {
                Ok(e) => println!("  {}  uuid {}", e.summary(), uuid_string(&e.uuid)),
                Err(e) => println!("  erro: {}", e),
            },
            FsKind::Apfs => match apfs::Container::open(pdev) {
                Ok(c) => {
                    println!("  Container APFS  uuid {}  bloco {} bytes  {} blocos  xid {}", uuid_string(&c.uuid), c.block_size, c.block_count, c.xid);
                    for v in &c.volumes {
                        println!(
                            "  Volume {}: \"{}\"  função: {}  {} arquivos, {} pastas{}{}{}  modificado {}",
                            v.index + 1,
                            v.name,
                            v.role_name(),
                            v.num_files,
                            v.num_dirs,
                            if v.encrypted { "  [CRIPTOGRAFADO]" } else { "" },
                            if v.case_insensitive { "" } else { "  [case-sensitive]" },
                            if v.sealed { "  [selado]" } else { "" },
                            fmt_time((v.last_mod / 1_000_000_000) as i64)
                        );
                    }
                }
                Err(e) => println!("  erro: {}", e),
            },
            _ => match hfsplus::HfsPlus::open(pdev) {
                Ok(h) => println!("  {}", h.summary()),
                Err(e) => println!("  erro: {}", e),
            },
        }
    }
    Ok(())
}

struct Opened {
    fs: Box<dyn FileSystem>,
    _dev: Rc<dyn BlockDevice>,
}

fn open_from_args(a: &Args) -> io::Result<Opened> {
    let spec = a.pos.first().ok_or_else(|| err("informe a origem (ex.: disco:1 ou imagem.raw)"))?;
    let dev = open_source(spec)?;
    let layout = partition::scan(&dev)?;
    let p = select_partition(&layout, a.part)?;
    let fs = open_fs(&dev, &p, a.vol, a.force, false)?;
    eprintln!("Partição {}: {}", p.index, fs.summary());
    Ok(Opened { fs, _dev: dev })
}

fn print_entry(e: &Entry, long: bool, fs: &dyn FileSystem) {
    if long {
        let link = if e.kind == Kind::Symlink { format!(" -> {}", fs.read_link(e).unwrap_or_default()) } else { String::new() };
        println!(
            "{} {:>12} {} {} {}{}",
            mode_string(e.mode, e.kind.letter()),
            if e.kind == Kind::Dir { String::from("<pasta>") } else { e.size.to_string() },
            fmt_time(e.mtime),
            if e.compressed { "C" } else { " " },
            e.name,
            link
        );
    } else {
        match e.kind {
            Kind::Dir => println!("{:>10}  {}/", "<pasta>", e.name),
            Kind::Symlink => println!("{:>10}  {} -> {}", "<link>", e.name, fs.read_link(e).unwrap_or_default()),
            _ => println!("{:>10}  {}", fmt_size(e.size), e.name),
        }
    }
}

fn cmd_ls(a: &Args) -> io::Result<()> {
    let o = open_from_args(a)?;
    let path = a.pos.get(1).map(String::as_str).unwrap_or("/");
    let e = fs::lookup(o.fs.as_ref(), path)?;
    if e.kind != Kind::Dir {
        print_entry(&e, true, o.fs.as_ref());
        return Ok(());
    }
    let list = o.fs.read_dir(&e)?;
    let (mut nd, mut nf, mut total) = (0, 0, 0u64);
    for c in &list {
        print_entry(c, a.long, o.fs.as_ref());
        match c.kind {
            Kind::Dir => nd += 1,
            Kind::File => {
                nf += 1;
                total += c.size;
            }
            _ => {}
        }
    }
    println!("\n{} pastas, {} arquivos ({})", nd, nf, fmt_size(total));
    Ok(())
}

fn print_tree(fs: &dyn FileSystem, e: &Entry, depth: usize, max: usize, prefix: &str, counts: &mut (u64, u64, u64)) {
    let list = match fs.read_dir(e) {
        Ok(l) => l,
        Err(err) => {
            println!("{}[erro: {}]", prefix, err);
            return;
        }
    };
    let n = list.len();
    for (i, c) in list.iter().enumerate() {
        let last = i + 1 == n;
        let branch = if last { "└── " } else { "├── " };
        match c.kind {
            Kind::Dir => {
                counts.0 += 1;
                println!("{}{}{}/", prefix, branch, c.name);
                if depth + 1 < max {
                    let np = format!("{}{}", prefix, if last { "    " } else { "│   " });
                    print_tree(fs, c, depth + 1, max, &np, counts);
                }
            }
            Kind::Symlink => println!("{}{}{} -> {}", prefix, branch, c.name, fs.read_link(c).unwrap_or_default()),
            _ => {
                counts.1 += 1;
                counts.2 += c.size;
                println!("{}{}{}  ({})", prefix, branch, c.name, fmt_size(c.size));
            }
        }
    }
}

fn cmd_arvore(a: &Args) -> io::Result<()> {
    let o = open_from_args(a)?;
    let path = a.pos.get(1).map(String::as_str).unwrap_or("/");
    let e = fs::lookup(o.fs.as_ref(), path)?;
    println!("{}", if path == "/" { "/".to_string() } else { path.to_string() });
    let mut counts = (0u64, 0u64, 0u64);
    print_tree(o.fs.as_ref(), &e, 0, a.depth.max(1), "", &mut counts);
    println!("\n{} pastas, {} arquivos ({}) até o nível {}", counts.0, counts.1, fmt_size(counts.2), a.depth.max(1));
    Ok(())
}

fn cmd_cat(a: &Args) -> io::Result<()> {
    let o = open_from_args(a)?;
    let path = a.pos.get(1).ok_or_else(|| err("informe o caminho do arquivo"))?;
    let e = fs::lookup(o.fs.as_ref(), path)?;
    if e.kind != Kind::File {
        return Err(err(format!("'{}' não é um arquivo", path)));
    }
    let mut r = o.fs.open(&e)?;
    let size = r.size();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut buf = vec![0u8; 1024 * 1024];
    let mut off = 0u64;
    while off < size {
        let n = r.read_at(off, &mut buf)?;
        if n == 0 {
            break;
        }
        out.write_all(&buf[..n])?;
        off += n as u64;
    }
    out.flush()?;
    Ok(())
}

fn cmd_hex(a: &Args) -> io::Result<()> {
    let spec = a.pos.first().ok_or_else(|| err("informe a origem"))?;
    let dev = open_source(spec)?;
    let parse = |s: &String| -> Option<u64> {
        let t = s.trim();
        if let Some(h) = t.strip_prefix("0x") { u64::from_str_radix(h, 16).ok() } else { t.parse().ok() }
    };
    let off = a.pos.get(1).and_then(parse).unwrap_or(0);
    let len = a.pos.get(2).and_then(parse).unwrap_or(512).min(1 << 20);
    let len = len.min(dev.size().saturating_sub(off));
    let mut buf = vec![0u8; len as usize];
    dev.read_at(off, &mut buf)?;
    let stdout = io::stdout();
    let mut out = stdout.lock();
    for (i, row) in buf.chunks(16).enumerate() {
        let hex: Vec<String> = row.iter().map(|b| format!("{:02x}", b)).collect();
        let asc: String = row.iter().map(|&b| if (32..127).contains(&b) { b as char } else { '.' }).collect();
        writeln!(out, "{:012x}  {:<47}  {}", off + i as u64 * 16, hex.join(" "), asc)?;
    }
    Ok(())
}

static CTRL_C: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

unsafe extern "system" fn ctrl_handler(_t: u32) -> i32 {
    CTRL_C.store(true, std::sync::atomic::Ordering::Relaxed);
    1
}

fn cmd_montar(a: &Args) -> io::Result<()> {
    let spec = a.pos.first().ok_or_else(|| err("informe a origem (ex.: disco:1 ou imagem.raw)"))?;
    let (lib_ver, drv_ver) = dokan::available().map_err(err)?;
    let letter = match a.pos.get(1) {
        Some(l) => l.trim_end_matches([':', '\\']).chars().next().map(|c| c.to_ascii_uppercase()).filter(|c| c.is_ascii_alphabetic()).ok_or_else(|| err("letra de unidade inválida"))?,
        None => *dokan::free_letters().iter().rev().next().ok_or_else(|| err("não há letra de unidade livre"))?,
    };
    if !dokan::free_letters().contains(&letter) {
        return Err(err(format!("a letra {}: já está em uso", letter)));
    }
    let src = Source { spec: spec.clone(), part: a.part, vol: a.vol, force: a.force };
    eprintln!("Dokan {}.{}.{} (driver {}). Abrindo a origem...", lib_ver / 100, (lib_ver / 10) % 10, lib_ver % 10, drv_ver);
    let svc = std::sync::Arc::new(fsservice::FsService::start(src, 0)?);
    eprintln!("{} \"{}\": montando como {}:", svc.info.fs_type, svc.info.label, letter);
    let admin = device::FileDevice::open_physical(0).map(|_| true).unwrap_or_else(|e| e.kind() != io::ErrorKind::PermissionDenied);
    let m = dokan::mount(svc.clone(), letter, admin).map_err(err)?;
    unsafe {
        winapi::um::consoleapi::SetConsoleCtrlHandler(Some(ctrl_handler), 1);
    }
    eprintln!("Unidade {}: montada (somente leitura). Pressione Ctrl+C para desmontar.", letter);
    while !CTRL_C.load(std::sync::atomic::Ordering::Relaxed) && m.is_alive() {
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    if m.is_alive() {
        eprintln!("Desmontando {}:...", letter);
    } else {
        eprintln!("A unidade {}: foi desmontada.", letter);
    }
    m.unmount();
    Ok(())
}

fn cmd_desmontar(a: &Args) -> io::Result<()> {
    let l = a.pos.first().ok_or_else(|| err("informe a letra da unidade"))?;
    let letter = l.trim_end_matches([':', '\\']).chars().next().map(|c| c.to_ascii_uppercase()).ok_or_else(|| err("letra inválida"))?;
    dokan::unmount_letter(letter).map_err(err)?;
    eprintln!("Unidade {}: desmontada.", letter);
    Ok(())
}

fn cmd_copiar(a: &Args, verify: bool) -> io::Result<()> {
    if a.pos.len() < if verify { 1 } else { 3 } {
        return Err(err("uso: macread copiar <origem> <caminho-no-mac> <pasta-destino>"));
    }
    let o = open_from_args(a)?;
    let src = a.pos.get(1).map(String::as_str).unwrap_or("/");
    let dest = Path::new(if verify { "." } else { &a.pos[2] });
    let e = fs::lookup(o.fs.as_ref(), src)?;
    let dest_long = copy::long_path(dest);
    if !a.dry_run && !verify {
        std::fs::create_dir_all(&dest_long)?;
    }
    let log_path = dest_long.join("macread-log.txt");
    let mut st = copy::Stats::new(if a.dry_run || verify { None } else { Some(&log_path) });
    let opts = copy::Options { overwrite: a.overwrite, symlinks: a.symlinks, verbose: a.verbose, dry_run: a.dry_run, verify };
    if verify {
        eprintln!("Verificando a leitura de '{}' (nada será gravado)", if src.is_empty() { "/" } else { src });
    } else {
        eprintln!(
            "{} '{}' ({}) para '{}'",
            if a.dry_run { "Simulando cópia de" } else { "Copiando" },
            if src.is_empty() { "/" } else { src },
            match e.kind {
                Kind::Dir => "pasta".to_string(),
                _ => fmt_size(e.size),
            },
            dest.display()
        );
    }
    let src_display = if src.starts_with('/') { src.to_string() } else { format!("/{}", src) };
    copy::copy_entry(o.fs.as_ref(), &e, &dest_long, &src_display, &opts, &mut st);
    st.finish();
    let secs = st.elapsed().as_secs_f64().max(0.001);
    eprintln!(
        "\nConcluído: {} arquivos ({}, {} comprimidos) e {} pastas em {:.1}s ({}/s). {} já existiam, {} links, {} erros.",
        st.files,
        fmt_size(st.bytes),
        st.compressed,
        st.dirs,
        secs,
        fmt_size((st.bytes as f64 / secs) as u64),
        st.skipped,
        st.symlinks,
        st.errors
    );
    if verify {
        return Ok(());
    }
    if st.errors > 0 || st.symlinks > 0 {
        eprintln!(
            "Detalhes {}em: {}",
            if st.errors > 0 { "dos erros " } else { "dos links " },
            copy::display_path(&log_path)
        );
    }
    Ok(())
}

fn main() {
    std::panic::set_hook(Box::new(|info| {
        let msg = info.payload().downcast_ref::<&str>().map(|s| s.to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_default();
        eprintln!("erro interno: {} ({})", msg, info.location().map(|l| l.to_string()).unwrap_or_default());
    }));
    let a = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("erro: {}\n", e);
            usage();
            std::process::exit(2);
        }
    };
    let result = match a.cmd.as_str() {
        "" | "ajuda" => {
            usage();
            Ok(())
        }
        "discos" | "disks" => cmd_discos(),
        "info" => cmd_info(&a),
        "ls" | "dir" | "listar" => cmd_ls(&a),
        "arvore" | "árvore" | "tree" => cmd_arvore(&a),
        "cat" | "ver" => cmd_cat(&a),
        "copiar" | "copy" | "cp" => cmd_copiar(&a, false),
        "verificar" | "verify" => cmd_copiar(&a, true),
        "hex" | "dump" => cmd_hex(&a),
        "montar" | "mount" => cmd_montar(&a),
        "desmontar" | "unmount" | "umount" => cmd_desmontar(&a),
        other => {
            eprintln!("comando desconhecido: {}\n", other);
            usage();
            std::process::exit(2);
        }
    };
    if let Err(e) = result {
        eprintln!("erro: {}", e);
        std::process::exit(1);
    }
}
