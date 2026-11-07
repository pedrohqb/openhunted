// SPDX-License-Identifier: GPL-3.0-or-later
//! Tabela de estados do Garathe (`0x478980 + 0x70·s`, `docs/re/garathe-movimentos.md`):
//! rótulo do `MINOTAUR.ANM` e atraso (`+0x18`) de cada estado semântico da máquina comum.

use super::weapons::{self, weapon_of};
use super::{Climb, JumpKind, Landing, State};
use crate::character::Character;

/// Mortes (`0x428863`): veneno 61 `MNDIE1`, geral 63 `MNDIE2B`, derrubado para a frente
/// (`+0x250 & 8`) 67 `MNFALL3`, para trás (`& 0x10`) 69 `MNDIE3`.
pub const DIE_POISON: u16 = 61;
pub const DIE_DEFAULT: u16 = 63;
pub const DIE_FORWARD: u16 = 67;
pub const DIE_BACK: u16 = 69;
/// O choque (121 `MNQUAKE`) começa no quadro 17 (`0x428713`).
pub const SHOCK_FRAME: usize = 17;
/// Fim do pulo para cima da corda (19): parado e sem gravidade até o quadro 9, então
/// `vy = −30` px/tique (`0x4296df`).
pub const ROPE_JUMP_FRAME: usize = 9;
pub const ROPE_JUMP_VY: i32 = -30;
/// Sons: aterrissagem do pulo longo, empurrar e pulo contra a parede (médio/longo).
pub const SOUND_LONG_LANDING: u16 = 5053;
pub const SOUND_PUSH: u16 = 5043;
pub const SPEECH_WALL_MEDIUM: u16 = 3734;
pub const SPEECH_WALL_LONG: u16 = 3740;
/// Investida do pulo longo (`0x429bda`): 7 de dano com tranco e empurrão.
pub const BODY_CHECK_DAMAGE: i32 = 7;
pub const BODY_CHECK_PUSH: i32 = 8;

/// Rótulo e atraso do estado `s` no Garathe.
pub(super) fn anim(s: State) -> (u16, u32) {
    let g = Character::Garathe;
    match s {
        State::Idle | State::Teleporting(_) | State::Shake => (1, 1),
        State::Run => (10, 0),
        State::Push => (46, 1),
        State::StandJump => (21, 0),
        State::RunUp => (24, 1),
        State::Jump(JumpKind::Medium) => (22, 1),
        State::Jump(_) => (25, 1),
        State::Fall => (27, 0),
        State::WildFall => (29, 0),
        State::Land(Landing::Fall) => (28, 0),
        State::Land(Landing::Medium) => (23, 1),
        State::Land(Landing::Long) => (24, 2),
        State::Land(Landing::Hard) => (30, 0),
        State::GetUp => (31, 0),
        State::Rise => (35, 0),
        State::Jolted => (63, 0),
        State::Pushed => (58, 0),
        State::KnockedDown(_) => (32, 1),
        State::Shocked(_) => (121, 1),
        State::Dying(label) => (label, 0),
        State::Dead => (0, 0),
        State::EnteringExit(_) => (12, 0),
        State::Fire { aim, w, alt, .. } => (
            weapon_of(g, w as usize).states[weapons::aim_slot(aim)][usize::from(alt)].0,
            0,
        ),
        State::Recover(w) => (weapon_of(g, w as usize).states[2][0].1.recover, 0),
        State::Ready(_, w) => (weapon_of(g, w as usize).ready, 0),
        State::Holster(w) => (weapon_of(g, w as usize).holster, 0),
        State::Stealth => (103, 0),
        State::JumpUp(false) => (20, 0),
        State::JumpUp(true) => (19, 0),
        State::Special(_) => (127, 0),
        State::Grab | State::LetGo => (4, 0),
        State::Climb(Climb::Vertical) => (52, 0),
        State::Climb(Climb::Side) => (53, 0),
        State::LayerDown(_) => (11, 0),
        State::LayerUp(_) => (12, 0),
        State::Crouch | State::StandUp => (37, 0),
        State::Crawl => (36, 0),
        State::Hide | State::Unhide => (54, 0),
        State::Hidden => (56, 0),
        State::BarHang => (47, 0),
        State::BarMove => (49, 1),
        State::BarPull | State::BarDown | State::BarUp => (50, 0),
    }
}

/// Rótulo do derrubado: 32 `BLASTFWD` se jogado para o lado em que olha, senão 33
/// `BLASTBK0` (`0x428531`, lado × sinal de `+0x244`).
pub(super) fn knocked_label(right: bool, facing_right: bool) -> u16 {
    if right == facing_right { 32 } else { 33 }
}

/// Estados que tocam o rótulo ao contrário: 5 (4), 38 (37), 55 (54), 51 (50), 26 (24).
pub(super) fn reversed(s: State) -> bool {
    matches!(
        s,
        State::LetGo | State::StandUp | State::Unhide | State::BarDown | State::Land(Landing::Long)
    )
}

/// Estados de arte espelhada (fim do tique nega `+0x16c`/`+0x164`): o andar 3 e os de arma.
pub(super) fn mirrored(s: State) -> bool {
    matches!(
        s,
        State::Stealth
            | State::Fire { .. }
            | State::Ready(..)
            | State::Holster(_)
            | State::Recover(_)
    )
}

/// Todos os rótulos usados (para conferir contra o `MINOTAUR.ANM` real).
pub const ALL_LABELS: [u16; 47] = [
    1,
    10,
    46,
    21,
    24,
    22,
    25,
    27,
    29,
    28,
    23,
    30,
    31,
    35,
    63,
    58,
    32,
    33,
    121,
    12,
    103,
    20,
    19,
    127,
    4,
    52,
    53,
    11,
    37,
    36,
    54,
    56,
    47,
    49,
    50,
    DIE_POISON,
    DIE_FORWARD,
    DIE_BACK,
    100,
    110,
    152,
    81,
    140,
    128,
    85,
    130,
    154,
];
