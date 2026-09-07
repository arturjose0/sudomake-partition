//! Funções utilitárias: leitura de inteiros, formatação e nomes de arquivo.

pub fn le16(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([b[o], b[o + 1]])
}
pub fn le32(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
pub fn le64(b: &[u8], o: usize) -> u64 {
    u64::from_le_bytes(b[o..o + 8].try_into().unwrap())
}
pub fn be16(b: &[u8], o: usize) -> u16 {
    u16::from_be_bytes([b[o], b[o + 1]])
}
pub fn be32(b: &[u8], o: usize) -> u32 {
    u32::from_be_bytes(b[o..o + 4].try_into().unwrap())
}
pub fn be64(b: &[u8], o: usize) -> u64 {
    u64::from_be_bytes(b[o..o + 8].try_into().unwrap())
}

pub fn err<E: Into<Box<dyn std::error::Error + Send + Sync>>>(msg: E) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, msg)
}

pub fn fmt_size(n: u64) -> String {
    const U: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];
    let mut v = n as f64;
    let mut i = 0;
    while v >= 1024.0 && i < U.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{} B", n)
    } else {
        format!("{:.1} {}", v, U[i])
    }
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[repr(C)]
#[derive(Default)]
struct SystemTime {
    year: u16,
    month: u16,
    day_of_week: u16,
    day: u16,
    hour: u16,
    minute: u16,
    second: u16,
    milliseconds: u16,
}

#[link(name = "kernel32")]
extern "system" {
    fn SystemTimeToTzSpecificLocalTime(
        tz: *const std::ffi::c_void,
        utc: *const SystemTime,
        local: *mut SystemTime,
    ) -> i32;
}

/// Formata segundos desde 1970 (UTC) como "AAAA-MM-DD HH:MM:SS" na hora local do Windows.
pub fn fmt_time(secs: i64) -> String {
    if secs <= 0 {
        return "-".to_string();
    }
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    let mut utc = SystemTime {
        year: y.clamp(1601, 30827) as u16,
        month: m as u16,
        day: d as u16,
        hour: (rem / 3600) as u16,
        minute: ((rem % 3600) / 60) as u16,
        second: (rem % 60) as u16,
        ..Default::default()
    };
    let mut local = SystemTime::default();
    let ok = unsafe { SystemTimeToTzSpecificLocalTime(std::ptr::null(), &utc, &mut local) };
    if ok != 0 {
        utc = local;
    }
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        utc.year, utc.month, utc.day, utc.hour, utc.minute, utc.second
    )
}

/// Converte um nome de arquivo do Mac para um nome válido no Windows (forma NFC, sem caracteres proibidos).
pub fn sanitize_name(name: &str) -> String {
    use unicode_normalization::UnicodeNormalization;
    let mut s: String = name
        .nfc()
        .map(|c| match c {
            '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if (c as u32) < 32 => '_',
            c => c,
        })
        .collect();
    while s.ends_with('.') || s.ends_with(' ') {
        s.pop();
    }
    if s.is_empty() {
        s.push('_');
    }
    const RESERVED: [&str; 22] = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    let stem = s.split('.').next().unwrap_or("").to_ascii_uppercase();
    if RESERVED.contains(&stem.as_str()) {
        s.insert(0, '_');
    }
    s
}

/// GUID no formato misto do Windows/GPT (3 primeiros campos little-endian).
pub fn guid_string(b: &[u8]) -> String {
    format!(
        "{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
        le32(b, 0),
        le16(b, 4),
        le16(b, 6),
        b[8],
        b[9],
        b[10],
        b[11],
        b[12],
        b[13],
        b[14],
        b[15]
    )
}

/// UUID em ordem de bytes direta (usado pelo APFS).
pub fn uuid_string(b: &[u8]) -> String {
    let h: Vec<String> = b.iter().map(|x| format!("{:02X}", x)).collect();
    format!(
        "{}-{}-{}-{}-{}",
        h[0..4].concat(),
        h[4..6].concat(),
        h[6..8].concat(),
        h[8..10].concat(),
        h[10..16].concat()
    )
}

pub fn mode_string(mode: u16, kind_char: char) -> String {
    let mut s = String::with_capacity(10);
    s.push(kind_char);
    let bits = ['r', 'w', 'x'];
    for i in (0..9).rev() {
        s.push(if mode & (1 << i) != 0 { bits[(8 - i) % 3] } else { '-' });
    }
    s
}

/// Decodifica UTF-16 (big ou little endian) com substituição de erros.
pub fn utf16_to_string(b: &[u8], big_endian: bool) -> String {
    let units: Vec<u16> = b
        .chunks_exact(2)
        .map(|c| if big_endian { u16::from_be_bytes([c[0], c[1]]) } else { u16::from_le_bytes([c[0], c[1]]) })
        .collect();
    char::decode_utf16(units.into_iter())
        .map(|r| r.unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect()
}

pub fn cstr(b: &[u8]) -> String {
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..end]).into_owned()
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize() {
        assert_eq!(sanitize_name("a:b/c"), "a_b_c");
        assert_eq!(sanitize_name("con"), "_con");
        assert_eq!(sanitize_name("arquivo. "), "arquivo");
        assert_eq!(sanitize_name("e\u{301}"), "\u{e9}"); // NFD -> NFC
    }

    #[test]
    fn dates() {
        assert!(fmt_time(0).starts_with('-'));
        assert!(fmt_time(1_642_148_381).starts_with("2022-01-14"));
    }

    #[test]
    fn guid() {
        let b = [0x28, 0x73, 0x2A, 0xC1, 0x1F, 0xF8, 0xD2, 0x11, 0xBA, 0x4B, 0x00, 0xA0, 0xC9, 0x3E, 0xC9, 0x3B];
        assert_eq!(guid_string(&b), "C12A7328-F81F-11D2-BA4B-00A0C93EC93B");
    }
}
