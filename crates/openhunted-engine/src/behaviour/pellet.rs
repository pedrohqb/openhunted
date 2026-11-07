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
/// Golpe do chicote do Garathe (108, `0x40cb6c`): divide-se a ±11,25° (`0x800`) depois de
/// 6–8 tiques até a 4ª geração; o dano sai das linhas `(108, …)` do `DAMAGE.TBL` (20,
/// explosão e derrubada de módulo 20).
pub const WHIP_SPLIT_ANGLE: i32 = 0x800;
pub const WHIP_SPLIT_WAIT: u32 = 7;
const WHIP_LAST_SPLIT: u8 = 3;

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
        }
    }

    /// Golpe do chicote do Garathe.
    pub fn whip(vx: i32, vy: i32) -> Self {
        Self {
            wait: WHIP_SPLIT_WAIT,
            whip: true,
            ..Self::new(vx, vy)
        }
    }

    fn split(&self) -> (u8, u32, i32) {
        if self.whip {
            (WHIP_LAST_SPLIT, WHIP_SPLIT_WAIT, WHIP_SPLIT_ANGLE)
        } else {
            (LAST_SPLIT, SPLIT_WAIT, SPLIT_ANGLE)
        }
    }
}

/// Velocidade `(vx, vy)` girada de `da` (16 bits), com a mesma rapidez.
pub fn rotated(vx: i32, vy: i32, da: i32) -> (i32, i32) {
    let speed = f64::from(vx).hypot(f64::from(vy)).round() as i32;
    let (c, s) = cos_sin14(angle16(vx, vy) + da);
    ((speed * c) >> 14, (speed * s) >> 14)
}

pub fn tick(e: &mut Entity, ctx: &Ctx, out: &mut Out) {
    let pos = (e.x, e.y);
    let Behaviour::Pellet(st) = &mut e.behaviour else {
        return;
    };
    st.prev = Some(pos);
    let mut split = None;
    let (last, wait, angle) = st.split();
    let whip = st.whip;
    if st.generation <= last {
        st.wait = st.wait.saturating_sub(1);
        if st.wait == 0 {
            split = Some((st.vx, st.vy, st.generation + 1));
        }
    }
    let v = (st.vx, st.vy);
    if let Some((vx, vy, generation)) = split {
        // dois novos no mesmo ponto, ±0x400, geração + 1; este some
        for da in [-angle, angle] {
            let (vx, vy) = rotated(vx, vy, da);
            let mut c = e.clone();
            c.behaviour = Behaviour::Pellet(PelletState {
                vx,
                vy,
                generation,
                wait,
                prev: None,
                spent: false,
                whip,
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
