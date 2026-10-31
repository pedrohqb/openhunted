// SPDX-License-Identifier: GPL-3.0-or-later
//! Arquivo de fases `hunter.hls`. Formato em `docs/formats/hls.md`.

use crate::bytes::{slice, u16_at};
use crate::{FormatError, ResourceArchive, Result};

const WHAT: &str = "hunter.hls";

/// Arquivo de fases: um índice no formato do `RESOURCE.MAP` seguido de um volume embutido,
/// com os offsets contados a partir do fim do índice.
pub struct LevelArchive {
    archive: ResourceArchive,
}

impl LevelArchive {
    /// Lê o conteúdo inteiro do `hunter.hls`.
    pub fn from_bytes(data: Vec<u8>) -> Result<Self> {
        let nvol = u16_at(&data, 4, WHAT)? as usize;
        let mut end = 6;
        for _ in 0..nvol {
            let count = u16_at(&data, end + 13, WHAT)? as usize;
            end += 13 + 2 + 8 * count;
        }
        let index = slice(&data, 0, end, WHAT)?;
        let volume = &data[end..];
        if nvol != 1 {
            return Err(FormatError::Unsupported {
                what: WHAT,
                detail: format!("{nvol} volumes embutidos"),
            });
        }
        let archive = ResourceArchive::from_map(index, |_| Ok(volume.to_vec()))?;
        Ok(Self { archive })
    }

    /// Nomes das entradas, na ordem do índice.
    pub fn names(&self) -> Vec<String> {
        self.archive
            .entries()
            .iter()
            .map(|e| e.name.clone())
            .collect()
    }

    /// Conteúdo de uma entrada (sem diferenciar maiúsculas).
    pub fn get(&self, name: &str) -> Result<Vec<u8>> {
        self.archive.get(name).map(<[u8]>::to_vec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::mkhls;

    #[test]
    fn hls_reads_embedded_volume() {
        let hls = mkhls(&[("ENGLEVEL.TXT", b"a\r\n"), ("T_J_001.LEV", b"lev")]);
        let la = LevelArchive::from_bytes(hls).unwrap();
        assert_eq!(la.names(), vec!["ENGLEVEL.TXT", "T_J_001.LEV"]);
        assert_eq!(la.get("t_j_001.lev").unwrap(), b"lev");
    }

    #[test]
    fn truncated_hls_never_panics() {
        let hls = mkhls(&[("A.LEV", b"aaaa"), ("B.LEV", b"bb")]);
        for n in 0..hls.len() {
            assert!(LevelArchive::from_bytes(hls[..n].to_vec()).is_err());
        }
    }
}
