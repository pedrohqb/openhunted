// SPDX-License-Identifier: GPL-3.0-or-later
//! Abertura dos dados do jogo a partir do caminho dado pelo usuário.

use std::path::Path;

use sha2::{Digest, Sha256};

use crate::{DataSource, DirSource, FormatError, IsoSource, LevelArchive, ResourceArchive, Result};

/// SHA-256 do `RESOURCE.MAP` da versão Windows 95 1.00.
const V100_MAP_SHA256: &str = "8eb654c4093b3c013705f109af4c205f4854871f220cb7d7a6965ed9557abd7d";

/// Edição do jogo identificada.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameVersion {
    /// Windows 95, versão 1.00.
    Win95V100,
    /// Desconhecida: abre mesmo assim, mas pode haver diferenças.
    Unknown,
}

/// Dados do jogo abertos: a fonte e o prefixo da pasta do jogo.
pub struct GameData {
    source: Box<dyn DataSource>,
    prefix: &'static str,
    version: GameVersion,
}

/// Abre os dados do jogo. `path` pode ser a raiz do CD, a pasta `Hunter/`,
/// o próprio `RESOURCE.MAP` ou uma imagem `.iso` do CD.
pub fn open_data(path: &Path) -> Result<GameData> {
    let is_iso = path.is_file()
        && path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("iso"));
    let source: Box<dyn DataSource> = if is_iso {
        Box::new(IsoSource::open(path)?)
    } else if path.is_file() {
        Box::new(DirSource::new(path.parent().unwrap_or(Path::new("."))))
    } else {
        Box::new(DirSource::new(path))
    };
    let prefix = ["", "HUNTER/"]
        .into_iter()
        .find(|p| source.exists(&format!("{p}RESOURCE.MAP")))
        .ok_or_else(|| {
            FormatError::NotFound(format!(
                "RESOURCE.MAP (procurado em {0}/RESOURCE.MAP e {0}/Hunter/RESOURCE.MAP)",
                path.display()
            ))
        })?;
    let map = source.read(&format!("{prefix}RESOURCE.MAP"))?;
    let digest = Sha256::digest(&map);
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    let version = if hex == V100_MAP_SHA256 {
        GameVersion::Win95V100
    } else {
        GameVersion::Unknown
    };
    Ok(GameData {
        source,
        prefix,
        version,
    })
}

impl GameData {
    /// Edição identificada pelo hash do `RESOURCE.MAP`.
    pub fn version(&self) -> GameVersion {
        self.version
    }

    /// Lê um arquivo relativo à pasta do jogo.
    pub fn read(&self, file: &str) -> Result<Vec<u8>> {
        self.source.read(&format!("{}{file}", self.prefix))
    }

    /// Carrega o arquivo de recursos (`RESOURCE.MAP` e volumes).
    pub fn resources(&self) -> Result<ResourceArchive> {
        let map = self.read("RESOURCE.MAP")?;
        ResourceArchive::from_map(&map, |v| self.read(v))
    }

    /// Carrega o arquivo de fases (`hunter.hls`).
    pub fn levels(&self) -> Result<LevelArchive> {
        LevelArchive::from_bytes(self.read("HUNTER.HLS")?)
    }
}
