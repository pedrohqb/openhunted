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
