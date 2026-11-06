// SPDX-License-Identifier: GPL-3.0-or-later
//! Armas do Jake (M7b): descritores `0x47df28…0x47e1a8` e registros de estado (18 `i32`,
//! `0x43ac05`), inventário com a arma corrente (`+0x254`), "melhor arma" e troca
//! (`0x43a8cc`, `0x43b0cb`). Tabelas em `docs/re/tiro-jake.md`, `armas.md`, `armas-2.md` e,
//! para os punhos, `furtividade.md` (registros `0x47d804…0x47d96c`).

use super::Aim;

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
}

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

/// Todos os rótulos dos estados de arma (para conferir contra o `JAKE.ANM`).
pub fn labels() -> Vec<u16> {
    let mut v = Vec::new();
    for w in [FISTS_W, PISTOL_W, SHOTGUN_W, ROCKETS_W, STARS_W, GRENADES_W] {
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
