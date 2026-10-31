// SPDX-License-Identifier: GPL-3.0-or-later
//! Corpo das fases: lista de objetos e grade de células. Formato em `docs/formats/lev.md`;
//! leitores originais `0x00434170` (objetos), `0x00432319` (células), `0x0042ff4c` (PR) e
//! `0x00430278` (DC).

use crate::bytes::{slice, u16_at, u32_at};
use crate::{FormatError, Result};

const WHAT: &str = "corpo da fase";

/// Objeto de uma fase (lista global ou de uma célula).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct LevelObject {
    /// Tipo do objeto (ver `docs/re/objetos.md`).
    pub kind: u32,
    pub x: i32,
    pub y: i32,
    /// Demais campos inteiros na ordem do arquivo (significado a decifrar).
    pub fields: Vec<i32>,
    /// Pares de pontos (versão ≥ 1006).
    pub points: Vec<(i32, i32)>,
    /// Tabela de valores (16 nas versões < 1004).
    pub values: Vec<i32>,
    /// Gatilhos (versão ≥ 1004): (valor, valor extra da versão ≥ 1005).
    pub triggers: Vec<(i32, Option<i32>)>,
    pub names: [String; 2],
}

/// Item da lista "PR" de uma célula.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Prop {
    pub kind: i16,
    pub rect: [i32; 4],
}

/// Item da lista "DC" de uma célula: decoração animada.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Decoration {
    pub animation: String,
    pub x: i32,
    pub y: i32,
    /// Campos `s16` na ordem do arquivo (significado a decifrar).
    pub fields: Vec<i16>,
}

/// Célula da grade da fase.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Cell {
    pub version: i16,
    /// Animação do cenário (ex.: `WORLD1.ANM`).
    pub animation: String,
    /// Arquivo de colisão (ex.: `C1_07.BRD`).
    pub border: String,
    /// Índice que escolhe o quadro do cenário (ver `docs/re/cenario.md`).
    pub index: i32,
    /// Bloco e colisão espelhados na horizontal (campo `+0x40`; ver `docs/re/cenario.md`).
    pub mirrored: bool,
    /// Demais campos inteiros na ordem do arquivo (significado a decifrar).
    pub fields: Vec<i32>,
    pub triggers: Vec<(i32, Option<i32>)>,
    pub objects: Vec<LevelObject>,
    pub props: Vec<Prop>,
    pub decorations: Vec<Decoration>,
}

struct Cur<'a> {
    d: &'a [u8],
    p: usize,
}

impl Cur<'_> {
    fn s16(&mut self) -> Result<i16> {
        let v = u16_at(self.d, self.p, WHAT)? as i16;
        self.p += 2;
        Ok(v)
    }
    fn i32(&mut self) -> Result<i32> {
        let v = u32_at(self.d, self.p, WHAT)? as i32;
        self.p += 4;
        Ok(v)
    }
    fn count(&mut self) -> Result<usize> {
        let n = self.s16()?;
        usize::try_from(n).map_err(|_| FormatError::Unsupported {
            what: WHAT,
            detail: format!("contagem negativa {n}"),
        })
    }
    fn sz(&mut self) -> Result<String> {
        let rest = slice(self.d, self.p, self.d.len().saturating_sub(self.p), WHAT)?;
        let end = rest
            .iter()
            .position(|&b| b == 0)
            .ok_or(FormatError::Truncated {
                what: WHAT,
                offset: self.p,
            })?;
        let s = rest[..end].iter().map(|&b| b as char).collect();
        self.p += end + 1;
        Ok(s)
    }
    /// Lê a versão de uma lista, tratando a marca opcional (`marca` + 6 bytes → versão 1).
    fn version(&mut self, mark: i16) -> Result<i16> {
        let v = self.s16()?;
        if v == mark {
            self.p += 6;
            return Ok(1);
        }
        Ok(v)
    }
}

/// Lê o corpo de uma fase: lista global de objetos e `cells` células. Exige que o corpo seja
/// consumido por inteiro.
pub fn parse_body(body: &[u8], cells: usize) -> Result<(Vec<LevelObject>, Vec<Option<Cell>>)> {
    let mut c = Cur { d: body, p: 0 };
    let objects = objects(&mut c)?;
    let mut out = Vec::with_capacity(cells.min(4096));
    for _ in 0..cells {
        out.push(cell(&mut c)?);
    }
    if c.p != body.len() {
        return Err(FormatError::Unsupported {
            what: WHAT,
            detail: format!("{} bytes não lidos no fim", body.len() - c.p),
        });
    }
    Ok((objects, out))
}

