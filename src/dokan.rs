//! Montagem de um volume como unidade do Windows (letra X:) usando o driver Dokan 2.
//! A biblioteca dokan2.dll é carregada em tempo de execução: se não existir, a montagem
//! não está disponível, mas o resto do programa funciona normalmente.

use std::ffi::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, Once};
use std::thread;
use std::time::Duration;

use winapi::shared::minwindef::FILETIME;
use winapi::um::fileapi::{GetLogicalDrives, BY_HANDLE_FILE_INFORMATION};
use winapi::um::libloaderapi::{GetProcAddress, LoadLibraryW};
use winapi::um::minwinbase::WIN32_FIND_DATAW;

use crate::fs::Kind;
use crate::fsservice::{FsService, Item};

const VOLUME_SD_MAX: usize = 1024 * 16;
const DOKAN_VERSION: u16 = 231;

const OPTION_WRITE_PROTECT: u32 = 1 << 3;
const OPTION_MOUNT_MANAGER: u32 = 1 << 6;

const STATUS_SUCCESS: i32 = 0;
const STATUS_ACCESS_DENIED: i32 = 0xC000_0022u32 as i32;
const STATUS_OBJECT_NAME_NOT_FOUND: i32 = 0xC000_0034u32 as i32;
const STATUS_OBJECT_NAME_COLLISION: i32 = 0xC000_0035u32 as i32;
const STATUS_NOT_A_DIRECTORY: i32 = 0xC000_0103u32 as i32;
const STATUS_FILE_IS_A_DIRECTORY: i32 = 0xC000_00BAu32 as i32;
const STATUS_INTERNAL_ERROR: i32 = 0xC000_00E5u32 as i32;
const STATUS_IO_DEVICE_ERROR: i32 = 0xC000_0185u32 as i32;

const FILE_ATTRIBUTE_READONLY: u32 = 0x01;
const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10;
const FILE_ATTRIBUTE_ARCHIVE: u32 = 0x20;

// CreateDisposition (NT)
const FILE_SUPERSEDE: u32 = 0;
const FILE_OPEN: u32 = 1;
const FILE_CREATE: u32 = 2;
const FILE_OPEN_IF: u32 = 3;
const FILE_OVERWRITE: u32 = 4;
const FILE_OVERWRITE_IF: u32 = 5;
const FILE_DIRECTORY_FILE: u32 = 0x2;
const FILE_NON_DIRECTORY_FILE: u32 = 0x40;
const WRITE_ACCESS_MASK: u32 = 0x2 | 0x4 | 0x10 | 0x100 | 0x1_0000 | 0x4_0000 | 0x8_0000 | 0x4000_0000 | 0x1000_0000;

#[repr(C)]
pub struct DokanOptions {
    version: u16,
    single_thread: u8,
    options: u32,
    global_context: u64,
    mount_point: *const u16,
    unc_name: *const u16,
    timeout: u32,
    allocation_unit_size: u32,
    sector_size: u32,
    volume_security_descriptor_length: u32,
    volume_security_descriptor: [u8; VOLUME_SD_MAX],
}

#[repr(C)]
pub struct DokanFileInfo {
    context: u64,
    dokan_context: u64,
    dokan_options: *const DokanOptions,
    processing_context: *mut c_void,
    process_id: u32,
    is_directory: u8,
    delete_pending: u8,
    paging_io: u8,
    synchronous_io: u8,
    nocache: u8,
    write_to_end_of_file: u8,
}

type FillFindData = unsafe extern "system" fn(*mut WIN32_FIND_DATAW, *mut DokanFileInfo) -> i32;
type CbName = Option<unsafe extern "system" fn(*const u16, *mut DokanFileInfo) -> i32>;
type CbNameVoid = Option<unsafe extern "system" fn(*const u16, *mut DokanFileInfo)>;
type CbNameI64 = Option<unsafe extern "system" fn(*const u16, i64, *mut DokanFileInfo) -> i32>;
type CbLock = Option<unsafe extern "system" fn(*const u16, i64, i64, *mut DokanFileInfo) -> i32>;

