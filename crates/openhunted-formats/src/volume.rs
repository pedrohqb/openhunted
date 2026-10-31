// SPDX-License-Identifier: GPL-3.0-or-later
//! Volume de recursos da Dynamix: `RESOURCE.MAP` mais `RESOURCE.001`, `.002`…
//! Formato descrito em `docs/formats/volume.md`.

use std::collections::HashMap;

use crate::bytes::{fixed_name, u16_at, u32_at};
use crate::{FormatError, Result};

const NAME_LEN: usize = 13;
const MAP_HEADER: usize = 4;

/// Uma entrada do arquivo de recursos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub volume: String,
    pub offset: u32,
    pub size: u32,
}

/// Arquivo de recursos carregado: buffers dos volumes e índice por nome.
pub struct ResourceArchive {
    volumes: Vec<Vec<u8>>,
    entries: Vec<Entry>,
    /// Para cada entrada: (índice do volume, início dos dados).
    locations: Vec<(usize, usize)>,
    index: HashMap<String, usize>,
}

impl ResourceArchive {
    /// Lê o mapa e carrega cada volume com `load_volume(nome)`.
    pub fn from_map(
        map: &[u8],
        mut load_volume: impl FnMut(&str) -> Result<Vec<u8>>,
    ) -> Result<Self> {
        const WHAT: &str = "RESOURCE.MAP";
        let nvol = u16_at(map, MAP_HEADER, WHAT)? as usize;
        let mut pos = MAP_HEADER + 2;
        let mut ar = Self {
            volumes: Vec::new(),
            entries: Vec::new(),
            locations: Vec::new(),
            index: HashMap::new(),
        };
        for v in 0..nvol {
            let vname = fixed_name(map, pos, NAME_LEN, WHAT)?;
            pos += NAME_LEN;
            let count = u16_at(map, pos, WHAT)? as usize;
            pos += 2;
            let vol = load_volume(&vname)?;
            for _ in 0..count {
                let offset = u32_at(map, pos + 4, WHAT)?;
                pos += 8;
                let (name, size, start) =
                    read_entry_header(&vol, offset as usize).map_err(|e| e.in_resource(&vname))?;
                ar.index
                    .entry(name.to_uppercase())
                    .or_insert(ar.entries.len());
                ar.entries.push(Entry {
                    name,
                    volume: vname.clone(),
                    offset,
                    size,
                });
                ar.locations.push((v, start));
            }
            ar.volumes.push(vol);
        }
        Ok(ar)
    }

    /// Número de entradas.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Indica se não há entradas.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Entradas na ordem do mapa.
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// Dados do recurso `name` (sem diferenciar maiúsculas).
    pub fn get(&self, name: &str) -> Result<&[u8]> {
        let &i = self
            .index
            .get(&name.to_uppercase())
            .ok_or_else(|| FormatError::NotFound(name.to_string()))?;
        let (v, start) = self.locations[i];
        let size = self.entries[i].size as usize;
        Ok(&self.volumes[v][start..start + size])
    }
}

/// Lê o cabeçalho de uma entrada no volume e confere que os dados cabem.
fn read_entry_header(vol: &[u8], offset: usize) -> Result<(String, u32, usize)> {
    const WHAT: &str = "entrada de volume";
    let name = fixed_name(vol, offset, NAME_LEN, WHAT)?;
    let size = u32_at(vol, offset + NAME_LEN, WHAT)?;
    let start = offset + NAME_LEN + 4;
    crate::bytes::slice(vol, start, size as usize, WHAT)?;
    Ok((name, size, start))
}
