//! Descompressão transparente de arquivos comprimidos pelo macOS ("decmpfs" / AppleFSCompression).
//! Tipos suportados: zlib (3/4), LZVN (7/8), sem compressão (1/9/10) e LZFSE (11/12).

use std::io::{self, Read};

use crate::fs::{BytesReader, FileReader};
use crate::util::*;

const MAGIC: u32 = 0x636D_7066; // "fpmc"
const CHUNK: u64 = 65_536;

pub fn header(x: &[u8]) -> Option<(u32, u64)> {
    if x.len() < 16 || le32(x, 0) != MAGIC {
        None
    } else {
        Some((le32(x, 4), le64(x, 8)))
    }
}

pub fn type_name(t: u32) -> &'static str {
    match t {
        1 | 9 => "sem compressão (xattr)",
        3 => "zlib (xattr)",
        4 => "zlib (resource fork)",
        7 => "LZVN (xattr)",
        8 => "LZVN (resource fork)",
        10 => "sem compressão (resource fork)",
        11 => "LZFSE (xattr)",
        12 => "LZFSE (resource fork)",
        13 | 14 => "LZBITMAP",
        _ => "desconhecido",
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Method {
    Zlib,
    Lzvn,
    Lzfse,
    Raw,
}

fn inflate(data: &[u8], expected: usize) -> io::Result<Vec<u8>> {
    let mut out = Vec::with_capacity(expected);
    flate2::read::ZlibDecoder::new(data).read_to_end(&mut out)?;
    Ok(out)
}

fn decode_block(method: Method, raw: &[u8], expected: usize) -> io::Result<Vec<u8>> {
    if raw.is_empty() {
        return Ok(Vec::new());
    }
    match method {
        Method::Zlib => {
            if raw[0] == 0xFF {
                Ok(raw[1..].to_vec())
            } else {
                inflate(raw, expected)
            }
        }
        Method::Lzvn => {
            if raw[0] == 0x06 {
                Ok(raw[1..].to_vec())
            } else {
                crate::lzvn::decode(raw, expected).map_err(err)
            }
        }
        Method::Lzfse => {
            let mut out = Vec::with_capacity(expected);
            lzfse_rust::decode_bytes(raw, &mut out).map_err(|e| err(format!("LZFSE: {:?}", e)))?;
            Ok(out)
        }
        Method::Raw => Ok(raw.to_vec()),
    }
}

/// Arquivo comprimido em blocos de 64 KB dentro do resource fork.
pub struct ChunkedReader {
    size: u64,
    method: Method,
    rsrc: Box<dyn FileReader>,
    /// (offset absoluto no resource fork, tamanho)
    chunks: Vec<(u64, u64)>,
    cache: Option<(usize, Vec<u8>)>,
}

fn read_exact(r: &mut dyn FileReader, off: u64, buf: &mut [u8]) -> io::Result<()> {
    let mut done = 0;
    while done < buf.len() {
        let n = r.read_at(off + done as u64, &mut buf[done..])?;
        if n == 0 {
            return Err(err("resource fork truncado"));
        }
        done += n;
    }
    Ok(())
}

impl ChunkedReader {
    fn chunk(&mut self, i: usize) -> io::Result<&[u8]> {
        if self.cache.as_ref().map_or(false, |(j, _)| *j == i) {
            return Ok(&self.cache.as_ref().unwrap().1);
        }
        let (off, len) = self.chunks[i];
        let expected = CHUNK.min(self.size - i as u64 * CHUNK) as usize;
        if len > 16 * 1024 * 1024 {
            return Err(err("bloco comprimido grande demais"));
        }
        let mut raw = vec![0u8; len as usize];
        read_exact(self.rsrc.as_mut(), off, &mut raw)?;
        let mut out = decode_block(self.method, &raw, expected)?;
        if out.len() != expected {
            if out.len() > expected {
                out.truncate(expected);
            } else {
                return Err(err(format!(
                    "bloco {} descomprimido com {} bytes (esperado {})",
                    i,
                    out.len(),
                    expected
                )));
            }
        }
        self.cache = Some((i, out));
        Ok(&self.cache.as_ref().unwrap().1)
    }
}

impl FileReader for ChunkedReader {
    fn size(&self) -> u64 {
        self.size
    }
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> io::Result<usize> {
        if off >= self.size {
            return Ok(0);
        }
        let n = (buf.len() as u64).min(self.size - off) as usize;
        let mut done = 0usize;
        while done < n {
            let cur = off + done as u64;
            let i = (cur / CHUNK) as usize;
            let within = (cur % CHUNK) as usize;
            let c = self.chunk(i)?;
            if within >= c.len() {
                return Err(err("bloco descomprimido curto demais"));
            }
            let take = (n - done).min(c.len() - within);
            buf[done..done + take].copy_from_slice(&c[within..within + take]);
            done += take;
        }
        Ok(n)
    }
}

fn read_u32_le(r: &mut dyn FileReader, off: u64) -> io::Result<u32> {
    let mut b = [0u8; 4];
    read_exact(r, off, &mut b)?;
    Ok(le32(&b, 0))
}

fn read_u32_be(r: &mut dyn FileReader, off: u64) -> io::Result<u32> {
    let mut b = [0u8; 4];
    read_exact(r, off, &mut b)?;
    Ok(be32(&b, 0))
}

/// Abre um arquivo comprimido a partir do conteúdo do xattr com.apple.decmpfs e do resource fork.
pub fn open(xattr: Vec<u8>, rsrc: Option<Box<dyn FileReader>>) -> io::Result<Box<dyn FileReader>> {
    let (ctype, size) = header(&xattr).ok_or_else(|| err("cabeçalho decmpfs inválido"))?;
    let payload = &xattr[16..];
    if size == 0 {
        return Ok(Box::new(BytesReader(Vec::new())));
    }
    let attr_result = |data: Vec<u8>| -> io::Result<Box<dyn FileReader>> {
        if data.len() as u64 != size {
            return Err(err(format!(
                "arquivo comprimido ({}) descomprimiu para {} bytes, esperado {}",
                type_name(ctype),
                data.len(),
                size
            )));
        }
        Ok(Box::new(BytesReader(data)))
    };
    match ctype {
        1 | 9 => attr_result(payload[..(size as usize).min(payload.len())].to_vec()),
        3 => attr_result(decode_block(Method::Zlib, payload, size as usize)?),
        7 => attr_result(decode_block(Method::Lzvn, payload, size as usize)?),
        11 => attr_result(decode_block(Method::Lzfse, payload, size as usize)?),
        4 | 8 | 10 | 12 => {
            let mut rsrc = rsrc.ok_or_else(|| err(format!("arquivo comprimido ({}) sem resource fork", type_name(ctype))))?;
            let nchunks = ((size + CHUNK - 1) / CHUNK) as usize;
            let mut chunks = Vec::with_capacity(nchunks);
            let method;
            if ctype == 4 {
                method = Method::Zlib;
                let data_off = read_u32_be(rsrc.as_mut(), 0)? as u64;
                let base = data_off + 4;
                let num = read_u32_le(rsrc.as_mut(), base)? as usize;
                if num < nchunks {
                    return Err(err(format!("tabela de blocos zlib com {} entradas, esperado {}", num, nchunks)));
                }
                let mut table = vec![0u8; nchunks * 8];
                read_exact(rsrc.as_mut(), base + 4, &mut table)?;
                for i in 0..nchunks {
                    let off = le32(&table, i * 8) as u64;
                    let len = le32(&table, i * 8 + 4) as u64;
                    chunks.push((base + off, len));
                }
            } else {
                method = match ctype {
                    8 => Method::Lzvn,
                    12 => Method::Lzfse,
                    _ => Method::Raw,
                };
                let first = read_u32_le(rsrc.as_mut(), 0)? as u64;
                let n = (first / 4).saturating_sub(1) as usize;
                if first % 4 != 0 || n < nchunks {
                    return Err(err(format!(
                        "tabela de blocos {} inválida ({} entradas, esperado {})",
                        type_name(ctype),
                        n,
                        nchunks
                    )));
                }
                let mut table = vec![0u8; (nchunks + 1) * 4];
                read_exact(rsrc.as_mut(), 0, &mut table)?;
                for i in 0..nchunks {
                    let a = le32(&table, i * 4) as u64;
                    let b = le32(&table, (i + 1) * 4) as u64;
                    if b < a {
                        return Err(err("tabela de blocos fora de ordem"));
                    }
                    chunks.push((a, b - a));
                }
            }
            Ok(Box::new(ChunkedReader { size, method, rsrc, chunks, cache: None }))
        }
        13 | 14 => Err(err("compressão LZBITMAP não suportada")),
        _ => Err(err(format!("tipo de compressão decmpfs desconhecido: {}", ctype))),
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::read_all;
    use std::io::Write;

    fn sample(n: usize) -> Vec<u8> {
        let mut v = Vec::with_capacity(n);
        let mut x = 12345u32;
        while v.len() < n {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            if x % 3 == 0 {
                v.extend_from_slice(b"texto repetido para comprimir bem ");
            } else {
                v.push((x >> 11) as u8);
            }
        }
        v.truncate(n);
        v
    }

    fn zlib(data: &[u8]) -> Vec<u8> {
        let mut e = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        e.write_all(data).unwrap();
        e.finish().unwrap()
    }

    fn header(ctype: u32, size: u64) -> Vec<u8> {
        let mut h = Vec::new();
        h.extend_from_slice(&MAGIC.to_le_bytes());
        h.extend_from_slice(&ctype.to_le_bytes());
        h.extend_from_slice(&size.to_le_bytes());
        h
    }

    fn check(reader: Box<dyn FileReader>, expected: &[u8]) {
        let mut r = reader;
        assert_eq!(r.size(), expected.len() as u64);
        let got = read_all(r.as_mut()).unwrap();
        assert_eq!(got.len(), expected.len());
        assert!(got == expected, "conteúdo diferente");
        // leitura parcial desalinhada
        if expected.len() > 70000 {
            let mut buf = vec![0u8; 5000];
            let n = r.read_at(65530, &mut buf).unwrap();
            assert_eq!(&buf[..n], &expected[65530..65530 + n]);
        }
    }

    #[test]
    fn zlib_attr_and_raw_marker() {
        let data = sample(3000);
        let mut x = header(3, data.len() as u64);
        x.extend_from_slice(&zlib(&data));
        check(open(x, None).unwrap(), &data);

        let mut raw = header(3, data.len() as u64);
        raw.push(0xFF);
        raw.extend_from_slice(&data);
        check(open(raw, None).unwrap(), &data);
    }

    #[test]
    fn zlib_rsrc_block_table() {
        let data = sample(200_000); // 4 blocos de 64 KB
        let blocks: Vec<Vec<u8>> = data.chunks(65536).map(zlib).collect();
        // fork de recursos: cabeçalho (256 bytes), depois dados
        let n = blocks.len() as u32;
        let mut body = Vec::new();
        body.extend_from_slice(&n.to_le_bytes());
        let mut off = 4 + 8 * n;
        let mut table = Vec::new();
        for b in &blocks {
            table.extend_from_slice(&off.to_le_bytes());
            table.extend_from_slice(&(b.len() as u32).to_le_bytes());
            off += b.len() as u32;
        }
        body.extend_from_slice(&table);
        for b in &blocks {
            body.extend_from_slice(b);
        }
        let mut rsrc = vec![0u8; 256];
        rsrc[0..4].copy_from_slice(&256u32.to_be_bytes()); // data offset
        rsrc[8..12].copy_from_slice(&((body.len() + 4) as u32).to_be_bytes());
        rsrc.extend_from_slice(&(body.len() as u32).to_be_bytes());
        rsrc.extend_from_slice(&body);
        let x = header(4, data.len() as u64);
        check(open(x, Some(Box::new(BytesReader(rsrc)))).unwrap(), &data);
    }

    #[test]
    fn lzvn_rsrc_chunk_table_with_stored_chunks() {
        // usa blocos "armazenados" (0x06 + dados) que o formato permite
        let data = sample(140_000);
        let chunks: Vec<Vec<u8>> = data
            .chunks(65536)
            .map(|c| {
                let mut v = vec![0x06u8];
                v.extend_from_slice(c);
                v
            })
            .collect();
        let n = chunks.len();
        let mut rsrc = Vec::new();
        let mut off = (4 * (n + 1)) as u32;
        rsrc.extend_from_slice(&off.to_le_bytes());
        for c in &chunks {
            off += c.len() as u32;
            rsrc.extend_from_slice(&off.to_le_bytes());
        }
        for c in &chunks {
            rsrc.extend_from_slice(c);
        }
        let x = header(8, data.len() as u64);
        check(open(x, Some(Box::new(BytesReader(rsrc)))).unwrap(), &data);
    }

    #[test]
    fn lzfse_attr() {
        let data = sample(2500);
        let mut enc = Vec::new();
        lzfse_rust::encode_bytes(&data, &mut enc).unwrap();
        let mut x = header(11, data.len() as u64);
        x.extend_from_slice(&enc);
        check(open(x, None).unwrap(), &data);
    }

    #[test]
    fn size_mismatch_is_error() {
        let data = sample(100);
        let mut x = header(3, 200);
        x.extend_from_slice(&zlib(&data));
        assert!(open(x, None).is_err());
    }
}