#[repr(C)]
struct DokanOperations {
    zw_create_file: Option<unsafe extern "system" fn(*const u16, *mut c_void, u32, u32, u32, u32, u32, *mut DokanFileInfo) -> i32>,
    cleanup: CbNameVoid,
    close_file: CbNameVoid,
    read_file: Option<unsafe extern "system" fn(*const u16, *mut c_void, u32, *mut u32, i64, *mut DokanFileInfo) -> i32>,
    write_file: Option<unsafe extern "system" fn(*const u16, *const c_void, u32, *mut u32, i64, *mut DokanFileInfo) -> i32>,
    flush_file_buffers: CbName,
    get_file_information: Option<unsafe extern "system" fn(*const u16, *mut BY_HANDLE_FILE_INFORMATION, *mut DokanFileInfo) -> i32>,
    find_files: Option<unsafe extern "system" fn(*const u16, FillFindData, *mut DokanFileInfo) -> i32>,
    find_files_with_pattern: Option<unsafe extern "system" fn(*const u16, *const u16, FillFindData, *mut DokanFileInfo) -> i32>,
    set_file_attributes: Option<unsafe extern "system" fn(*const u16, u32, *mut DokanFileInfo) -> i32>,
    set_file_time: Option<unsafe extern "system" fn(*const u16, *const FILETIME, *const FILETIME, *const FILETIME, *mut DokanFileInfo) -> i32>,
    delete_file: CbName,
    delete_directory: CbName,
    move_file: Option<unsafe extern "system" fn(*const u16, *const u16, u8, *mut DokanFileInfo) -> i32>,
    set_end_of_file: CbNameI64,
    set_allocation_size: CbNameI64,
    lock_file: CbLock,
    unlock_file: CbLock,
    get_disk_free_space: Option<unsafe extern "system" fn(*mut u64, *mut u64, *mut u64, *mut DokanFileInfo) -> i32>,
    get_volume_information: Option<unsafe extern "system" fn(*mut u16, u32, *mut u32, *mut u32, *mut u32, *mut u16, u32, *mut DokanFileInfo) -> i32>,
    mounted: Option<unsafe extern "system" fn(*const u16, *mut DokanFileInfo) -> i32>,
    unmounted: Option<unsafe extern "system" fn(*mut DokanFileInfo) -> i32>,
    get_file_security: Option<unsafe extern "system" fn(*const u16, *mut u32, *mut c_void, u32, *mut u32, *mut DokanFileInfo) -> i32>,
    set_file_security: Option<unsafe extern "system" fn(*const u16, *mut u32, *mut c_void, u32, *mut DokanFileInfo) -> i32>,
    find_streams: Option<unsafe extern "system" fn(*const u16, *mut c_void, *mut c_void, *mut DokanFileInfo) -> i32>,
}

