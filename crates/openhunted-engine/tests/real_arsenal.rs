// SPDX-License-Identifier: GPL-3.0-or-later
//! As tabelas de armas do port (`jake::weapons`, perfis dos personagens) conferem com as do
//! `Hunter.exe` dos dados reais (OPENHUNTED_DATA/Hunter/Hunter.exe): descritores de 32 `i32`
//! (`0x47df28…0x47e1a8` do Jake, `0x47e994…0x47ec14` do Garathe, seletor `0x0043ab05`),
//! registros de estado de 18 `i32` (`0x0043ac05`), listas de armas (`0x47df0c`,
//! `0x47e978`) e registros do especial (`0x47d7bc`, `0x47e228`). Auditoria M17
//! (`docs/auditoria/armas.md`).

use openhunted_engine::character::Character;
use openhunted_engine::jake::weapons::{self, Reg, Weapon};

/// O executável dos dados, com as seções para traduzir endereços.
struct Exe {
    data: Vec<u8>,
    /// (endereço virtual, tamanho virtual, deslocamento no arquivo, tamanho no arquivo).
    sections: Vec<(u32, u32, u32, u32)>,
}

impl Exe {
    fn load() -> Option<Exe> {
        let Some(p) = std::env::var_os("OPENHUNTED_DATA") else {
            eprintln!("pulado: OPENHUNTED_DATA não definido");
            return None;
        };
        let dir = std::path::Path::new(&p);
        let find = |d: &std::path::Path, name: &str| {
            std::fs::read_dir(d).ok()?.flatten().find_map(|e| {
                e.file_name()
                    .to_str()
                    .is_some_and(|n| n.eq_ignore_ascii_case(name))
                    .then(|| e.path())
            })
        };
        let path = find(dir, "Hunter")
            .and_then(|d| find(&d, "Hunter.exe"))
            .expect("Hunter/Hunter.exe nos dados");
        let data = std::fs::read(path).unwrap();
        let u16_at = |o: usize| u16::from_le_bytes([data[o], data[o + 1]]) as usize;
        let u32_at = |o: usize| u32::from_le_bytes(data[o..o + 4].try_into().unwrap());
        let pe = u32_at(0x3c) as usize;
        let n = u16_at(pe + 6);
        let opt = u16_at(pe + 20);
        let base = u32_at(pe + 24 + 28);
        let sections = (0..n)
            .map(|i| {
                let o = pe + 24 + opt + 40 * i;
                (
                    base + u32_at(o + 12),
                    u32_at(o + 8),
                    u32_at(o + 20),
                    u32_at(o + 16),
                )
            })
            .collect();
        Some(Exe { data, sections })
    }

    fn i32s(&self, addr: u32, n: usize) -> Vec<i32> {
        let &(va, _, off, _) = self
            .sections
            .iter()
            .find(|&&(va, vs, _, rs)| addr >= va && addr < va + vs.max(rs))
            .expect("endereço fora das seções");
        let o = (off + addr - va) as usize;
        (0..n)
            .map(|i| i32::from_le_bytes(self.data[o + 4 * i..o + 4 * i + 4].try_into().unwrap()))
            .collect()
    }
}

/// Os 18 `i32` de um registro como o port os guarda.
fn port_reg(r: &Reg, variant: u32) -> Vec<i32> {
    let c = r.casing.unwrap_or(weapons::CasingReg {
        dx: (0, 0),
        dy: 0,
        v: (0, 0),
    });
    vec![
        variant as i32,
        r.hit.0 as i32,
        r.hit.1 as i32,
        r.fire.0 as i32,
        r.fire.1 as i32,
        r.again as i32,
        r.reeval as i32,
        i32::from(r.recover),
        r.dx.0,
        r.dx.1,
        r.dy,
        r.v.0,
        r.v.1,
        c.dx.0,
        c.dx.1,
        c.dy,
        c.v.0,
        c.v.1,
    ]
}

