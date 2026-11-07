// SPDX-License-Identifier: GPL-3.0-or-later
//! Chumbo da espingarda (tipo 20, `0x0040ceec`; `docs/re/armas-2.md`): voa reto e se divide
//! duas vezes (espera 2 tiques, ±0x400), chegando a 4 chumbos em −11,25°, 0°, 0°, +11,25°.

use super::{Ctx, Out, angle16, cos_sin14, projectile};
use crate::entity::{Behaviour, Entity};

/// Linha `(20, 0)`: 10 de dano, derrubada, módulo do empurrão 20, efeito 15 de raio 12.
pub const DAMAGE: i32 = 10;
pub const PUSH: i32 = 20;
pub const IMPACT_RADIUS: i32 = 12;
/// Desvio de cada divisão (`±0x400` em 16 bits).
pub const SPLIT_ANGLE: i32 = 0x400;
/// Tiques entre as divisões.
pub const SPLIT_WAIT: u32 = 2;
/// Última geração que ainda se divide.
const LAST_SPLIT: u8 = 2;
/// Golpe do chicote do Garathe (108, `0x40cb6c`): divide-se a ±11,25° (`0x800`) até a 4ª
/// geração; o dano sai das linhas `(108, …)` do `DAMAGE.TBL` (20, explosão e derrubada de
/// módulo 20). O 1º tique só arma (`+0x27c = 1`, `+0x278` = 6–8 sorteado, `+0x274 = 10`):
/// a 1ª geração se divide no tique 7–9; cada filho sorteia os seus 6–8 e encolhe (tamanho
/// `+0x1f0` = `+0x274` do pai / 2 + 1; `+0x274` do filho = 3/4 do pai + 1: 10 → 6 → 5 → 4).
pub const WHIP_SPLIT_ANGLE: i32 = 0x800;
const WHIP_LAST_SPLIT: u8 = 3;
const WHIP_SHRINK: i32 = 10;

/// Espera de uma divisão do chicote: `rand % 6 / 2 + 6` (6–8).
fn whip_wait(ctx: &mut Ctx) -> u32 {
    6 + ctx.rng.below(6) / 2
}

#[derive(Debug, Clone)]
pub struct PelletState {
    /// Velocidade em 1/8 px por tique.
    pub vx: i32,
    pub vy: i32,
    pub generation: u8,
    pub wait: u32,
    pub prev: Option<(i32, i32)>,
    /// Tocou o cenário neste tique (a cena ainda testa objetos antes da faísca).
    pub spent: bool,
    /// É o golpe do chicote (108), não o chumbo.
    pub whip: bool,
    /// Chicote: medida de encolhimento dos filhos (`+0x274`).
    pub shrink: i32,
}

impl PelletState {
    pub fn new(vx: i32, vy: i32) -> Self {
        Self {
            vx,
            vy,
            generation: 1,
            wait: SPLIT_WAIT,
            prev: None,
            spent: false,
            whip: false,
            shrink: 0,
        }
    }

    /// Golpe do chicote do Garathe: geração 0 até o 1º tique o armar.
    pub fn whip(vx: i32, vy: i32) -> Self {
        Self {
            generation: 0,
            wait: 0,
            whip: true,
            ..Self::new(vx, vy)
        }
    }

    fn split(&self) -> (u8, i32) {
        if self.whip {
            (WHIP_LAST_SPLIT, WHIP_SPLIT_ANGLE)
        } else {
            (LAST_SPLIT, SPLIT_ANGLE)
        }
    }
}

/// Velocidade `(vx, vy)` girada de `da` (16 bits), com a mesma rapidez.
pub fn rotated(vx: i32, vy: i32, da: i32) -> (i32, i32) {
    let speed = f64::from(vx).hypot(f64::from(vy)).round() as i32;
    let (c, s) = cos_sin14(angle16(vx, vy) + da);
    ((speed * c) >> 14, (speed * s) >> 14)
}

pub fn tick(e: &mut Entity, ctx: &mut Ctx, out: &mut Out) {
    let pos = (e.x, e.y);
    let Behaviour::Pellet(st) = &mut e.behaviour else {
        return;
    };
    st.prev = Some(pos);
    let mut split = None;
    let (last, angle) = st.split();
    let (whip, shrink) = (st.whip, st.shrink);
    if whip && st.generation == 0 {
        // 1º tique do chicote: só arma
        st.generation = 1;
        st.wait = whip_wait(ctx);
        st.shrink = WHIP_SHRINK;
    } else if st.generation <= last {
        st.wait = st.wait.saturating_sub(1);
        if st.wait == 0 {
            split = Some((st.vx, st.vy, st.generation + 1));
        }
    }
    let v = (st.vx, st.vy);
    if let Some((vx, vy, generation)) = split {
        // dois novos no mesmo ponto, ±ângulo, geração + 1; este some
        for da in [-angle, angle] {
            let (vx, vy) = rotated(vx, vy, da);
            let mut c = e.clone();
            let wait = if whip { whip_wait(ctx) } else { SPLIT_WAIT };
            if whip {
                if let Some(sp) = c.sprite.as_mut() {
                    sp.scale.0 = shrink / 2 + 1;
                }
            }
            c.behaviour = Behaviour::Pellet(PelletState {
                vx,
                vy,
                generation,
                wait,
                prev: None,
                spent: false,
                whip,
                shrink: if whip { shrink * 3 / 4 + 1 } else { 0 },
            });
            out.spawn.push(c);
        }
        e.alive = false;
        return;
    }
    if projectile::inside_wall(e, ctx) {
        e.alive = false;
        out.impacts.push((e.center().0, e.center().1, e.layer));
        return;
    }
    let Some(f) = projectile::fly(e, v, ctx, false) else {
        e.alive = false;
        return;
    };
    if f.outside {
        e.alive = false;
    } else if f.blocked {
        if let Behaviour::Pellet(st) = &mut e.behaviour {
            st.spent = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotation_keeps_speed_and_turns_by_the_split_angle() {
        let (vx, vy) = rotated(160, 0, SPLIT_ANGLE);
        assert_eq!((vx, vy), (159, 15), "+5,6° (y para baixo)");
        let (vx, vy) = rotated(vx, vy, SPLIT_ANGLE);
        assert!(
            (155..=157).contains(&vx) && (30..=31).contains(&vy),
            "+11,25°: {vx}, {vy}"
        );
    }
}
