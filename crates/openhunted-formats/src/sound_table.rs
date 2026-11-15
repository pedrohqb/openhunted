// SPDX-License-Identifier: GPL-3.0-or-later
//! `SOUND.TBL`: sons por quadro de animação (`0x0043c0ce`, `docs/re/som.md`).
//!
//! Texto até `0x1a`; `i32` versão (1000); três grupos, cada um com `i32` n e n listas
//! (`i32` contagem + contagem valores). Cada valor: bits 0–14 = id do som, bit 15 = laço,
//! bits 16–31 = prioridade. O grupo 2 é indexado pelo tipo do objeto (até 110 listas) e cada
//! lista pelo quadro absoluto da animação.

use crate::bytes::u32_at;
use crate::{FormatError, Result};

/// Um som da tabela.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TableSound {
    pub id: u16,
    pub looped: bool,
    pub priority: u16,
}

impl TableSound {
    /// Decodifica um valor de 32 bits (id 0 → nada).
    pub fn decode(v: u32) -> Option<Self> {
        let id = (v & 0x7fff) as u16;
        (id != 0).then_some(Self {
            id,
            looped: v & 0x8000 != 0,
            priority: (v >> 16) as u16,
        })
    }
}

/// A tabela inteira: os três grupos, com os valores crus.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SoundTable {
    pub version: i32,
    pub groups: [Vec<Vec<u32>>; 3],
}

/// Listas do grupo 2 lidas pelo original (`0x43c0ce`: o excedente é pulado).
pub const MAX_TYPES: usize = 110;

impl SoundTable {
    pub fn parse(data: &[u8]) -> Result<Self> {
        const WHAT: &str = "SOUND.TBL";
        let mut off =
            data.iter()
                .position(|&b| b == 0x1a)
                .map(|p| p + 1)
                .ok_or(FormatError::Truncated {
                    what: WHAT,
                    offset: data.len(),
                })?;
        let next = |off: &mut usize| -> Result<u32> {
            let v = u32_at(data, *off, WHAT)?;
            *off += 4;
            Ok(v)
        };
        let version = next(&mut off)? as i32;
        let mut groups: [Vec<Vec<u32>>; 3] = Default::default();
        for g in groups.iter_mut() {
            let n = next(&mut off)? as usize;
            for _ in 0..n {
                let c = next(&mut off)? as usize;
                if c > data.len() / 4 {
                    return Err(FormatError::Truncated {
                        what: WHAT,
                        offset: off,
                    });
                }
                let list = (0..c).map(|_| next(&mut off)).collect::<Result<Vec<_>>>()?;
                g.push(list);
            }
        }
        Ok(Self { version, groups })
    }

    /// O som do quadro absoluto `frame` do objeto de tipo `kind` (grupo 2), se houver.
    pub fn frame_sound(&self, kind: usize, frame: usize) -> Option<TableSound> {
        if kind >= MAX_TYPES {
            return None;
        }
        let v = *self.groups[2].get(kind)?.get(frame)?;
        TableSound::decode(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synth() -> Vec<u8> {
        let mut d = b"texto\x1a".to_vec();
        let mut put = |v: u32| d.extend_from_slice(&v.to_le_bytes());
        put(1000);
        put(0); // grupo 0 vazio
        put(1); // grupo 1: uma lista vazia
        put(0);
        put(2); // grupo 2: tipos 0 e 1
        put(0);
        put(3);
        put(0);
        put(9 << 16 | 3040);
        put(2 << 16 | 0x8000 | 3016);
        d
    }

    #[test]
    fn decodes_frames_of_each_type() {
        let t = SoundTable::parse(&synth()).unwrap();
        assert_eq!(t.version, 1000);
        assert_eq!(t.frame_sound(1, 0), None, "id 0: nada");
        assert_eq!(
            t.frame_sound(1, 1),
            Some(TableSound {
                id: 3040,
                looped: false,
                priority: 9
            })
        );
        let l = t.frame_sound(1, 2).unwrap();
        assert!(l.looped && l.id == 3016 && l.priority == 2);
        assert_eq!(t.frame_sound(1, 3), None);
        assert_eq!(t.frame_sound(5, 0), None);
    }

    #[test]
    fn truncated_table_is_an_error() {
        let d = synth();
        assert!(SoundTable::parse(&d[..d.len() - 2]).is_err());
        assert!(SoundTable::parse(b"sem marca").is_err());
    }
}
