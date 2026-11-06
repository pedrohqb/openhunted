// SPDX-License-Identifier: GPL-3.0-or-later
//! Cerca elétrica (tipo 40, `0x00413f40`/`0x004140ab`; `docs/re/barril-fogo-cerca.md`): par de
//! postes; a faísca corre de um lado a outro e, faiscando, dá choque em quem está nela.

use super::{Ctx, Event, Hazard, Out};
use crate::damage::Pending;
use crate::entity::{Behaviour, Entity};
use crate::physics::Rect;
use crate::sound::{Cue, PRIORITY_FRAME};
use crate::triggers::satisfied;

/// Variantes (tabela `0x004722d0`, 36 bytes): nome, passo da faísca (`+0x26c`), `+0x270`,
/// curso (`+0x264`), `+0x268`.
#[derive(Debug, Clone, Copy)]
pub struct Variant {
    pub name: &'static str,
    pub step: i32,
    pub travel: i32,
}

pub const VARIANTS: [Variant; 3] = [
    Variant {
        name: "A Electric Fence",
        step: 1,
        travel: 50,
    },
    Variant {
        name: "B Electric Fence",
        step: 2,
        travel: 50,
    },
    Variant {
        name: "C Electric Fence",
        step: 3,
        travel: 100,
    },
];

/// Linha 40 do `DAMAGE.TBL`: natureza elétrica, 5 de dano.
pub const DAMAGE: i32 = 5;
/// Recarga depois de um acerto.
pub const RECHARGE: u32 = 10;
/// % ao fim de cada ciclo: parada → faiscando; faiscando → continuar.
pub const START_PERCENT: u32 = 20;
pub const CONTINUE_PERCENT: u32 = 40;
/// "Electric Fence Zap" (sorteio).
pub const SOUNDS: [u16; 2] = [3135, 3136];
/// Estados (= rótulos do `OEF.ANM`; o par usa 3 e 4).
pub const IDLE: u8 = 1;
pub const SPARKING: u8 = 2;
const PARTNER_IDLE: u16 = 3;
const PARTNER_SPARKING: u16 = 4;
/// Distância horizontal máxima do centro da vítima ao ponto da faísca.
const REACH: i32 = 8;
/// Altura do poste (quadros do `oef01`: 45 px): faixa vertical do choque [ruling].
pub const POST_HEIGHT: i32 = 45;

#[derive(Debug, Clone)]
pub struct FenceState {
    pub variant: usize,
    pub state: u8,
    /// Faísca (`+0x27c`): 0 … curso − 4.
    pub spark: i32,
    pub forward: bool,
    pub recharge: u32,
    /// O par (cópia `0x10` criada em `0x4145a5`) já existe.
    pub partner: bool,
    /// Esta é a cópia: posição do poste principal.
    pub master: Option<(i32, i32)>,
}

impl FenceState {
    pub fn new(variant: i32) -> Self {
        Self {
            variant: variant.clamp(0, 2) as usize,
            state: IDLE,
            spark: 0,
            forward: true,
            recharge: 0,
            partner: false,
            master: None,
        }
    }

    pub fn variant(&self) -> &'static Variant {
        &VARIANTS[self.variant]
    }

    fn advance_spark(&mut self) {
        let v = *self.variant();
        let end = (v.travel - 4).max(0);
        self.spark += if self.forward { v.step } else { -v.step };
        if self.spark >= end {
            self.spark = end;
            self.forward = false;
        } else if self.spark <= 0 {
            self.spark = 0;
            self.forward = true;
        }
    }
}

/// Rótulo do par conforme o estado do poste principal.
pub fn partner_label(state: u8) -> u16 {
    if state == SPARKING {
        PARTNER_SPARKING
    } else {
        PARTNER_IDLE
    }
}

/// Faixa do choque: coluna de ±8 px no ponto da faísca, na altura do poste.
pub fn zap_rect(e: &Entity, spark: i32) -> Option<Rect> {
    let b = e.bounds()?;
    let x = b.x + 5 + spark;
    Some(Rect {
        x: x - REACH,
        y: b.y,
        w: 2 * REACH + 1,
        h: POST_HEIGHT,
    })
}

