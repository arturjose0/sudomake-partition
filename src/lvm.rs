//! Volumes lógicos LVM2: lê o rótulo "LABELONE", o cabeçalho do PV e os metadados em texto,
//! e mapeia volumes lógicos lineares para offsets no disco.

use std::io;

use crate::device::BlockDevice;
use crate::util::*;

#[derive(Clone, Debug)]
pub struct LogicalVolume {
    pub vg: String,
    pub name: String,
    pub size: u64,
    pub segtype: String,
    /// false se algum segmento está em outro disco ou usa um tipo não suportado
    pub complete: bool,
    /// (offset no volume lógico, offset no dispositivo base, tamanho), em bytes
    pub runs: Vec<(u64, u64, u64)>,
}

#[derive(Debug, Clone)]
enum Val {
    Str(String),
    Num(i64),
    List(Vec<Val>),
    Sec(Vec<(String, Val)>),
}

impl Val {
    fn get<'a>(&'a self, key: &str) -> Option<&'a Val> {
        match self {
            Val::Sec(items) => items.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
    fn num(&self, key: &str) -> Option<i64> {
        match self.get(key)? {
            Val::Num(n) => Some(*n),
            _ => None,
        }
    }
    fn string(&self, key: &str) -> Option<&str> {
        match self.get(key)? {
            Val::Str(s) => Some(s.as_str()),
            _ => None,
        }
    }
    fn sections(&self) -> Vec<(&str, &Val)> {
        match self {
            Val::Sec(items) => items.iter().filter(|(_, v)| matches!(v, Val::Sec(_))).map(|(k, v)| (k.as_str(), v)).collect(),
            _ => Vec::new(),
        }
    }
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}

impl<'a> Parser<'a> {
    fn skip_ws(&mut self) {
        while self.i < self.s.len() {
            let c = self.s[self.i];
            if c == b'#' {
                while self.i < self.s.len() && self.s[self.i] != b'\n' {
                    self.i += 1;
                }
            } else if c.is_ascii_whitespace() || c == 0 {
                self.i += 1;
            } else {
                break;
            }
        }
    }

    fn peek(&self) -> Option<u8> {
        self.s.get(self.i).copied()
    }

    fn ident(&mut self) -> String {
        let start = self.i;
        while self.i < self.s.len() {
            let c = self.s[self.i];
            if c.is_ascii_whitespace() || c == b'=' || c == b'{' || c == b'}' || c == 0 {
                break;
            }
            self.i += 1;
        }
        String::from_utf8_lossy(&self.s[start..self.i]).into_owned()
    }

    fn value(&mut self) -> Val {
        self.skip_ws();
        match self.peek() {
            Some(b'"') => {
                self.i += 1;
                let mut out = Vec::new();
                while self.i < self.s.len() && self.s[self.i] != b'"' {
                    if self.s[self.i] == b'\\' && self.i + 1 < self.s.len() {
                        self.i += 1;
                    }
                    out.push(self.s[self.i]);
                    self.i += 1;
                }
                self.i += 1;
                Val::Str(String::from_utf8_lossy(&out).into_owned())
            }
            Some(b'[') => {
                self.i += 1;
                let mut items = Vec::new();
                loop {
                    self.skip_ws();
                    match self.peek() {
                        None => break,
                        Some(b']') => {
                            self.i += 1;
                            break;
                        }
                        Some(b',') => {
                            self.i += 1;
                        }
                        _ => items.push(self.value()),
                    }
                }
                Val::List(items)
            }
            _ => {
                let start = self.i;
                while self.i < self.s.len() {
                    let c = self.s[self.i];
                    if c.is_ascii_whitespace() || c == b',' || c == b']' || c == b'}' || c == 0 {
                        break;
                    }
                    self.i += 1;
                }
                let t = String::from_utf8_lossy(&self.s[start..self.i]).into_owned();
                match t.parse::<i64>() {
                    Ok(n) => Val::Num(n),
                    Err(_) => Val::Str(t),
                }
            }
        }
    }

