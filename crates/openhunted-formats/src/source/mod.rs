// SPDX-License-Identifier: GPL-3.0-or-later
//! Fontes de dados: de onde os arquivos do jogo são lidos.

mod dir;
mod iso;

pub use dir::DirSource;
pub use iso::IsoSource;

use crate::Result;

/// Uma origem de arquivos do jogo (pasta ou imagem de CD).
pub trait DataSource: Send + Sync {
    /// Lê o arquivo `path`, relativo à raiz, separado por '/', sem diferenciar maiúsculas.
    fn read(&self, path: &str) -> Result<Vec<u8>>;
    /// Indica se o arquivo existe.
    fn exists(&self, path: &str) -> bool;
}

/// Normaliza um caminho: maiúsculas, '\\' vira '/', remove sufixos ';<n>',
/// pontos finais de componente, '/' inicial e componentes vazios.
pub fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
        .split('/')
        .filter(|c| !c.is_empty())
        .map(normalize_component)
        .collect::<Vec<_>>()
        .join("/")
}

/// Normaliza um nome de arquivo ou pasta isolado.
pub(crate) fn normalize_component(name: &str) -> String {
    let base = match name.rfind(';') {
        Some(i) if name[i + 1..].bytes().all(|b| b.is_ascii_digit()) => &name[..i],
        _ => name,
    };
    base.trim_end_matches('.').to_uppercase()
}

/// Divide um caminho já normalizado em componentes.
pub(crate) fn components(path: &str) -> Vec<String> {
    let norm = normalize_path(path);
    if norm.is_empty() {
        Vec::new()
    } else {
        norm.split('/').map(str::to_string).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_handles_case_separators_and_versions() {
        assert_eq!(
            normalize_path("/Hunter\\resource.map;1"),
            "HUNTER/RESOURCE.MAP"
        );
        assert_eq!(normalize_path("README.;1"), "README");
        assert_eq!(normalize_path("a//b/"), "A/B");
    }
}
