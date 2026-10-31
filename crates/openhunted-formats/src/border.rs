// SPDX-License-Identifier: GPL-3.0-or-later
//! Cenários de colisão (`.BRD`, "Climber Border File"). Formato em `docs/formats/brd.md`.

use crate::bytes::{slice, u16_at};
use crate::{FormatError, Result, find_chunk};

const WHAT: &str = "cenário BRD";
const SIGNATURE: &[u8] = b"Climber Border File";

/// Caixa de colisão de um cenário.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct BorderBox {
    /// Tipo (o original o guarda num byte).
    pub kind: u16,
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
}

/// Cenário de colisão (`HED:` + `BDR:`), lido como em `0x00430afb`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Border {
    /// Assinatura do cabeçalho ("Climber Border File").
    pub name: String,
    pub version: i16,
    pub boxes: Vec<BorderBox>,
}

impl Border {
    /// Lê um recurso `.BRD`.
    pub fn parse(data: &[u8]) -> Result<Self> {
        let hed = find_chunk(data, b"HED:")?
            .ok_or(FormatError::NotFound("bloco HED:".into()))?
            .data;
        let raw = slice(hed, 0, 20, WHAT)?;
        let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
        if &raw[..end] != SIGNATURE {
            return Err(FormatError::BadMagic {
                what: WHAT,
                expected: SIGNATURE,
                found: raw[..end].to_vec(),
            });
        }
        let version = u16_at(hed, 20, WHAT)? as i16;
        if version > 0 {
            return Err(FormatError::Unsupported {
                what: WHAT,
                detail: format!("versão {version}"),
            });
        }
        let count = u16_at(hed, 22, WHAT)? as i16;
        let count = usize::try_from(count).unwrap_or(0);
        let bdr = find_chunk(data, b"BDR:")?
            .ok_or(FormatError::NotFound("bloco BDR:".into()))?
            .data;
        let mut boxes = Vec::with_capacity(count.min(4096));
        for i in 0..count {
            let p = i * 10;
            boxes.push(BorderBox {
                kind: u16_at(bdr, p, WHAT)?,
                x: u16_at(bdr, p + 2, WHAT)? as i16,
                y: u16_at(bdr, p + 4, WHAT)? as i16,
                width: u16_at(bdr, p + 6, WHAT)?,
                height: u16_at(bdr, p + 8, WHAT)?,
            });
        }
        Ok(Border {
            name: String::from_utf8_lossy(&raw[..end]).into_owned(),
            version,
            boxes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::mkbrd;

    #[test]
    fn brd_parses_synthetic() {
        let b = Border::parse(&mkbrd(&[(2, 0, 0, 128, 12), (2, 50, -12, 20, 30)])).unwrap();
        assert!(b.name.starts_with("Climber"));
        assert_eq!(b.boxes.len(), 2);
        assert_eq!(
            b.boxes[1],
            BorderBox {
                kind: 2,
                x: 50,
                y: -12,
                width: 20,
                height: 30
            }
        );
    }

    #[test]
    fn wrong_signature_is_bad_magic() {
        let mut v = mkbrd(&[]);
        v[8] = b'X';
        assert!(matches!(
            Border::parse(&v),
            Err(crate::FormatError::BadMagic { .. })
        ));
    }

    #[test]
    fn truncated_brd_never_panics() {
        let v = mkbrd(&[(2, 0, 0, 128, 12), (1, 1, 1, 1, 1)]);
        for n in 0..v.len() {
            assert!(Border::parse(&v[..n]).is_err());
        }
    }
}
