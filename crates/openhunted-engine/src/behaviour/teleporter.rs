// SPDX-License-Identifier: GPL-3.0-or-later
//! Teletransporte (tipo 46, `0x0041324c`/`0x004132ce`; `docs/re/objetos-tutorial.md`): leva
//! o Jake que pisa nele a um destino sorteado da lista de gatilhos.

use super::{Ctx, Out};
use crate::entity::{Behaviour, Entity};
use crate::rng::Rng;

/// Duração da viagem (fases 1 e 2 do clone: ≈ 42 tiques); o passageiro muda de lugar na
/// metade.
pub const TRANSIT_TICKS: u32 = 42;
/// Espera do teletransporte de destino.
pub const DEST_COOLDOWN: u32 = 40;
/// Rótulos do `OBO.ANM`: parado (`obo01`) e o feixe (`obo00`).
pub const LABEL_IDLE: u16 = 2;
pub const LABEL_BEAM: u16 = 1;

#[derive(Debug, Clone, Default)]
pub struct TeleporterState {
    /// Tiques desde a partida, durante a viagem.
    pub transit: Option<u32>,
    /// Só libera depois deste tique (`+0x274`).
    pub until: u32,
    /// O último passageiro (`+0x278`) ainda está em cima: não leva de novo até ele sair.
    pub last_on: bool,
}

/// Pedido de viagem: o teletransporte (posição, largura, camada) e os ids de destino.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub layer: i32,
    pub targets: Vec<i32>,
}

pub fn tick(e: &mut Entity, ctx: &Ctx, out: &mut Out) {
    let touching = e.layer == ctx.jake_layer
        && e.bounds()
            .zip(ctx.jake_bounds())
            .is_some_and(|(t, j)| t.overlaps(&j));
    let w = e.sprite_rect().or(e.bounds()).map_or(0, |r| r.w);
    let Behaviour::Teleporter(st) = &mut e.behaviour else {
        return;
    };
    if let Some(n) = st.transit {
        let n = n + 1;
        if n >= TRANSIT_TICKS {
            st.transit = None;
            super::restart(e, LABEL_IDLE);
        } else {
            st.transit = Some(n);
            if let Some(sp) = e.sprite.as_mut() {
                sp.player.tick_once();
            }
        }
        return;
    }
    if !touching {
        st.last_on = false;
    }
    if !touching || st.last_on || ctx.tick <= st.until {
        if let Some(sp) = e.sprite.as_mut() {
            let anim = sp.anim.clone();
            sp.player.tick(&anim);
        }
        return;
    }
    let targets: Vec<i32> = e
        .triggers
        .iter()
        .map(|&(id, _)| id)
        .filter(|&id| id != 0)
        .collect();
    if targets.is_empty() {
        return;
    }
    st.transit = Some(0);
    st.last_on = true;
    st.until = ctx.tick;
    e.on = true;
    out.teleport = Some(Request {
        x: e.x,
        y: e.y,
        w,
        layer: e.layer,
        targets,
    });
    super::restart(e, LABEL_BEAM);
}

/// Destino da viagem: sorteia um id da lista entre os que existem; o clone vai ao centro x do
/// destino e desce/sobe em células inteiras. Devolve o deslocamento do passageiro e o índice
/// do destino. Só valem destinos na camada do teletransporte [ruling: a troca de camada do
/// Jake é de outra fatia; levado a outra camada, ele cairia nos sólidos desta].
pub fn destination(
    entities: &[Entity],
    req: &Request,
    rng: &mut Rng,
) -> Option<((i32, i32), usize)> {
    let found: Vec<usize> = req
        .targets
        .iter()
        .filter_map(|&id| {
            entities
                .iter()
                .position(|e| e.alive && e.id == id && e.layer == req.layer)
        })
        .collect();
    if found.is_empty() {
        return None;
    }
    let i = found[rng.below(found.len() as u32) as usize];
    let d = &entities[i];
    let cx = d.center().0;
    let dx = cx - req.w / 2 - req.x;
    let dy = crate::scene::CELL
        * (d.y.div_euclid(crate::scene::CELL) - req.y.div_euclid(crate::scene::CELL));
    Some(((dx, dy), i))
}

/// O destino que é outro teletransporte ganha espera e lembra o passageiro.
pub fn arrive(dest: &mut Entity, tick: u32) {
    if let Behaviour::Teleporter(st) = &mut dest.behaviour {
        st.until = tick + DEST_COOLDOWN;
        st.last_on = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviour::test_support::{ctx, ctx_with_jake, logic, zone};
    use crate::entity::TELEPORTER;

    fn teleporter() -> Entity {
        let mut t = logic(TELEPORTER, 374, 367, &[]);
        t.boxes = vec![crate::Rect {
            x: 9,
            y: 0,
            w: 6,
            h: 8,
        }];
        t.triggers = vec![(1, Some(11)), (0, Some(0))];
        t.behaviour = Behaviour::Teleporter(TeleporterState::default());
        t
    }

    #[test]
    fn stepping_on_it_asks_for_a_trip_to_the_zone() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(370, 328);
        let mut c = ctx(&j, &w, &on);
        c.tick = 5;
        let mut t = teleporter();
        let mut out = Out::default();
        tick(&mut t, &c, &mut out);
        let req = out.teleport.clone().expect("viagem");
        assert_eq!(req.targets, vec![1]);
        // a zona id 1 da T_J_005 em (438, 98), 32 × 16: centro x 454
        let mut z = zone(438, 98, 32, 16, &[32, 16]);
        z.id = 1;
        let ents = vec![t.clone(), z];
        let mut rng = Rng::new(3);
        let req = Request { w: 24, ..req };
        let ((dx, dy), i) = destination(&ents, &req, &mut rng).unwrap();
        assert_eq!((dx, dy, i), (454 - 12 - 374, -256, 1));
        // durante a viagem e logo depois, com o Jake ainda em cima, não leva de novo
        for _ in 0..60 {
            let mut o = Out::default();
            tick(&mut t, &c, &mut o);
            assert!(o.teleport.is_none());
        }
        // saiu e voltou: leva
        let (j2, w2) = ctx_with_jake(800, 328);
        tick(&mut t, &ctx(&j2, &w2, &on), &mut Out::default());
        let mut o = Out::default();
        c.tick = 100;
        tick(&mut t, &c, &mut o);
        assert!(o.teleport.is_some());
    }
}
