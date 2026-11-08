// SPDX-License-Identifier: GPL-3.0-or-later
//! `PLAY.VCR`: gravação de comandos de desenho (`docs/formats/vcr.md`). O executável não lê
//! o arquivo; o formato vem dos dados: bloco `VCR:` com a fita (comandos de 1 byte com
//! argumentos de tamanho fixo) e bloco `TAG:` com os nomes que os comandos citam.

use crate::bytes::{slice, u16_at};
use crate::chunk::chunks;
use crate::{FormatError, Result};

const WHAT: &str = "VCR";

/// Comandos da fita.
pub mod op {
    pub const HEADER: u8 = 0xf2;
    pub const CLOSE_RECT: u8 = 0xf5;
    pub const CLIP: u8 = 0xf6;
    pub const TINT: u8 = 0xf7;
    pub const RECT: u8 = 0xf8;
    pub const MODE_OFF: u8 = 0xf9;
    pub const MODE_ON: u8 = 0xfa;
    pub const DRAW_SCALED: u8 = 0xfd;
    pub const SOUND: u8 = 0xf4;
    pub const STATUS_BAR: u8 = 0xef;
    pub const DRAW: u8 = 0xf1;
    pub const END_FRAME: u8 = 0xfc;
    pub const END: u8 = 0xfb;
}

/// Tamanho do comando com o próprio byte; `None` para um byte que não é comando.
pub fn op_len(code: u8) -> Option<usize> {
    Some(match code {
        op::HEADER | op::CLIP | op::TINT | op::RECT => 9,
        op::MODE_OFF | op::MODE_ON => 3,
        op::DRAW_SCALED => 18,
        op::SOUND | op::DRAW => 13,
        op::CLOSE_RECT | op::STATUS_BAR | op::END_FRAME | op::END => 1,
        _ => return None,
    })
}

/// Um comando: o código e os argumentos (sem o código).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Op {
    pub code: u8,
    pub args: Vec<u8>,
}

impl Op {
    /// `i16` do argumento na posição `at` (bytes).
    pub fn i16(&self, at: usize) -> Option<i16> {
        let b = self.args.get(at..at + 2)?;
        Some(i16::from_le_bytes([b[0], b[1]]))
    }

    pub fn u16(&self, at: usize) -> Option<u16> {
        self.i16(at).map(|v| v as u16)
    }
}

/// O arquivo inteiro.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vcr {
    pub ops: Vec<Op>,
    /// Marcas do `TAG:` (id, nome), na ordem do arquivo.
    pub tags: Vec<(u16, String)>,
}

impl Vcr {
    pub fn parse(data: &[u8]) -> Result<Self> {
        let mut tape = None;
        let mut tags = Vec::new();
        for c in chunks(data) {
            let c = c?;
            match &c.tag {
                b"VCR:" => tape = Some(parse_tape(c.data)?),
                b"TAG:" => tags = parse_tags(c.data)?,
                _ => {}
            }
        }
        let ops = tape.ok_or(FormatError::BadMagic {
            what: WHAT,
            expected: b"VCR:",
            found: data.get(..4).unwrap_or_default().to_vec(),
        })?;
        Ok(Self { ops, tags })
    }

    /// Regrava o arquivo (o `VCR:` e depois o `TAG:`).
    pub fn to_bytes(&self) -> Vec<u8> {
        let tape: Vec<u8> = self
            .ops
            .iter()
            .flat_map(|o| std::iter::once(o.code).chain(o.args.iter().copied()))
            .collect();
        let mut t = (self.tags.len() as u16).to_le_bytes().to_vec();
        for (id, name) in &self.tags {
            t.extend_from_slice(&id.to_le_bytes());
            t.extend(name.chars().map(|c| c as u8));
            t.push(0);
        }
        let mut out = Vec::with_capacity(tape.len() + t.len() + 16);
        for (tag, body) in [(b"VCR:", &tape), (b"TAG:", &t)] {
            out.extend_from_slice(tag);
            out.extend_from_slice(&(body.len() as u32).to_le_bytes());
            out.extend_from_slice(body);
        }
        out
    }

