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

#[test]
fn real_standard_formats() {
    use openhunted_formats::{Palette, Sound, chunks, parse_text_list};
    let path = data_or_skip!();
    let ar = open_real_archive(&path);
    let pal = Palette::parse(ar.get("AVACADO.PAL").unwrap()).unwrap();
    assert_eq!(pal.0[0], [0xff, 0x00, 0xbd]);
    let snd = Sound::parse(ar.get("SX_3000.WAV").unwrap()).unwrap();
    assert_eq!(
        (snd.sample_rate, snd.channels, snd.bits_per_sample),
        (11025, 1, 8)
    );
    assert_eq!(parse_text_list(ar.get("SYSTEM.RES").unwrap())[0], "OKAY");
    let mut pals = 0;
    let mut wavs = 0;
    for e in ar.entries() {
        let data = ar.get(&e.name).unwrap();
        if e.name.to_uppercase().ends_with(".PAL") {
            Palette::parse(data).unwrap_or_else(|err| panic!("{}: {err}", e.name));
            pals += 1;
        } else if e.name.to_uppercase().ends_with(".WAV") {
            Sound::parse(data).unwrap_or_else(|err| panic!("{}: {err}", e.name));
            wavs += 1;
        }
    }
    assert_eq!((pals, wavs), (26, 280));
    let tags: Vec<[u8; 4]> = chunks(ar.get("SABOUT.ANM").unwrap())
        .map(|c| c.unwrap().tag)
        .collect();
    assert_eq!(tags, vec![*b"ANM:", *b"TAG:", *b"LAB:", *b"PAL:"]);
}

#[test]
fn sabout_decompresses_to_320x256() {
    let path = data_or_skip!();
    let ar = open_real_archive(&path);
    let bmp = ar.get("SABOUT.BMP").unwrap();
    // "BMP:" + tamanho (8), versão + 1 quadro (4), tabela 4 × 1, largura e altura (4).
    assert_eq!(&bmp[16..20], &[0x40, 0x01, 0x00, 0x01]);
    let (pixels, _) = openhunted_formats::unpack_block(&bmp[20..]).unwrap();
    assert_eq!(pixels.len(), 320 * 256);
}

#[test]
fn real_bitmaps() {
    use openhunted_formats::Bitmap;
    let path = data_or_skip!();
    let ar = open_real_archive(&path);
    let bmp = |n: &str| Bitmap::parse(ar.get(n).unwrap()).unwrap();
    let itf = bmp("ITF.BMP");
    assert_eq!(itf.frames.len(), 7);
    assert_eq!((itf.frames[0].width, itf.frames[0].height), (124, 25));
    assert_eq!(itf.frames[0].pixels[0], 0x89);
    let but = bmp("SABOTBUT.BMP");
    assert_eq!(but.frames.len(), 6);
    assert_eq!((but.frames[0].width, but.frames[0].height), (36, 32));
    let bak = bmp("SEGOCBAK.BMP");
    assert_eq!(
        (bak.frames.len(), bak.frames[0].width, bak.frames[0].height),
        (1, 640, 480)
    );
    assert_eq!(bmp("SEGOCBUT.BMP").frames.len(), 63);
    let mut n = 0;
    for e in ar
        .entries()
        .iter()
        .filter(|e| e.name.to_uppercase().ends_with(".BMP"))
    {
        let b = Bitmap::parse(ar.get(&e.name).unwrap())
            .unwrap_or_else(|err| panic!("{}: {err}", e.name));
        for f in &b.frames {
            assert_eq!(
                f.pixels.len(),
                f.width as usize * f.height as usize,
                "{}",
                e.name
            );
        }
        n += 1;
    }
    assert_eq!(n, 1654);
}

#[test]
fn real_animations() {
    use openhunted_formats::Animation;
    let path = data_or_skip!();
    let ar = open_real_archive(&path);
    let sabout = Animation::parse(ar.get("SABOUT.ANM").unwrap()).unwrap();
    assert!(
        sabout
            .bitmaps
            .iter()
            .any(|(_, n)| n.to_lowercase().starts_with("sabout."))
    );
    // Referências quebradas que já vêm assim nos dados originais.
    let known_missing = [
        ("WORLD1.ANM", "CDUMMY.BMP"),
        ("WORLD2.ANM", "C2C03D.BMP"),
        ("WORLD2.ANM", "C2C03E.BMP"),
        ("WORLD2.ANM", "C2C03F.BMP"),
        ("WORLD2.ANM", "CDUMMY.BMP"),
    ];
    let mut missing = Vec::new();
    let mut n = 0;
    for e in ar
        .entries()
        .iter()
        .filter(|e| e.name.to_uppercase().ends_with(".ANM"))
    {
        let a = Animation::parse(ar.get(&e.name).unwrap())
            .unwrap_or_else(|err| panic!("{}: {err}", e.name));
        for (_, bmp) in &a.bitmaps {
            if ar.get(bmp).is_err() {
                missing.push((e.name.to_uppercase(), bmp.to_uppercase()));
            }
        }
        n += 1;
    }
    missing.sort();
    let expected: Vec<(String, String)> = known_missing
        .iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect();
    assert_eq!(missing, expected);
    assert_eq!(n, 87);
}