fn objects(c: &mut Cur) -> Result<Vec<LevelObject>> {
    let v = c.version(0x424f)?;
    let n = c.count()?;
    if n > 0 && !(1003..=1013).contains(&v) {
        return Err(FormatError::Unsupported {
            what: WHAT,
            detail: format!("lista de objetos versão {v}"),
        });
    }
    let mut out = Vec::with_capacity(n.min(4096));
    for _ in 0..n {
        let (kind, x, y) = (c.i32()? as u32, c.i32()?, c.i32()?);
        let mut fields = Vec::new();
        for _ in 0..8 {
            fields.push(c.i32()?); // 15c, 160, 174, 178, 17c, 180, 184, 18c
        }
        if v >= 1007 {
            fields.push(c.i32()?); // 1ec
        }
        for _ in 0..7 {
            fields.push(c.i32()?); // 194, 198, 19c, 1b8, 254, 258, 25c
        }
        if v < 1012 {
            c.i32()?; // descartado pelo original
        }
        if v >= 1010 {
            fields.push(c.i32()?); // 1ac
            fields.push(c.i32()?); // 1a8
        }
        let mut points = Vec::new();
        if v >= 1006 {
            for _ in 0..c.count()? {
                points.push((c.i32()?, c.i32()?));
            }
        }
        let nv = if v >= 1004 { c.count()? } else { 16 };
        let mut values = Vec::with_capacity(nv.min(4096));
        for _ in 0..nv {
            values.push(c.i32()?);
        }
        let mut triggers = Vec::new();
        if v >= 1004 {
            fields.push(c.i32()?); // 118
            for _ in 0..c.count()? {
                let a = c.i32()?;
                let b = if v >= 1005 { Some(c.i32()?) } else { None };
                triggers.push((a, b));
            }
        }
        let names = [c.sz()?, c.sz()?];
        fields.push(c.i32()?);
        fields.push(c.i32()?);
        out.push(LevelObject {
            kind,
            x,
            y,
            fields,
            points,
            values,
            triggers,
            names,
        });
    }
    Ok(out)
}

fn cell(c: &mut Cur) -> Result<Option<Cell>> {
    let mut v = c.s16()?;
    if v == 0x4543 {
        c.s16()?;
        c.i32()?;
        v = 1;
    }
    if v == 0 {
        return Ok(None);
    }
    if !(1005..=1010).contains(&v) {
        // Versões < 1005 guardam o cenário num recurso à parte (bloco CTG:); não vistas.
        return Err(FormatError::Unsupported {
            what: WHAT,
            detail: format!("célula versão {v}"),
        });
    }
    let animation = c.sz()?;
    let border = c.sz()?;
    let index = c.i32()?;
    let mut fields = Vec::new();
    let s = |c: &mut Cur, f: &mut Vec<i32>| -> Result<()> {
        f.push(c.s16()? as i32);
        Ok(())
    };
    if v >= 1006 {
        s(c, &mut fields)?; // +4
    }
    s(c, &mut fields)?; // 5c
    s(c, &mut fields)?; // 60
    fields.push(c.i32()?); // 64 (ponto)
    fields.push(c.i32()?);
    s(c, &mut fields)?; // 6c
    for _ in 0..4 {
        fields.push(c.i32()?); // 70 (retângulo)
    }
    s(c, &mut fields)?; // 90
    s(c, &mut fields)?; // 88
    s(c, &mut fields)?; // 98
    if v >= 1006 {
        s(c, &mut fields)?; // 40
        fields.push(c.i32()?); // 38
    } else {
        fields.push(c.i32()?); // 40
        s(c, &mut fields)?; // 38
    }
    let mirrored = fields[fields.len() - 2] != 0;
    s(c, &mut fields)?; // 9c
    for _ in 0..c.count()? {
        s(c, &mut fields)?; // tabela em 44
    }
    let mut triggers = Vec::new();
    for _ in 0..c.count()? {
        let a = c.i32()?;
        let b = if v >= 1004 { Some(c.i32()?) } else { None };
        triggers.push((a, b));
    }
    let objects = objects(c)?;
    let props = prop_list(c)?;
    let decorations = decoration_list(c)?;
    Ok(Some(Cell {
        version: v,
        animation,
        border,
        index,
        mirrored,
        fields,
        triggers,
        objects,
        props,
        decorations,
    }))
}

fn prop_list(c: &mut Cur) -> Result<Vec<Prop>> {
    let v = c.version(0x5250)?;
    if v > 1001 {
        return Err(FormatError::Unsupported {
            what: WHAT,
            detail: format!("lista PR versão {v}"),
        });
    }
    let n = c.count()?;
    let mut out = Vec::with_capacity(n.min(4096));
    for _ in 0..n {
        out.push(Prop {
            kind: c.s16()?,
            rect: [c.i32()?, c.i32()?, c.i32()?, c.i32()?],
        });
    }
    Ok(out)
}