fn check_weapon(exe: &Exe, name: &str, addr: u32, w: &Weapon) {
    let d = exe.i32s(addr, 32);
    assert_eq!(
        (d[0], d[1]),
        (i32::from(w.ready), i32::from(w.holster)),
        "{name}: pronto/guardar"
    );
    assert_eq!(d[4], w.step as i32, "{name}: passo");
    let kind = |k: Option<weapons::ShotKind>| k.map_or(0, |k| k.kind() as i32);
    assert_eq!(
        (d[6], d[7]),
        (kind(w.shot), kind(w.casing)),
        "{name}: projétil/cápsula"
    );
    assert_eq!(d[29] != 0, w.spread, "{name}: rotina própria (desvio da rajada)");
    // dez estados: cima ×2, diag. cima ×2, frente ×2, diag. baixo ×2, baixo ×2; o port junta
    // diag. baixo e baixo (iguais em todas as armas)
    for k in 0..10 {
        let slot = [0, 0, 1, 1, 2, 2, 3, 3, 3, 3][k];
        let (label, r) = w.states[slot][k % 2];
        assert_eq!(d[8 + k], i32::from(label), "{name}: estado {k}");
        let raw = exe.i32s(d[18 + k] as u32, 18);
        let variant = w.blow.map_or(0, |b| b[slot]);
        assert_eq!(raw, port_reg(&r, variant), "{name}: registro do estado {label}");
    }
}

#[test]
fn weapon_descriptors_and_registers_match_the_executable() {
    let Some(exe) = Exe::load() else {
        return;
    };
    let jake = [
        ("punhos", 0x47df28, weapons::FISTS),
        ("pistola", 0x47dfa8, weapons::PISTOL),
        ("espingarda", 0x47e028, weapons::SHOTGUN),
        ("foguetes", 0x47e0a8, weapons::ROCKETS),
        ("estrelas", 0x47e128, weapons::STARS),
        ("granadas", 0x47e1a8, weapons::GRENADES),
    ];
    for (name, addr, w) in jake {
        check_weapon(&exe, name, addr, weapons::weapon_of(Character::Jake, w));
    }
    let garathe = [
        ("garras", 0x47e994, weapons::CLAWS),
        ("clava", 0x47ea14, weapons::CLUB),
        ("chicote", 0x47ea94, weapons::WHIP),
        ("facas", 0x47eb14, weapons::KNIVES),
        ("estrelas G", 0x47eb94, weapons::G_STARS),
        ("granadas G", 0x47ec14, weapons::G_GRENADES),
    ];
    for (name, addr, w) in garathe {
        check_weapon(&exe, name, addr, weapons::weapon_of(Character::Garathe, w));
    }
}

#[test]
fn weapon_orders_and_special_registers_match_the_executable() {
    let Some(exe) = Exe::load() else {
        return;
    };
    for (ch, list, special) in [
        (Character::Jake, 0x47df0c, 0x47d7bc),
        (Character::Garathe, 0x47e978, 0x47e228),
    ] {
        let p = ch.profile();
        let order: Vec<i32> = p.order.iter().map(|&w| w as i32).collect();
        assert_eq!(exe.i32s(list, 7), [&order[..], &[0]].concat(), "{ch:?}: lista");
        // registro do especial: janela `+0x04..+0x08` (com `t` já somado: o port guarda o
        // primeiro tique útil, 1 quando a janela começa em 0) e fim `+0x18`
        let r = exe.i32s(special, 18);
        assert_eq!(
            (r[1].max(1), r[2], r[6]),
            (
                p.special.first as i32,
                p.special.last as i32,
                p.special.end as i32
            ),
            "{ch:?}: especial"
        );
        // estado do especial de todas as armas (`desc+0x0c`): 9 no Jake, 127 no Garathe
        let first = if ch == Character::Jake {
            0x47df28
        } else {
            0x47e994
        };
        assert_eq!(
            exe.i32s(first + 0x0c, 1)[0],
            if ch == Character::Jake { 9 } else { 127 }
        );
    }
}