#[derive(Clone, Copy)]
struct Api {
    init: unsafe extern "system" fn(),
    main: unsafe extern "system" fn(*mut DokanOptions, *mut DokanOperations) -> i32,
    remove_mount_point: unsafe extern "system" fn(*const u16) -> i32,
    version: unsafe extern "system" fn() -> u32,
    driver_version: unsafe extern "system" fn() -> u32,
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn load_api() -> Result<Api, String> {
    unsafe {
        let name = wide("dokan2.dll");
        let lib = LoadLibraryW(name.as_ptr());
        if lib.is_null() {
            return Err("o driver Dokan (dokan2.dll) não está instalado".into());
        }
        macro_rules! proc {
            ($n:expr, $t:ty) => {{
                let p = GetProcAddress(lib, concat!($n, "\0").as_ptr() as *const i8);
                if p.is_null() {
                    return Err(format!("função {} não encontrada em dokan2.dll", $n));
                }
                std::mem::transmute::<_, $t>(p)
            }};
        }
        Ok(Api {
            init: proc!("DokanInit", unsafe extern "system" fn()),
            main: proc!("DokanMain", unsafe extern "system" fn(*mut DokanOptions, *mut DokanOperations) -> i32),
            remove_mount_point: proc!("DokanRemoveMountPoint", unsafe extern "system" fn(*const u16) -> i32),
            version: proc!("DokanVersion", unsafe extern "system" fn() -> u32),
            driver_version: proc!("DokanDriverVersion", unsafe extern "system" fn() -> u32),
        })
    }
}

/// Versão do Dokan instalado (biblioteca, driver), ou erro explicando o que falta.
pub fn available() -> Result<(u32, u32), String> {
    let api = load_api()?;
    let (v, d) = unsafe { ((api.version)(), (api.driver_version)()) };
    if d == 0 {
        return Err("o driver Dokan está instalado mas não está carregado (reinicie o Windows ou reinstale o Dokan)".into());
    }
    Ok((v, d))
}

/// Letras de unidade livres, de D: a Z:.
pub fn free_letters() -> Vec<char> {
    let mask = unsafe { GetLogicalDrives() };
    (b'D'..=b'Z').filter(|l| mask & (1 << (l - b'A')) == 0).map(|l| l as char).collect()
}

struct MountState {
    svc: Arc<FsService>,
    label: Vec<u16>,
    fs_name: Vec<u16>,
    serial: u32,
    total: u64,
    mounted_tx: Mutex<Option<mpsc::Sender<()>>>,
}

pub struct Mount {
    pub letter: char,
    thread: Option<thread::JoinHandle<i32>>,
    api: Api,
}

fn unix_to_filetime(secs: i64) -> FILETIME {
    let t = if secs <= 0 { 0u64 } else { (secs as u64 + 11_644_473_600) * 10_000_000 };
    FILETIME { dwLowDateTime: t as u32, dwHighDateTime: (t >> 32) as u32 }
}

unsafe fn state<'a>(info: *mut DokanFileInfo) -> Option<&'a MountState> {
    if info.is_null() || (*info).dokan_options.is_null() {
        return None;
    }
    let p = (*(*info).dokan_options).global_context as *const MountState;
    if p.is_null() {
        None
    } else {
        Some(&*p)
    }
}

unsafe fn path_of(name: *const u16) -> String {
    if name.is_null() {
        return "/".into();
    }
    let mut len = 0;
    while *name.add(len) != 0 {
        len += 1;
    }
    let s = String::from_utf16_lossy(std::slice::from_raw_parts(name, len));
    s.replace('\\', "/")
}

fn attrs_of(item: &Item) -> u32 {
    if item.entry.kind == Kind::Dir {
        FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_READONLY
    } else {
        FILE_ATTRIBUTE_ARCHIVE | FILE_ATTRIBUTE_READONLY
    }
}

fn guard<F: FnOnce() -> i32>(f: F) -> i32 {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(v) => v,
        Err(_) => STATUS_INTERNAL_ERROR,
    }
}

unsafe extern "system" fn cb_create(
    name: *const u16,
    _sec: *mut c_void,
    access: u32,
    _attrs: u32,
    _share: u32,
    disposition: u32,
    options: u32,
    info: *mut DokanFileInfo,
) -> i32 {
    guard(|| {
        let st = match state(info) {
            Some(s) => s,
            None => return STATUS_INTERNAL_ERROR,
        };
        let path = path_of(name);
        let item = match st.svc.resolve(&path) {
            Ok(i) => i,
            Err(_) => return STATUS_IO_DEVICE_ERROR,
        };
        match item {
            None => {
                if matches!(disposition, FILE_CREATE | FILE_OPEN_IF | FILE_OVERWRITE_IF) {
                    STATUS_ACCESS_DENIED
                } else {
                    STATUS_OBJECT_NAME_NOT_FOUND
                }
            }
            Some(it) => {
                if disposition == FILE_CREATE {
                    return STATUS_OBJECT_NAME_COLLISION;
                }
                if matches!(disposition, FILE_SUPERSEDE | FILE_OVERWRITE | FILE_OVERWRITE_IF) {
                    return STATUS_ACCESS_DENIED;
                }
                let _ = FILE_OPEN;
                if it.entry.kind == Kind::Dir {
                    if options & FILE_NON_DIRECTORY_FILE != 0 {
                        return STATUS_FILE_IS_A_DIRECTORY;
                    }
                    (*info).is_directory = 1;
                } else if options & FILE_DIRECTORY_FILE != 0 {
                    return STATUS_NOT_A_DIRECTORY;
                }
                if access & WRITE_ACCESS_MASK != 0 {
                    return STATUS_ACCESS_DENIED;
                }
                STATUS_SUCCESS
            }
        }
    })
}

unsafe extern "system" fn cb_cleanup(_name: *const u16, _info: *mut DokanFileInfo) {}
unsafe extern "system" fn cb_close(_name: *const u16, _info: *mut DokanFileInfo) {}

