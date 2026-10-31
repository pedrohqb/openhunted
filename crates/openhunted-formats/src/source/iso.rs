// SPDX-License-Identifier: GPL-3.0-or-later
//! Fonte de dados numa imagem ISO 9660 (com Joliet, quando houver).

use std::path::Path;

use super::{DataSource, components, normalize_component};
use crate::{FormatError, Result};

const SECTOR: usize = 2048;
const FIRST_DESCRIPTOR: usize = 16;
const MAX_DESCRIPTORS: usize = 64;
const ROOT_RECORD_OFFSET: usize = 156;

/// Lê arquivos de uma imagem ISO 9660, preferindo a árvore Joliet.
pub struct IsoSource {
    image: Vec<u8>,
    root: Extent,
    joliet: bool,
}

#[derive(Clone, Copy)]
struct Extent {
    offset: usize,
    size: usize,
    is_dir: bool,
}

impl IsoSource {
    /// Abre a imagem em `path` (lida inteira para a memória).
    pub fn open(path: &Path) -> Result<Self> {
        let image = std::fs::read(path).map_err(|source| FormatError::Io {
            path: path.display().to_string(),
            source,
        })?;
        Self::from_bytes(image)
    }

    fn from_bytes(image: Vec<u8>) -> Result<Self> {
        let mut primary = None;
        let mut joliet = None;
        for i in 0..MAX_DESCRIPTORS {
            let start = (FIRST_DESCRIPTOR + i) * SECTOR;
            let desc = image
                .get(start..start + SECTOR)
                .ok_or(FormatError::Truncated {
                    what: "descritor de volume ISO",
                    offset: start,
                })?;
            if &desc[1..6] != b"CD001" {
                return Err(FormatError::BadMagic {
                    what: "descritor de volume ISO",
                    expected: b"CD001",
                    found: desc[1..6].to_vec(),
                });
            }
            match desc[0] {
                1 if primary.is_none() => primary = Some(start),
                2 if joliet.is_none() && is_joliet_escape(&desc[88..91]) => joliet = Some(start),
                255 => break,
                _ => {}
            }
        }
        let (desc, is_joliet) = match (joliet, primary) {
            (Some(j), _) => (j, true),
            (None, Some(p)) => (p, false),
            (None, None) => {
                return Err(FormatError::NotFound("descritor primário ISO".into()));
            }
        };
        let root_rec = &image[desc + ROOT_RECORD_OFFSET..desc + ROOT_RECORD_OFFSET + 34];
        let root = parse_record(root_rec)?.1;
        Ok(Self {
            image,
            root,
            joliet: is_joliet,
        })
    }

    fn lookup(&self, path: &str) -> Option<Extent> {
        let mut cur = self.root;
        for comp in components(path) {
            if !cur.is_dir {
                return None;
            }
            cur = self.find_in_dir(cur, &comp)?;
        }
        Some(cur)
    }

    fn find_in_dir(&self, dir: Extent, wanted: &str) -> Option<Extent> {
        let data = self
            .image
            .get(dir.offset..dir.offset.checked_add(dir.size)?)?;
        let mut pos = 0;
        while pos < data.len() {
            let len = data[pos] as usize;
            if len == 0 {
                // Registros não atravessam setores: pula para o próximo.
                pos = (pos / SECTOR + 1) * SECTOR;
                continue;
            }
            let rec = data.get(pos..pos + len)?;
            let (name, ext) = parse_record(rec).ok()?;
            if name != [0] && name != [1] && self.decode_name(name) == wanted {
                return Some(ext);
            }
            pos += len;
        }
        None
    }

    fn decode_name(&self, raw: &[u8]) -> String {
        let s = if self.joliet {
            let units: Vec<u16> = raw
                .chunks_exact(2)
                .map(|c| u16::from_be_bytes([c[0], c[1]]))
                .collect();
            String::from_utf16_lossy(&units)
        } else {
            String::from_utf8_lossy(raw).into_owned()
        };
        normalize_component(&s)
    }
}

fn is_joliet_escape(esc: &[u8]) -> bool {
    matches!(esc, b"%/@" | b"%/C" | b"%/E")
}

/// Lê um registro de diretório: devolve o nome cru e o extent.
fn parse_record(rec: &[u8]) -> Result<(&[u8], Extent)> {
    let trunc = FormatError::Truncated {
        what: "registro de diretório ISO",
        offset: 0,
    };
    if rec.len() < 34 {
        return Err(trunc);
    }
    let lba = u32::from_le_bytes([rec[2], rec[3], rec[4], rec[5]]) as usize;
    let size = u32::from_le_bytes([rec[10], rec[11], rec[12], rec[13]]) as usize;
    let is_dir = rec[25] & 0x02 != 0;
    let name_len = rec[32] as usize;
    let name = rec.get(33..33 + name_len).ok_or(trunc)?;
    let offset = lba.checked_mul(SECTOR).ok_or(FormatError::Truncated {
        what: "registro de diretório ISO",
        offset: 2,
    })?;
    Ok((
        name,
        Extent {
            offset,
            size,
            is_dir,
        },
    ))
}

impl DataSource for IsoSource {
    fn read(&self, path: &str) -> Result<Vec<u8>> {
        let ext = self
            .lookup(path)
            .filter(|e| !e.is_dir)
            .ok_or_else(|| FormatError::NotFound(path.to_string()))?;
        self.image
            .get(ext.offset..ext.offset.saturating_add(ext.size))
            .map(<[u8]>::to_vec)
            .ok_or(FormatError::Truncated {
                what: "arquivo na imagem ISO",
                offset: ext.offset,
            })
    }

    fn exists(&self, path: &str) -> bool {
        self.lookup(path).is_some_and(|e| !e.is_dir)
    }
}
