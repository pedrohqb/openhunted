// SPDX-License-Identifier: GPL-3.0-or-later
//! Armas do Jake (M7b): descritores `0x47df28…0x47e1a8` e registros de estado (18 `i32`,
//! `0x43ac05`), inventário com a arma corrente (`+0x254`), "melhor arma" e troca
//! (`0x43a8cc`, `0x43b0cb`). Tabelas em `docs/re/tiro-jake.md`, `armas.md`, `armas-2.md` e,
//! para os punhos, `furtividade.md` (registros `0x47d804…0x47d96c`).

use super::Aim;
use crate::character::Character;

/// Números das armas no inventário (par = arma).
pub const FISTS: usize = 1;
pub const PISTOL: usize = 2;
pub const SHOTGUN: usize = 3;
pub const ROCKETS: usize = 4;
pub const STARS: usize = 5;
pub const GRENADES: usize = 6;
/// Lista do Jake, da pior para a melhor (`0x0047df0c`).
pub const ORDER: [usize; 6] = [FISTS, GRENADES, STARS, PISTOL, SHOTGUN, ROCKETS];
/// Tiques da pose "pronto" depois de trocar de arma (`+0x26c = 7`).
pub const SWITCH_POSE_TICKS: u32 = 7;

/// O que sai do cano (tipo do projétil de `0x40bff8`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShotKind {
    /// Bala da pistola (19).
    Bullet,
    /// Chumbo da espingarda (20).
    Pellet,
    /// Foguete (21).
    Rocket,
    /// Cápsula da pistola (22).
    Casing,
    /// Cartucho da espingarda (23).
    Shell,
    /// Granada (26).
    Grenade,
    /// Estrela ninja (27).
    Star,
    /// Faca do Garathe (25, `0x40ba54`).
    Knife,
    /// Golpe do chicote do Garathe (108, `0x40cb6c`).
    Whip,
}

impl ShotKind {
    pub fn kind(self) -> u32 {
        match self {
            ShotKind::Bullet => 19,
            ShotKind::Pellet => 20,
            ShotKind::Rocket => 21,
            ShotKind::Casing => 22,
            ShotKind::Shell => 23,
            ShotKind::Grenade => 26,
            ShotKind::Star => 27,
            ShotKind::Knife => 25,
            ShotKind::Whip => 108,
        }
    }
    pub fn is_casing(self) -> bool {
        matches!(self, ShotKind::Casing | ShotKind::Shell)
    }
}

/// Cápsula de um registro: dx (olhando à esquerda, à direita), dy, velocidade (vx com o
/// sinal pelo lado, vy) em px/tique.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CasingReg {
    pub dx: (i32, i32),
    pub dy: i32,
    pub v: (i32, i32),
}

/// Registro de um estado de tiro (`0x43ac05`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reg {
    /// Janela do golpe corpo a corpo `+0x04..+0x08` (soco).
    pub hit: (u32, u32),
    /// Janela de disparo `+0x0c..+0x10`: dispara se `a ≤ t < b + passo`.
    pub fire: (u32, u32),
    /// `+0x14`: depois, segurando ATIRAR → novo disparo; movendo → sai.
    pub again: u32,
    /// `+0x18`: depois, reavalia (sem ATIRAR → recuperação).
    pub reeval: u32,
    /// `+0x1c`: rótulo de recuperação (0 = direto a "pronto").
    pub recover: u16,
    /// Boca: dx (esquerda, direita) e dy.
    pub dx: (i32, i32),
    pub dy: i32,
    /// Velocidade do projétil: |vx| (sinal pelo lado) e vy, px/tique.
    pub v: (i32, i32),
    pub casing: Option<CasingReg>,
}

const fn reg(fire: (u32, u32), again: u32, reeval: u32, recover: u16) -> Reg {
    Reg {
        hit: (0, 0),
        fire,
        again,
        reeval,
        recover,
        dx: (0, 0),
        dy: 0,
        v: (0, 0),
        casing: None,
    }
}

