// SPDX-License-Identifier: GPL-3.0-or-later
//! Extração de recursos para arquivos.

use std::path::Path;

use openhunted_formats::{Bitmap, Palette, ResourceArchive};

/// Nome de arquivo seguro para um recurso: troca todo caractere fora de
/// `[A-Za-z0-9._-]` e os pontos iniciais por `_`; nome vazio ou só com pontos vira `_`.
/// Nunca contém separador de caminho.
pub fn safe_file_name(resource: &str) -> String {
    if resource.chars().all(|c| c == '.') {
        return "_".to_string();
    }
    let mut leading = true;
    resource
        .chars()
        .map(|c| {
            let ok = c.is_ascii_alphanumeric() || c == '_' || c == '-' || (c == '.' && !leading);
            if c != '.' {
                leading = false;
            }
            if ok { c } else { '_' }
        })
        .collect()
}

/// Extrai para `out` os recursos cuja extensão bate com `filter` (todos, se `None`).
/// Bitmaps viram PNG indexado com `palette`; os demais são copiados como estão.
/// Devolve o número de arquivos escritos.
pub fn extract(
    ar: &ResourceArchive,
    out: &Path,
    filter: Option<&str>,
    palette_name: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    std::fs::create_dir_all(out)?;
    let mut palette: Option<Palette> = None;
    let mut written = 0;
    for e in ar.entries() {
        if !matches_type(&e.name, filter) {
            continue;
        }
        let data = ar.get(&e.name)?;
        let base = safe_file_name(&e.name);
        if extension(&e.name) == "BMP" {
            if palette.is_none() {
                let raw = ar
                    .get(palette_name)
                    .map_err(|err| format!("paleta {palette_name}: {err}"))?;
                palette = Some(Palette::parse(raw)?);
            }
            let pal = palette.as_ref().ok_or("paleta indisponível")?;
            let bmp = Bitmap::parse(data).map_err(|err| err.in_resource(&e.name))?;
            for (i, f) in bmp.frames.iter().enumerate() {
                write_png(&out.join(format!("{base}_{i:03}.png")), f, pal)?;
                written += 1;
            }
        } else {
            std::fs::write(out.join(&base), data)?;
            written += 1;
        }
    }
    Ok(written)
}

/// Extensão em maiúsculas.
pub fn extension(name: &str) -> String {
    name.rsplit_once('.')
        .map(|(_, x)| x.to_uppercase())
        .unwrap_or_default()
}

/// Indica se o recurso tem a extensão pedida (sem diferenciar maiúsculas).
pub fn matches_type(name: &str, filter: Option<&str>) -> bool {
    filter.is_none_or(|t| extension(name) == t.trim_start_matches('.').to_uppercase())
}

fn write_png(
    path: &Path,
    frame: &openhunted_formats::Frame,
    pal: &Palette,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = std::io::BufWriter::new(std::fs::File::create(path)?);
    let mut enc = png::Encoder::new(file, u32::from(frame.width), u32::from(frame.height));
    enc.set_color(png::ColorType::Indexed);
    enc.set_depth(png::BitDepth::Eight);
    enc.set_palette(pal.0.concat());
    let mut w = enc.write_header()?;
    w.write_image_data(&frame.pixels)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_file_name_blocks_traversal() {
        assert_eq!(safe_file_name("../X\\Y.BMP"), "___X_Y.BMP");
        assert!(!safe_file_name("a/b").contains('/'));
        assert_eq!(safe_file_name(".."), "_");
        assert_eq!(safe_file_name(""), "_");
        assert_eq!(safe_file_name("!DECALW1.ANM"), "_DECALW1.ANM");
        assert_eq!(safe_file_name("SX_3000.WAV"), "SX_3000.WAV");
    }
}
