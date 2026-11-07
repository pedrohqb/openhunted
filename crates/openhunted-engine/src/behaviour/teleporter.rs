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
    /// Pediu viagem neste tique; a cena confirma (`settle`) depois de achar o destino.
    pub asked: bool,
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
    // o passageiro conta pela caixa do corpo mesmo invisível: em viagem ele não "saiu"
    let jake = ctx.jake.filter(|j| !j.is_dead());
    let travelling = jake.is_some_and(|j| j.is_teleporting());
    let touching_raw = e.layer == ctx.jake_layer
        && e.bounds()
            .zip(jake.map(|j| j.body.bounds()))
            .is_some_and(|(t, j)| t.overlaps(&j));
    let touching = touching_raw && ctx.jake_bounds().is_some();
    let w = e.sprite_rect().or(e.bounds()).map_or(0, |r| r.w);
    let Behaviour::Teleporter(st) = &mut e.behaviour else {
        return;
    };
    st.asked = false;
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
    if !touching_raw && !travelling {
        st.last_on = false;
    }
    // um pedido por tique: vale o primeiro da lista
    if !touching || st.last_on || ctx.tick <= st.until || out.teleport.is_some() {
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
    st.asked = true;
    out.teleport = Some(Request {
        x: e.x,
        y: e.y,
        w,
        layer: e.layer,
        targets,
    });
}

/// Confirma o pedido do tique (`0x4136ef`): com destino (`dest`, índice), o teletransporte
/// que pediu liga (`0x40`), toca o feixe e fica ocupado pela viagem, e o destino recebe a
/// espera; sem destino, nada muda nele [ruling: no original o sorteio é na metade da viagem;
/// aqui, sem destino, não há viagem].
pub fn settle(entities: &mut [Entity], dest: Option<usize>, tick: u32) {
    let Some(src) = entities
        .iter()
        .position(|e| matches!(&e.behaviour, Behaviour::Teleporter(s) if s.asked))
    else {
        return;
    };
    let e = &mut entities[src];
    if let Behaviour::Teleporter(st) = &mut e.behaviour {
        st.asked = false;
        if dest.is_none() {
            return;
        }
        st.transit = Some(0);
        st.last_on = true;
        st.until = tick;
    }
    e.on = true;
    super::restart(e, LABEL_BEAM);
    if let Some(i) = dest.filter(|&i| i != src) {
        arrive(&mut entities[i], tick);
    }
}

/// Destino da viagem: sorteia um id da lista entre os que existem; o clone vai ao centro x do
/// destino e desce/sobe em células inteiras, na camada do destino (`0x41347e`). Devolve o
/// deslocamento do passageiro e o índice do destino (a camada é a dele).
pub fn destination(
    entities: &[Entity],
    req: &Request,
    rng: &mut Rng,
) -> Option<((i32, i32), usize)> {
    let found: Vec<usize> = req
        .targets
        .iter()
        .filter_map(|&id| entities.iter().position(|e| e.alive && e.id == id))
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

/// O destino que é outro teletransporte ganha 40 tiques de espera, contados de quando o
/// clone muda de lugar (metade da viagem, `0x41347e`), e lembra o passageiro (`+0x278`).
/// `tick` é o da partida.
pub fn arrive(dest: &mut Entity, tick: u32) {
    if let Behaviour::Teleporter(st) = &mut dest.behaviour {
        st.until = tick + crate::jake::TELEPORT_MOVE + DEST_COOLDOWN;
        st.last_on = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviour::test_support::{ctx, ctx_with_jake, logic, zone};
    use crate::entity::Entity;
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
        settle(std::slice::from_mut(&mut t), Some(0), 5);
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

    /// Achado da revisão: o par I_J_104 (id 1 ↔ 2) mandava o Jake de volta para sempre. O
    /// destino só conta os 40 tiques a partir da mudança de lugar e não esquece o passageiro
    /// enquanto ele viaja.
    #[test]
    fn destination_remembers_the_travelling_passenger() {
        let on = |_: i32| None;
        // o Jake em viagem, longe do destino (ainda no lugar de partida)
        let (mut j, w) = ctx_with_jake(0, 328);
        j.start_teleport((370, 0));
        let mut d = teleporter();
        arrive(&mut d, 10);
        let mut c = ctx(&j, &w, &on);
        c.tick = 11;
        tick(&mut d, &c, &mut Out::default());
        let Behaviour::Teleporter(st) = &d.behaviour else {
            unreachable!()
        };
        assert!(st.last_on, "esqueceu o passageiro em viagem");
        // espera de 40 tiques contada da mudança de lugar (metade da viagem)
        assert_eq!(st.until, 10 + crate::jake::TELEPORT_MOVE + DEST_COOLDOWN);
    }

    /// O destino vale em qualquer camada (`0x41347e`: o clone e o passageiro vão à camada do
    /// destino); na `I_J_005` o da camada 0 leva ao id 4, na camada 2.
    #[test]
    fn destination_on_another_layer_is_valid() {
        let mut t = teleporter();
        t.triggers = vec![(4, Some(0))];
        let mut d = teleporter();
        d.id = 4;
        d.layer = 2;
        d.y = 367 + 256;
        let req = Request {
            x: t.x,
            y: t.y,
            w: 24,
            layer: 0,
            targets: vec![4],
        };
        let ents = vec![t, d];
        let ((dx, dy), i) = destination(&ents, &req, &mut Rng::new(1)).expect("destino");
        assert_eq!((i, dy), (1, 256));
        assert_eq!(dx, ents[1].center().0 - 12 - 374);
    }

    /// Sem destino válido o teletransporte não toca o feixe nem fica 42 tiques ocupado; dois
    /// pedidos no mesmo tique: vale o primeiro.
    #[test]
    fn no_destination_leaves_the_source_idle_and_first_request_wins() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(370, 328);
        let mut c = ctx(&j, &w, &on);
        c.tick = 5;
        let mut a = teleporter();
        let mut b = teleporter();
        b.x += 1;
        let mut out = Out::default();
        tick(&mut a, &c, &mut out);
        tick(&mut b, &c, &mut out);
        assert_eq!(out.teleport.as_ref().map(|r| r.x), Some(374));
        let busy =
            |e: &Entity| matches!(&e.behaviour, Behaviour::Teleporter(s) if s.transit.is_some());
        assert!(
            !busy(&a) && !busy(&b) && !a.on,
            "ocupado antes de achar o destino"
        );
        // sem destino: cancela
        let mut ents = vec![a, b];
        settle(&mut ents, None, 5);
        assert!(!busy(&ents[0]) && !ents[0].on);
        // com destino: parte
        let mut out = Out::default();
        tick(&mut ents[0], &c, &mut out);
        assert!(out.teleport.is_some());
        settle(&mut ents, Some(1), 5);
        assert!(busy(&ents[0]) && ents[0].on);
        assert!(!busy(&ents[1]));
    }
}
