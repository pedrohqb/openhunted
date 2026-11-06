// SPDX-License-Identifier: GPL-3.0-or-later
//! Fogo (tipo 42, `0x004100a2`/`0x0041028c`/`0x004104a3`; `docs/re/barril-fogo-cerca.md`):
//! cresce e míngua pelos `values`, queima 1 por tique quem cruza a caixa.

use super::{Ctx, Hazard, Out};
use crate::damage::Pending;
use crate::entity::{Behaviour, Entity};
use crate::physics::Rect;
use crate::triggers::satisfied;

/// Altura máxima padrão e de reserva.
pub const MAX_HEIGHT: i32 = 96;
pub const DEFAULT_HEIGHT: i32 = 32;
/// Linha 42 do `DAMAGE.TBL`: natureza fogo, 1 de dano direto.
pub const DAMAGE: i32 = 1;
/// `values[13]` (`+0x294`) ≥ 1000: o fogo já foi iniciado e os campos valem como estão.
const STARTED: i32 = 1000;

#[derive(Debug, Clone)]
pub struct FireState {
    pub height: i32,
    /// Ritmo (`+0x280`): > 0 cresce `r % 10` a cada `r / 10` tiques; < 0 míngua; 0 estável.
    pub rate: i32,
    /// Tiques até o próximo passo (`+0x284`).
    pub timer: i32,
    /// Âncora: centro x e base y (`+0x288/+0x28c`).
    pub anchor: (i32, i32),
    pub max: i32,
    /// Altura do quadro de referência do `OFI.ANM` (escala = altura / esta).
    pub frame_h: i32,
    pub visible: bool,
}

impl FireState {
    /// `values`: 0 crescer, 1 minguar, 2 espera no máximo, 3 máximo, 4 altura inicial,
    /// 10/11 âncora e 13 marca de iniciado. `pos`/`size` = posição e tamanho do sprite na
    /// fase (para a âncora e a altura de quem não foi iniciado).
    pub fn new(values: &[i32], pos: (i32, i32), size: (i32, i32), frame_h: i32) -> Self {
        let v = |i: usize| values.get(i).copied().unwrap_or(0);
        let started = v(13) >= STARTED;
        let h = if started { v(4) } else { size.1 };
        let height = if h <= 0 {
            DEFAULT_HEIGHT
        } else {
            h.min(MAX_HEIGHT)
        };
        let anchor = if started && (v(10) != 0 || v(11) != 0) {
            (v(10), v(11))
        } else {
            (pos.0 + size.0 / 2, pos.1 + size.1)
        };
        let rate = if v(0) != 0 { v(0) } else { -v(1) };
        Self {
            height,
            rate,
            timer: rate.abs() / 10,
            anchor,
            max: if v(3) > 0 { v(3) } else { MAX_HEIGHT },
            frame_h: frame_h.max(1),
            visible: true,
        }
    }

    /// Caixa (0, 0, altura·512/169, altura), posta pela âncora.
    pub fn rect(&self) -> Rect {
        let w = self.height * 512 / 169;
        Rect {
            x: self.anchor.0 - w / 2,
            y: self.anchor.1 - self.height,
            w,
            h: self.height,
        }
    }

    /// Crescer/minguar; `false` = apagou.
    fn grow(&mut self, values: &[i32]) -> bool {
        let v = |i: usize| values.get(i).copied().unwrap_or(0);
        if self.rate == 0 {
            return true;
        }
        self.timer -= 1;
        if self.timer > 0 {
            return true;
        }
        if self.rate > 0 {
            self.height += self.rate % 10;
            self.timer = self.rate / 10;
            if self.height > self.max {
                self.height = self.max;
                self.timer = v(2);
                self.rate = -v(1);
            }
        } else {
            self.height -= (-self.rate) % 10;
            self.timer = (-self.rate) / 10;
            if self.height <= 0 {
                return false;
            }
        }
        true
    }
}

