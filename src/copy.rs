//! Cópia recursiva de arquivos para o Windows, com log de erros e retomada.

use std::fs;
use std::io::{self, Write};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, UNIX_EPOCH};

use crate::fs::{Entry, FileSystem, Kind};
use crate::util::*;

pub struct Options {
    pub overwrite: bool,
    pub symlinks: bool,
    pub verbose: bool,
    pub dry_run: bool,
    /// Só lê os arquivos (testa a leitura), sem gravar nada.
    pub verify: bool,
}

pub struct Stats {
    pub files: u64,
    pub dirs: u64,
    pub bytes: u64,
    pub skipped: u64,
    pub errors: u64,
    pub symlinks: u64,
    pub compressed: u64,
    start: Instant,
    last_print: Instant,
    log_path: Option<PathBuf>,
    log: Option<fs::File>,
    printed_progress: bool,
    /// Chamado periodicamente com (arquivos, bytes, nome atual); usado pela interface gráfica.
    pub hook: Option<Box<dyn FnMut(u64, u64, &str)>>,
    /// Sinal de cancelamento (interface gráfica).
    pub cancel: Option<Arc<AtomicBool>>,
    /// Primeiras mensagens de erro (para mostrar na interface).
    pub error_list: Vec<String>,
    /// Sem saída no terminal (interface gráfica).
    pub quiet: bool,
}

impl Stats {
    pub fn new(log_path: Option<&Path>) -> Stats {
        Stats {
            files: 0,
            dirs: 0,
            bytes: 0,
            skipped: 0,
            errors: 0,
            symlinks: 0,
            compressed: 0,
            start: Instant::now(),
            last_print: Instant::now(),
            log_path: log_path.map(Path::to_path_buf),
            log: None,
            printed_progress: false,
            hook: None,
            cancel: None,
            error_list: Vec::new(),
            quiet: false,
        }
    }

    pub fn cancelled(&self) -> bool {
        self.cancel.as_ref().map_or(false, |c| c.load(Ordering::Relaxed))
    }

    /// Abre o arquivo de log só quando há algo a registrar.
    fn log_file(&mut self) -> Option<&mut fs::File> {
        if self.log.is_none() {
            if let Some(p) = &self.log_path {
                if let Ok(f) = fs::OpenOptions::new().create(true).append(true).open(p) {
                    self.log = Some(f);
                }
            }
        }
        self.log.as_mut()
    }

    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    fn error(&mut self, path: &str, msg: &str) {
        self.errors += 1;
        self.clear_progress();
        let line = format!("[ERRO] {}: {}", path, msg);
        if !self.quiet {
            eprintln!("{}", line);
        }
        if self.error_list.len() < 200 {
            self.error_list.push(line.clone());
        }
        if let Some(f) = self.log_file() {
            let _ = writeln!(f, "{}", line);
        }
    }

    fn note(&mut self, msg: &str) {
        let line = msg.to_string();
        if let Some(f) = self.log_file() {
            let _ = writeln!(f, "{}", line);
        }
    }

    fn clear_progress(&mut self) {
        if self.printed_progress {
            eprint!("\r{:80}\r", "");
            self.printed_progress = false;
        }
    }

    fn progress(&mut self, current: &str, force: bool) {
        if !force && self.last_print.elapsed() < Duration::from_millis(250) {
            return;
        }
        self.last_print = Instant::now();
        if let Some(h) = &mut self.hook {
            h(self.files, self.bytes, current);
        }
        if self.quiet {
            return;
        }
        let secs = self.start.elapsed().as_secs_f64().max(0.001);
        let rate = self.bytes as f64 / secs;
        let name: String = current.chars().rev().take(38).collect::<Vec<_>>().into_iter().rev().collect();
        let line = format!(
            "{} arquivos, {} ({}/s) {}",
            self.files,
            fmt_size(self.bytes),
            fmt_size(rate as u64),
            name
        );
        eprint!("\r{:<78}", line.chars().take(78).collect::<String>());
        self.printed_progress = true;
    }

    pub fn finish(&mut self) {
        self.clear_progress();
    }
}

/// Converte para caminho absoluto com prefixo \\?\ (permite caminhos longos).
pub fn long_path(p: &Path) -> PathBuf {
    let abs = std::path::absolute(p).unwrap_or_else(|_| p.to_path_buf());
    let s = abs.to_string_lossy().to_string();
    if s.starts_with(r"\\?\") {
        abs
    } else if s.starts_with(r"\\") {
        PathBuf::from(format!(r"\\?\UNC\{}", &s[2..]))
    } else {
        PathBuf::from(format!(r"\\?\{}", s))
    }
}

