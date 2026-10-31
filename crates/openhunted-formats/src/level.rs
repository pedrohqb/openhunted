// SPDX-License-Identifier: GPL-3.0-or-later
//! Fases (`.LEV`) e a lista de missões (`ENGLEVEL.TXT`). Formato em `docs/formats/lev.md`.

use crate::bytes::u16_at;
use crate::lists::asciiz;
use crate::{Cell, FormatError, LevelObject, Result, find_chunk, parse_body};

const WHAT: &str = "fase LEV";
const MAX_VERSION: u16 = 1006;

/// Fase: cabeçalho do bloco `LEV:` (lido como em `0x0043124b`), objetos e células.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Level {
    pub name: String,
    pub palette: String,
    pub version: u16,
    pub title: String,
    pub description: String,
    /// Notas do autor da fase (versão ≥ 1003).
    pub notes: String,
    /// Campo das versões ≥ 1004, de significado ainda desconhecido.
    pub field_1004: i16,
    /// Dimensões da grade de células (x × y × z).
    pub grid_x: u16,
    pub grid_y: u16,
    pub grid_z: u16,
    /// Primeiro valor extra das versões ≥ 1006; coincide com o `p=` da lista de missões
    /// (1 = Jake, 2 = Garathe, 3 e 4 = dois jogadores).
    pub player: u16,
    /// Segundo valor extra das versões ≥ 1006; coincide com o segundo número do `p=`.
    pub extra: u16,
    /// Lista global de objetos.
    pub objects: Vec<LevelObject>,
    /// Células da grade, na ordem z, y, x (x varia mais rápido); `None` = célula vazia.
    pub cells: Vec<Option<Cell>>,
}

impl Level {
    /// Lê uma fase `.LEV`.
    pub fn parse(data: &[u8]) -> Result<Self> {
        let chunk = find_chunk(data, b"LEV:")?.ok_or(FormatError::NotFound("bloco LEV:".into()))?;
        let d = chunk.data;
        let (name, pos) = asciiz(d, 0, WHAT)?;
        let (palette, mut pos) = asciiz(d, pos, WHAT)?;
        let version = u16_at(d, pos, WHAT)?;
        pos += 2;
        if version > MAX_VERSION {
            return Err(FormatError::Unsupported {
                what: WHAT,
                detail: format!("versão {version}"),
            });
        }
        let (mut title, mut description, mut notes) = (String::new(), String::new(), String::new());
        if version >= 1002 {
            let (a, p) = asciiz(d, pos, WHAT)?;
            let (b, p) = asciiz(d, p, WHAT)?;
            pos = p;
            // Com '|' no nome, o original lê as duas strings num buffer temporário.
            if !name.contains('|') {
                (title, description) = (a, b);
            }
        }
        if version >= 1003 {
            let (n, p) = asciiz(d, pos, WHAT)?;
            notes = n;
            pos = p;
        }
        let r = |o: usize| u16_at(d, o, WHAT);
        let mut field_1004 = 0;
        if version >= 1004 {
            field_1004 = r(pos)? as i16;
            pos += 2;
        }
        let grid_x = if version >= 1000 {
            pos += 2;
            r(pos - 2)?
        } else {
            version
        };
        let (grid_y, grid_z) = (r(pos)?, r(pos + 2)?);
        pos += 4;
        let (mut player, mut extra) = (0, 0);
        if version >= 1006 {
            (player, extra) = (r(pos)?, r(pos + 2)?);
        }
        let body = data.get(8 + d.len()..).unwrap_or_default();
        let ncells = grid_x as usize * grid_y as usize * grid_z as usize;
        let (objects, cells) = parse_body(body, ncells)?;
        Ok(Level {
            name,
            palette,
            version,
            title,
            description,
            notes,
            field_1004,
            grid_x,
            grid_y,
            grid_z,
            player,
            extra,
            objects,
            cells,
        })
    }
}