pub fn tick(e: &mut Entity, ctx: &mut Ctx, out: &mut Out) {
    let trig = satisfied(&e.triggers, ctx.on);
    let Behaviour::Fence(st) = &mut e.behaviour else {
        return;
    };
    if st.master.is_some() {
        return; // o par só acompanha o principal (a cena troca o rótulo)
    }
    if !st.partner {
        st.partner = true;
        let mut p = e.clone();
        p.x += 10;
        p.y += 5;
        p.flags |= 0x10;
        p.id = 0;
        p.triggers.clear();
        if let Behaviour::Fence(ps) = &mut p.behaviour {
            ps.partner = true;
            ps.master = Some((e.x, e.y));
        }
        super::restart(&mut p, PARTNER_IDLE);
        out.spawn.push(p);
    }
    let Behaviour::Fence(st) = &mut e.behaviour else {
        return;
    };
    if !e.on {
        if !trig {
            return;
        }
        e.on = true;
    }
    st.advance_spark();
    zap(e, ctx, out);
    // fim de ciclo do rótulo: 20% de começar a faiscar, 40% de continuar
    let Behaviour::Fence(st) = &e.behaviour else {
        return;
    };
    let state = st.state;
    let wrapped = match e.sprite.as_mut() {
        Some(sp) => {
            let anim = sp.anim.clone();
            let before = sp.player.frame();
            sp.player.tick(&anim);
            sp.player.frame() <= before
        }
        None => true,
    };
    if wrapped {
        let roll = ctx.rng.below(100);
        let next = match state {
            IDLE if roll < START_PERCENT => SPARKING,
            SPARKING if roll >= CONTINUE_PERCENT => IDLE,
            s => s,
        };
        if let Behaviour::Fence(st) = &mut e.behaviour {
            st.state = next;
        }
        if next != state {
            super::restart(e, next as u16);
        }
    }
}

/// Recarga e choque (`0x41494b`): faiscando e sem recarga, fere quem está na faísca.
fn zap(e: &mut Entity, ctx: &mut Ctx, out: &mut Out) {
    let zone = {
        let Behaviour::Fence(st) = &e.behaviour else {
            return;
        };
        zap_rect(e, st.spark)
    };
    let Behaviour::Fence(st) = &mut e.behaviour else {
        return;
    };
    if st.recharge > 0 {
        st.recharge -= 1;
        return;
    }
    if st.state != SPARKING {
        return;
    }
    let Some(zone) = zone else {
        return;
    };
    let zap = Pending {
        amount: DAMAGE,
        electric: true,
        ..Default::default()
    };
    out.hazards.push(Hazard {
        rect: zone,
        layer: e.layer,
        damage: zap,
    });
    let hit = ctx.jake_layer == e.layer
        && ctx.jake_bounds().is_some_and(|j| {
            let cx = j.x + j.w / 2;
            (cx - (zone.x + REACH)).abs() <= REACH && j.y < zone.y + zone.h && zone.y < j.y + j.h
        });
    if hit {
        out.jake_damage.add(&zap);
        st.recharge = RECHARGE;
        let id = SOUNDS[ctx.rng.below(2) as usize];
        out.events.push(Event::Sound(Cue::at(
            id,
            PRIORITY_FRAME,
            (zone.x + REACH, zone.y + zone.h / 2),
            e.layer,
        )));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviour::test_support::{ctx, ctx_with_jake, logic};
    use crate::entity::FENCE;

    fn fence(x: i32, y: i32) -> Entity {
        let mut f = logic(FENCE, x, y, &[0]);
        f.flags = 0xa4829;
        f.boxes = vec![Rect {
            x: 0,
            y: 0,
            w: 5,
            h: 5,
        }];
        f.behaviour = Behaviour::Fence(FenceState::new(0));
        f
    }

    #[test]
    fn spark_runs_back_and_forth() {
        let mut s = FenceState::new(0);
        let xs: Vec<i32> = (0..100)
            .map(|_| {
                s.advance_spark();
                s.spark
            })
            .collect();
        assert_eq!(*xs.iter().max().unwrap(), 46);
        assert_eq!(xs[45], 46);
        assert_eq!(xs[46], 45);
        assert_eq!(xs[91], 0);
    }

    #[test]
    fn fence_spawns_its_partner_and_zaps_jake_while_sparking() {
        let on = |_: i32| None;
        // Jake sob o começo da faísca (centro x ≈ fence.x + 5)
        let (j, w) = ctx_with_jake(95, 0);
        let mut c = ctx(&j, &w, &on);
        let mut f = fence(100, 0);
        let mut out = Out::default();
        tick(&mut f, &mut c, &mut out);
        assert_eq!(out.spawn.len(), 1, "o par");
        let Behaviour::Fence(p) = &out.spawn[0].behaviour else {
            panic!()
        };
        assert_eq!(p.master, Some((100, 0)));
        assert_eq!((out.spawn[0].x, out.spawn[0].y), (110, 5));
        // força a faísca no começo e o estado 2
        let Behaviour::Fence(st) = &mut f.behaviour else {
            panic!()
        };
        st.state = SPARKING;
        st.spark = 0;
        st.forward = false;
        let mut out = Out::default();
        let mut zaps = vec![];
        for t in 0..12 {
            if let Behaviour::Fence(st) = &mut f.behaviour {
                st.state = SPARKING;
                st.spark = 0;
            }
            let before = out.jake_damage.amount;
            tick(&mut f, &mut c, &mut out);
            if out.jake_damage.amount > before {
                zaps.push(t);
            }
        }
        assert_eq!(zaps, vec![0, 11], "5 de dano a cada 10 tiques de recarga");
        assert!(out.jake_damage.electric);
        assert!(out.events.iter().any(|e| matches!(
            e,
            Event::Sound(c) if SOUNDS.contains(&c.id)
        )));
    }
}
