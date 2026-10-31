// SPDX-License-Identifier: GPL-3.0-or-later

mod support;

use openhunted_formats::test_support::mkiso;
use openhunted_formats::{DataSource, DirSource, FormatError, IsoSource};

fn iso_file(bytes: &[u8]) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    std::io::Write::write_all(&mut f, bytes).unwrap();
    f
}

#[test]
fn dir_source_is_case_insensitive() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("Hunter")).unwrap();
    std::fs::write(dir.path().join("Hunter/hunter.hls"), b"hls").unwrap();
    let src = DirSource::new(dir.path());
    assert_eq!(src.read("HUNTER/HUNTER.HLS").unwrap(), b"hls");
    assert!(src.exists("hunter/Hunter.hls"));
}

#[test]
fn iso_plain_strips_version_suffix() {
    let img = mkiso(&[("Hunter/RESOURCE.MAP", b"abc")], false);
    let f = iso_file(&img);
    let src = IsoSource::open(f.path()).unwrap();
    assert_eq!(src.read("hunter/resource.map").unwrap(), b"abc");
}

#[test]
fn iso_joliet_preserves_lowercase_names() {
    let img = mkiso(&[("Hunter/hunter.hls", b"x"), ("readme.wri", b"y")], true);
    let f = iso_file(&img);
    let src = IsoSource::open(f.path()).unwrap();
    assert!(src.exists("HUNTER/HUNTER.HLS"));
    assert_eq!(src.read("README.WRI").unwrap(), b"y");
}

#[test]
fn missing_file_is_not_found() {
    let img = mkiso(&[("a.bin", b"a")], true);
    let f = iso_file(&img);
    let src = IsoSource::open(f.path()).unwrap();
    assert!(matches!(
        src.read("nada.bin"),
        Err(FormatError::NotFound(_))
    ));
    let dir = tempfile::tempdir().unwrap();
    assert!(matches!(
        DirSource::new(dir.path()).read("nada.bin"),
        Err(FormatError::NotFound(_))
    ));
}

#[test]
fn truncated_iso_errors() {
    let img = mkiso(&[("Hunter/RESOURCE.MAP", b"abc")], true);
    let f = iso_file(&img[..16 * 2048 + 10]);
    assert!(IsoSource::open(f.path()).is_err());
}

#[test]
fn real_iso_has_resource_map() {
    let path = data_or_skip!();
    if path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("iso"))
        != Some(true)
    {
        eprintln!("pulado: OPENHUNTED_DATA não é um .iso");
        return;
    }
    let src = IsoSource::open(&path).unwrap();
    assert_eq!(src.read("Hunter/RESOURCE.MAP").unwrap().len(), 18532);
}
