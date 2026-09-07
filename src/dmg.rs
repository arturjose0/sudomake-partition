//! Leitura de imagens DMG da Apple (formato UDIF): blocos zero, brutos, ADC, zlib, bzip2, LZFSE e LZMA.

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::io::{self, Read};
use std::os::windows::fs::FileExt;
use std::rc::Rc;

use crate::device::BlockDevice;
use crate::util::*;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind {
    Zero,
    Raw,
    Adc,
    Zlib,
    Bzip2,
    Lzfse,
    Lzma,
}

struct Chunk {
    start: u64,
    len: u64,
    kind: Kind,
    coff: u64,
    clen: u64,
}

pub struct DmgDevice {
    file: File,
    size: u64,
    chunks: Vec<Chunk>,
    cache: RefCell<(HashMap<usize, Rc<Vec<u8>>>, VecDeque<usize>)>,
    name: String,
}

fn read_exact_at(f: &File, mut off: u64, mut buf: &mut [u8]) -> io::Result<()> {
    while !buf.is_empty() {
        let n = f.seek_read(buf, off)?;
        if n == 0 {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "DMG truncado"));
        }
        off += n as u64;
        buf = &mut buf[n..];
    }
    Ok(())
}

fn base64_decode(s: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let mut acc = 0u32;
    let mut bits = 0u32;
    for &c in s {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => continue, // espaços, quebras de linha e '='
        } as u32;
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    out
}

/// Descompressão ADC (Apple Data Compression), usada em DMGs antigos.
fn adc_decode(src: &[u8], expected: usize) -> io::Result<Vec<u8>> {
    let mut out: Vec<u8> = Vec::with_capacity(expected);
    let mut i = 0;
    let n = src.len();
    let bad = || err("dados ADC inválidos");
    while i < n {
        let b = src[i] as usize;
        let (len, dist) = if b & 0x80 != 0 {
            let l = (b & 0x7F) + 1;
            i += 1;
            if i + l > n {
                return Err(bad());
            }
            out.extend_from_slice(&src[i..i + l]);
            i += l;
            continue;
        } else if b & 0x40 != 0 {
            if i + 3 > n {
                return Err(bad());
            }
            let l = (b & 0x3F) + 4;
            let d = (((src[i + 1] as usize) << 8) | src[i + 2] as usize) + 1;
            i += 3;
            (l, d)
        } else {
            if i + 2 > n {
                return Err(bad());
            }
            let l = ((b >> 2) & 0x0F) + 3;
            let d = (((b & 3) << 8) | src[i + 1] as usize) + 1;
            i += 2;
            (l, d)
        };
        if dist > out.len() {
            return Err(bad());
        }
        let start = out.len() - dist;
        for j in 0..len {
            let v = out[start + j];
            out.push(v);
        }
    }
    Ok(out)
}

impl DmgDevice {
    /// Devolve `None` se o arquivo não for um DMG (sem trailer "koly").
    pub fn open(path: &str) -> io::Result<Option<DmgDevice>> {
        let file = File::open(path)?;
        let flen = file.metadata()?.len();
        if flen < 1024 {
            return Ok(None);
        }
        let mut koly = [0u8; 512];
        read_exact_at(&file, flen - 512, &mut koly)?;
        if &koly[0..4] != b"koly" {
            return Ok(None);
        }
        let data_fork_off = be64(&koly, 24);
        let xml_off = be64(&koly, 216);
        let xml_len = be64(&koly, 224);
        let sector_count = be64(&koly, 492);
        if xml_len == 0 || xml_off + xml_len > flen || xml_len > 256 * 1024 * 1024 {
            return Err(err("DMG sem tabela de blocos legível (imagem criptografada ou formato não suportado)"));
        }
        let mut xml = vec![0u8; xml_len as usize];
        read_exact_at(&file, xml_off, &mut xml)?;
        let xml = String::from_utf8_lossy(&xml).into_owned();

        let blkx_pos = xml.find("<key>blkx</key>").ok_or_else(|| err("DMG sem seção blkx"))?;
        let rest = &xml[blkx_pos..];
        let arr_start = rest.find("<array>").ok_or_else(|| err("DMG: blkx sem array"))?;
        let arr_end = rest[arr_start..].find("</array>").map(|e| arr_start + e).unwrap_or(rest.len());
        let arr = &rest[arr_start..arr_end];

        let mut chunks = Vec::new();
        let mut pos = 0;
        while let Some(s) = arr[pos..].find("<data>") {
            let ds = pos + s + 6;
            let de = match arr[ds..].find("</data>") {
                Some(e) => ds + e,
                None => break,
            };
            pos = de + 7;
            let mish = base64_decode(arr[ds..de].as_bytes());
            if mish.len() < 204 || &mish[0..4] != b"mish" {
                continue;
            }
            let first_sector = be64(&mish, 8);
            let data_off = be64(&mish, 24);
            let n = be32(&mish, 200) as usize;
            for i in 0..n {
                let b = 204 + i * 40;
                if b + 40 > mish.len() {
                    break;
                }
                let ty = be32(&mish, b);
                let sect = be64(&mish, b + 8);
                let cnt = be64(&mish, b + 16);
                let coff = be64(&mish, b + 24);
                let clen = be64(&mish, b + 32);
                let kind = match ty {
                    0x0000_0000 | 0x0000_0002 => Kind::Zero,
                    0x0000_0001 => Kind::Raw,
                    0x8000_0004 => Kind::Adc,
                    0x8000_0005 => Kind::Zlib,
                    0x8000_0006 => Kind::Bzip2,
                    0x8000_0007 => Kind::Lzfse,
                    0x8000_0008 => Kind::Lzma,
                    0x7FFF_FFFE | 0xFFFF_FFFF => continue,
                    other => return Err(err(format!("DMG com tipo de bloco não suportado: {:#x}", other))),
                };
                if cnt == 0 {
                    continue;
                }
                chunks.push(Chunk {
                    start: (first_sector + sect) * 512,
                    len: cnt * 512,
                    kind,
                    coff: data_fork_off + data_off + coff,
                    clen,
                });
            }
        }
        if chunks.is_empty() {
            return Err(err("DMG sem blocos de dados"));
        }
        chunks.sort_by_key(|c| c.start);
        let max_end = chunks.iter().map(|c| c.start + c.len).max().unwrap_or(0);
        let size = if sector_count > 0 { sector_count * 512 } else { max_end }.max(max_end);
        Ok(Some(DmgDevice {
            file,
            size,
            chunks,
            cache: RefCell::new((HashMap::new(), VecDeque::new())),
            name: path.to_string(),
        }))
    }