const fn gun(base: Reg, dx: (i32, i32), dy: i32, v: (i32, i32)) -> Reg {
    Reg { dx, dy, v, ..base }
}

const fn shell(r: Reg, dx: (i32, i32), dy: i32, v: (i32, i32)) -> Reg {
    Reg {
        casing: Some(CasingReg { dx, dy, v }),
        ..r
    }
}

const fn punch(hit: (u32, u32), again: u32, reeval: u32) -> Reg {
    Reg {
        hit,
        fire: (0, 0),
        again,
        reeval,
        recover: 0,
        dx: (0, 0),
        dy: 0,
        v: (0, 0),
        casing: None,
    }
}

/// Descritor de arma (`0x0043ab05`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Weapon {
    pub ready: u16,
    pub holster: u16,
    /// Passo do contador `t` por tique.
    pub step: u32,
    pub shot: Option<ShotKind>,
    pub casing: Option<ShotKind>,
    /// Pares de estados (rótulo, registro) por mira: cima, diag. cima, frente, diag./baixo.
    pub states: [[(u16, Reg); 2]; 4],
    /// Desvio da rajada da pistola (rotina própria `0x43bab7`).
    pub spread: bool,
    /// Som do tiro tocado pelo quadro ao entrar no estado.
    pub sound: Option<u16>,
    /// Golpe corpo a corpo por mira (cima, diag., frente, baixo): a linha do `DAMAGE.TBL`.
    pub blow: Option<[Blow; 4]>,
    /// A linha até a boca só é testada até a metade (chicote, `0x43bbe3`).
    pub half_line: bool,
}

/// Linha do `DAMAGE.TBL` de um golpe: dano, derrubada (senão empurrão) e empurrão (vx pelo
/// lado, vy).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Blow {
    pub damage: i32,
    pub knockdown: bool,
    pub push: (i32, i32),
}

const fn blow(damage: i32, knockdown: bool, push: (i32, i32)) -> Blow {
    Blow {
        damage,
        knockdown,
        push,
    }
}

/// Soco do Jake, linha `(1, 0, 0)`: 8 com empurrão (2, 0).
const JAKE_PUNCH: Blow = blow(8, false, (2, 0));

const P_BASE: Reg = shell(reg((1, 3), 6, 12, 22), (22, -4), 0, (-10, -10));
const S_BASE: Reg = reg((1, 1), 8, 12, 112);
const R_BASE: Reg = reg((1, 1), 24, 32, 73);
const N_BASE: Reg = reg((7, 7), 11, 12, 131);
const G_BASE: Reg = reg((7, 7), 11, 12, 188);

const fn pair(l: u16, r: Reg) -> [(u16, Reg); 2] {
    [(l, r), (l, r)]
}

/// Punhos (`0x47df28`): 152/153 cima e diagonal, 150/151 frente, 154/155 baixo.
pub const FISTS_W: Weapon = Weapon {
    ready: 146,
    holster: 147,
    step: 1,
    shot: None,
    casing: None,
    states: [
        [(152, punch((4, 5), 6, 9)), (153, punch((5, 6), 10, 13))],
        [(152, punch((4, 5), 6, 9)), (153, punch((5, 6), 10, 13))],
        [(150, punch((2, 3), 5, 8)), (151, punch((4, 5), 8, 11))],
        [(154, punch((4, 5), 8, 11)), (155, punch((3, 4), 9, 12))],
    ],
    spread: false,
    sound: None,
    blow: Some([JAKE_PUNCH; 4]),
    half_line: false,
};

pub const PISTOL_W: Weapon = Weapon {
    ready: 18,
    holster: 19,
    step: 1,
    shot: Some(ShotKind::Bullet),
    casing: Some(ShotKind::Casing),
    states: [
        pair(56, gun(P_BASE, (9, 12), -35, (0, -20))),
        pair(54, gun(P_BASE, (-16, 24), -23, (20, -20))),
        pair(23, gun(P_BASE, (-25, 25), 2, (20, 0))),
        pair(55, gun(P_BASE, (-23, 27), 33, (20, 20))),
    ],
    spread: true,
    sound: Some(3040),
    blow: None,
    half_line: false,
};

