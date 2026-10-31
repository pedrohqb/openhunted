// SPDX-License-Identifier: GPL-3.0-or-later
//! Paletas RIFF `PAL ` (formato padrão do Windows).

use crate::bytes::{slice, u16_at};
use crate::riff::{require, riff_chunks};
use crate::{FormatError, Result};

/// Paleta de 256 cores RGB.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Palette(pub [[u8; 3]; 256]);

impl Palette {
    /// Lê uma paleta RIFF `PAL ` (bloco `data`: versão `0x0300`, contagem, entradas RGBF).
    pub fn parse(data: &[u8]) -> Result<Self> {
        const WHAT: &str = "paleta";
        let chunks = riff_chunks(data, b"PAL ", WHAT)?;
        let d = require(&chunks, b"data", WHAT)?;
        let version = u16_at(d, 0, WHAT)?;
        if version != 0x0300 {
            return Err(FormatError::Unsupported {
                what: WHAT,
                detail: format!("versão {version:#06x}"),
            });
        }
        let count = u16_at(d, 2, WHAT)? as usize;
        let entries = slice(d, 4, count * 4, WHAT)?;
        let mut colors = [[0u8; 3]; 256];
        for (c, e) in colors.iter_mut().zip(entries.chunks_exact(4)) {
            *c = [e[0], e[1], e[2]];
        }
        Ok(Palette(colors))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn synth_pal(first: [u8; 3]) -> Vec<u8> {
        let mut entries = Vec::new();
        entries.extend_from_slice(&0x0300u16.to_le_bytes());
        entries.extend_from_slice(&256u16.to_le_bytes());
        for i in 0..256 {
            let c = if i == 0 { first } else { [i as u8, 0, 0] };
            entries.extend_from_slice(&[c[0], c[1], c[2], 0]);
        }
        let mut data = b"data".to_vec();
        data.extend_from_slice(&(entries.len() as u32).to_le_bytes());
        data.extend(entries);
        let mut v = b"RIFF".to_vec();
        v.extend_from_slice(&((data.len() + 4) as u32).to_le_bytes());
        v.extend_from_slice(b"PAL ");
        v.extend(data);
        v
    }

    #[test]
    fn palette_parses_riff() {
        let p = Palette::parse(&synth_pal([255, 0, 189])).unwrap();
        assert_eq!(p.0[0], [255, 0, 189]);
        assert_eq!(p.0[7], [7, 0, 0]);
    }

    #[test]
    fn truncated_palette_never_panics() {
        let d = synth_pal([1, 2, 3]);
        for n in 0..d.len() {
            assert!(Palette::parse(&d[..n]).is_err());
        }
    }
}
