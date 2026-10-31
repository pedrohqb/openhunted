// SPDX-License-Identifier: GPL-3.0-or-later
//! Fontes (`.PFT`) da biblioteca gráfica Phoenix. Formato em `docs/formats/pft.md`.

use crate::bytes::{slice, u32_at};
use crate::{FormatError, Result};

const WHAT: &str = "fonte PFT";
/// Limite de sanidade para larguras de glifo e alturas (as fontes reais têm de 18 a 46 px).
const MAX_DIM: usize = 255;

/// Glifo: largura e pixels (índices, sem preenchimento), com a altura da fonte.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Glyph {
    pub width: u16,
    pub pixels: Vec<u8>,
}

/// Fonte: altura comum e um glifo opcional por byte Latin-1.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Font {
    pub height: u16,
    /// Os 11 campos do cabeçalho `head` da fonte, depois da palavra de versão
    /// (ver `docs/formats/pft.md`).
    pub header: [u32; 11],
    pub glyphs: Vec<Option<Glyph>>,
}

fn tag_at(d: &[u8], pos: usize) -> Result<([u8; 4], usize)> {
    let t = slice(d, pos, 4, WHAT)?;
    Ok(([t[0], t[1], t[2], t[3]], u32_at(d, pos + 4, WHAT)? as usize))
}

fn expect(d: &[u8], pos: usize, tag: &'static [u8; 4]) -> Result<usize> {
    let (t, size) = tag_at(d, pos)?;
    if &t != tag {
        return Err(FormatError::BadMagic {
            what: WHAT,
            expected: tag,
            found: t.to_vec(),
        });
    }
    Ok(size)
}

impl Font {
    /// Lê uma fonte `PFNT` (cabeçalho `head` e um `PBMA` com um `PBMP` por glifo).
    /// Como o leitor original (`0x00455719`), percorre os blocos internos e não confia nos
    /// tamanhos declarados dos contêineres, que nos dados vêm menores que o conteúdo.
    pub fn parse(data: &[u8]) -> Result<Self> {
        expect(data, 0, b"PFNT")?;
        let head_size = expect(data, 8, b"head")?;
        let head = slice(data, 16, head_size, WHAT)?;
        let mut header = [0u32; 11];
        for (i, h) in header.iter_mut().enumerate() {
            *h = u32_at(head, 4 + 4 * i, WHAT)?;
        }
        let first_char = header[1] as usize;
        let count = header[2] as usize;
        let height = u16::try_from(header[3])
            .ok()
            .filter(|&h| h as usize <= MAX_DIM)
            .ok_or_else(|| FormatError::Unsupported {
                what: WHAT,
                detail: format!("altura {}", header[3]),
            })?;
        let mut pos = 16 + head_size;
        expect(data, pos, b"PBMA")?;
        pos += 8;
        let arr_head = expect(data, pos, b"head")?;
        let bitmaps = u32_at(data, pos + 12, WHAT)? as usize;
        pos += 8 + arr_head;
        let mut glyphs: Vec<Option<Glyph>> = vec![None; 256];
        // Código de caractere de cada glifo: bloco `rmap` quando existe; senão sequencial.
        let mut codes: Option<Vec<usize>> = None;
        let mut read = 0;
        while read < bitmaps {
            let (t, size) = tag_at(data, pos)?;
            match &t {
                b"rmap" => {
                    let raw = slice(data, pos + 8, size, WHAT)?;
                    codes = Some(
                        raw.chunks_exact(4)
                            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]) as usize)
                            .collect(),
                    );
                    pos += 8 + size;
                }
                b"PBMP" => {
                    let (g, next) = read_glyph(data, pos + 8)?;
                    let code = match &codes {
                        Some(c) => c.get(read).copied(),
                        None => Some(first_char + read),
                    };
                    if let Some(slot) = code.and_then(|c| glyphs.get_mut(c)) {
                        *slot = Some(g);
                    }
                    read += 1;
                    pos = next;
                }
                _ => pos += 8 + size,
            }
        }
        if count != bitmaps {
            return Err(FormatError::Unsupported {
                what: WHAT,
                detail: format!("{count} caracteres para {bitmaps} glifos"),
            });
        }
        Ok(Font {
            height,
            header,
            glyphs,
        })
    }
}

/// Lê `head` + `data` de um glifo; devolve o glifo e a posição seguinte.
fn read_glyph(d: &[u8], pos: usize) -> Result<(Glyph, usize)> {
    let hs = expect(d, pos, b"head")?;
    let width = u32_at(d, pos + 12, WHAT)? as usize;
    let height = u32_at(d, pos + 16, WHAT)? as usize;
    let depth = u32_at(d, pos + 20, WHAT)?;
    if depth != 8 || width == 0 || width > MAX_DIM || height == 0 || height > MAX_DIM {
        return Err(FormatError::Unsupported {
            what: WHAT,
            detail: format!("glifo {width}x{height} com {depth} bits"),
        });
    }
    let dpos = pos + 8 + hs;
    let ds = expect(d, dpos, b"data")?;
    let stride = (width + 3) & !3;
    let raw = slice(d, dpos + 8, ds, WHAT)?;
    let need = stride * height;
    let rows = slice(raw, 0, need, WHAT)?;
    let mut pixels = Vec::with_capacity(width * height);
    for r in 0..height {
        pixels.extend_from_slice(&rows[r * stride..r * stride + width]);
    }
    Ok((
        Glyph {
            width: width as u16,
            pixels,
        },
        dpos + 8 + ds,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::mkpft;

    #[test]
    fn pft_parses_synthetic() {
        let f = Font::parse(&mkpft(
            65,
            &[(1, vec![1, 2]), (5, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10])],
        ))
        .unwrap();
        assert_eq!(f.height, 2);
        assert_eq!(f.glyphs.len(), 256);
        assert!(f.glyphs[64].is_none());
        let a = f.glyphs[b'A' as usize].as_ref().unwrap();
        assert_eq!((a.width, a.pixels.clone()), (1, vec![1, 2]));
        let b = f.glyphs[b'B' as usize].as_ref().unwrap();
        assert_eq!(b.width, 5);
        assert_eq!(b.pixels, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    }

    #[test]
    fn absurd_dimensions_are_rejected() {
        // glifo de largura 65535 e altura 0 (passa na conferência de tamanho dos dados)
        let mut v = mkpft(65, &[(1, vec![1, 2])]);
        let at = v.windows(4).rposition(|w| w == b"head").unwrap();
        v[at + 12..at + 16].copy_from_slice(&65535u32.to_le_bytes());
        v[at + 16..at + 20].copy_from_slice(&0u32.to_le_bytes());
        assert!(Font::parse(&v).is_err());
        // altura da fonte 4000
        let mut v = mkpft(65, &[(1, vec![1, 2])]);
        v[16 + 4 + 12..16 + 4 + 16].copy_from_slice(&4000u32.to_le_bytes());
        assert!(Font::parse(&v).is_err());
    }

    #[test]
    fn non_font_is_bad_magic() {
        assert!(matches!(
            Font::parse(b"RIFF\0\0\0\0"),
            Err(crate::FormatError::BadMagic { .. })
        ));
    }

    #[test]
    fn truncated_pft_never_panics() {
        let v = mkpft(65, &[(1, vec![1, 2]), (5, vec![0; 10])]);
        for n in 0..v.len() {
            assert!(Font::parse(&v[..n]).is_err());
        }
    }
}
