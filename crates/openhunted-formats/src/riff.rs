// SPDX-License-Identifier: GPL-3.0-or-later
//! Leitura mínima de contêineres RIFF.

use crate::bytes::{slice, u32_at};
use crate::{FormatError, Result};

/// Confere o cabeçalho `RIFF <tam> <form>` e devolve os sub-blocos `(id, dados)`.
pub(crate) fn riff_chunks<'a>(
    data: &'a [u8],
    form: &'static [u8; 4],
    what: &'static str,
) -> Result<Vec<([u8; 4], &'a [u8])>> {
    let head = slice(data, 0, 12, what)?;
    if &head[0..4] != b"RIFF" || &head[8..12] != form {
        return Err(FormatError::BadMagic {
            what,
            expected: form,
            found: head[8..12].to_vec(),
        });
    }
    let riff_len = u32_at(data, 4, what)? as usize;
    let body = slice(data, 8, riff_len, what)?;
    let mut out = Vec::new();
    let mut pos = 4;
    while pos + 8 <= body.len() {
        let id = slice(body, pos, 4, what)?;
        let len = u32_at(body, pos + 4, what)? as usize;
        let chunk = slice(body, pos + 8, len, what)?;
        out.push(([id[0], id[1], id[2], id[3]], chunk));
        pos += 8 + len + (len & 1);
    }
    Ok(out)
}

/// Procura o sub-bloco `id` e falha se não houver.
pub(crate) fn require<'a>(
    chunks: &[([u8; 4], &'a [u8])],
    id: &[u8; 4],
    what: &'static str,
) -> Result<&'a [u8]> {
    chunks
        .iter()
        .find(|(c, _)| c == id)
        .map(|(_, d)| *d)
        .ok_or(FormatError::Truncated { what, offset: 0 })
}