#[test]
fn real_borders_and_interfaces() {
    use openhunted_formats::{Border, Interface};
    let path = data_or_skip!();
    let ar = open_real_archive(&path);
    let b = Border::parse(ar.get("C1A38A0.BRD").unwrap()).unwrap();
    assert!(b.name.starts_with("Climber"));
    assert_eq!(b.boxes.len(), 3);
    let (mut brd, mut itf) = (0, 0);
    for e in ar.entries() {
        let up = e.name.to_uppercase();
        let data = ar.get(&e.name).unwrap();
        if up.ends_with(".BRD") {
            Border::parse(data).unwrap_or_else(|err| panic!("{}: {err}", e.name));
            brd += 1;
        } else if up.ends_with(".ITF") {
            let i = Interface::parse(data).unwrap_or_else(|err| panic!("{}: {err}", e.name));
            assert!(ar.get(&i.animation).is_ok(), "{}: {}", e.name, i.animation);
            itf += 1;
        }
    }
    assert_eq!((brd, itf), (225, 16));
    let about = Interface::parse(ar.get("SABOUT.ITF").unwrap()).unwrap();
    assert_eq!(about.controls.len(), 3);
    assert_eq!(about.controls[1].name.as_deref(), Some("AB_DONE"));
    assert_eq!(about.controls[1].label, Some(2));
}

#[test]
fn real_fonts() {
    use openhunted_formats::Font;
    let path = data_or_skip!();
    let ar = open_real_archive(&path);
    let mut n = 0;
    for e in ar
        .entries()
        .iter()
        .filter(|e| e.name.to_uppercase().ends_with(".PFT"))
    {
        let f =
            Font::parse(ar.get(&e.name).unwrap()).unwrap_or_else(|err| panic!("{}: {err}", e.name));
        let a = f.glyphs[b'A' as usize]
            .as_ref()
            .unwrap_or_else(|| panic!("{}: sem 'A'", e.name));
        assert!(a.width > 0, "{}", e.name);
        n += 1;
    }
    assert_eq!(n, 5);
    assert_eq!(
        Font::parse(ar.get("GARAM12.PFT").unwrap()).unwrap().height,
        18
    );
}

#[test]
fn real_levels() {
    use openhunted_formats::{Level, open_data, parse_mission_list, parse_text_list};
    let path = data_or_skip!();
    let gd = open_data(&path).unwrap();
    let la = gd.levels().unwrap();
    let names: Vec<String> = la.names().iter().map(|n| n.to_uppercase()).collect();
    assert_eq!(names.len(), 201);
    // LEVELS.RES lista 82 fases; 5 delas não estão no hunter.hls da versão 1.00.
    let ar = gd.resources().unwrap();
    let mut missing: Vec<String> = parse_text_list(ar.get("LEVELS.RES").unwrap())
        .into_iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.trim().to_uppercase())
        .filter(|l| !names.contains(l))
        .collect();
    missing.sort();
    assert_eq!(
        missing,
        [
            "T_J_002.LEV",
            "T_J_004.LEV",
            "T_J_008.LEV",
            "T_J_010.LEV",
            "T_M_009.LEV"
        ]
    );
    let missions = parse_mission_list(&la.get("ENGLEVEL.TXT").unwrap()).unwrap();
    assert_eq!(missions.len(), 100);
    let first_jake = missions.iter().find(|m| m.player == 1).unwrap();
    assert_eq!(first_jake.file.to_uppercase(), "T_J_001.LEV");
    assert_eq!(first_jake.title, "TUTORIAL: Jake Gets a Gun");
    for m in &missions {
        let l =
            Level::parse(&la.get(&m.file).unwrap()).unwrap_or_else(|e| panic!("{}: {e}", m.file));
        assert_eq!(l.version, 1006, "{}", m.file);
        assert_eq!((l.player, l.extra), (m.player, m.extra), "{}", m.file);
    }
    let t = Level::parse(&la.get("T_J_001.LEV").unwrap()).unwrap();
    assert_eq!((t.grid_x, t.grid_y, t.grid_z), (14, 1, 1));
    assert_eq!(t.palette, "WORLD.PAL");
}

#[test]
fn every_resource_parses_by_type() {
    use openhunted_formats::{Animation, Bitmap, Border, Font, Interface, Palette, Sound};
    let path = data_or_skip!();
    let ar = open_real_archive(&path);
    let mut undecoded = BTreeMap::new();
    for e in ar.entries() {
        let data = ar.get(&e.name).unwrap();
        let ext = e
            .name
            .rsplit_once('.')
            .map(|(_, x)| x.to_uppercase())
            .unwrap_or_default();
        let ok = match ext.as_str() {
            "BMP" => Bitmap::parse(data).map(|_| ()),
            "PAL" => Palette::parse(data).map(|_| ()),
            "WAV" => Sound::parse(data).map(|_| ()),
            "ANM" => Animation::parse(data).map(|_| ()),
            "BRD" => Border::parse(data).map(|_| ()),
            "ITF" => Interface::parse(data).map(|_| ()),
            "PFT" => Font::parse(data).map(|_| ()),
            "RES" => Ok(()),
            other => {
                *undecoded.entry(other.to_string()).or_insert(0) += 1;
                Ok(())
            }
        };
        ok.unwrap_or_else(|err| panic!("{}: {err}", e.name));
    }
    // Pendentes para as próximas etapas (listados em docs/formats/README.md).
    let expected: BTreeMap<String, i32> = [("SKE", 2), ("TBL", 3), ("VCR", 1)]
        .iter()
        .map(|(k, v)| (k.to_string(), *v))
        .collect();
    assert_eq!(undecoded, expected);
}
