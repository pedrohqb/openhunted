// SPDX-License-Identifier: GPL-3.0-or-later

use openhunted_formats::test_support::mkarchive;
use openhunted_formats::{FormatError, ResourceArchive};

fn loader(vols: Vec<(String, Vec<u8>)>) -> impl FnMut(&str) -> openhunted_formats::Result<Vec<u8>> {
    move |name: &str| {
        vols.iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, d)| d.clone())
            .ok_or_else(|| FormatError::NotFound(name.to_string()))
    }
}

#[test]
fn reads_entries_across_volumes() {
    let (map, vols) = mkarchive(&[
        ("RESOURCE.001", &[("A.BMP", b"aaaa")]),
        ("RESOURCE.002", &[("B.WAV", b"bb"), ("C.PAL", b"c")]),
    ]);
    let ar = ResourceArchive::from_map(&map, loader(vols)).unwrap();
    assert_eq!(ar.len(), 3);
    assert_eq!(ar.get("b.wav").unwrap(), b"bb");
    assert_eq!(ar.get("A.BMP").unwrap(), b"aaaa");
    assert_eq!(ar.entries()[2].name, "C.PAL");
    assert_eq!(ar.entries()[2].volume, "RESOURCE.002");
    assert!(matches!(ar.get("Z.BMP"), Err(FormatError::NotFound(_))));
}

#[test]
fn duplicate_name_keeps_first() {
    let (map, vols) = mkarchive(&[("RESOURCE.001", &[("A.BMP", b"first"), ("A.BMP", b"second")])]);
    let ar = ResourceArchive::from_map(&map, loader(vols)).unwrap();
    assert_eq!(ar.get("A.BMP").unwrap(), b"first");
}

#[test]
fn truncated_map_never_panics() {
    let (map, vols) = mkarchive(&[
        ("RESOURCE.001", &[("A.BMP", b"aaaa")]),
        ("RESOURCE.002", &[("B.WAV", b"bb")]),
    ]);
    for n in 0..map.len() {
        let _ = ResourceArchive::from_map(&map[..n], loader(vols.clone()));
    }
}

#[test]
fn truncated_volume_never_panics() {
    let (map, vols) = mkarchive(&[("RESOURCE.001", &[("A.BMP", b"aaaa"), ("B.BMP", b"bbbb")])]);
    let vol = &vols[0].1;
    for n in 0..vol.len() {
        let cut = vec![(vols[0].0.clone(), vol[..n].to_vec())];
        assert!(ResourceArchive::from_map(&map, loader(cut)).is_err());
    }
}

#[test]
fn entry_beyond_volume_is_truncated_error() {
    let (mut map, vols) = mkarchive(&[("RESOURCE.001", &[("A.BMP", b"aaaa")])]);
    // offset da única entrada: depois de 4+2+13+2+4(hash)
    let off = 4 + 2 + 13 + 2 + 4;
    map[off..off + 4].copy_from_slice(&1_000_000u32.to_le_bytes());
    let err = ResourceArchive::from_map(&map, loader(vols)).err().unwrap();
    // O erro vem envolvido com o nome do volume; a causa é o truncamento.
    let FormatError::InResource { name, source } = &err else {
        panic!("esperado InResource: {err}");
    };
    assert_eq!(name, "RESOURCE.001");
    assert!(matches!(**source, FormatError::Truncated { .. }), "{err}");
}
