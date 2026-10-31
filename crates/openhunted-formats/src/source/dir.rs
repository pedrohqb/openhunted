// SPDX-License-Identifier: GPL-3.0-or-later
//! Fonte de dados numa pasta do sistema de arquivos.

use std::path::{Path, PathBuf};

use super::{DataSource, components, normalize_component};
use crate::{FormatError, Result};

/// Lê arquivos de uma pasta, resolvendo nomes sem diferenciar maiúsculas.
pub struct DirSource {
    root: PathBuf,
}

impl DirSource {
    /// Cria uma fonte com raiz em `root`.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn resolve(&self, path: &str) -> Option<PathBuf> {
        let mut cur = self.root.clone();
        for comp in components(path) {
            cur = find_entry(&cur, &comp)?;
        }
        Some(cur)
    }
}

fn find_entry(dir: &Path, wanted: &str) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .find(|e| normalize_component(&e.file_name().to_string_lossy()) == wanted)
        .map(|e| e.path())
}

impl DataSource for DirSource {
    fn read(&self, path: &str) -> Result<Vec<u8>> {
        let full = self
            .resolve(path)
            .filter(|p| p.is_file())
            .ok_or_else(|| FormatError::NotFound(path.to_string()))?;
        std::fs::read(&full).map_err(|source| FormatError::Io {
            path: full.display().to_string(),
            source,
        })
    }

    fn exists(&self, path: &str) -> bool {
        self.resolve(path).is_some_and(|p| p.is_file())
    }
}