    fn body(&mut self, depth: u32) -> Vec<(String, Val)> {
        let mut items = Vec::new();
        loop {
            self.skip_ws();
            match self.peek() {
                None => break,
                Some(b'}') => {
                    self.i += 1;
                    break;
                }
                _ => {}
            }
            let name = self.ident();
            if name.is_empty() {
                self.i += 1;
                continue;
            }
            self.skip_ws();
            match self.peek() {
                Some(b'{') => {
                    self.i += 1;
                    if depth > 16 {
                        break;
                    }
                    let b = self.body(depth + 1);
                    items.push((name, Val::Sec(b)));
                }
                Some(b'=') => {
                    self.i += 1;
                    let v = self.value();
                    items.push((name, v));
                }
                _ => {}
            }
        }
        items
    }
}

fn parse_metadata(text: &[u8]) -> Vec<(String, Val)> {
    let mut p = Parser { s: text, i: 0 };
    p.body(0)
}

/// Procura um PV LVM2 no início da região [start, start+len) e devolve os volumes lógicos.
pub fn scan(dev: &dyn BlockDevice, start: u64, len: u64) -> io::Result<Option<Vec<LogicalVolume>>> {
    if len < 8192 || start + 8192 > dev.size() {
        return Ok(None);
    }
    let mut head = vec![0u8; 2048];
    dev.read_at(start, &mut head)?;
    let sector = match (0..4).find(|s| &head[s * 512..s * 512 + 8] == b"LABELONE") {
        Some(s) => s,
        None => return Ok(None),
    };
    let lh = &head[sector * 512..(sector + 1) * 512];
    if &lh[24..32] != b"LVM2 001" {
        return Ok(None);
    }
    let pv_off = le32(lh, 20) as usize;
    if pv_off + 40 > lh.len() {
        return Ok(None);
    }
    let pv = &lh[pv_off..];
    let my_id = String::from_utf8_lossy(&pv[0..32]).into_owned();
    let mut p = 40;
    // áreas de dados
    while p + 16 <= pv.len() {
        let (o, s) = (le64(pv, p), le64(pv, p + 8));
        p += 16;
        if o == 0 && s == 0 {
            break;
        }
    }
    let mut mdas = Vec::new();
    while p + 16 <= pv.len() {
        let (o, s) = (le64(pv, p), le64(pv, p + 8));
        p += 16;
        if o == 0 && s == 0 {
            break;
        }
        mdas.push((o, s));
    }

    let mut text: Option<Vec<u8>> = None;
    for (mo, _) in mdas {
        if mo + 512 > len {
            continue;
        }
        let mut hdr = vec![0u8; 512];
        dev.read_at(start + mo, &mut hdr)?;
        if &hdr[4..20] != b" LVM2 x[5A%r0N*>" {
            continue;
        }
        let mda_start = le64(&hdr, 24);
        let mda_size = le64(&hdr, 32);
        let mut q = 40;
        while q + 24 <= hdr.len() {
            let off = le64(&hdr, q);
            let size = le64(&hdr, q + 8);
            let flags = le32(&hdr, q + 20);
            q += 24;
            if off == 0 {
                break;
            }
            if flags & 1 != 0 || size == 0 || size > 16 * 1024 * 1024 || mda_size == 0 {
                continue;
            }
            let mut buf = vec![0u8; size as usize];
            if off + size <= mda_size {
                dev.read_at(start + mda_start + off, &mut buf)?;
            } else {
                let first = (mda_size - off) as usize;
                dev.read_at(start + mda_start + off, &mut buf[..first])?;
                dev.read_at(start + mda_start + 512, &mut buf[first..])?;
            }
            text = Some(buf);
            break;
        }
        if text.is_some() {
            break;
        }
    }
    let text = match text {
        Some(t) => t,
        None => {
            eprintln!("aviso: PV LVM sem metadados legíveis");
            return Ok(Some(Vec::new()));
        }
    };

    let items = parse_metadata(&text);
    const TRAILER: [&str; 5] = ["contents", "version", "description", "creation_host", "creation_time"];
    let (vg_name, vg) = match items.iter().find(|(k, v)| matches!(v, Val::Sec(_)) && !TRAILER.contains(&k.as_str())) {
        Some((k, v)) => (k.clone(), v),
        None => return Ok(Some(Vec::new())),
    };
    let es = vg.num("extent_size").unwrap_or(0).max(0) as u64 * 512;
    if es == 0 {
        return Ok(Some(Vec::new()));
    }
    let mut pvs = std::collections::HashMap::new();
    if let Some(pvsec) = vg.get("physical_volumes") {
        for (name, v) in pvsec.sections() {
            let id: String = v.string("id").unwrap_or("").chars().filter(|c| *c != '-').collect();
            let pe_start = v.num("pe_start").unwrap_or(0).max(0) as u64 * 512;
            pvs.insert(name.to_string(), (id, pe_start));
        }
    }
    let mut out = Vec::new();
    if let Some(lvsec) = vg.get("logical_volumes") {
        for (lvname, lv) in lvsec.sections() {
            let visible = match lv.get("status") {
                Some(Val::List(l)) => l.iter().any(|x| matches!(x, Val::Str(s) if s == "VISIBLE")),
                _ => true,
            };
            if !visible {
                continue;
            }
            let mut runs = Vec::new();
            let mut complete = true;
            let mut segtype = String::from("linear");
            let mut size = 0u64;
            for (segname, seg) in lv.sections() {
                if !segname.starts_with("segment") {
                    continue;
                }
                let se = seg.num("start_extent").unwrap_or(0).max(0) as u64;
                let ec = seg.num("extent_count").unwrap_or(0).max(0) as u64;
                size = size.max((se + ec) * es);
                let ty = seg.string("type").unwrap_or("striped").to_string();
                let stripes = seg.num("stripe_count").unwrap_or(1);
                let linear = ty == "linear" || (ty == "striped" && stripes == 1);
                if !linear {
                    complete = false;
                    segtype = ty;
                    continue;
                }
                let (pvname, pe) = match seg.get("stripes") {
                    Some(Val::List(l)) if l.len() >= 2 => match (&l[0], &l[1]) {
                        (Val::Str(p), Val::Num(n)) => (p.clone(), (*n).max(0) as u64),
                        _ => {
                            complete = false;
                            continue;
                        }
                    },
                    _ => {
                        complete = false;
                        continue;
                    }
                };
                match pvs.get(&pvname) {
                    Some((id, pe_start)) if *id == my_id => {
                        runs.push((se * es, start + pe_start + pe * es, ec * es));
                    }
                    _ => {
                        complete = false;
                        segtype = "em outro disco".into();
                    }
                }
            }
            runs.sort_by_key(|r| r.0);
            out.push(LogicalVolume { vg: vg_name.clone(), name: lvname.to_string(), size, segtype, complete, runs });
        }
    }
    Ok(Some(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_text() {
        let t = br#"vg0 {
id = "abc"
seqno = 3
extent_size = 8192 # comentario
physical_volumes {
pv0 { id = "AAAA-BBBB" device = "/dev/sda2" pe_start = 2048 pe_count = 10 }
}
logical_volumes {
root {
status = ["READ", "WRITE", "VISIBLE"]
segment_count = 1
segment1 {
start_extent = 0
extent_count = 5
type = "striped"
stripe_count = 1
stripes = [
"pv0", 2
]
}
}
}
}
contents = "Text Format Volume Group"
version = 1
"#;
        let items = parse_metadata(t);
        let (name, vg) = items.iter().find(|(_, v)| matches!(v, Val::Sec(_))).unwrap();
        assert_eq!(name, "vg0");
        assert_eq!(vg.num("extent_size"), Some(8192));
        let lv = vg.get("logical_volumes").unwrap().get("root").unwrap();
        let seg = lv.get("segment1").unwrap();
        assert_eq!(seg.string("type"), Some("striped"));
        match seg.get("stripes") {
            Some(Val::List(l)) => assert_eq!(l.len(), 2),
            _ => panic!("stripes"),
        }
    }
}