unsafe extern "system" fn cb_read(name: *const u16, buffer: *mut c_void, len: u32, read: *mut u32, offset: i64, info: *mut DokanFileInfo) -> i32 {
    guard(|| {
        let st = match state(info) {
            Some(s) => s,
            None => return STATUS_INTERNAL_ERROR,
        };
        if !read.is_null() {
            *read = 0;
        }
        if offset < 0 || buffer.is_null() {
            return STATUS_SUCCESS;
        }
        let path = path_of(name);
        match st.svc.read(&path, offset as u64, len as usize) {
            Ok(data) => {
                let n = data.len().min(len as usize);
                std::ptr::copy_nonoverlapping(data.as_ptr(), buffer as *mut u8, n);
                if !read.is_null() {
                    *read = n as u32;
                }
                STATUS_SUCCESS
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    STATUS_OBJECT_NAME_NOT_FOUND
                } else {
                    STATUS_IO_DEVICE_ERROR
                }
            }
        }
    })
}

unsafe fn fill_info(item: &Item, serial: u32, out: &mut BY_HANDLE_FILE_INFORMATION) {
    let e = &item.entry;
    out.dwFileAttributes = attrs_of(item);
    out.ftCreationTime = unix_to_filetime(if e.crtime > 0 { e.crtime } else { e.mtime });
    out.ftLastAccessTime = unix_to_filetime(e.mtime);
    out.ftLastWriteTime = unix_to_filetime(e.mtime);
    out.dwVolumeSerialNumber = serial;
    let size = if e.kind == Kind::Dir { 0 } else { e.size };
    out.nFileSizeHigh = (size >> 32) as u32;
    out.nFileSizeLow = size as u32;
    out.nNumberOfLinks = 1;
    out.nFileIndexHigh = (e.id >> 32) as u32;
    out.nFileIndexLow = e.id as u32;
}

unsafe extern "system" fn cb_get_info(name: *const u16, buffer: *mut BY_HANDLE_FILE_INFORMATION, info: *mut DokanFileInfo) -> i32 {
    guard(|| {
        let st = match state(info) {
            Some(s) => s,
            None => return STATUS_INTERNAL_ERROR,
        };
        if buffer.is_null() {
            return STATUS_INTERNAL_ERROR;
        }
        let path = path_of(name);
        match st.svc.resolve(&path) {
            Ok(Some(it)) => {
                fill_info(&it, st.serial, &mut *buffer);
                STATUS_SUCCESS
            }
            Ok(None) => STATUS_OBJECT_NAME_NOT_FOUND,
            Err(_) => STATUS_IO_DEVICE_ERROR,
        }
    })
}

unsafe fn find_data(name: &str, attrs: u32, size: u64, crtime: i64, mtime: i64) -> WIN32_FIND_DATAW {
    let mut fd: WIN32_FIND_DATAW = std::mem::zeroed();
    fd.dwFileAttributes = attrs;
    fd.ftCreationTime = unix_to_filetime(if crtime > 0 { crtime } else { mtime });
    fd.ftLastAccessTime = unix_to_filetime(mtime);
    fd.ftLastWriteTime = unix_to_filetime(mtime);
    fd.nFileSizeHigh = (size >> 32) as u32;
    fd.nFileSizeLow = size as u32;
    let units: Vec<u16> = name.encode_utf16().take(259).collect();
    fd.cFileName[..units.len()].copy_from_slice(&units);
    fd
}

unsafe extern "system" fn cb_find_files(name: *const u16, fill: FillFindData, info: *mut DokanFileInfo) -> i32 {
    guard(|| {
        let st = match state(info) {
            Some(s) => s,
            None => return STATUS_INTERNAL_ERROR,
        };
        let path = path_of(name);
        let items = match st.svc.list(&path) {
            Ok(v) => v,
            Err(e) => {
                return if e.kind() == std::io::ErrorKind::NotFound { STATUS_OBJECT_NAME_NOT_FOUND } else { STATUS_IO_DEVICE_ERROR }
            }
        };
        if path.trim_matches('/').len() > 0 {
            let mut d = find_data(".", FILE_ATTRIBUTE_DIRECTORY, 0, 0, 0);
            fill(&mut d, info);
            let mut d = find_data("..", FILE_ATTRIBUTE_DIRECTORY, 0, 0, 0);
            fill(&mut d, info);
        }
        for it in &items {
            let size = if it.entry.kind == Kind::Dir { 0 } else { it.entry.size };
            let mut d = find_data(&it.name, attrs_of(it), size, it.entry.crtime, it.entry.mtime);
            fill(&mut d, info);
        }
        STATUS_SUCCESS
    })
}