/// Uma entrada da lista de missões (`ENGLEVEL.TXT`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct MissionInfo {
    pub file: String,
    /// Primeiro número do `p=`: 1 = Jake, 2 = Garathe, 3 e 4 = dois jogadores.
    pub player: u16,
    /// Segundo número do `p=`.
    pub extra: u16,
    pub title: String,
    /// Descrição, com as sequências `\n` convertidas em quebra de linha.
    pub description: String,
}

/// Lê a lista de missões: blocos separados por linha em branco, cada um com o nome do
/// arquivo e as linhas `p=`, `t=` e `d=`.
pub fn parse_mission_list(data: &[u8]) -> Result<Vec<MissionInfo>> {
    let text: String = data.iter().map(|&b| b as char).collect();
    let text = text.replace('\r', "");
    let mut out = Vec::new();
    for block in text.split("\n\n").filter(|b| !b.trim().is_empty()) {
        let mut lines = block.lines().filter(|l| !l.trim().is_empty());
        let file = lines.next().unwrap_or_default().trim().to_string();
        let (mut p, mut title, mut description) = (None, String::new(), String::new());
        for l in lines {
            if let Some(v) = l.strip_prefix("p=") {
                p = v.split_once(',').and_then(|(a, b)| {
                    Some((a.trim().parse::<u16>().ok()?, b.trim().parse::<u16>().ok()?))
                });
            } else if let Some(v) = l.strip_prefix("t=") {
                title = v.to_string();
            } else if let Some(v) = l.strip_prefix("d=") {
                description = v.replace("\\n", "\n");
            }
        }
        let (player, extra) = p.ok_or_else(|| FormatError::Unsupported {
            what: "lista de missões",
            detail: format!("linha p= ausente ou inválida em {file}"),
        })?;
        out.push(MissionInfo {
            file,
            player,
            extra,
            title,
            description,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::mklev;

    #[test]
    fn level_header_parses() {
        let l = Level::parse(&mklev()).unwrap();
        assert_eq!(l.name, "T_J_001.LEV");
        assert_eq!(l.palette, "WORLD.PAL");
        assert_eq!(l.version, 1006);
        assert_eq!(l.title, "Teste");
        assert_eq!(l.notes, "notas");
        assert_eq!((l.grid_x, l.grid_y, l.grid_z), (2, 1, 1));
        assert_eq!((l.player, l.extra), (1, 36));
        assert!(l.objects.is_empty());
        assert_eq!(l.cells, vec![None, None]);
    }

    #[test]
    fn newer_level_version_is_unsupported() {
        let mut v = mklev();
        let pos = 8 + "T_J_001.LEV\0WORLD.PAL\0".len();
        v[pos..pos + 2].copy_from_slice(&1007u16.to_le_bytes());
        assert!(matches!(
            Level::parse(&v),
            Err(crate::FormatError::Unsupported { .. })
        ));
    }

    #[test]
    fn truncated_level_never_panics() {
        let v = mklev();
        for n in 0..v.len() {
            assert!(Level::parse(&v[..n]).is_err());
        }
    }

    #[test]
    fn mission_list_parses_blocks() {
        let txt = b"T_J_001.lev\r\np=1,36\r\nt=TUTORIAL: Jake\r\nd=OBJECTIVES:\\nEscape.\r\n\r\nI_M_003.LEV\r\np=2,35\r\nt=INTRO\r\nd=Run\r\n";
        let m = parse_mission_list(txt).unwrap();
        assert_eq!(m.len(), 2);
        assert_eq!(m[0].file, "T_J_001.lev");
        assert_eq!((m[0].player, m[0].extra), (1, 36));
        assert_eq!(m[0].title, "TUTORIAL: Jake");
        assert_eq!(m[0].description, "OBJECTIVES:\nEscape.");
        assert_eq!(m[1].player, 2);
    }

    #[test]
    fn mission_list_rejects_bad_p_line() {
        assert!(parse_mission_list(b"A.LEV\r\np=x\r\nt=a\r\nd=b\r\n").is_err());
    }
}