fn decoration_list(c: &mut Cur) -> Result<Vec<Decoration>> {
    let v = c.version(0x4344)?;
    if v > 1004 {
        return Err(FormatError::Unsupported {
            what: WHAT,
            detail: format!("lista DC versão {v}"),
        });
    }
    let n = c.count()?;
    let mut out = Vec::with_capacity(n.min(4096));
    for _ in 0..n {
        let animation = c.sz()?;
        let mut fields = Vec::new();
        for _ in 0..5 {
            fields.push(c.s16()?); // 38, 18, ?, ?, 24
        }
        let (x, y) = (c.i32()?, c.i32()?);
        fields.push(c.s16()?); // 40
        if v > 1001 {
            fields.push(c.s16()?); // 44
            fields.push(c.s16()?); // 48
        }
        if v >= 1004 {
            fields.push(c.s16()?); // 30
        }
        out.push(Decoration {
            animation,
            x,
            y,
            fields,
        });
    }
    Ok(out)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Escritor de bytes para montar corpos sintéticos.
    #[derive(Default)]
    pub(crate) struct W(pub Vec<u8>);
    impl W {
        pub fn s16(&mut self, v: i16) -> &mut Self {
            self.0.extend_from_slice(&v.to_le_bytes());
            self
        }
        pub fn i32(&mut self, v: i32) -> &mut Self {
            self.0.extend_from_slice(&v.to_le_bytes());
            self
        }
        pub fn sz(&mut self, s: &str) -> &mut Self {
            self.0.extend_from_slice(s.as_bytes());
            self.0.push(0);
            self
        }
    }

    /// Um objeto versão 1013 do tipo `kind` em (x, y).
    pub(crate) fn object(w: &mut W, kind: i32, x: i32, y: i32) {
        w.i32(kind).i32(x).i32(y);
        for _ in 0..18 {
            w.i32(0); // 15c, 160, 174, 178, 17c, 180, 184, 18c, 1ec, 7 valores, 1ac, 1a8
        }
        w.s16(1).i32(5).i32(6); // um par de pontos
        w.s16(2).i32(7).i32(8); // tabela de 2 valores
        w.i32(9).s16(1).i32(10).i32(11); // campo 118 e um gatilho
        w.sz("nomeA").sz("nomeB").i32(12).i32(13);
    }

    /// Corpo: 1 objeto global, uma célula com 1 objeto, 1 PR e 1 DC, e uma célula vazia.
    pub(crate) fn synth_body() -> Vec<u8> {
        let mut w = W::default();
        w.s16(1013).s16(1);
        object(&mut w, 102, 223, 70);
        w.s16(1010)
            .sz("WORLD1.ANM")
            .sz("C1_07.BRD")
            .i32(10071)
            .s16(247);
        w.s16(0).s16(0).i32(0).i32(0).s16(1);
        w.i32(0).i32(0).i32(0).i32(0);
        w.s16(1).s16(0).s16(1).s16(0).i32(1).s16(0);
        w.s16(5).s16(0).s16(0).s16(0).s16(0).s16(0);
        w.s16(1).i32(0).i32(0);
        w.s16(1013).s16(1);
        object(&mut w, 30, 1514, 31);
        w.s16(1001).s16(1).s16(64).i32(90).i32(69).i32(35).i32(32);
        w.s16(1004).s16(1).sz("DECO.ANM");
        w.s16(1)
            .s16(2)
            .s16(3)
            .s16(4)
            .s16(5)
            .i32(40)
            .i32(50)
            .s16(6)
            .s16(7)
            .s16(8)
            .s16(9);
        w.s16(0); // célula vazia
        w.0
    }

    #[test]
    fn body_parses_objects_and_cells() {
        let (objs, cells) = parse_body(&synth_body(), 2).unwrap();
        assert_eq!(objs.len(), 1);
        assert_eq!((objs[0].kind, objs[0].x, objs[0].y), (102, 223, 70));
        assert_eq!(objs[0].names, ["nomeA".to_string(), "nomeB".to_string()]);
        assert_eq!(objs[0].points, vec![(5, 6)]);
        assert_eq!(cells.len(), 2);
        let c = cells[0].as_ref().unwrap();
        assert_eq!(
            (c.animation.as_str(), c.border.as_str(), c.index),
            ("WORLD1.ANM", "C1_07.BRD", 10071)
        );
        assert_eq!(c.objects[0].kind, 30);
        assert_eq!(
            c.props,
            vec![Prop {
                kind: 64,
                rect: [90, 69, 35, 32]
            }]
        );
        assert_eq!(c.decorations[0].animation, "DECO.ANM");
        assert_eq!((c.decorations[0].x, c.decorations[0].y), (40, 50));
        assert!(cells[1].is_none());
    }

    #[test]
    fn leftover_bytes_are_an_error() {
        let mut b = synth_body();
        b.push(0xee);
        assert!(parse_body(&b, 2).is_err());
    }

    #[test]
    fn truncated_body_never_panics() {
        let b = synth_body();
        for n in 0..b.len() {
            assert!(parse_body(&b[..n], 2).is_err());
        }
    }
}
