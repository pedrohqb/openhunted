// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::Path;

use assert_cmd::Command;
use openhunted_formats::test_support::{mkanm, mkarchive, mkbmp, mkpal};

/// Grava um CD sintético (pasta Hunter/ com RESOURCE.MAP e um volume).
fn synthetic_cd(dir: &Path) {
    let bmp = mkbmp(&[(2, 2, vec![1, 2, 3, 4])]);
    let pal = mkpal();
    let anm = mkanm();
    let (map, vols) = mkarchive(&[(
        "RESOURCE.001",
        &[
            ("TEST.BMP", bmp.as_slice()),
            ("OAN.ANM", anm.as_slice()),
            ("WORLD.PAL", pal.as_slice()),
            ("X.WAV", b"RIFFxxxx".as_slice()),
        ],
    )]);
    std::fs::create_dir_all(dir.join("Hunter")).unwrap();
    std::fs::write(dir.join("Hunter/RESOURCE.MAP"), map).unwrap();
    for (n, d) in vols {
        std::fs::write(dir.join("Hunter").join(n), d).unwrap();
    }
}

fn ohtool() -> Command {
    Command::cargo_bin("ohtool").unwrap()
}

#[test]
fn list_prints_all_entries() {
    let cd = tempfile::tempdir().unwrap();
    synthetic_cd(cd.path());
    let out = ohtool()
        .arg("--data")
        .arg(cd.path())
        .arg("list")
        .output()
        .unwrap();
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 4);
    assert!(lines[0].starts_with("TEST.BMP\tRESOURCE.001\t"));
    let only = ohtool()
        .arg("--data")
        .arg(cd.path())
        .args(["list", "--type", "wav"])
        .output()
        .unwrap();
    assert_eq!(String::from_utf8(only.stdout).unwrap().lines().count(), 1);
}

#[test]
fn info_shows_bitmap_frames() {
    let cd = tempfile::tempdir().unwrap();
    synthetic_cd(cd.path());
    let out = ohtool()
        .arg("--data")
        .arg(cd.path())
        .args(["info", "test.bmp"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("BMP:"), "{text}");
    assert!(text.contains("2x2"), "{text}");
}

#[test]
fn extract_writes_png_with_dimensions() {
    let cd = tempfile::tempdir().unwrap();
    synthetic_cd(cd.path());
    let out = tempfile::tempdir().unwrap();
    ohtool()
        .arg("--data")
        .arg(cd.path())
        .arg("extract")
        .arg("--out")
        .arg(out.path())
        .assert()
        .success();
    let png = std::fs::File::open(out.path().join("TEST.BMP_000.png")).unwrap();
    let reader = png::Decoder::new(std::io::BufReader::new(png))
        .read_info()
        .unwrap();
    assert_eq!((reader.info().width, reader.info().height), (2, 2));
    assert_eq!(
        std::fs::read(out.path().join("X.WAV")).unwrap(),
        b"RIFFxxxx"
    );
}

#[test]
fn extract_into_existing_dir_overwrites_only_own_files() {
    let cd = tempfile::tempdir().unwrap();
    synthetic_cd(cd.path());
    let out = tempfile::tempdir().unwrap();
    std::fs::write(out.path().join("meu.txt"), b"fica").unwrap();
    std::fs::write(out.path().join("X.WAV"), b"velho").unwrap();
    ohtool()
        .arg("--data")
        .arg(cd.path())
        .args(["extract", "--type", "WAV", "--out"])
        .arg(out.path())
        .assert()
        .success();
    assert_eq!(std::fs::read(out.path().join("meu.txt")).unwrap(), b"fica");
    assert_eq!(
        std::fs::read(out.path().join("X.WAV")).unwrap(),
        b"RIFFxxxx"
    );
    assert!(!out.path().join("TEST.BMP_000.png").exists());
}

#[test]
fn missing_data_path_exits_nonzero_with_message() {
    let empty = tempfile::tempdir().unwrap();
    let out = ohtool()
        .arg("--data")
        .arg(empty.path())
        .arg("list")
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("RESOURCE.MAP"));
}

#[test]
fn unknown_version_warns_on_stderr() {
    let cd = tempfile::tempdir().unwrap();
    synthetic_cd(cd.path());
    let out = ohtool()
        .arg("--data")
        .arg(cd.path())
        .arg("list")
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&out.stderr).contains("aviso"));
}

#[test]
#[ignore = "lento: extrai todos os bitmaps reais"]
fn real_extract_all_bitmaps() {
    let Some(data) = std::env::var_os("OPENHUNTED_DATA") else {
        eprintln!("pulado: OPENHUNTED_DATA não definido");
        return;
    };
    let ar = openhunted_formats::open_data(Path::new(&data))
        .unwrap()
        .resources()
        .unwrap();
    let mut frames = 0;
    for e in ar
        .entries()
        .iter()
        .filter(|e| e.name.to_uppercase().ends_with(".BMP"))
    {
        frames += openhunted_formats::Bitmap::parse(ar.get(&e.name).unwrap())
            .unwrap()
            .frames
            .len();
    }
    let out = tempfile::tempdir().unwrap();
    ohtool()
        .arg("--data")
        .arg(&data)
        .args(["extract", "--type", "BMP", "--out"])
        .arg(out.path())
        .assert()
        .success();
    assert_eq!(std::fs::read_dir(out.path()).unwrap().count(), frames);
}