unsafe extern "system" fn cb_disk_free(free: *mut u64, total: *mut u64, total_free: *mut u64, info: *mut DokanFileInfo) -> i32 {
    guard(|| {
        let st = match state(info) {
            Some(s) => s,
            None => return STATUS_INTERNAL_ERROR,
        };
        if !free.is_null() {
            *free = 0;
        }
        if !total.is_null() {
            *total = st.total;
        }
        if !total_free.is_null() {
            *total_free = 0;
        }
        STATUS_SUCCESS
    })
}

unsafe fn copy_wide(src: &[u16], dst: *mut u16, cap: u32) {
    if dst.is_null() || cap == 0 {
        return;
    }
    let n = src.len().min(cap as usize - 1);
    std::ptr::copy_nonoverlapping(src.as_ptr(), dst, n);
    *dst.add(n) = 0;
}

unsafe extern "system" fn cb_volume_info(
    name_buf: *mut u16,
    name_cap: u32,
    serial: *mut u32,
    max_comp: *mut u32,
    flags: *mut u32,
    fs_buf: *mut u16,
    fs_cap: u32,
    info: *mut DokanFileInfo,
) -> i32 {
    guard(|| {
        let st = match state(info) {
            Some(s) => s,
            None => return STATUS_INTERNAL_ERROR,
        };
        copy_wide(&st.label, name_buf, name_cap);
        copy_wide(&st.fs_name, fs_buf, fs_cap);
        if !serial.is_null() {
            *serial = st.serial;
        }
        if !max_comp.is_null() {
            *max_comp = 255;
        }
        if !flags.is_null() {
            // FILE_CASE_PRESERVED_NAMES | FILE_UNICODE_ON_DISK | FILE_READ_ONLY_VOLUME
            *flags = 0x2 | 0x4 | 0x8_0000;
        }
        STATUS_SUCCESS
    })
}

unsafe extern "system" fn cb_mounted(_mp: *const u16, info: *mut DokanFileInfo) -> i32 {
    guard(|| {
        if let Some(st) = state(info) {
            if let Ok(mut g) = st.mounted_tx.lock() {
                if let Some(tx) = g.take() {
                    let _ = tx.send(());
                }
            }
        }
        STATUS_SUCCESS
    })
}

unsafe extern "system" fn cb_unmounted(_info: *mut DokanFileInfo) -> i32 {
    STATUS_SUCCESS
}

fn error_text(code: i32) -> String {
    match code {
        0 => "ok".into(),
        -1 => "erro genérico do Dokan".into(),
        -2 => "letra de unidade inválida ou já em uso".into(),
        -3 => "o driver Dokan não está instalado".into(),
        -4 => "o driver Dokan não pôde ser iniciado".into(),
        -5 => "o Dokan não conseguiu montar a unidade".into(),
        -6 => "ponto de montagem inválido".into(),
        -7 => "versão do Dokan incompatível (instale o Dokan 2.x mais recente)".into(),
        other => format!("erro Dokan {}", other),
    }
}

static INIT: Once = Once::new();

