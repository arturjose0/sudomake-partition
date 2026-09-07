//! Acesso a dispositivos de bloco: imagens em arquivo e discos físicos do Windows.

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::ffi::c_void;
use std::fs::{File, OpenOptions};
use std::io;
use std::os::windows::fs::{FileExt, OpenOptionsExt};
use std::os::windows::io::AsRawHandle;
use std::rc::Rc;

#[link(name = "kernel32")]
extern "system" {
    fn DeviceIoControl(
        hdevice: *mut c_void,
        dwiocontrolcode: u32,
        lpinbuffer: *const c_void,
        ninbuffersize: u32,
        lpoutbuffer: *mut c_void,
        noutbuffersize: u32,
        lpbytesreturned: *mut u32,
        lpoverlapped: *mut c_void,
    ) -> i32;
}

const IOCTL_DISK_GET_LENGTH_INFO: u32 = 0x0007_405C;
const IOCTL_DISK_GET_DRIVE_GEOMETRY_EX: u32 = 0x0007_00A0;
const IOCTL_STORAGE_QUERY_PROPERTY: u32 = 0x002D_1400;
const FILE_SHARE_READ_WRITE: u32 = 0x1 | 0x2;

pub trait BlockDevice {
    fn size(&self) -> u64;
    fn read_at(&self, off: u64, buf: &mut [u8]) -> io::Result<()>;
    fn describe(&self) -> String;
}

#[allow(dead_code)]
pub struct DiskInfo {
    pub size: u64,
    pub sector: u32,
    pub model: String,
    pub serial: String,
    pub bus: String,
}

pub struct FileDevice {
    file: File,
    size: u64,
    align: u64,
    name: String,
}

#[allow(dead_code)]
fn _describe_used(d: &dyn BlockDevice) -> String {
    d.describe()
}

fn ioctl(f: &File, code: u32, inp: &[u8], out: &mut [u8]) -> io::Result<usize> {
    let mut ret: u32 = 0;
    let inp_ptr = if inp.is_empty() { std::ptr::null() } else { inp.as_ptr() as *const c_void };
    let ok = unsafe {
        DeviceIoControl(
            f.as_raw_handle() as *mut c_void,
            code,
            inp_ptr,
            inp.len() as u32,
            out.as_mut_ptr() as *mut c_void,
            out.len() as u32,
            &mut ret,
            std::ptr::null_mut(),
        )
    };
    if ok == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(ret as usize)
    }
}

fn bus_name(b: u32) -> &'static str {
    match b {
        1 => "SCSI",
        2 => "ATAPI",
        3 => "ATA",
        4 => "FireWire",
        7 => "USB",
        8 => "RAID",
        9 => "iSCSI",
        10 => "SAS",
        11 => "SATA",
        12 => "SD",
        13 => "MMC",
        14 | 15 => "Virtual",
        16 => "Storage Spaces",
        17 => "NVMe",
        18 => "SCM",
        19 => "UFS",
        _ => "?",
    }
}

pub fn query_disk(f: &File) -> io::Result<DiskInfo> {
    let mut sector = 512u32;
    let mut size = 0u64;
    let mut geo = [0u8; 256];
    if ioctl(f, IOCTL_DISK_GET_DRIVE_GEOMETRY_EX, &[], &mut geo).is_ok() {
        sector = crate::util::le32(&geo, 20);
        size = crate::util::le64(&geo, 24);
    }
    let mut len = [0u8; 8];
    if ioctl(f, IOCTL_DISK_GET_LENGTH_INFO, &[], &mut len).is_ok() {
        size = crate::util::le64(&len, 0);
    }
    if size == 0 {
        return Err(io::Error::new(io::ErrorKind::Other, "não foi possível obter o tamanho do disco"));
    }
    let mut model = String::new();
    let mut serial = String::new();
    let mut bus = String::from("?");
    let query = [0u8; 12]; // StorageDeviceProperty, PropertyStandardQuery
    let mut out = vec![0u8; 4096];
    if let Ok(n) = ioctl(f, IOCTL_STORAGE_QUERY_PROPERTY, &query, &mut out) {
        let out = &out[..n.max(40).min(out.len())];
        let get = |off: u32| -> String {
            let o = off as usize;
            if o == 0 || o >= out.len() {
                String::new()
            } else {
                crate::util::cstr(&out[o..]).trim().to_string()
            }
        };
        if out.len() >= 36 {
            let vendor = get(crate::util::le32(out, 12));
            let product = get(crate::util::le32(out, 16));
            serial = get(crate::util::le32(out, 24));
            bus = bus_name(crate::util::le32(out, 28)).to_string();
            model = format!("{} {}", vendor, product).trim().to_string();
        }
    }
    if sector == 0 || !sector.is_power_of_two() {
        sector = 512;
    }
    Ok(DiskInfo { size, sector, model, serial, bus })
}