pub fn display_path(p: &Path) -> String {
    let s = p.to_string_lossy();
    s.strip_prefix(r"\\?\UNC\").map(|r| format!(r"\\{}", r)).unwrap_or_else(|| s.strip_prefix(r"\\?\").unwrap_or(&s).to_string())
}

pub fn copy_entry(fs: &dyn FileSystem, e: &Entry, dest_dir: &Path, src_path: &str, opts: &Options, st: &mut Stats) {
    match e.kind {
        Kind::Dir => {
            let target = if e.name.is_empty() { dest_dir.to_path_buf() } else { dest_dir.join(sanitize_name(&e.name)) };
            if !opts.dry_run && !opts.verify {
                if let Err(err) = fs::create_dir_all(&target) {
                    st.error(src_path, &format!("não foi possível criar a pasta '{}': {}", display_path(&target), err));
                    return;
                }
            }
            st.dirs += 1;
            let children = match catch_unwind(AssertUnwindSafe(|| fs.read_dir(e))) {
                Ok(Ok(c)) => c,
                Ok(Err(err)) => {
                    st.error(src_path, &format!("erro ao listar a pasta: {}", err));
                    return;
                }
                Err(_) => {
                    st.error(src_path, "erro interno ao listar a pasta (metadados corrompidos?)");
                    return;
                }
            };
            for c in children {
                if st.cancelled() {
                    return;
                }
                let child_path = if src_path.ends_with('/') { format!("{}{}", src_path, c.name) } else { format!("{}/{}", src_path, c.name) };
                copy_entry(fs, &c, &target, &child_path, opts, st);
            }
            if !opts.dry_run && !opts.verify && e.mtime > 0 {
                // FILE_FLAG_BACKUP_SEMANTICS é necessário para abrir uma pasta no Windows
                use std::os::windows::fs::OpenOptionsExt;
                if let Ok(f) = fs::OpenOptions::new().write(true).custom_flags(0x0200_0000).open(&target) {
                    let _ = f.set_modified(UNIX_EPOCH + Duration::from_secs(e.mtime as u64));
                }
            }
        }
        Kind::File => {
            if st.cancelled() {
                return;
            }
            let target = dest_dir.join(sanitize_name(&e.name));
            if !opts.overwrite && !opts.verify {
                if let Ok(md) = fs::metadata(&target) {
                    if md.is_file() && md.len() == e.size {
                        st.skipped += 1;
                        if opts.verbose && !st.quiet {
                            st.clear_progress();
                            eprintln!("(já existe) {}", src_path);
                        }
                        return;
                    }
                }
            }
            if opts.verbose && !st.quiet {
                st.clear_progress();
                eprintln!("{} ({}{})", src_path, fmt_size(e.size), if e.compressed { ", comprimido" } else { "" });
            }
            st.progress(&e.name, false);
            if opts.dry_run {
                st.files += 1;
                st.bytes += e.size;
                return;
            }
            let cancel = st.cancel.clone();
            let result = catch_unwind(AssertUnwindSafe(|| copy_file(fs, e, &target, opts.verify, cancel.as_ref())));
            match result {
                Ok(Ok(n)) => {
                    st.files += 1;
                    st.bytes += n;
                    if e.compressed {
                        st.compressed += 1;
                    }
                    if e.mtime > 0 && !opts.verify {
                        if let Ok(f) = fs::File::options().write(true).open(&target) {
                            let _ = f.set_modified(UNIX_EPOCH + Duration::from_secs(e.mtime as u64));
                        }
                    }
                }
                Ok(Err(err)) => {
                    if !opts.verify {
                        let _ = fs::remove_file(&target);
                    }
                    if err.kind() != io::ErrorKind::Interrupted {
                        st.error(src_path, &err.to_string());
                    }
                }
                Err(_) => {
                    if !opts.verify {
                        let _ = fs::remove_file(&target);
                    }
                    st.error(src_path, "erro interno ao ler o arquivo (metadados corrompidos?)");
                }
            }
        }
        Kind::Symlink => {
            st.symlinks += 1;
            let target = fs.read_link(e).unwrap_or_else(|_| "?".into());
            if opts.symlinks && !opts.dry_run {
                let p = dest_dir.join(format!("{}.symlink.txt", sanitize_name(&e.name)));
                let _ = fs::write(&p, format!("link simbólico para: {}\r\n", target));
            }
            st.note(&format!("[LINK] {} -> {}", src_path, target));
        }
        Kind::Other => {
            st.note(&format!("[IGNORADO] {} (tipo especial)", src_path));
        }
    }
}

fn copy_file(fs: &dyn FileSystem, e: &Entry, target: &Path, verify: bool, cancel: Option<&Arc<AtomicBool>>) -> io::Result<u64> {
    let mut reader = fs.open(e)?;
    let size = reader.size();
    let mut out: Box<dyn Write> = if verify { Box::new(io::sink()) } else { Box::new(fs::File::create(target)?) };
    let mut buf = vec![0u8; 1024 * 1024];
    let mut off = 0u64;
    while off < size {
        if cancel.map_or(false, |c| c.load(Ordering::Relaxed)) {
            return Err(io::Error::new(io::ErrorKind::Interrupted, "cancelado"));
        }
        let n = reader.read_at(off, &mut buf)?;
        if n == 0 {
            break;
        }
        out.write_all(&buf[..n])?;
        off += n as u64;
    }
    if off != size {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            format!("arquivo terminou em {} bytes, esperado {}", off, size),
        ));
    }
    out.flush()?;
    Ok(off)
}