/// Monta o serviço de sistema de arquivos como a unidade `letter` (somente leitura).
pub fn mount(svc: Arc<FsService>, letter: char, admin: bool) -> Result<Mount, String> {
    let api = load_api()?;
    let (lib_ver, drv_ver) = unsafe { ((api.version)(), (api.driver_version)()) };
    if drv_ver == 0 {
        return Err("o driver Dokan está instalado mas não está carregado (reinicie o Windows ou reinstale o Dokan)".into());
    }
    if lib_ver < 200 {
        return Err(format!("o Dokan instalado ({}) é antigo demais; é preciso o Dokan 2.x", lib_ver));
    }
    INIT.call_once(|| unsafe { (api.init)() });

    let info = svc.info.clone();
    let mut serial = 0x4D41_4352u32; // "MACR"
    for b in info.label.bytes() {
        serial = serial.wrapping_mul(31).wrapping_add(b as u32);
    }
    let label = if info.label.is_empty() { format!("macread {}", info.fs_type) } else { info.label.clone() };
    let (tx, rx) = mpsc::channel::<()>();
    let state = Arc::new(MountState {
        svc,
        label: label.encode_utf16().take(32).collect(),
        fs_name: info.fs_type.encode_utf16().take(31).collect(),
        serial,
        total: info.total_bytes,
        mounted_tx: Mutex::new(Some(tx)),
    });
    let (res_tx, res_rx) = mpsc::channel::<i32>();
    let mount_point = wide(&format!("{}:\\", letter));
    let mount_point_main = mount_point.clone();
    let version = lib_ver.min(DOKAN_VERSION as u32) as u16;
    let state_ptr = Arc::into_raw(state.clone()) as u64;
    let t = thread::Builder::new()
        .name("macread-dokan".into())
        .spawn(move || {
            let mut opts = DokanOptions {
                version,
                single_thread: 0,
                options: OPTION_WRITE_PROTECT | if admin { OPTION_MOUNT_MANAGER } else { 0 },
                global_context: state_ptr,
                mount_point: mount_point.as_ptr(),
                unc_name: std::ptr::null(),
                timeout: 120_000,
                allocation_unit_size: 4096,
                sector_size: 512,
                volume_security_descriptor_length: 0,
                volume_security_descriptor: [0u8; VOLUME_SD_MAX],
            };
            let mut ops = DokanOperations {
                zw_create_file: Some(cb_create),
                cleanup: Some(cb_cleanup),
                close_file: Some(cb_close),
                read_file: Some(cb_read),
                write_file: None,
                flush_file_buffers: None,
                get_file_information: Some(cb_get_info),
                find_files: Some(cb_find_files),
                find_files_with_pattern: None,
                set_file_attributes: None,
                set_file_time: None,
                delete_file: None,
                delete_directory: None,
                move_file: None,
                set_end_of_file: None,
                set_allocation_size: None,
                lock_file: None,
                unlock_file: None,
                get_disk_free_space: Some(cb_disk_free),
                get_volume_information: Some(cb_volume_info),
                mounted: Some(cb_mounted),
                unmounted: Some(cb_unmounted),
                get_file_security: None,
                set_file_security: None,
                find_streams: None,
            };
            let r = unsafe { (api.main)(&mut opts, &mut ops) };
            let _ = res_tx.send(r);
            unsafe { drop(Arc::from_raw(state_ptr as *const MountState)) };
            r
        })
        .map_err(|e| format!("não foi possível criar a thread de montagem: {}", e))?;

    // espera o callback Mounted ou o retorno (com erro) do DokanMain
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    loop {
        if rx.try_recv().is_ok() {
            return Ok(Mount { letter, thread: Some(t), api });
        }
        if let Ok(code) = res_rx.try_recv() {
            let _ = t.join();
            return Err(error_text(code));
        }
        if std::time::Instant::now() > deadline {
            unsafe { (api.remove_mount_point)(mount_point_main.as_ptr()) };
            let _ = t.join();
            return Err("tempo esgotado ao montar a unidade".into());
        }
        thread::sleep(Duration::from_millis(50));
    }
}

impl Mount {
    /// false quando a unidade já foi desmontada (por outro processo ou pelo Explorador).
    pub fn is_alive(&self) -> bool {
        self.thread.as_ref().map_or(false, |t| !t.is_finished())
    }

    pub fn unmount(mut self) {
        let mp = wide(&format!("{}:\\", self.letter));
        unsafe { (self.api.remove_mount_point)(mp.as_ptr()) };
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Drop for Mount {
    fn drop(&mut self) {
        if let Some(t) = self.thread.take() {
            let mp = wide(&format!("{}:\\", self.letter));
            unsafe { (self.api.remove_mount_point)(mp.as_ptr()) };
            let _ = t.join();
        }
    }
}

/// Desmonta uma unidade Dokan montada por qualquer processo.
pub fn unmount_letter(letter: char) -> Result<(), String> {
    let api = load_api()?;
    let mp = wide(&format!("{}:\\", letter));
    let ok = unsafe { (api.remove_mount_point)(mp.as_ptr()) };
    if ok != 0 {
        Ok(())
    } else {
        Err(format!("não foi possível desmontar {}:", letter))
    }
}
