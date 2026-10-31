// SPDX-License-Identifier: GPL-3.0-or-later
//! Blocos `TAG:` + `u32` tamanho, usados nos recursos próprios da Dynamix.

use crate::bytes::{slice, u32_at};
use crate::{FormatError, Result};

const CONTAINER: u32 = 0x8000_0000;

/// Um bloco: tag com o ':' final (ex.: `b"BMP:"`) e os dados.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chunk<'a> {
    pub tag: [u8; 4],
    pub data: &'a [u8],
}

/// Itera sobre os blocos consecutivos de `data`. Para no primeiro erro.
pub fn chunks(data: &[u8]) -> impl Iterator<Item = Result<Chunk<'_>>> {
    let mut pos = 0;
    let mut failed = false;
    std::iter::from_fn(move || {
        if failed || pos >= data.len() {
            return None;
        }
        let r = read_chunk(data, pos);
        match &r {
            Ok(c) => pos += 8 + c.data.len(),
            Err(_) => failed = true,
        }
        Some(r)
    })
}

/// Procura o primeiro bloco com a tag dada.
pub fn find_chunk<'a>(data: &'a [u8], tag: &[u8; 4]) -> Result<Option<Chunk<'a>>> {
    for c in chunks(data) {
        let c = c?;
        if &c.tag == tag {
            return Ok(Some(c));
        }
    }
    Ok(None)
}

fn read_chunk(data: &[u8], pos: usize) -> Result<Chunk<'_>> {
    const WHAT: &str = "bloco";
    let t = slice(data, pos, 4, WHAT)?;
    if t[3] != b':' || !t[..3].iter().all(u8::is_ascii_graphic) {
        return Err(FormatError::BadMagic {
            what: WHAT,
            expected: b"???:",
            found: t.to_vec(),
        });
    }
    let raw = u32_at(data, pos + 4, WHAT)?;
    if raw & CONTAINER != 0 {
        return Err(FormatError::Unsupported {
            what: WHAT,
            detail: format!("bloco contêiner {}", String::from_utf8_lossy(t)),
        });
    }
    let body = slice(data, pos + 8, raw as usize, WHAT)?;
    Ok(Chunk {
        tag: [t[0], t[1], t[2], t[3]],
        data: body,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(tag: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut v = tag.to_vec();
        v.extend_from_slice(&(data.len() as u32).to_le_bytes());
        v.extend_from_slice(data);
        v
    }

    #[test]
    fn chunks_iterates_and_rejects_overrun() {
        let mut data = block(b"BMP:", b"abc");
        data.extend(block(b"TAG:", b"xy"));
        let got: Vec<_> = chunks(&data).collect::<crate::Result<_>>().unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(&got[0].tag, b"BMP:");
        assert_eq!(got[0].data, b"abc");
        assert_eq!(got[1].data, b"xy");
        assert!(find_chunk(&data, b"TAG:").unwrap().is_some());
        assert!(find_chunk(&data, b"PAL:").unwrap().is_none());

        let mut bad = b"BMP:".to_vec();
        bad.extend_from_slice(&100u32.to_le_bytes());
        bad.extend_from_slice(b"short");
        let r: crate::Result<Vec<_>> = chunks(&bad).collect();
        assert!(matches!(r, Err(crate::FormatError::Truncated { .. })));

        for n in 0..data.len() {
            let _: Vec<_> = chunks(&data[..n]).collect();
        }
    }

    #[test]
    fn container_flag_is_unsupported() {
        let mut data = b"BMP:".to_vec();
        data.extend_from_slice(&0x8000_0000u32.to_le_bytes());
        let r: crate::Result<Vec<_>> = chunks(&data).collect();
        assert!(matches!(r, Err(crate::FormatError::Unsupported { .. })));
    }

    #[test]
    fn non_tag_is_bad_magic() {
        let r: crate::Result<Vec<_>> = chunks(b"RIFF\x04\x00\x00\x00abcd").collect();
        assert!(matches!(r, Err(crate::FormatError::BadMagic { .. })));
    }
}
