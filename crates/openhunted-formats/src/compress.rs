// SPDX-License-Identifier: GPL-3.0-or-later
//! Descompressão dos recursos. Formato em `docs/formats/compressao.md`.

use crate::bytes::{slice, u8_at, u32_at};
use crate::{FormatError, Result};

const WHAT: &str = "compressão";

/// Descomprime `src` (os dados comprimidos, sem cabeçalho) pelo método `method & 0x7f`.
/// O resultado tem de ter exatamente `expected_len` bytes.
pub fn decompress(method: u8, src: &[u8], expected_len: usize) -> Result<Vec<u8>> {
    let out = match method & 0x7f {
        // Cópia: os tamanhos armazenado e descomprimido têm de coincidir.
        0 => src.to_vec(),
        3 => lz_flags(src, expected_len)?,
        m => {
            return Err(FormatError::Unsupported {
                what: WHAT,
                detail: format!("método {m}"),
            });
        }
    };
    if out.len() != expected_len {
        return Err(FormatError::Truncated {
            what: WHAT,
            offset: out.len(),
        });
    }
    Ok(out)
}

/// Lê um bloco comprimido completo: `u8` tipo, `u32` tamanho armazenado (dados + 4),
/// `u32` tamanho descomprimido e os dados. Devolve o resultado e os bytes consumidos.
pub fn unpack_block(data: &[u8]) -> Result<(Vec<u8>, usize)> {
    let method = u8_at(data, 0, WHAT)?;
    let stored = u32_at(data, 1, WHAT)? as usize;
    let packed_len = stored.checked_sub(4).ok_or(FormatError::Truncated {
        what: WHAT,
        offset: 1,
    })?;
    let unpacked_len = u32_at(data, 5, WHAT)? as usize;
    let packed = slice(data, 9, packed_len, WHAT)?;
    Ok((decompress(method, packed, unpacked_len)?, 9 + packed_len))
}