    fn chunk_data(&self, i: usize) -> io::Result<Rc<Vec<u8>>> {
        if let Some(d) = self.cache.borrow().0.get(&i) {
            return Ok(d.clone());
        }
        let c = &self.chunks[i];
        let expected = c.len as usize;
        let out = if c.kind == Kind::Zero {
            vec![0u8; expected]
        } else {
            if c.clen > 512 * 1024 * 1024 {
                return Err(err("bloco DMG grande demais"));
            }
            let mut raw = vec![0u8; c.clen as usize];
            read_exact_at(&self.file, c.coff, &mut raw)?;
            let mut out = match c.kind {
                Kind::Raw => raw,
                Kind::Zlib => {
                    let mut o = Vec::with_capacity(expected);
                    flate2::read::ZlibDecoder::new(&raw[..]).read_to_end(&mut o)?;
                    o
                }
                Kind::Bzip2 => {
                    let mut o = Vec::with_capacity(expected);
                    bzip2_rs::DecoderReader::new(&raw[..]).read_to_end(&mut o)?;
                    o
                }
                Kind::Lzfse => {
                    let mut o = Vec::with_capacity(expected);
                    lzfse_rust::decode_bytes(&raw, &mut o).map_err(|e| err(format!("LZFSE no DMG: {:?}", e)))?;
                    o
                }
                Kind::Lzma => {
                    let mut o = Vec::with_capacity(expected);
                    lzma_rs::lzma_decompress(&mut &raw[..], &mut o).map_err(|e| err(format!("LZMA no DMG: {:?}", e)))?;
                    o
                }
                Kind::Adc => adc_decode(&raw, expected)?,
                Kind::Zero => unreachable!(),
            };
            if out.len() < expected {
                out.resize(expected, 0);
            } else if out.len() > expected {
                out.truncate(expected);
            }
            out
        };
        let rc = Rc::new(out);
        let mut cache = self.cache.borrow_mut();
        if cache.0.len() >= 48 {
            if let Some(old) = cache.1.pop_front() {
                cache.0.remove(&old);
            }
        }
        cache.0.insert(i, rc.clone());
        cache.1.push_back(i);
        Ok(rc)
    }
}

impl BlockDevice for DmgDevice {
    fn size(&self) -> u64 {
        self.size
    }
    fn describe(&self) -> String {
        format!("{} (DMG)", self.name)
    }
    fn read_at(&self, off: u64, buf: &mut [u8]) -> io::Result<()> {
        if off + buf.len() as u64 > self.size {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "leitura fora da imagem DMG"));
        }
        buf.fill(0);
        let mut done = 0usize;
        while done < buf.len() {
            let cur = off + done as u64;
            let i = self.chunks.partition_point(|c| c.start + c.len <= cur);
            if i >= self.chunks.len() {
                break; // além do último bloco: zeros
            }
            let c = &self.chunks[i];
            if c.start > cur {
                // buraco entre blocos: zeros
                let gap = (c.start - cur).min((buf.len() - done) as u64) as usize;
                done += gap;
                continue;
            }
            let data = self.chunk_data(i)?;
            let within = (cur - c.start) as usize;
            let n = (buf.len() - done).min(data.len() - within);
            buf[done..done + n].copy_from_slice(&data[within..within + n]);
            done += n;
        }
        Ok(())
    }
}
