// SPDX-License-Identifier: GPL-3.0-or-later
//! Listas `u16` contagem + (`u16` id, nome ASCIIZ), usadas em `TAG:` e `LAB:`.

use crate::bytes::u16_at;
use crate::{FormatError, Result};

/// Lê uma lista de pares (id, nome).
pub(crate) fn id_name_list(data: &[u8], what: &'static str) -> Result<Vec<(u16, String)>> {
    let count = u16_at(data, 0, what)? as usize;
    let mut pos = 2;
    let mut out = Vec::with_capacity(count.min(4096));
    for _ in 0..count {
        let id = u16_at(data, pos, what)?;
        pos += 2;
        let (name, next) = asciiz(data, pos, what)?;
        pos = next;
        out.push((id, name));
    }
    Ok(out)
}

/// Lê uma string terminada em zero a partir de `pos`; devolve a string e a posição seguinte.
pub(crate) fn asciiz(data: &[u8], pos: usize, what: &'static str) -> Result<(String, usize)> {
    let rest = data
        .get(pos..)
        .ok_or(FormatError::Truncated { what, offset: pos })?;
    let end = rest
        .iter()
        .position(|&b| b == 0)
        .ok_or(FormatError::Truncated { what, offset: pos })?;
    Ok((
        rest[..end].iter().map(|&b| b as char).collect(),
        pos + end + 1,
    ))
}
