// SPDX-License-Identifier: GPL-3.0-or-later
//! Apoio aos testes de integração.

/// Devolve o `PathBuf` de `OPENHUNTED_DATA` ou pula o teste.
#[macro_export]
macro_rules! data_or_skip {
    () => {
        match std::env::var_os("OPENHUNTED_DATA") {
            Some(p) => std::path::PathBuf::from(p),
            None => {
                eprintln!("pulado: OPENHUNTED_DATA não definido");
                return;
            }
        }
    };
}

use std::path::Path;

use openhunted_formats::{DataSource, DirSource, IsoSource, ResourceArchive};

/// Abre o arquivo de recursos real a partir de um `.iso` ou de uma pasta
/// (raiz do CD ou `Hunter/`).
#[allow(dead_code)]
pub fn open_real_archive(path: &Path) -> ResourceArchive {
    let src: Box<dyn DataSource> = if path.is_file() {
        Box::new(IsoSource::open(path).unwrap())
    } else {
        Box::new(DirSource::new(path))
    };
    let prefix = if src.exists("RESOURCE.MAP") {
        ""
    } else {
        "Hunter/"
    };
    let map = src.read(&format!("{prefix}RESOURCE.MAP")).unwrap();
    ResourceArchive::from_map(&map, |v| src.read(&format!("{prefix}{v}"))).unwrap()
}
