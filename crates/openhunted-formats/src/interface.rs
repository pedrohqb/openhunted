// SPDX-License-Identifier: GPL-3.0-or-later
//! Interfaces (`.ITF`): telas com controles ligados a uma animação.
//! Formato em `docs/formats/itf.md`.

use crate::bytes::{slice, u16_at};
use crate::lists::{asciiz, id_name_list};
use crate::{FormatError, Result, find_chunk};

const WHAT: &str = "interface ITF";
const MAX_VERSION: u16 = 1103;
const CONTROL_TYPES: u16 = 7;
/// Tipo de controle que traz 18 bytes extras (campo de edição; `0x00462060`).
const EDIT_CONTROL: u16 = 4;
const EDIT_EXTRA: usize = 18;

/// Controle de uma interface.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Control {
    /// Tipo (0 a 6; tabela em `0x00483a08`).
    pub kind: u16,
    pub id: u16,
    /// Nome do controle, do bloco `TAG:` (pelo id).
    pub name: Option<String>,
    /// Campo de significado ainda desconhecido (0 nos dados vistos).
    pub field_a: i16,
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
    /// Campo de significado ainda desconhecido (versão ≥ 1103; senão 1).
    pub field_f: u16,
    /// Rótulo da animação associado (`None` quando gravado como 0 ou 0xFFFF).
    pub label: Option<u16>,
    /// Origem do desenho (versão ≥ 1102; nas anteriores, `(-x, -y)`).
    pub origin_x: i16,
    pub origin_y: i16,
    /// Bytes extras do campo de edição (tipo 4).
    pub extra: Vec<u8>,
}

/// Interface: animação de base e controles.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Interface {
    pub version: u16,
    /// Nome do arquivo de animação (ex.: `sabout.ANM`).
    pub animation: String,
    pub controls: Vec<Control>,
}

impl Interface {
    /// Lê um recurso `.ITF` (blocos `ITF:` e `TAG:`), como em `0x00461310`.
    pub fn parse(data: &[u8]) -> Result<Self> {
        let d = find_chunk(data, b"ITF:")?
            .ok_or(FormatError::NotFound("bloco ITF:".into()))?
            .data;
        let names = match find_chunk(data, b"TAG:")? {
            Some(c) => id_name_list(c.data, WHAT)?,
            None => Vec::new(),
        };
        let magic = slice(d, 0, 2, WHAT)?;
        if magic != b"TB" {
            return Err(FormatError::BadMagic {
                what: WHAT,
                expected: b"TB",
                found: magic.to_vec(),
            });
        }
        let version = u16_at(d, 2, WHAT)?;
        if version > MAX_VERSION {
            return Err(FormatError::Unsupported {
                what: WHAT,
                detail: format!("versão {version}"),
            });
        }
        let (animation, mut pos) = asciiz(d, 4, WHAT)?;
        let count = u16_at(d, pos, WHAT)? as usize;
        pos += 2;
        for i in 0..count {
            let t = u16_at(d, pos + 2 * i, WHAT)?;
            if t >= CONTROL_TYPES {
                return Err(FormatError::Unsupported {
                    what: WHAT,
                    detail: format!("tipo de controle {t}"),
                });
            }
        }
        pos += 2 * count;
        let mut controls = Vec::with_capacity(count.min(4096));
        for _ in 0..count {
            let r = |o: usize| u16_at(d, o, WHAT);
            let kind = r(pos)?;
            if kind >= CONTROL_TYPES {
                return Err(FormatError::Unsupported {
                    what: WHAT,
                    detail: format!("tipo de controle {kind}"),
                });
            }
            let id = r(pos + 2)?;
            let field_a = r(pos + 4)? as i16;
            let x = r(pos + 6)? as i16;
            let y = r(pos + 8)? as i16;
            let width = r(pos + 10)?;
            let height = r(pos + 12)?;
            pos += 14;
            let field_f = if version >= 1103 {
                pos += 2;
                r(pos - 2)?
            } else {
                1
            };
            let raw_label = r(pos)?;
            pos += 2;
            let (origin_x, origin_y) = if version >= 1102 {
                pos += 4;
                (r(pos - 4)? as i16, r(pos - 2)? as i16)
            } else {
                (x.wrapping_neg(), y.wrapping_neg())
            };
            let extra = if kind == EDIT_CONTROL {
                pos += EDIT_EXTRA;
                slice(d, pos - EDIT_EXTRA, EDIT_EXTRA, WHAT)?.to_vec()
            } else {
                Vec::new()
            };
            controls.push(Control {
                kind,
                id,
                name: names.iter().find(|(i, _)| *i == id).map(|(_, n)| n.clone()),
                field_a,
                x,
                y,
                width,
                height,
                field_f,
                label: (raw_label != 0 && raw_label != 0xffff).then_some(raw_label),
                origin_x,
                origin_y,
                extra,
            });
        }
        Ok(Interface {
            version,
            animation,
            controls,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::mkitf;

    #[test]
    fn itf_parses_synthetic() {
        let i = Interface::parse(&mkitf()).unwrap();
        assert_eq!(i.version, 1103);
        assert_eq!(i.animation, "teste.ANM");
        assert_eq!(i.controls.len(), 2);
        let c = &i.controls[1];
        assert_eq!(
            (c.kind, c.id, c.x, c.y, c.width, c.height),
            (4, 0x80e9, 30, 40, 50, 60)
        );
        assert_eq!(c.label, Some(2));
        assert_eq!((c.origin_x, c.origin_y), (-30, -40));
        assert_eq!(c.extra.len(), 18);
        assert_eq!(c.name.as_deref(), Some("T_NOME"));
        assert_eq!(i.controls[0].label, None);
    }

    #[test]
    fn unknown_control_type_is_unsupported() {
        let mut v = mkitf();
        // tipos ficam logo depois de "TB", versão, nome (10 bytes) e contagem
        let off = 8 + 2 + 2 + 10 + 2;
        v[off..off + 2].copy_from_slice(&7u16.to_le_bytes());
        assert!(matches!(
            Interface::parse(&v),
            Err(crate::FormatError::Unsupported { .. })
        ));
    }

    #[test]
    fn truncated_itf_never_panics() {
        let v = mkitf();
        for n in 0..v.len() {
            let _ = Interface::parse(&v[..n]);
        }
    }
}
