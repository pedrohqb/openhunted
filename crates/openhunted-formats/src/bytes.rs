// SPDX-License-Identifier: GPL-3.0-or-later
//! Leitura de inteiros little-endian com checagem de limites.

use crate::{FormatError, Result};

pub(crate) fn slice<'a>(
    data: &'a [u8],
    off: usize,
    len: usize,
    what: &'static str,
) -> Result<&'a [u8]> {
    off.checked_add(len)
        .and_then(|end| data.get(off..end))
        .ok_or(FormatError::Truncated { what, offset: off })
}

pub(crate) fn u16_at(data: &[u8], off: usize, what: &'static str) -> Result<u16> {
    let b = slice(data, off, 2, what)?;
    Ok(u16::from_le_bytes([b[0], b[1]]))
}

pub(crate) fn u32_at(data: &[u8], off: usize, what: &'static str) -> Result<u32> {
    let b = slice(data, off, 4, what)?;
    Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// Lê um nome ASCIIZ de tamanho fixo (Latin-1).
pub(crate) fn fixed_name(
    data: &[u8],
    off: usize,
    len: usize,
    what: &'static str,
) -> Result<String> {
    let raw = slice(data, off, len, what)?;
    let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
    Ok(raw[..end].iter().map(|&b| b as char).collect())
}
