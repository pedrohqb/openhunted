// SPDX-License-Identifier: GPL-3.0-or-later
//! Cache de bitmaps e animações lidos do arquivo de recursos.

use std::collections::HashMap;
use std::rc::Rc;

use openhunted_formats::{Animation, Bitmap, ResourceArchive, Result};

/// Cache de recursos decodificados, por nome (sem diferenciar maiúsculas).
pub struct Assets {
    archive: ResourceArchive,
    bitmaps: HashMap<String, Rc<Bitmap>>,
    animations: HashMap<String, Rc<Animation>>,
}

impl Assets {
    pub fn new(archive: ResourceArchive) -> Self {
        Self {
            archive,
            bitmaps: HashMap::new(),
            animations: HashMap::new(),
        }
    }

    /// O arquivo de recursos por trás do cache.
    pub fn archive(&self) -> &ResourceArchive {
        &self.archive
    }

    pub fn bitmap(&mut self, name: &str) -> Result<Rc<Bitmap>> {
        let key = name.to_uppercase();
        if let Some(b) = self.bitmaps.get(&key) {
            return Ok(b.clone());
        }
        let data = self.archive.get(name)?;
        let b = Rc::new(Bitmap::parse(data).map_err(|e| e.in_resource(name))?);
        self.bitmaps.insert(key, b.clone());
        Ok(b)
    }

    pub fn animation(&mut self, name: &str) -> Result<Rc<Animation>> {
        let key = name.to_uppercase();
        if let Some(a) = self.animations.get(&key) {
            return Ok(a.clone());
        }
        let data = self.archive.get(name)?;
        let a = Rc::new(Animation::parse(data).map_err(|e| e.in_resource(name))?);
        self.animations.insert(key, a.clone());
        Ok(a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openhunted_formats::test_support::{mkanm, mkarchive, mkbmp};

    #[test]
    fn caches_and_reports_missing() {
        let (bmp, anm) = (mkbmp(&[(1, 1, vec![5])]), mkanm());
        let (map, vols) = mkarchive(&[(
            "RESOURCE.001",
            &[("OAN00.BMP", bmp.as_slice()), ("OAN.ANM", anm.as_slice())],
        )]);
        let ar =
            openhunted_formats::ResourceArchive::from_map(&map, |_| Ok(vols[0].1.clone())).unwrap();
        let mut a = Assets::new(ar);
        let b1 = a.bitmap("oan00.bmp").unwrap();
        let b2 = a.bitmap("OAN00.BMP").unwrap();
        assert!(std::rc::Rc::ptr_eq(&b1, &b2));
        assert_eq!(a.animation("OAN.ANM").unwrap().frames.len(), 1);
        assert!(a.bitmap("NADA.BMP").is_err());
    }
}
