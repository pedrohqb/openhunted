// SPDX-License-Identifier: GPL-3.0-or-later
//! Bitmaps `BMP:` com um ou mais quadros. Formato em `docs/formats/bmp.md`.

use crate::bytes::{slice, u8_at, u16_at};
use crate::compress::unpack_block;
use crate::{FormatError, Result, find_chunk};

const WHAT: &str = "bitmap";
const MAX_VERSION: u16 = 1000;

/// Um quadro: índices de paleta, linha a linha, na ordem em que estão gravados.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub width: u16,
    pub height: u16,
    pub pixels: Vec<u8>,
}

/// Bitmap com um ou mais quadros.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bitmap {
    pub frames: Vec<Frame>,
}

impl Bitmap {
    /// Lê um recurso que contém um bloco `BMP:`. Como o original (`0x0045b914`), lê os
    /// quadros em sequência e ignora a tabela de offsets.
    pub fn parse(data: &[u8]) -> Result<Self> {
        let chunk = find_chunk(data, b"BMP:")?.ok_or(FormatError::BadMagic {
            what: WHAT,
            expected: b"BMP:",
            found: data.get(..4).unwrap_or(data).to_vec(),
        })?;
        let d = chunk.data;
        let version = u16_at(d, 0, WHAT)?;
        if version > MAX_VERSION {
            return Err(FormatError::Unsupported {
                what: WHAT,
                detail: format!("versão {version}"),
            });
        }
        let count = u16_at(d, 2, WHAT)? as usize;
        let mut pos = 4 + 4 * count;
        let mut frames = Vec::with_capacity(count);
        for _ in 0..count {
            let width = u16_at(d, pos, WHAT)?;
            let height = u16_at(d, pos + 2, WHAT)?;
            pos += 4;
            let size = width as usize * height as usize;
            let kind = u8_at(d, pos, WHAT)?;
            let pixels = if kind == 0 {
                // cru: tipo, u32 tamanho, largura × altura bytes
                let px = slice(d, pos + 5, size, WHAT)?.to_vec();
                pos += 5 + size;
                px
            } else {
                let (px, used) = unpack_block(slice(d, pos, d.len() - pos, WHAT)?)?;
                if px.len() != size {
                    return Err(FormatError::Truncated {
                        what: WHAT,
                        offset: pos,
                    });
                }
                pos += used;
                px
            };
            frames.push(Frame {
                width,
                height,
                pixels,
            });
        }
        Ok(Bitmap { frames })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Monta um recurso `BMP:` com quadros crus.
    pub(crate) fn synth_bmp(frames: &[(u16, u16, Vec<u8>)]) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&1000u16.to_le_bytes());
        body.extend_from_slice(&(frames.len() as u16).to_le_bytes());
        body.extend(std::iter::repeat_n(0u8, 4 * frames.len()));
        for (w, h, px) in frames {
            body.extend_from_slice(&w.to_le_bytes());
            body.extend_from_slice(&h.to_le_bytes());
            body.push(0);
            body.extend_from_slice(&(px.len() as u32).to_le_bytes());
            body.extend_from_slice(px);
        }
        let mut v = b"BMP:".to_vec();
        v.extend_from_slice(&(body.len() as u32).to_le_bytes());
        v.extend(body);
        v
    }

    #[test]
    fn raw_single_frame() {
        let b = Bitmap::parse(&synth_bmp(&[(2, 2, vec![1, 2, 3, 4])])).unwrap();
        assert_eq!(b.frames.len(), 1);
        assert_eq!((b.frames[0].width, b.frames[0].height), (2, 2));
        assert_eq!(b.frames[0].pixels, vec![1, 2, 3, 4]);
    }

    #[test]
    fn compressed_and_raw_frames() {
        let mut body = Vec::new();
        body.extend_from_slice(&1000u16.to_le_bytes());
        body.extend_from_slice(&2u16.to_le_bytes());
        body.extend_from_slice(&[0u8; 8]);
        // quadro 0: 3×1 comprimido (método 3, literais "abc")
        body.extend_from_slice(&3u16.to_le_bytes());
        body.extend_from_slice(&1u16.to_le_bytes());
        body.push(0x83);
        body.extend_from_slice(&10u32.to_le_bytes());
        body.extend_from_slice(&3u32.to_le_bytes());
        body.extend_from_slice(&[0x00, 3, 0x00, b'a', b'b', b'c']);
        // quadro 1: 1×1 cru
        body.extend_from_slice(&1u16.to_le_bytes());
        body.extend_from_slice(&1u16.to_le_bytes());
        body.push(0);
        body.extend_from_slice(&1u32.to_le_bytes());
        body.push(9);
        let mut v = b"BMP:".to_vec();
        v.extend_from_slice(&(body.len() as u32).to_le_bytes());
        v.extend(body);
        let b = Bitmap::parse(&v).unwrap();
        assert_eq!(b.frames[0].pixels, b"abc");
        assert_eq!(b.frames[1].pixels, vec![9]);
    }

    #[test]
    fn newer_version_is_unsupported() {
        let mut v = synth_bmp(&[(1, 1, vec![0])]);
        v[8..10].copy_from_slice(&1001u16.to_le_bytes());
        assert!(matches!(
            Bitmap::parse(&v),
            Err(crate::FormatError::Unsupported { .. })
        ));
    }

    #[test]
    fn truncated_bitmap_never_panics() {
        let v = synth_bmp(&[(2, 2, vec![1, 2, 3, 4]), (1, 1, vec![5])]);
        for n in 0..v.len() {
            assert!(Bitmap::parse(&v[..n]).is_err());
        }
    }
}
