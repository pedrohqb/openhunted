// SPDX-License-Identifier: GPL-3.0-or-later
//! Erros de leitura dos dados do jogo.

/// Erro ao ler ou decodificar os dados originais.
#[derive(Debug, thiserror::Error)]
pub enum FormatError {
    #[error("erro de E/S em {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("não encontrado: {0}")]
    NotFound(String),
    #[error("{what}: dados truncados (offset {offset})")]
    Truncated { what: &'static str, offset: usize },
    #[error("{what}: assinatura inválida (esperado {expected:02x?}, encontrado {found:02x?})")]
    BadMagic {
        what: &'static str,
        expected: &'static [u8],
        found: Vec<u8>,
    },
    #[error("{what}: não suportado ({detail})")]
    Unsupported { what: &'static str, detail: String },
    #[error("recurso {name}: {source}")]
    InResource {
        name: String,
        #[source]
        source: Box<FormatError>,
    },
}

/// Resultado com [`FormatError`].
pub type Result<T> = std::result::Result<T, FormatError>;

impl FormatError {
    /// Anexa o nome do recurso em que o erro ocorreu.
    pub fn in_resource(self, name: &str) -> FormatError {
        FormatError::InResource {
            name: name.to_string(),
            source: Box::new(self),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_resource_wraps_and_displays_name() {
        let err = FormatError::Truncated {
            what: "bitmap",
            offset: 12,
        }
        .in_resource("SABOUT.BMP");
        let msg = err.to_string();
        assert!(msg.contains("SABOUT.BMP"), "{msg}");
        assert!(msg.contains("bitmap"), "{msg}");
    }
}