impl FileDevice {
    pub fn open_image(path: &str) -> io::Result<FileDevice> {
        let file = File::open(path)?;
        let size = file.metadata()?.len();
        Ok(FileDevice { file, size, align: 1, name: path.to_string() })
    }

    pub fn open_physical(n: u32) -> io::Result<FileDevice> {
        Self::open_device(&format!(r"\\.\PhysicalDrive{}", n))
    }

    pub fn open_device(path: &str) -> io::Result<FileDevice> {
        let file = OpenOptions::new().read(true).share_mode(FILE_SHARE_READ_WRITE).open(path)?;
        let info = query_disk(&file)?;
        Ok(FileDevice { file, size: info.size, align: info.sector.max(512) as u64, name: path.to_string() })
    }
}

fn read_exact_at(f: &File, mut off: u64, mut buf: &mut [u8]) -> io::Result<()> {
    while !buf.is_empty() {
        let n = f.seek_read(buf, off)?;
        if n == 0 {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "fim inesperado do dispositivo"));
        }
        off += n as u64;
        buf = &mut buf[n..];
    }
    Ok(())
}

impl BlockDevice for FileDevice {
    fn size(&self) -> u64 {
        self.size
    }
    fn describe(&self) -> String {
        self.name.clone()
    }
    fn read_at(&self, off: u64, buf: &mut [u8]) -> io::Result<()> {
        if buf.is_empty() {
            return Ok(());
        }
        let end = off
            .checked_add(buf.len() as u64)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "offset inválido"))?;
        if end > self.size {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                format!("leitura fora do dispositivo (offset {} + {} > {})", off, buf.len(), self.size),
            ));
        }
        if self.align <= 1 {
            return read_exact_at(&self.file, off, buf);
        }
        let a = self.align;
        let start = off / a * a;
        let mut aend = (end + a - 1) / a * a;
        if aend > self.size {
            aend = self.size;
        }
        if start == off && aend == end {
            return read_exact_at(&self.file, off, buf);
        }
        let mut tmp = vec![0u8; (aend - start) as usize];
        read_exact_at(&self.file, start, &mut tmp)?;
        let s = (off - start) as usize;
        buf.copy_from_slice(&tmp[s..s + buf.len()]);
        Ok(())
    }
}

/// Janela (partição) dentro de outro dispositivo.
pub struct SubDevice {
    base: Rc<dyn BlockDevice>,
    off: u64,
    len: u64,
}

impl SubDevice {
    pub fn new(base: Rc<dyn BlockDevice>, off: u64, len: u64) -> SubDevice {
        let len = len.min(base.size().saturating_sub(off));
        SubDevice { base, off, len }
    }
}

impl BlockDevice for SubDevice {
    fn size(&self) -> u64 {
        self.len
    }
    fn describe(&self) -> String {
        format!("{} @ {}", self.base.describe(), self.off)
    }
    fn read_at(&self, off: u64, buf: &mut [u8]) -> io::Result<()> {
        if off + buf.len() as u64 > self.len {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "leitura fora da partição"));
        }
        self.base.read_at(self.off + off, buf)
    }
}

/// Dispositivo composto por trechos de outro (volume lógico LVM): (offset aqui, offset na base, tamanho).
pub struct MappedDevice {
    base: Rc<dyn BlockDevice>,
    runs: Vec<(u64, u64, u64)>,
    size: u64,
    name: String,
}

impl MappedDevice {
    pub fn new(base: Rc<dyn BlockDevice>, runs: Vec<(u64, u64, u64)>, size: u64, name: String) -> MappedDevice {
        let mut runs = runs;
        runs.sort_by_key(|r| r.0);
        MappedDevice { base, runs, size, name }
    }
}

