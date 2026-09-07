//! Decodificador LZVN (formato usado pelo macOS em arquivos comprimidos e no LZFSE).

fn need(src: &[u8], i: usize, n: usize) -> Result<(), String> {
    if i + n > src.len() {
        Err("fluxo LZVN truncado".to_string())
    } else {
        Ok(())
    }
}

fn literals(src: &[u8], i: &mut usize, dst: &mut Vec<u8>, l: usize) -> Result<(), String> {
    need(src, *i, l)?;
    dst.extend_from_slice(&src[*i..*i + l]);
    *i += l;
    Ok(())
}

fn copy_match(dst: &mut Vec<u8>, m: usize, d: usize) -> Result<(), String> {
    if d == 0 || d > dst.len() {
        return Err(format!("distância LZVN inválida ({} com {} bytes escritos)", d, dst.len()));
    }
    let start = dst.len() - d;
    if d >= m {
        dst.extend_from_within(start..start + m);
    } else {
        for j in 0..m {
            let b = dst[start + j];
            dst.push(b);
        }
    }
    Ok(())
}

/// Descomprime um fluxo LZVN bruto. `expected` é o tamanho esperado da saída (limite superior).
pub fn decode(src: &[u8], expected: usize) -> Result<Vec<u8>, String> {
    let mut dst: Vec<u8> = Vec::with_capacity(expected);
    let mut i = 0usize;
    let mut d = 0usize;
    let n = src.len();
    while i < n {
        let op = src[i] as usize;
        let (l, m);
        match op {
            0x06 => break, // fim do fluxo
            0x0E | 0x16 => {
                i += 1;
                continue;
            }
            0x1E | 0x26 | 0x2E | 0x36 | 0x3E | 0x70..=0x7F | 0xD0..=0xDF => {
                return Err(format!("opcode LZVN inválido {:#04x} na posição {}", op, i));
            }
            0xE0 => {
                need(src, i, 2)?;
                let l = src[i + 1] as usize + 16;
                i += 2;
                literals(src, &mut i, &mut dst, l)?;
                continue;
            }
            0xE1..=0xEF => {
                i += 1;
                literals(src, &mut i, &mut dst, op & 0xF)?;
                continue;
            }
            0xF0 => {
                need(src, i, 2)?;
                let m = src[i + 1] as usize + 16;
                i += 2;
                copy_match(&mut dst, m, d)?;
                continue;
            }
            0xF1..=0xFF => {
                i += 1;
                copy_match(&mut dst, op & 0xF, d)?;
                continue;
            }
            0xA0..=0xBF => {
                need(src, i, 3)?;
                let b1 = src[i + 1] as usize;
                let b2 = src[i + 2] as usize;
                l = (op >> 3) & 3;
                m = (((op & 7) << 2) | (b1 & 3)) + 3;
                d = (b1 >> 2) | (b2 << 6);
                i += 3;
            }
            _ => match op & 7 {
                7 => {
                    need(src, i, 3)?;
                    l = op >> 6;
                    m = ((op >> 3) & 7) + 3;
                    d = src[i + 1] as usize | ((src[i + 2] as usize) << 8);
                    i += 3;
                }
                6 => {
                    l = op >> 6;
                    m = ((op >> 3) & 7) + 3;
                    i += 1;
                }
                _ => {
                    need(src, i, 2)?;
                    l = op >> 6;
                    m = ((op >> 3) & 7) + 3;
                    d = ((op & 7) << 8) | src[i + 1] as usize;
                    i += 2;
                }
            },
        }
        literals(src, &mut i, &mut dst, l)?;
        copy_match(&mut dst, m, d)?;
        if dst.len() > expected + 65536 {
            return Err("saída LZVN maior que o esperado".into());
        }
    }
    if dst.len() > expected {
        dst.truncate(expected);
    }
    Ok(dst)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Usa o codificador LZFSE (que gera blocos LZVN "bvxn" para entradas pequenas) como referência.
    #[test]
    fn roundtrip_against_lzfse_rust() {
        let mut checked = 0;
        for seed in 0..64u32 {
            let mut data = Vec::new();
            let mut x = seed.wrapping_mul(2_654_435_761).wrapping_add(7);
            let len = 200 + (seed as usize * 37) % 3500;
            while data.len() < len {
                x ^= x << 13;
                x ^= x >> 17;
                x ^= x << 5;
                match x % 5 {
                    0 => data.extend_from_slice(b"abcabcabcabc"),
                    1 => data.push((x >> 8) as u8),
                    2 => data.extend_from_slice(b"Lorem ipsum dolor sit amet "),
                    3 => {
                        let l = data.len();
                        if l > 40 {
                            let s = (x as usize >> 3) % (l - 20);
                            let e = s + 5 + (x as usize % 15);
                            let piece = data[s..e].to_vec();
                            data.extend_from_slice(&piece);
                        } else {
                            data.push(b'z');
                        }
                    }
                    _ => data.extend(std::iter::repeat(0u8).take(1 + (x as usize % 30))),
                }
            }
            data.truncate(len);
            let mut enc = Vec::new();
            lzfse_rust::encode_bytes(&data, &mut enc).unwrap();
            if &enc[0..4] == b"bvxn" {
                let n_raw = u32::from_le_bytes(enc[4..8].try_into().unwrap()) as usize;
                let n_payload = u32::from_le_bytes(enc[8..12].try_into().unwrap()) as usize;
                let payload = &enc[12..12 + n_payload];
                let out = decode(payload, n_raw).expect("decode");
                assert_eq!(out, data, "seed {}", seed);
                checked += 1;
            }
        }
        assert!(checked > 0, "nenhum bloco LZVN gerado pelo codificador de referência");
    }
}
