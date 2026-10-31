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
