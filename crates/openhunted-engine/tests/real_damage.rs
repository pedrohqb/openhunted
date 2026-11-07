// SPDX-License-Identifier: GPL-3.0-or-later
//! A transcrição do `DAMAGE.TBL` (`openhunted_engine::damage::table`) confere com o arquivo
//! dos dados reais (OPENHUNTED_DATA): cabeçalho ASCIIZ, `i32` n e n registros de 15 `i32`.

use openhunted_engine::damage::table::RAW;
use openhunted_formats::open_data;

#[test]
fn transcribed_damage_table_matches_the_file() {
    let Some(p) = std::env::var_os("OPENHUNTED_DATA") else {
        eprintln!("pulado: OPENHUNTED_DATA não definido");
        return;
    };
    let gd = open_data(std::path::Path::new(&p)).unwrap();
    let res = gd.resources().unwrap();
    let d = res.get("DAMAGE.TBL").unwrap();
    let mut off = d.iter().position(|&b| b == 0).unwrap() + 1;
    let int = |o: usize| i32::from_le_bytes(d[o..o + 4].try_into().unwrap());
    let n = int(off) as usize;
    off += 4;
    assert_eq!(n, RAW.len());
    for (k, row) in RAW.iter().enumerate() {
        let file: Vec<i32> = (0..15).map(|i| int(off + 60 * k + 4 * i)).collect();
        assert_eq!(&file[..], &row[..], "registro {k}");
    }
}