pub fn tick(e: &mut Entity, ctx: &Ctx, out: &mut Out) {
    let visible = satisfied(&e.triggers, ctx.on);
    let values = e.values.clone();
    let Behaviour::Fire(st) = &mut e.behaviour else {
        return;
    };
    st.visible = visible;
    e.on = visible;
    if !visible {
        return;
    }
    if !st.grow(&values) {
        e.alive = false;
        return;
    }
    let r = st.rect();
    let (h, fh) = (st.height, st.frame_h);
    e.x = r.x;
    e.y = r.y;
    e.boxes = vec![Rect {
        x: 0,
        y: 0,
        w: r.w,
        h: r.h,
    }];
    if let Some(sp) = e.sprite.as_mut() {
        sp.scale = (h, fh);
    }
    let burn = Pending {
        amount: DAMAGE,
        fire: true,
        ..Default::default()
    };
    if ctx.jake_layer == e.layer && ctx.jake_bounds().is_some_and(|j| j.overlaps(&r)) {
        out.jake_damage.add(&burn);
    }
    out.hazards.push(Hazard {
        rect: r,
        layer: e.layer,
        damage: burn,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviour::test_support::{ctx, ctx_with_jake, logic};
    use crate::entity::FIRE;

    /// Os `values` do fogo da `T_J_005`.
    const T_J_005: [i32; 14] = [19, 0, 999, 32, 38, 308, 999, 0, 19, 1, 1164, 381, 38, 1000];

    fn fire(values: &[i32]) -> Entity {
        let mut f = logic(FIRE, 1107, 343, values);
        f.flags = 0x8002d;
        f.behaviour = Behaviour::Fire(FireState::new(values, (1107, 343), (115, 38), 63));
        f
    }

    #[test]
    fn tutorial_fire_settles_at_32_px_and_burns_one_per_tick() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(1150, 334);
        let c = ctx(&j, &w, &on);
        let mut f = fire(&T_J_005);
        let mut out = Out::default();
        tick(&mut f, &c, &mut out);
        let Behaviour::Fire(st) = &f.behaviour else {
            panic!()
        };
        assert_eq!(st.height, 32);
        assert_eq!(
            st.rect(),
            Rect {
                x: 1116,
                y: 349,
                w: 96,
                h: 32
            }
        );
        assert_eq!(out.jake_damage.amount, 1);
        assert!(out.jake_damage.fire);
        assert_eq!(out.hazards.len(), 1);
        for _ in 0..50 {
            tick(&mut f, &c, &mut out);
        }
        let Behaviour::Fire(st) = &f.behaviour else {
            panic!()
        };
        assert_eq!(st.height, 32, "values[1] = 0: estável");
        assert_eq!(out.jake_damage.amount, 51);
    }

    #[test]
    fn fire_grows_shrinks_and_hides_on_false_triggers() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(0, 0);
        let c = ctx(&j, &w, &on);
        // cresce 5 px a cada 2 tiques de 10 até 20, espera 3, míngua 4 por tique
        let vals = [25, 14, 3, 20, 10, 0, 0, 0, 0, 0, 100, 200, 0, 1000];
        let mut f = fire(&vals);
        let mut hs = vec![];
        for _ in 0..16 {
            if !f.alive {
                break;
            }
            tick(&mut f, &c, &mut Out::default());
            if let Behaviour::Fire(st) = &f.behaviour {
                hs.push(st.height);
            }
        }
        assert_eq!(hs[..6], [10, 15, 15, 20, 20, 20]);
        assert!(!f.alive, "apagou: {hs:?}");
        let off = |_: i32| Some(false);
        let c = ctx(&j, &w, &off);
        let mut f = fire(&T_J_005);
        f.triggers = vec![(3, Some(11))];
        let mut out = Out::default();
        tick(&mut f, &c, &mut out);
        assert!(!f.on && out.hazards.is_empty());
    }
}