impl BlockDevice for MappedDevice {
    fn size(&self) -> u64 {
        self.size
    }
    fn describe(&self) -> String {
        format!("{} (LVM {})", self.base.describe(), self.name)
    }
    fn read_at(&self, off: u64, buf: &mut [u8]) -> io::Result<()> {
        if off + buf.len() as u64 > self.size {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "leitura fora do volume lógico"));
        }
        buf.fill(0);
        let end = off + buf.len() as u64;
        let start_i = self.runs.partition_point(|r| r.0 + r.2 <= off);
        for r in &self.runs[start_i..] {
            if r.0 >= end {
                break;
            }
            let s = r.0.max(off);
            let t = (r.0 + r.2).min(end);
            if t <= s {
                continue;
            }
            self.base.read_at(r.1 + (s - r.0), &mut buf[(s - off) as usize..(t - off) as usize])?;
        }
        Ok(())
    }
}

/// Cache de páginas para acelerar a leitura de metadados (árvores B).
pub struct CachedDevice {
    base: Rc<dyn BlockDevice>,
    page: u64,
    cap: usize,
    cache: RefCell<(HashMap<u64, Rc<Vec<u8>>>, VecDeque<u64>)>,
}

impl CachedDevice {
    pub fn new(base: Rc<dyn BlockDevice>) -> CachedDevice {
        CachedDevice { base, page: 8192, cap: 4096, cache: RefCell::new((HashMap::new(), VecDeque::new())) }
    }

    fn page_data(&self, idx: u64) -> io::Result<Rc<Vec<u8>>> {
        if let Some(p) = self.cache.borrow().0.get(&idx) {
            return Ok(p.clone());
        }
        let start = idx * self.page;
        let len = self.page.min(self.base.size().saturating_sub(start));
        let mut v = vec![0u8; len as usize];
        self.base.read_at(start, &mut v)?;
        let rc = Rc::new(v);
        let mut c = self.cache.borrow_mut();
        if c.0.len() >= self.cap {
            if let Some(old) = c.1.pop_front() {
                c.0.remove(&old);
            }
        }
        c.0.insert(idx, rc.clone());
        c.1.push_back(idx);
        Ok(rc)
    }
}

impl BlockDevice for CachedDevice {
    fn size(&self) -> u64 {
        self.base.size()
    }
    fn describe(&self) -> String {
        self.base.describe()
    }
    fn read_at(&self, off: u64, buf: &mut [u8]) -> io::Result<()> {
        if buf.len() as u64 > 64 * 1024 {
            return self.base.read_at(off, buf);
        }
        if off + buf.len() as u64 > self.base.size() {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "leitura fora do dispositivo"));
        }
        let mut done = 0usize;
        while done < buf.len() {
            let cur = off + done as u64;
            let idx = cur / self.page;
            let within = (cur % self.page) as usize;
            let p = self.page_data(idx)?;
            if within >= p.len() {
                return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "leitura fora do dispositivo"));
            }
            let n = (buf.len() - done).min(p.len() - within);
            buf[done..done + n].copy_from_slice(&p[within..within + n]);
            done += n;
        }
        Ok(())
    }
}

#[allow(dead_code)]
pub struct DiskEntry {
    pub number: u32,
    pub info: Option<DiskInfo>,
    pub readable: bool,
    pub error: Option<String>,
}

/// Enumera \\.\PhysicalDrive0..31.
pub fn enumerate_disks() -> Vec<DiskEntry> {
    let mut out = Vec::new();
    for n in 0..32u32 {
        let path = format!(r"\\.\PhysicalDrive{}", n);
        match OpenOptions::new().read(true).share_mode(FILE_SHARE_READ_WRITE).open(&path) {
            Ok(f) => {
                let info = query_disk(&f).ok();
                out.push(DiskEntry { number: n, info, readable: true, error: None });
            }
            Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
                let info = OpenOptions::new()
                    .access_mode(0)
                    .share_mode(FILE_SHARE_READ_WRITE)
                    .open(&path)
                    .ok()
                    .and_then(|f| query_disk(&f).ok());
                out.push(DiskEntry {
                    number: n,
                    info,
                    readable: false,
                    error: Some("sem permissão (execute como Administrador)".into()),
                });
            }
            Err(_) => {}
        }
    }
    out
}
