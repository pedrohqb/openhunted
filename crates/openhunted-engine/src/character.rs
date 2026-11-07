// SPDX-License-Identifier: GPL-3.0-or-later
//! Personagens jogáveis: o Jake (tipo 1, `0x00422aa9`) e o Garathe Den (tipo 2,
//! `0x00427b68`). As duas máquinas do original têm a mesma estrutura; o port usa uma só
//! máquina semântica (`jake::State`) e põe aqui o que difere (`docs/re/garathe.md`,
//! `garathe-movimentos.md`).

/// Quem o jogador controla.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Character {
    #[default]
    Jake,
    Garathe,
}

/// Janela do especial BAIXO+ATIRAR (registro `0x47d7bc` no Jake, `0x47e228` no Garathe):
/// o efeito vale com `first ≤ t ≤ last`, custa `self_damage` por tique e o estado acaba com
/// `t > end`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Special {
    pub first: u32,
    pub last: u32,
    pub end: u32,
    pub self_damage: i32,
}

/// Fatores k da velocidade pelo esqueleto (`0x00406fc0`) por movimento.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkeK {
    /// Andar furtivo (Jake 2/149) ou de luta (Garathe 3/103).
    pub walk: i32,
    /// Pulo parado (27 / 21).
    pub stand_jump: i32,
    /// Pulo para cima (16 / 20).
    pub jump_up: i32,
    /// Engatinhar (86) / rastejar (36).
    pub crawl: i32,
    /// Mão ante mão na barra (107 / 49).
    pub bar: i32,
    /// Corda, escada ou rede (30, 32 / 52, 53).
    pub rope: i32,
}

/// O que difere entre os personagens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Profile {
    /// Tipo do objeto na fase.
    pub kind: u32,
    /// Corrida em px/tique (Jake `0x00478928`; Garathe `0x0047d144`).
    pub run: i32,
    /// Velocidade ao empurrar a parede (Garathe `vx = ±8`, `0x0042a9a4`).
    pub push: i32,
    /// Lançamento (vx, vy) dos pulos médio e longo e, no Jake, do estendido.
    pub medium: (i32, i32),
    pub long: (i32, i32),
    pub extended: Option<(i32, i32)>,
    /// Alcances sondados para escolher o pulo: vx · 2|vy| / g (`0x00425144`, `0x00429a0b`).
    pub probe_medium: i32,
    pub probe_long: i32,
    pub probe_extended: Option<i32>,
    pub k: SkeK,
    /// Espaço livre acima para levantar, atirar ou pular agachado (Jake 30, Garathe 20).
    pub headroom: i32,
    pub special: Special,
    /// Fator da compensação do desenho (Jake 12/20; Garathe 587/1000, `0x427a3d`).
    pub draw: (i32, i32),
    /// Lista das armas da pior para a melhor (`0x0047df0c` / `0x0047e978`).
    pub order: [usize; 6],
    /// Bit de `+0x178` do item que este personagem pode pegar (`itens.md`).
    pub take_bit: i32,
    /// Esqueleto e id esperado.
    pub ske: &'static str,
    pub ske_id: i16,
    /// BRDs de rastejar e da barra.
    pub crawl_brd: &'static str,
    pub bar_brd: &'static str,
}

pub const JAKE: Profile = Profile {
    kind: 1,
    run: 12,
    push: 12,
    medium: (10, -11),
    long: (10, -15),
    extended: Some((17, -15)),
    probe_medium: 110,
    probe_long: 150,
    probe_extended: Some(255),
    k: SkeK {
        walk: 3,
        stand_jump: 3,
        jump_up: 3,
        crawl: 2,
        bar: 4,
        rope: 2,
    },
    headroom: 30,
    special: Special {
        first: 1,
        last: 5,
        end: 5,
        self_damage: 3,
    },
    draw: (12, 20),
    order: [1, 6, 5, 2, 3, 4],
    take_bit: 0x100,
    ske: "JAKE.SKE",
    ske_id: 1,
    crawl_brd: "JK_CRAWL.BRD",
    bar_brd: "JK_BARUP.BRD",
};

pub const GARATHE: Profile = Profile {
    kind: 2,
    run: 15,
    push: 8,
    // `0x47d128/0x47d12c` e `0x47d130/0x47d134`
    medium: (13, -12),
    long: (25, -14),
    extended: None,
    probe_medium: 156,
    probe_long: 350,
    probe_extended: None,
    k: SkeK {
        walk: 3,
        stand_jump: 3,
        jump_up: 2,
        crawl: 5,
        bar: 6,
        rope: 2,
    },
    headroom: 20,
    special: Special {
        first: 5,
        last: 20,
        end: 24,
        self_damage: 1,
    },
    draw: (587, 1000),
    order: [1, 2, 5, 4, 6, 3],
    take_bit: 0x200,
    ske: "MINOTAUR.SKE",
    ske_id: 2,
    crawl_brd: "MN_CRAWL.BRD",
    bar_brd: "MN_BARUP.BRD",
};

impl Character {
    pub fn profile(self) -> &'static Profile {
        match self {
            Character::Jake => &JAKE,
            Character::Garathe => &GARATHE,
        }
    }

    /// Pelo tipo do objeto (1 Jake, 2 Garathe).
    pub fn from_kind(kind: u32) -> Option<Character> {
        match kind {
            1 => Some(Character::Jake),
            2 => Some(Character::Garathe),
            _ => None,
        }
    }

    /// Fala equivalente (`0x40b9eb(obj, fala do Jake, fala do outro, …)`).
    pub fn speech(self, jake: u16) -> u16 {
        match (self, jake) {
            (Character::Jake, s) => s,
            (Character::Garathe, 3700) => 3735,
            (Character::Garathe, 3717) => 3737,
            (Character::Garathe, 3704) => 3736,
            (Character::Garathe, 3718) => 3731,
            (Character::Garathe, 3715) => 3730,
            (Character::Garathe, 3714) => 3732,
            (Character::Garathe, 3719) => 3741,
            (Character::Garathe, 3712) => 3734,
            (Character::Garathe, s) => s,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jumps_reach_the_probe_distances() {
        // alcance = vx · 2|vy| / g, com g = 2 px/tique²
        for p in [JAKE, GARATHE] {
            assert_eq!(p.medium.0 * -p.medium.1, p.probe_medium);
            assert_eq!(p.long.0 * -p.long.1, p.probe_long);
        }
        assert_eq!(JAKE.extended.map(|e| e.0 * -e.1), JAKE.probe_extended);
        assert_eq!(Character::from_kind(2), Some(Character::Garathe));
        assert_eq!(Character::Garathe.speech(3715), 3730);
        assert_eq!(Character::Jake.speech(3715), 3715);
    }
}
