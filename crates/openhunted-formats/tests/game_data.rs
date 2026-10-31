// SPDX-License-Identifier: GPL-3.0-or-later

mod support;

use openhunted_formats::test_support::{mkarchive, mkiso};
use openhunted_formats::{GameVersion, open_data};

fn synthetic_cd() -> (tempfile::TempDir, Vec<(String, Vec<u8>)>) {
    let (map, vols) = mkarchive(&[
        ("RESOURCE.001", &[("A.BMP", b"aaaa".as_slice())]),
        ("RESOURCE.002", &[("B.WAV", b"bb".as_slice())]),
    ]);
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("Hunter")).unwrap();
    std::fs::write(dir.path().join("Hunter/RESOURCE.MAP"), &map).unwrap();
    let mut files = vec![("Hunter/RESOURCE.MAP".to_string(), map)];
    for (name, data) in vols {
        std::fs::write(dir.path().join("Hunter").join(&name), &data).unwrap();
        files.push((format!("Hunter/{name}"), data));
    }
    (dir, files)
}

#[test]
fn opens_cd_root_hunter_dir_map_file_and_iso() {
    let (dir, files) = synthetic_cd();
    let refs: Vec<(&str, &[u8])> = files
        .iter()
        .map(|(n, d)| (n.as_str(), d.as_slice()))
        .collect();
    let iso = dir.path().join("cd.iso");
    std::fs::write(&iso, mkiso(&refs, true)).unwrap();
    let paths = [
        dir.path().to_path_buf(),
        dir.path().join("Hunter"),
        dir.path().join("Hunter/RESOURCE.MAP"),
        iso,
    ];
    for p in &paths {
        let gd = open_data(p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
        let ar = gd.resources().unwrap();
        assert_eq!(ar.len(), 2, "{}", p.display());
        assert_eq!(ar.get("b.wav").unwrap(), b"bb");
    }
}

#[test]
fn unknown_version_still_opens() {
    let (dir, _) = synthetic_cd();
    let gd = open_data(dir.path()).unwrap();
    assert_eq!(gd.version(), GameVersion::Unknown);
    assert!(gd.resources().is_ok());
}

#[test]
fn missing_map_lists_tried_paths() {
    let dir = tempfile::tempdir().unwrap();
    let err = open_data(dir.path()).err().unwrap();
    assert!(err.to_string().contains("RESOURCE.MAP"), "{err}");
}

#[test]
fn real_data_is_v100() {
    let path = data_or_skip!();
    assert_eq!(open_data(&path).unwrap().version(), GameVersion::Win95V100);
}

#[test]
fn levels_reads_hunter_hls() {
    use openhunted_formats::test_support::mkhls;
    let (dir, _) = synthetic_cd();
    std::fs::write(
        dir.path().join("Hunter/hunter.hls"),
        mkhls(&[("ENGLEVEL.TXT", b"x".as_slice())]),
    )
    .unwrap();
    let gd = open_data(dir.path()).unwrap();
    assert_eq!(gd.levels().unwrap().names(), vec!["ENGLEVEL.TXT"]);
}