    /// Quadros de tela gravados (comandos `FC`).
    pub fn frames(&self) -> usize {
        self.ops.iter().filter(|o| o.code == op::END_FRAME).count()
    }

    pub fn tag(&self, id: u16) -> Option<&str> {
        self.tags
            .iter()
            .find(|(i, _)| *i == id)
            .map(|(_, n)| n.as_str())
    }
}

fn parse_tape(d: &[u8]) -> Result<Vec<Op>> {
    let mut ops = Vec::new();
    let mut p = 0;
    while p < d.len() {
        let code = d[p];
        let n = op_len(code).ok_or_else(|| FormatError::Unsupported {
            what: WHAT,
            detail: format!("comando 0x{code:02x} no offset {p}"),
        })?;
        let args = slice(d, p + 1, n - 1, WHAT)?.to_vec();
        ops.push(Op { code, args });
        p += n;
    }
    Ok(ops)
}

fn parse_tags(d: &[u8]) -> Result<Vec<(u16, String)>> {
    let n = usize::from(u16_at(d, 0, WHAT)?);
    let mut p = 2;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let id = u16_at(d, p, WHAT)?;
        let rest = d.get(p + 2..).unwrap_or_default();
        let end = rest
            .iter()
            .position(|&b| b == 0)
            .ok_or(FormatError::Truncated {
                what: WHAT,
                offset: p + 2,
            })?;
        out.push((id, rest[..end].iter().map(|&b| b as char).collect()));
        p += 2 + end + 1;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synth() -> Vcr {
        let o = |code: u8| Op {
            code,
            args: vec![7; op_len(code).unwrap() - 1],
        };
        Vcr {
            ops: vec![
                o(op::HEADER),
                o(op::CLIP),
                o(op::DRAW_SCALED),
                o(op::STATUS_BAR),
                o(op::DRAW),
                o(op::END_FRAME),
                o(op::END),
            ],
            tags: vec![(1, "WORLD1.ANM,11310".into()), (2, "SX_3061.WAV".into())],
        }
    }

    #[test]
    fn round_trip_and_unknown_commands() {
        let v = synth();
        let b = v.to_bytes();
        assert_eq!(&b[..4], b"VCR:");
        let back = Vcr::parse(&b).unwrap();
        assert_eq!(back, v);
        assert_eq!(back.frames(), 1);
        assert_eq!(back.tag(2), Some("SX_3061.WAV"));
        // um byte que não é comando
        let mut bad = b.clone();
        bad[8] = 0x10;
        assert!(Vcr::parse(&bad).is_err());
        // comando cortado
        let mut short = b"VCR:".to_vec();
        short.extend_from_slice(&2u32.to_le_bytes());
        short.extend_from_slice(&[op::CLIP, 0]);
        assert!(Vcr::parse(&short).is_err());
    }

    #[test]
    fn real_play_vcr_reads_whole_and_writes_back_equal() {
        let Some(p) = std::env::var_os("OPENHUNTED_DATA") else {
            eprintln!("pulado: OPENHUNTED_DATA não definido");
            return;
        };
        let gd = crate::open_data(std::path::Path::new(&p)).unwrap();
        let ar = gd.resources().unwrap();
        let data = ar.get("PLAY.VCR").unwrap();
        let v = Vcr::parse(data).unwrap();
        assert_eq!(v.to_bytes(), data, "relido byte a byte");
        assert_eq!(v.frames(), 634);
        assert_eq!(v.tags.len(), 121);
        assert_eq!(v.ops.first().map(|o| o.code), Some(op::HEADER));
        assert_eq!(v.ops.last().map(|o| o.code), Some(op::END));
        assert_eq!(v.tag(1), Some("WORLD1.ANM,11310"));
        // o som do primeiro `F4` é uma marca do `TAG:`
        let snd = v.ops.iter().find(|o| o.code == op::SOUND).unwrap();
        assert_eq!(snd.u16(0), Some(3061));
        assert!(v.tags.iter().any(|(_, n)| n == "SX_3061.WAV"));
    }
}