#[test]
fn closed_stdout_does_not_panic() {
    // Saída grande o bastante para não caber no buffer do pipe.
    let names: Vec<String> = (0..20_000).map(|i| format!("F{i:05}.WAV")).collect();
    let files: Vec<(&str, &[u8])> = names
        .iter()
        .map(|n| (n.as_str(), b"x".as_slice()))
        .collect();
    let (map, vols) = mkarchive(&[("RESOURCE.001", &files)]);
    let cd = tempfile::tempdir().unwrap();
    std::fs::write(cd.path().join("RESOURCE.MAP"), map).unwrap();
    std::fs::write(cd.path().join("RESOURCE.001"), &vols[0].1).unwrap();
    let mut child = std::process::Command::new(assert_cmd::cargo::cargo_bin("ohtool"))
        .arg("--data")
        .arg(cd.path())
        .arg("list")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut first = [0u8; 16];
    std::io::Read::read_exact(child.stdout.as_mut().unwrap(), &mut first).unwrap();
    drop(child.stdout.take());
    let out = child.wait_with_output().unwrap();
    assert_ne!(
        out.status.code(),
        Some(101),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn anm_exports_json() {
    let cd = tempfile::tempdir().unwrap();
    synthetic_cd(cd.path());
    let info = ohtool()
        .arg("--data")
        .arg(cd.path())
        .args(["info", "OAN.ANM"])
        .output()
        .unwrap();
    assert!(info.status.success());
    let text = String::from_utf8(info.stdout).unwrap();
    assert!(text.contains("\"frames\""), "{text}");
    let out = tempfile::tempdir().unwrap();
    ohtool()
        .arg("--data")
        .arg(cd.path())
        .args(["extract", "--type", "ANM", "--out"])
        .arg(out.path())
        .assert()
        .success();
    let json = std::fs::read_to_string(out.path().join("OAN.ANM.json")).unwrap();
    assert!(json.contains("oan00.bmp"), "{json}");
}

#[test]
fn brd_and_itf_export_json() {
    use openhunted_formats::test_support::{mkbrd, mkitf};
    let brd = mkbrd(&[(2, 0, 0, 128, 12)]);
    let itf = mkitf();
    let (map, vols) = mkarchive(&[(
        "RESOURCE.001",
        &[("A.BRD", brd.as_slice()), ("B.ITF", itf.as_slice())],
    )]);
    let cd = tempfile::tempdir().unwrap();
    std::fs::write(cd.path().join("RESOURCE.MAP"), map).unwrap();
    std::fs::write(cd.path().join("RESOURCE.001"), &vols[0].1).unwrap();
    let out = tempfile::tempdir().unwrap();
    ohtool()
        .arg("--data")
        .arg(cd.path())
        .args(["extract", "--out"])
        .arg(out.path())
        .assert()
        .success();
    let brd_json = std::fs::read_to_string(out.path().join("A.BRD.json")).unwrap();
    assert!(brd_json.contains("\"boxes\""), "{brd_json}");
    let itf_json = std::fs::read_to_string(out.path().join("B.ITF.json")).unwrap();
    assert!(itf_json.contains("T_NOME"), "{itf_json}");
}

#[test]
fn pft_exports_glyph_sheet() {
    use openhunted_formats::test_support::mkpft;
    let pft = mkpft(65, &[(1, vec![1, 2]), (2, vec![1, 2, 3, 4])]);
    let pal = mkpal();
    let (map, vols) = mkarchive(&[(
        "RESOURCE.001",
        &[("F.PFT", pft.as_slice()), ("WORLD.PAL", pal.as_slice())],
    )]);
    let cd = tempfile::tempdir().unwrap();
    std::fs::write(cd.path().join("RESOURCE.MAP"), map).unwrap();
    std::fs::write(cd.path().join("RESOURCE.001"), &vols[0].1).unwrap();
    let out = tempfile::tempdir().unwrap();
    ohtool()
        .arg("--data")
        .arg(cd.path())
        .args(["extract", "--type", "PFT", "--out"])
        .arg(out.path())
        .assert()
        .success();
    let png = std::fs::File::open(out.path().join("F.PFT.png")).unwrap();
    let reader = png::Decoder::new(std::io::BufReader::new(png))
        .read_info()
        .unwrap();
    // 16 colunas de células com a largura do maior glifo (2) e 16 linhas da altura (2).
    assert_eq!((reader.info().width, reader.info().height), (32, 32));
}
