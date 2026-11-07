// SPDX-License-Identifier: GPL-3.0-or-later
//! Estrela ninja (tipo 27, `0x0040bc98`; `docs/re/armas-2.md`): voo em onda perpendicular
//! (±32 px, período de 32 tiques); na parede crava e vira item (1 estrela) por 100 tiques;
//! num objeto, 5 de dano e área 20 (raio 30).

use super::{Ctx, Event, Out, cos_sin14, projectile};
use crate::entity::{Behaviour, Entity};

/// Linhas `(27, …)`: 5 diretos com empurrão; explosão de raio 30, empurrão 30, área 20.
pub const DAMAGE: i32 = 5;
pub const RADIUS: i32 = 30;
pub const PUSH: i32 = 30;
pub const AREA: i32 = 20;
/// Amplitude da onda (px) e passo da fase por tique (32 tiques por volta).
pub const WAVE: i32 = 32;
const PHASE_STEP: i32 = 2048;
/// Vida da estrela cravada (`+0x264 = 100`).
pub const STUCK_TICKS: u32 = 100;
/// Recuo ao cravar (px) e som "Knife stick in wall".
const BACK_OFF: i32 = 4;
pub const SOUND_STICK: u16 = 3030;
/// Rótulos do `ONS.ANM`: voo e cravada.
pub const LABEL_FLY: u16 = 1;
pub const LABEL_STUCK: u16 = 5;

#[derive(Debug, Clone)]
pub struct StarState {
    /// Velocidade base (1/8 px).
    pub vx: i32,
    pub vy: i32,
    /// Tiques de voo e fase inicial (0, ou 0x8000 espelhada).
    pub k: i32,
    pub phase: i32,
    pub prev: Option<(i32, i32)>,
    /// Cravada: tiques restantes como item.
    pub stuck: Option<u32>,
    /// É a faca do Garathe (25, `0x40ba54`): reta, sem onda.
    pub knife: bool,
    /// Caixa da faca cravada (`okn02.brd`, `0x40badc`); vazia, a caixa fica a do voo.
    pub stuck_boxes: Vec<crate::physics::Rect>,
}

/// Faca (`OKN.ANM`): rótulo 2 `okn01` girando no voo e 3 `okn02` cravada
/// (`0x40ba82`: `0x406b6c(obj, 3, 3)`, com a caixa trocada para `okn02.brd`). O dano sai das
/// linhas `(25, …)` do `DAMAGE.TBL` (12 com empurrão; o barril explode).
pub const KNIFE_LABEL_FLY: u16 = 2;
pub const KNIFE_LABEL_STUCK: u16 = 3;
pub const KNIFE_STUCK_BRD: &str = "OKN02.BRD";

impl StarState {
    pub fn new(vx: i32, vy: i32) -> Self {
        Self {
            vx,
            vy,
            k: 0,
            phase: if vx < 0 { 0x8000 } else { 0 },
            prev: None,
            stuck: None,
            knife: false,
            stuck_boxes: vec![],
        }
    }

    /// Faca do Garathe: voa reta.
    pub fn knife(vx: i32, vy: i32) -> Self {
        Self {
            phase: 0,
            knife: true,
            ..Self::new(vx, vy)
        }
    }

    /// Deslocamento perpendicular (px) da onda no tique `k`.
    fn wave(&self, k: i32) -> i32 {
        let (_, s) = cos_sin14((k % 32) * PHASE_STEP + self.phase);
        (WAVE * s) >> 14
    }

    /// Velocidade deste tique: a base mais a variação da onda na perpendicular.
    pub fn velocity(&self) -> (i32, i32) {
        if self.knife {
            return (self.vx, self.vy);
        }
        let d = self.wave(self.k + 1) - self.wave(self.k);
        let len = f64::from(self.vx).hypot(f64::from(self.vy)).max(1.0);
        let (px, py) = (-f64::from(self.vy) / len, f64::from(self.vx) / len);
        (
            self.vx + (px * f64::from(d * 8)).round() as i32,
            self.vy + (py * f64::from(d * 8)).round() as i32,
        )
    }

    pub fn in_flight(&self) -> bool {
        self.stuck.is_none()
    }
}

/// Sorteio do avanço do 1º tique: 0–39 em 1/8 px.
pub const FIRST_ADVANCE_RAND: u32 = 40;

/// Avanço do 1º tique (`0x40bc98`): `r`/8 px (`r` = `rand % 40`) na direção da velocidade
/// `v`, em px inteiros (truncado). O port o aplica ao criar a estrela, onde há sorteio.
pub fn first_advance(v: (i32, i32), r: u32) -> (i32, i32) {
    let len = f64::from(v.0).hypot(f64::from(v.1));
    if len == 0.0 {
        return (0, 0);
    }
    let d = f64::from(r) / 8.0;
    (
        (f64::from(v.0) / len * d) as i32,
        (f64::from(v.1) / len * d) as i32,
    )
}

pub fn tick(e: &mut Entity, ctx: &Ctx, out: &mut Out) {
    let pos = (e.x, e.y);
    let Behaviour::Star(st) = &mut e.behaviour else {
        return;
    };
    if let Some(n) = st.stuck {
        // item no chão: o Jake recolhe 1 estrela; acabado o prazo, some
        if n == 0 {
            e.alive = false;
            return;
        }
        st.stuck = Some(n - 1);
        super::pickup::collect_star(e, ctx, out);
        return;
    }
    st.prev = Some(pos);
    let v = st.velocity();
    st.k += 1;
    let Some(f) = projectile::fly(e, v, ctx, false) else {
        e.alive = false;
        return;
    };
    if f.outside {
        e.alive = false;
        return;
    }
    if f.blocked {
        let Behaviour::Star(st) = &mut e.behaviour else {
            return;
        };
        let len = f64::from(st.vx).hypot(f64::from(st.vy)).max(1.0);
        e.x -= (f64::from(st.vx) / len * f64::from(BACK_OFF)).round() as i32;
        e.y -= (f64::from(st.vy) / len * f64::from(BACK_OFF)).round() as i32;
        st.stuck = Some(STUCK_TICKS);
        let label = if st.knife {
            KNIFE_LABEL_STUCK
        } else {
            LABEL_STUCK
        };
        if !st.stuck_boxes.is_empty() {
            e.boxes = std::mem::take(&mut st.stuck_boxes);
        }
        e.flags |= 0x28;
        super::restart(e, label);
        out.events.push(Event::Sound(crate::sound::Cue::at(
            SOUND_STICK,
            crate::sound::PRIORITY_FRAME,
            e.center(),
            e.layer,
        )));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_advance_is_a_fraction_along_the_flight() {
        assert_eq!(first_advance((144, 0), 0), (0, 0));
        assert_eq!(first_advance((144, 0), 39), (4, 0), "39/8 px");
        assert_eq!(first_advance((-144, 0), 16), (-2, 0));
        assert_eq!(first_advance((0, -80), 24), (0, -3));
        assert_eq!(first_advance((0, 0), 30), (0, 0));
    }

    #[test]
    fn wave_returns_to_the_line_after_a_period() {
        let mut st = StarState::new(18 * 8, 0);
        let (mut x, mut y, mut max) = (0, 0, 0);
        for _ in 0..32 {
            let (vx, vy) = st.velocity();
            x += vx;
            y += vy;
            st.k += 1;
            max = max.max((y / 8).abs());
        }
        assert_eq!(x / 8, 18 * 32);
        assert!(y.abs() <= 8, "volta à linha: {y}");
        assert!((30..=33).contains(&max), "±32 px: {max}");
    }
}