/// Método 3: LZ com fluxo de flags separado (ver `docs/re/compressao.md`).
fn lz_flags(src: &[u8], expected_len: usize) -> Result<Vec<u8>> {
    let n = u8_at(src, 0, WHAT)?;
    let mut pos = 1;
    let mut tokens: usize = 0;
    if n > 1 {
        tokens |= (u8_at(src, pos, WHAT)? as usize) << 24;
        tokens |= (u8_at(src, pos + 1, WHAT)? as usize) << 16;
        pos += 2;
    }
    if n > 0 {
        tokens |= (u8_at(src, pos, WHAT)? as usize) << 8;
        pos += 1;
    }
    tokens |= u8_at(src, pos, WHAT)? as usize;
    pos += 1;

    let mut flag_pos = pos;
    let mut data_pos = pos + tokens.div_ceil(8);
    // Cada palavra de referência gera no máximo 66 bytes: limita a reserva pelo que os
    // dados conseguem produzir, em vez de confiar no tamanho declarado.
    let mut out = Vec::with_capacity(expected_len.min(src.len().saturating_mul(33)));
    let mut flags = 0u8;
    for i in 0..tokens {
        if i % 8 == 0 {
            flags = u8_at(src, flag_pos, WHAT)?;
            flag_pos += 1;
        }
        if flags & 0x80 != 0 {
            let w =
                u16::from_be_bytes([u8_at(src, data_pos, WHAT)?, u8_at(src, data_pos + 1, WHAT)?]);
            data_pos += 2;
            let len = (w & 0x3f) as usize + 3;
            let dist = (w >> 6) as usize + 1;
            let start = out.len().checked_sub(dist).ok_or(FormatError::Truncated {
                what: "referência LZ antes do início",
                offset: data_pos - 2,
            })?;
            if out.len() + len > expected_len {
                return Err(FormatError::Truncated {
                    what: WHAT,
                    offset: out.len(),
                });
            }
            for k in 0..len {
                let b = out[start + k];
                out.push(b);
            }
        } else {
            if out.len() >= expected_len {
                return Err(FormatError::Truncated {
                    what: WHAT,
                    offset: out.len(),
                });
            }
            out.push(u8_at(src, data_pos, WHAT)?);
            data_pos += 1;
        }
        flags <<= 1;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FormatError;

    #[test]
    fn method0_copies() {
        assert_eq!(decompress(0, b"abc", 3).unwrap(), b"abc");
    }

    #[test]
    fn lz_literals_only() {
        // T = 3 (n = 0), flags 0x00, dados "abc".
        assert_eq!(
            decompress(3, &[0x00, 3, 0x00, b'a', b'b', b'c'], 3).unwrap(),
            b"abc"
        );
    }

    #[test]
    fn lz_backreference_overlapping() {
        // literal 'a', depois referência distância 1, comprimento 5 → "aaaaaa".
        let src = [0x00, 2, 0x40, b'a', 0x00, 0x02];
        assert_eq!(decompress(3, &src, 6).unwrap(), b"aaaaaa");
    }

    #[test]
    fn lz_two_byte_count_and_flag_byte_boundary() {
        // n = 1: T = 0x0009; nove literais; dois bytes de flags.
        let mut src = vec![0x01, 0x00, 0x09, 0x00, 0x00];
        src.extend_from_slice(b"123456789");
        assert_eq!(decompress(3, &src, 9).unwrap(), b"123456789");
    }

    #[test]
    fn lz_flag_reload_after_eight_tokens() {
        // 8 literais e uma referência no 9º token (bit alto do 2º byte de flags).
        let mut src = vec![0x00, 9, 0x00, 0x80];
        src.extend_from_slice(b"abcdefgh");
        // distância 8, comprimento 3: w = (7 << 6) | 0 = 0x01c0
        src.extend_from_slice(&[0x01, 0xc0]);
        assert_eq!(decompress(3, &src, 11).unwrap(), b"abcdefghabc");
    }

    #[test]
    fn unknown_or_unseen_method_is_unsupported() {
        for m in [1u8, 2, 4, 0x7f] {
            assert!(matches!(
                decompress(m, b"x", 1),
                Err(FormatError::Unsupported { .. })
            ));
        }
    }

    #[test]
    fn huge_declared_size_errors_without_allocating() {
        let too_big = isize::MAX as usize + 1;
        assert!(decompress(3, &[0x00, 3, 0x00, b'a', b'b', b'c'], too_big).is_err());
    }

    #[test]
    fn reference_before_start_errors() {
        let src = [0x00, 1, 0x80, 0x00, 0x00];
        assert!(decompress(3, &src, 3).is_err());
    }

    #[test]
    fn wrong_expected_len_errors() {
        assert!(decompress(3, &[0x00, 3, 0x00, b'a', b'b', b'c'], 4).is_err());
        assert!(decompress(0, b"abc", 2).is_err());
    }

    #[test]
    fn truncated_stream_never_panics() {
        let mut src = vec![0x00, 9, 0x00, 0x80];
        src.extend_from_slice(b"abcdefgh");
        src.extend_from_slice(&[0x01, 0xc0]);
        for n in 0..src.len() {
            assert!(decompress(3, &src[..n], 11).is_err());
        }
    }

    #[test]
    fn unpack_block_reads_header() {
        // tipo 0x83, armazenado = 4 + 6, descomprimido = 3
        let mut blk = vec![0x83];
        blk.extend_from_slice(&10u32.to_le_bytes());
        blk.extend_from_slice(&3u32.to_le_bytes());
        blk.extend_from_slice(&[0x00, 3, 0x00, b'a', b'b', b'c']);
        blk.push(0xee); // lixo depois do bloco
        let (out, used) = unpack_block(&blk).unwrap();
        assert_eq!(out, b"abc");
        assert_eq!(used, blk.len() - 1);
    }
}