pub const SHOTGUN_W: Weapon = Weapon {
    ready: 108,
    holster: 109,
    step: 1,
    shot: Some(ShotKind::Pellet),
    casing: Some(ShotKind::Shell),
    states: [
        pair(
            125,
            shell(gun(S_BASE, (12, 7), -35, (0, -20)), (15, 0), -5, (-10, 0)),
        ),
        pair(
            123,
            shell(
                gun(S_BASE, (-20, 20), -23, (20, -20)),
                (35, -5),
                0,
                (-10, -10),
            ),
        ),
        pair(
            122,
            shell(gun(S_BASE, (-25, 40), 6, (20, 0)), (30, 20), 0, (-10, -10)),
        ),
        pair(
            124,
            shell(
                gun(S_BASE, (-15, 25), 35, (20, 20)),
                (-20, 20),
                35,
                (10, -10),
            ),
        ),
    ],
    spread: false,
    sound: Some(3045),
    blow: None,
    half_line: false,
};

/// Foguetes: "cima" usa o estado diagonal 67 (não sobem na vertical).
pub const ROCKETS_W: Weapon = Weapon {
    ready: 62,
    holster: 64,
    step: 1,
    shot: Some(ShotKind::Rocket),
    casing: None,
    states: [
        pair(67, gun(R_BASE, (-20, 18), -23, (30, -30))),
        pair(67, gun(R_BASE, (-20, 18), -23, (30, -30))),
        pair(66, gun(R_BASE, (-25, 25), 4, (30, 0))),
        pair(68, gun(R_BASE, (-25, 23), 35, (30, 30))),
    ],
    spread: false,
    sound: Some(3035),
    blow: None,
    half_line: false,
};

pub const STARS_W: Weapon = Weapon {
    ready: 127,
    holster: 128,
    step: 2,
    shot: Some(ShotKind::Star),
    casing: None,
    states: [
        pair(144, gun(N_BASE, (8, 8), -15, (0, -10))),
        pair(142, gun(N_BASE, (-5, 18), -10, (10, -10))),
        pair(141, gun(N_BASE, (-25, 30), 7, (18, 0))),
        pair(143, gun(N_BASE, (-15, 25), 35, (10, 10))),
    ],
    spread: false,
    sound: Some(3110),
    blow: None,
    half_line: false,
};

/// Granadas. Ruling: o 3 do lançamento para cima leva o sinal do lado, como os demais.
pub const GRENADES_W: Weapon = Weapon {
    ready: 186,
    holster: 187,
    step: 2,
    shot: Some(ShotKind::Grenade),
    casing: None,
    states: [
        pair(196, gun(G_BASE, (8, 8), -15, (3, -20))),
        pair(194, gun(G_BASE, (-5, 18), -10, (20, -20))),
        pair(193, gun(G_BASE, (-25, 30), 2, (20, 0))),
        pair(195, gun(G_BASE, (-15, 25), 35, (20, 20))),
    ],
    spread: false,
    sound: None,
    blow: None,
    half_line: false,
};

/// Descritor da arma `w` (0 e as desconhecidas → punhos, como em `0x0043ab05`).
pub fn weapon(w: usize) -> &'static Weapon {
    match w {
        PISTOL => &PISTOL_W,
        SHOTGUN => &SHOTGUN_W,
        ROCKETS => &ROCKETS_W,
        STARS => &STARS_W,
        GRENADES => &GRENADES_W,
        _ => &FISTS_W,
    }
}

/// Armas do Garathe (pares do inventário, `docs/re/garathe.md`).
pub const CLAWS: usize = 1;
pub const CLUB: usize = 2;
pub const WHIP: usize = 3;
pub const KNIVES: usize = 4;
pub const G_GRENADES: usize = 5;
pub const G_STARS: usize = 6;

