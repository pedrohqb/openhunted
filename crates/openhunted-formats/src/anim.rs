// SPDX-License-Identifier: GPL-3.0-or-later
//! Animações (`.ANM`). Formato em `docs/formats/anm.md`.

use crate::bytes::{slice, u16_at};
use crate::lists::{asciiz, id_name_list};
use crate::{FormatError, Result, chunks};

const WHAT: &str = "animação";
const MAX_VERSION: u16 = 1004;
/// Tamanho de cada instrução em palavras (opcode incluído), por opcode (`0x00480608`).
const OP_WORDS: [usize; 11] = [1, 1, 6, 7, 6, 2, 2, 6, 4, 1, 1];

/// Quadro de uma animação: retângulo relativo à origem e início do roteiro.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct AnimFrame {
    pub x: i16,
    pub y: i16,
    pub width: i16,
    pub height: i16,
    /// Offset, em palavras, da primeira instrução do quadro no roteiro.
    pub script_offset: u16,
    /// Valor extra das versões ≥ 1002 (o original o descarta).
    pub extra: i16,
}

/// Rótulo: quadro inicial, id e nome (do bloco `LAB:`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct AnimLabel {
    pub frame: u16,
    pub id: u16,
    pub name: Option<String>,
}

/// Instrução do roteiro.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Instruction {
    /// Posição em palavras desde o início do roteiro.
    pub offset: u16,
    pub op: u16,
    pub args: Vec<i16>,
}

/// Animação.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Animation {
    pub version: u16,
    pub width: u16,
    pub height: u16,
    /// Três valores das versões ≥ 1002 (o original os descarta).
    pub extra: [i16; 3],
    /// Tabela `A` (vazia na maioria dos ANM; quando não, mapeia quadros lógicos em físicos (`docs/formats/anm.md`) da versão 1.00 vistos até aqui).
    pub table_a: Vec<u16>,
    pub frames: Vec<AnimFrame>,
    pub labels: Vec<AnimLabel>,
    pub script: Vec<Instruction>,
    /// Bitmaps usados: (id, nome do arquivo), do bloco `TAG:`.
    pub bitmaps: Vec<(u16, String)>,
    /// Nome do arquivo de paleta do bloco `PAL:`, sem o caminho de autoria.
    pub palette: Option<String>,
}

impl Animation {
    /// Lê um recurso `.ANM` (blocos `ANM:`, `TAG:`, `LAB:`, `PAL:`).
    pub fn parse(data: &[u8]) -> Result<Self> {
        let mut anm = None;
        let mut bitmaps = Vec::new();
        let mut label_names = Vec::new();
        let mut palette = None;
        for c in chunks(data) {
            let c = c?;
            match &c.tag {
                b"ANM:" => anm = Some(c.data),
                b"TAG:" => bitmaps = id_name_list(c.data, WHAT)?,
                b"LAB:" => label_names = id_name_list(c.data, WHAT)?,
                b"PAL:" => {
                    let (path, _) = asciiz(c.data, 0, WHAT)?;
                    palette = path.rsplit(['\\', '/']).next().map(str::to_string);
                }
                _ => {}
            }
        }
        let d = anm.ok_or(FormatError::BadMagic {
            what: WHAT,
            expected: b"ANM:",
            found: data.get(..4).unwrap_or(data).to_vec(),
        })?;
        let mut a = parse_body(d)?;
        for l in &mut a.labels {
            l.name = label_names
                .iter()
                .find(|(id, _)| *id == l.id)
                .map(|(_, n)| n.clone());
        }
        a.bitmaps = bitmaps;
        a.palette = palette;
        Ok(a)
    }
}

