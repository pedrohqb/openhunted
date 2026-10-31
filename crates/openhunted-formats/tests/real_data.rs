// SPDX-License-Identifier: GPL-3.0-or-later
//! Testes sobre os dados reais do jogo (OPENHUNTED_DATA).

mod support;

use std::collections::BTreeMap;

use support::open_real_archive;

#[test]
fn real_archive_inventory() {
    let path = data_or_skip!();
    let ar = open_real_archive(&path);
    assert_eq!(ar.len(), 2312);
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for e in ar.entries() {
        let ext = e
            .name
            .rsplit_once('.')
            .map(|(_, x)| x.to_uppercase())
            .unwrap_or_default();
        *counts.entry(ext).or_default() += 1;
    }
    let expected = [
        ("BMP", 1654),
        ("WAV", 280),
        ("BRD", 225),
        ("ANM", 87),
        ("PAL", 26),
        ("ITF", 16),
        ("RES", 13),
        ("PFT", 5),
        ("TBL", 3),
        ("SKE", 2),
        ("VCR", 1),
    ];
    for (ext, n) in expected {
        assert_eq!(counts.get(ext).copied(), Some(n), "{ext}");
    }
    assert!(ar.get("SX_3000.WAV").unwrap().starts_with(b"RIFF"));
    assert!(ar.get("avacado.pal").unwrap().starts_with(b"RIFF"));
}