const fn melee(hit: (u32, u32), again: u32, reeval: u32) -> Reg {
    punch(hit, again, reeval)
}

/// Garras (`0x47e994`): 106/107 cima e diagonal, 104/105 frente, 108/109 baixo; golpe 3..4,
/// `+0x14/+0x18` 8/8. Linha `(2, 0, 0)`: 15 com empurrão (2, 0).
pub const CLAWS_W: Weapon = Weapon {
    ready: 100,
    holster: 101,
    step: 1,
    shot: None,
    casing: None,
    states: [
        [(106, melee((3, 4), 8, 8)), (107, melee((3, 4), 8, 8))],
        [(106, melee((3, 4), 8, 8)), (107, melee((3, 4), 8, 8))],
        [(104, melee((3, 4), 8, 8)), (105, melee((3, 4), 8, 8))],
        [(108, melee((3, 4), 8, 8)), (109, melee((3, 4), 8, 8))],
    ],
    spread: false,
    sound: None,
    blow: Some([blow(15, false, (2, 0)); 4]),
    half_line: false,
};

/// Clava (`0x47ea14`): 116 cima e diagonal, 114 frente, 118 baixo; golpe 5..7, 11/12.
/// Variantes 1/2/3: 15 com derrubada (20, 30) / (20, 10) / (20, −10).
pub const CLUB_W: Weapon = Weapon {
    ready: 110,
    holster: 111,
    step: 1,
    shot: None,
    casing: None,
    states: [
        pair(116, melee((5, 7), 11, 12)),
        pair(116, melee((5, 7), 11, 12)),
        pair(114, melee((5, 7), 11, 12)),
        pair(118, melee((5, 7), 11, 12)),
    ],
    spread: false,
    sound: None,
    blow: Some([
        blow(15, true, (20, 30)),
        blow(15, true, (20, 30)),
        blow(15, true, (20, 10)),
        blow(15, true, (20, -10)),
    ]),
    half_line: false,
};

const fn whip_reg(dx: (i32, i32), dy: i32, v: (i32, i32)) -> Reg {
    Reg {
        hit: (8, 10),
        ..gun(reg((9, 9), 24, 24, 154), dx, dy, v)
    }
}

/// Chicote (`0x47ea94`): 162 cima, 160 diagonal, 159 frente, 161 baixo; golpe 8..10, disparo
/// do projétil 108 em 9, 24/24, recuperação 154. Variantes 4–7: 18 com derrubada.
pub const WHIP_W: Weapon = Weapon {
    ready: 152,
    holster: 153,
    step: 2,
    shot: Some(ShotKind::Whip),
    casing: None,
    states: [
        pair(162, whip_reg((13, 21), -75, (0, -20))),
        pair(160, whip_reg((-60, 90), -55, (20, -20))),
        pair(159, whip_reg((-75, 100), 20, (20, 0))),
        pair(161, whip_reg((-65, 90), 90, (20, 20))),
    ],
    spread: false,
    sound: None,
    blow: Some([
        blow(18, true, (20, 20)),
        blow(18, true, (20, 20)),
        blow(18, true, (20, 10)),
        blow(18, true, (20, -20)),
    ]),
    half_line: true,
};

const fn throw(recover: u16) -> Reg {
    reg((8, 8), 12, 20, recover)
}

/// Facas (`0x47eb14`): 98 cima, 96 diagonal, 95 frente, 97 baixo; disparo em 8, 12/20,
/// recuperação 85; projétil 25.
pub const KNIVES_W: Weapon = Weapon {
    ready: 81,
    holster: 82,
    step: 2,
    shot: Some(ShotKind::Knife),
    casing: None,
    states: [
        pair(98, gun(throw(85), (13, 19), -35, (0, -20))),
        pair(96, gun(throw(85), (-35, 55), -23, (20, -20))),
        pair(95, gun(throw(85), (-35, 65), 17, (20, 0))),
        pair(97, gun(throw(85), (-20, 50), 45, (20, 20))),
    ],
    spread: false,
    sound: None,
    blow: None,
    half_line: false,
};