fn parse_body(d: &[u8]) -> Result<Animation> {
    let magic = slice(d, 0, 2, WHAT)?;
    if magic != b"TB" {
        return Err(FormatError::BadMagic {
            what: WHAT,
            expected: b"TB",
            found: magic.to_vec(),
        });
    }
    let r = |off: usize| u16_at(d, off, WHAT);
    let s = |off: usize| u16_at(d, off, WHAT).map(|v| v as i16);
    let version = r(2)?;
    if version > MAX_VERSION {
        return Err(FormatError::Unsupported {
            what: WHAT,
            detail: format!("versão {version}"),
        });
    }
    let (na, nb, nc, nd) = (
        r(4)? as usize,
        r(6)? as usize,
        r(8)? as usize,
        r(10)? as usize,
    );
    let (width, height) = (r(12)?, r(14)?);
    let mut pos = 16;
    let mut extra = [0i16; 3];
    if version >= 1002 {
        for (i, e) in extra.iter_mut().enumerate() {
            *e = s(pos + 2 * i)?;
        }
        pos += 6;
    }
    let mut table_a = Vec::with_capacity(na.min(4096));
    for i in 0..na {
        table_a.push(r(pos + 2 * i)?);
    }
    pos += 2 * na;
    let mut frames = Vec::with_capacity(nb.min(4096));
    for _ in 0..nb {
        let mut f = AnimFrame {
            x: s(pos)?,
            y: s(pos + 2)?,
            width: s(pos + 4)?,
            height: s(pos + 6)?,
            script_offset: r(pos + 8)?,
            extra: 0,
        };
        pos += 10;
        if version >= 1002 {
            f.extra = s(pos)?;
            pos += 2;
        }
        frames.push(f);
    }
    let mut labels = Vec::with_capacity(nc.min(4096));
    for _ in 0..nc {
        labels.push(AnimLabel {
            frame: r(pos)?,
            id: r(pos + 2)?,
            name: None,
        });
        pos += 4;
    }
    let script_bytes = slice(d, pos, nd, WHAT)?;
    let mut script = Vec::new();
    let total = nd / 2;
    let mut w = 0;
    while w < total {
        let op = u16_at(script_bytes, 2 * w, WHAT)?;
        let size = *OP_WORDS.get(op as usize).ok_or(FormatError::Unsupported {
            what: WHAT,
            detail: format!("opcode {op} na palavra {w}"),
        })?;
        let mut args = Vec::with_capacity(size - 1);
        for k in 1..size {
            args.push(u16_at(script_bytes, 2 * (w + k), WHAT)? as i16);
        }
        script.push(Instruction {
            offset: w as u16,
            op,
            args,
        });
        w += size;
    }
    Ok(Animation {
        version,
        width,
        height,
        extra,
        table_a,
        frames,
        labels,
        script,
        bitmaps: Vec::new(),
        palette: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(tag: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut v = tag.to_vec();
        v.extend_from_slice(&(data.len() as u32).to_le_bytes());
        v.extend_from_slice(data);
        v
    }

    fn words(ws: &[i16]) -> Vec<u8> {
        ws.iter().flat_map(|w| w.to_le_bytes()).collect()
    }

    /// Animação sintética: 1 quadro que desenha o quadro 0 do bitmap 2.
    pub(crate) fn synth_anm() -> Vec<u8> {
        let script = words(&[2, 2, 0, 5, 6, 0, 0, 1]);
        let mut anm = b"TB".to_vec();
        anm.extend(words(&[
            1004,
            0,
            1,
            1,
            script.len() as i16,
            22,
            17,
            20,
            55,
            66,
        ]));
        anm.extend(words(&[-7, 1, 20, 20, 0, 0])); // quadro
        anm.extend(words(&[0, 1])); // rótulo: quadro 0, id 1
        anm.extend(script);
        let mut tag = words(&[1, 2]);
        tag.extend_from_slice(b"oan00.bmp\0");
        let mut lab = words(&[1, 1]);
        lab.extend_from_slice(b"parado\0");
        let mut v = block(b"ANM:", &anm);
        v.extend(block(b"TAG:", &tag));
        v.extend(block(b"LAB:", &lab));
        v.extend(block(b"PAL:", b"J:\\hunter\\DLS\\world1.pal\0"));
        v
    }

    #[test]
    fn anm_parses_synthetic() {
        let a = Animation::parse(&synth_anm()).unwrap();
        assert_eq!(a.version, 1004);
        assert_eq!((a.width, a.height), (22, 17));
        assert_eq!(a.frames.len(), 1);
        assert_eq!((a.frames[0].x, a.frames[0].y), (-7, 1));
        assert_eq!(a.labels[0].frame, 0);
        assert_eq!(a.labels[0].name.as_deref(), Some("parado"));
        assert_eq!(a.bitmaps, vec![(2, "oan00.bmp".to_string())]);
        assert_eq!(a.palette.as_deref(), Some("world1.pal"));
        assert_eq!(a.script.len(), 3);
        assert_eq!(a.script[0].op, 2);
        assert_eq!(a.script[0].args, vec![2, 0, 5, 6, 0]);
        assert_eq!(a.script[1].offset, 6);
    }

    #[test]
    fn unknown_opcode_is_unsupported() {
        let mut v = synth_anm();
        // primeiro opcode do roteiro: depois de 8 + 2 + 20 + 12 + 4 bytes
        let off = 8 + 2 + 20 + 12 + 4;
        v[off..off + 2].copy_from_slice(&11i16.to_le_bytes());
        assert!(matches!(
            Animation::parse(&v),
            Err(crate::FormatError::Unsupported { .. })
        ));
    }

    #[test]
    fn truncated_anm_never_panics() {
        let v = synth_anm();
        for n in 0..v.len() {
            let _ = Animation::parse(&v[..n]);
        }
    }
}
