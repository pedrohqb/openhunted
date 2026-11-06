// SPDX-License-Identifier: GPL-3.0-or-later
//! Esqueletos (`.SKE`, "Hunter Hunted Skeleton File"): captura de movimento por rótulo da
//! animação. Formato em `docs/formats/ske.md`; leitor original `0x00409dff`.

use crate::bytes::{slice, u16_at};
use crate::{FormatError, Result};

const WHAT: &str = "esqueleto SKE";
const SIGNATURE: &[u8] = b"Hunter Hunted Skeleton File";
/// Fim do texto do cabeçalho (o leitor descarta tudo até ele, inclusive).
const EOF_MARK: u8 = 0x1a;
/// Limite de segurança para quadros e juntas por entrada (dados corrompidos).
const MAX_FRAMES: usize = 4096;
const MAX_JOINTS: usize = 256;

/// Uma entrada do esqueleto: a do rótulo de mesmo índice no `.ANM` do personagem.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct SkeletonEntry {
    /// Ângulo de giro do corpo em torno do eixo vertical (0x10000 = 360°).
    pub angle: u16,
    /// Deslocamento fixo da raiz.
    pub offset: (i16, i16),
    /// Posição acumulada da raiz em cada quadro (unidades do esqueleto; y para cima; o
    /// "para a frente" é x negativo).
    pub roots: Vec<(i16, i16)>,
    /// Juntas por quadro (0 = sem juntas).
    pub joints_per_frame: usize,
    /// Juntas (x, y, z), quadro a quadro: `roots.len() · joints_per_frame` triplas.
    pub joints: Vec<(i16, i16, i16)>,
}

impl SkeletonEntry {
    /// Número de quadros.
    pub fn frames(&self) -> usize {
        self.roots.len()
    }

    /// Juntas do quadro `f` (vazio fora do intervalo).
    pub fn joints_of(&self, f: usize) -> &[(i16, i16, i16)] {
        let m = self.joints_per_frame;
        self.joints
            .get(f.saturating_mul(m)..f.saturating_add(1).saturating_mul(m))
            .unwrap_or(&[])
    }
}

/// Arquivo de esqueleto inteiro.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Skeleton {
    /// Identificador do personagem (1 = Jake, 2 = Garathe).
    pub id: i16,
    /// Largura e altura de referência (K, em unidades do esqueleto).
    pub width: i16,
    pub height: i16,
    /// Entradas indexadas pelo rótulo do `.ANM`.
    pub entries: Vec<SkeletonEntry>,
}

fn i16_at(data: &[u8], off: usize) -> Result<i16> {
    Ok(u16_at(data, off, WHAT)? as i16)
}

impl Skeleton {
    /// Lê um recurso `.SKE`.
    pub fn parse(data: &[u8]) -> Result<Self> {
        Self::parse_with_len(data).map(|(s, _)| s)
    }

    /// Lê um recurso `.SKE` e devolve também quantos bytes foram consumidos.
    pub fn parse_with_len(data: &[u8]) -> Result<(Self, usize)> {
        if !data.starts_with(SIGNATURE) {
            let n = data.len().min(SIGNATURE.len());
            return Err(FormatError::BadMagic {
                what: WHAT,
                expected: SIGNATURE,
                found: data[..n].to_vec(),
            });
        }
        let mut p = data
            .iter()
            .position(|&b| b == EOF_MARK)
            .ok_or(FormatError::Truncated {
                what: WHAT,
                offset: data.len(),
            })?
            + 1;
        let id = i16_at(data, p)?;
        let count = usize::try_from(i16_at(data, p + 2)?).unwrap_or(0);
        let width = i16_at(data, p + 4)?;
        let height = i16_at(data, p + 6)?;
        p += 8;
        let mut entries = Vec::with_capacity(count.min(1024));
        for _ in 0..count {
            let n = usize::try_from(i16_at(data, p)?).unwrap_or(0);
            if n > MAX_FRAMES {
                return Err(FormatError::Unsupported {
                    what: WHAT,
                    detail: format!("{n} quadros"),
                });
            }
            let angle = u16_at(data, p + 2, WHAT)?;
            let offset = (i16_at(data, p + 4)?, i16_at(data, p + 6)?);
            p += 8;
            let raw = slice(data, p, n * 4, WHAT)?;
            let roots = raw
                .chunks_exact(4)
                .map(|c| {
                    (
                        i16::from_le_bytes([c[0], c[1]]),
                        i16::from_le_bytes([c[2], c[3]]),
                    )
                })
                .collect();
            p += n * 4;
            let m = usize::try_from(i16_at(data, p)?).unwrap_or(0);
            if m > MAX_JOINTS {
                return Err(FormatError::Unsupported {
                    what: WHAT,
                    detail: format!("{m} juntas"),
                });
            }
            p += 2;
            let raw = slice(data, p, n * m * 6, WHAT)?;
            let joints = raw
                .chunks_exact(6)
                .map(|c| {
                    (
                        i16::from_le_bytes([c[0], c[1]]),
                        i16::from_le_bytes([c[2], c[3]]),
                        i16::from_le_bytes([c[4], c[5]]),
                    )
                })
                .collect();
            p += n * m * 6;
            entries.push(SkeletonEntry {
                angle,
                offset,
                roots,
                joints_per_frame: m,
                joints,
            });
        }
        Ok((
            Skeleton {
                id,
                width,
                height,
                entries,
            },
            p,
        ))
    }

    /// Entrada do rótulo `label`, se houver e tiver quadros.
    pub fn entry(&self, label: u16) -> Option<&SkeletonEntry> {
        self.entries
            .get(usize::from(label))
            .filter(|e| !e.roots.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::mkske;

    #[test]
    fn parses_header_entries_roots_and_joints() {
        let data = mkske(
            1,
            (20, 57),
            &[
                (0, (0, 0), vec![], 0),
                (
                    0x4000,
                    (1, -2),
                    vec![(0, 0), (-5, 3)],
                    2, // 2 juntas
                ),
            ],
        );
        let s = Skeleton::parse(&data).unwrap();
        assert_eq!((s.id, s.width, s.height), (1, 20, 57));
        assert_eq!(s.entries.len(), 2);
        assert!(s.entry(0).is_none(), "entrada vazia");
        let e = s.entry(1).unwrap();
        assert_eq!((e.angle, e.offset), (0x4000, (1, -2)));
        assert_eq!(e.roots, vec![(0, 0), (-5, 3)]);
        assert_eq!(e.frames(), 2);
        assert_eq!(e.joints.len(), 4);
        assert_eq!(e.joints_of(1), &[(10, 11, 12), (13, 14, 15)]);
        assert!(e.joints_of(2).is_empty());
    }

    #[test]
    fn rejects_bad_signature_and_truncation() {
        assert!(matches!(
            Skeleton::parse(b"Climber Border File"),
            Err(FormatError::BadMagic { .. })
        ));
        let data = mkske(1, (20, 57), &[(0, (0, 0), vec![(1, 2), (3, 4)], 0)]);
        for cut in [data.len() - 1, data.len() - 5, 40] {
            assert!(Skeleton::parse(&data[..cut]).is_err(), "corte em {cut}");
        }
        assert!(Skeleton::parse(b"Hunter Hunted Skeleton File.\r\n\0").is_err());
    }
}