/// Granadas do Garathe (`0x47ec14`): 150, 148, 147, 149; recuperação 142; cima (3, −20).
pub const G_GRENADES_W: Weapon = Weapon {
    ready: 140,
    holster: 141,
    step: 2,
    shot: Some(ShotKind::Grenade),
    casing: None,
    states: [
        pair(150, gun(throw(142), (13, 19), -35, (3, -20))),
        pair(148, gun(throw(142), (-35, 55), -23, (20, -20))),
        pair(147, gun(throw(142), (-35, 65), 2, (20, 0))),
        pair(149, gun(throw(142), (-20, 50), 45, (20, 20))),
    ],
    spread: false,
    sound: None,
    blow: None,
    half_line: false,
};

/// Estrelas do Garathe (`0x47eb94`): 138, 136, 135, 137; recuperação 130; cima 20/24.
pub const G_STARS_W: Weapon = Weapon {
    ready: 128,
    holster: 129,
    step: 2,
    shot: Some(ShotKind::Star),
    casing: None,
    states: [
        pair(138, gun(reg((8, 8), 20, 24, 130), (13, 19), -35, (0, -20))),
        pair(136, gun(throw(130), (-35, 55), -23, (20, -20))),
        pair(135, gun(throw(130), (-35, 65), 2, (20, 0))),
        pair(137, gun(throw(130), (-20, 50), 45, (20, 20))),
    ],
    spread: false,
    sound: None,
    blow: None,
    half_line: false,
};

/// Descritor da arma `w` do personagem (`0x0043ab05`, ramo do Garathe em `0x43ab82`).
pub fn weapon_of(ch: Character, w: usize) -> &'static Weapon {
    match ch {
        Character::Jake => weapon(w),
        Character::Garathe => match w {
            CLUB => &CLUB_W,
            WHIP => &WHIP_W,
            KNIVES => &KNIVES_W,
            G_GRENADES => &G_GRENADES_W,
            G_STARS => &G_STARS_W,
            _ => &CLAWS_W,
        },
    }
}

/// Nome da arma do personagem para o HUD de texto.
pub fn name_of(ch: Character, w: usize) -> &'static str {
    match ch {
        Character::Jake => name(w),
        Character::Garathe => match w {
            CLAWS => "GARRAS",
            CLUB => "CLAVA",
            WHIP => "CHICOTE",
            KNIVES => "FACAS",
            G_GRENADES => "GRANADAS",
            G_STARS => "ESTRELAS",
            _ => "-",
        },
    }
}

/// Índice da mira nos pares do descritor.
pub fn aim_slot(aim: Aim) -> usize {
    match aim {
        Aim::Up => 0,
        Aim::DiagUp => 1,
        Aim::Forward => 2,
        Aim::DiagDown => 3,
    }
}

/// Nome da arma para o HUD de texto (os do jogo estão nos ícones da barra).
pub fn name(w: usize) -> &'static str {
    match w {
        FISTS => "PUNHOS",
        PISTOL => "PISTOLA",
        SHOTGUN => "ESPINGARDA",
        ROCKETS => "FOGUETES",
        STARS => "ESTRELAS",
        GRENADES => "GRANADAS",
        _ => "-",
    }
}

/// Todos os rótulos dos estados de arma do Jake (para conferir contra o `JAKE.ANM`).
pub fn labels() -> Vec<u16> {
    labels_of(Character::Jake)
}

/// Rótulos dos estados de arma do personagem.
pub fn labels_of(ch: Character) -> Vec<u16> {
    let mut v = Vec::new();
    for w in 1..=6 {
        let w = weapon_of(ch, w);
        v.extend([w.ready, w.holster]);
        for p in w.states {
            for (l, r) in p {
                v.push(l);
                if r.recover != 0 {
                    v.push(r.recover);
                }
            }
        }
    }
    v.sort_unstable();
    v.dedup();
    v
}
