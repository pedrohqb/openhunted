// SPDX-License-Identifier: GPL-3.0-or-later
//! Interações entre objetos que a cena resolve depois do tique de cada um (cada
//! comportamento só vê o Jake): dano a objetos, raios contra objetos, fogo e cerca contra
//! barris, e o par da cerca.

use super::{Blast, Hazard, Out};
use crate::damage::Pending;
use crate::entity::{Behaviour, Entity, LASER_EYE};

/// Soma dano pendente a um objeto que o trata: mina (vida), barril e olho-laser (pendente,
/// lido no próximo tique deles). Devolve `true` se o objeto leva dano.
pub fn hurt(e: &mut Entity, p: &Pending) -> bool {
    // inimigos da M7a: explosões e raios não dão crédito ao jogador
    if super::enemy::hurt(e, p, false) {
        return true;
    }
    match &mut e.behaviour {
        Behaviour::Mine(_) => {
            e.health -= p.amount.max(1);
            true
        }
        Behaviour::Barrel(st) => {
            st.hurt.add(p);
            true
        }
        Behaviour::LaserEye(st) => {
            st.hurt.add(p);
            true
        }
        // granada com dano de explosão pendente detona no próximo tique (`0x40d819`)
        Behaviour::Grenade(st) if p.explosion => {
            st.fuse = 0;
            true
        }
        // estrela cravada atingida por explosão some (`armas-2.md`)
        Behaviour::Star(st) if p.explosion && !st.in_flight() => {
            e.alive = false;
            true
        }
        _ => false,
    }
}

/// Explosão `b` sobre um objeto (`0x0040d499`): mesma camada, centro a menos de 1,5 · raio.
pub fn blast(b: &Blast, e: &mut Entity) {
    if !e.alive {
        return;
    }
    // a Morte oculta (`0x2`, sem colisão) não está na área (spec M10, revisão)
    if e.flags & 0x2 != 0 && matches!(e.behaviour, Behaviour::Death(_)) {
        return;
    }
    if let Some(p) = super::explosion::area_effect(b, e.center(), e.layer) {
        hurt(e, &p);
    }
}

/// O objeto barra raios e balas: presente, sólido (`0x80003 → 0x80001`, sem `0x20`) e com
/// caixas do BRD.
fn solid(e: &Entity) -> bool {
    e.alive && e.solid_shapes && e.flags & 0x80003 == 0x80001 && e.flags & 0x20 == 0
}

/// Raios em voo contra os objetos sólidos da camada (só o olho que atirou fica de fora): a
/// vítima leva o dano do raio (natureza explosão: barril explode) e o raio para.
pub fn laser_hits(entities: &mut [Entity], out: &mut Out) {
    let bolts: Vec<usize> = (0..entities.len())
        .filter(|&i| {
            entities[i].alive && matches!(&entities[i].behaviour, Behaviour::Laser(s) if s.moving())
        })
        .collect();
    for bi in bolts {
        let Some(bb) = super::laser::swept_bounds(&entities[bi]) else {
            continue;
        };
        let layer = entities[bi].layer;
        let owner = match &entities[bi].behaviour {
            Behaviour::Laser(s) => s.owner,
            _ => None,
        };
        let victim = (0..entities.len()).find(|&i| {
            let e = &entities[i];
            i != bi
                && solid(e)
                && e.layer == layer
                && !(e.kind == LASER_EYE && owner == Some((e.x, e.y)))
                // os raios não ferem os robôs de segurança, que os disparam (spec M7a)
                && e.kind != crate::entity::SECURITY_ROBOT
                && !matches!(
                    e.behaviour,
                    Behaviour::Laser(_) | Behaviour::Bullet(_) | Behaviour::Casing(_)
                )
                && e.bounds().is_some_and(|b| b.overlaps(&bb))
        });
        let Some(vi) = victim else { continue };
        let Behaviour::Laser(st) = &mut entities[bi].behaviour else {
            continue;
        };
        let p = st.damage();
        st.stop();
        hurt(&mut entities[vi], &p);
        let c = entities[bi].center();
        out.effects.push((
            Blast {
                center: c,
                layer,
                radius: super::laser::HIT_RADIUS,
                push: 0,
                damage: 0,
            },
            super::laser::HIT_LABEL,
        ));
    }
}

/// Fogo e cerca contra os demais objetos que cruzam a área (o Jake já foi tratado).
/// A faísca da cerca pega pelo centro x (como no Jake) e, com acerto, recarrega a cerca.
pub fn apply_hazards(entities: &mut [Entity], hazards: &[Hazard]) {
    for h in hazards {
        let mut hit = false;
        for e in entities
            .iter_mut()
            .filter(|e| e.alive && e.layer == h.layer)
        {
            if !matches!(e.behaviour, Behaviour::Barrel(_) | Behaviour::AiPlayer(_)) {
                continue;
            }
            let inside = e.bounds().is_some_and(|b| match h.source {
                Some(_) => super::fence::in_spark(&h.rect, &b),
                None => b.overlaps(&h.rect),
            });
            if inside {
                // a cerca faz o barril explodir (linha (40, 28): natureza 2)
                let mut p = h.damage;
                if p.electric {
                    p.explosion = true;
                }
                hurt(e, &p);
                hit = true;
            }
        }
        let Some(src) = h.source.filter(|_| hit) else {
            continue;
        };
        for e in entities.iter_mut().filter(|e| (e.x, e.y) == src) {
            if let Behaviour::Fence(st) = &mut e.behaviour {
                if st.master.is_none() {
                    st.recharge = super::fence::RECHARGE;
                }
            }
        }
    }
}

/// O par da cerca mostra os rótulos 3/4 conforme o estado do poste principal.
pub fn sync_fences(entities: &mut [Entity]) {
    let masters: Vec<((i32, i32), u8)> = entities
        .iter()
        .filter_map(|e| match &e.behaviour {
            Behaviour::Fence(s) if s.master.is_none() && e.alive => Some(((e.x, e.y), s.state)),
            _ => None,
        })
        .collect();
    for e in entities.iter_mut() {
        let Behaviour::Fence(s) = &e.behaviour else {
            continue;
        };
        let Some(m) = s.master else { continue };
        let Some(&(_, state)) = masters.iter().find(|(p, _)| *p == m) else {
            continue;
        };
        let label = super::fence::partner_label(state);
        let current = e.sprite.as_ref().and_then(super::label_of);
        if current != Some(label) {
            super::restart(e, label);
        } else if let Some(sp) = e.sprite.as_mut() {
            let anim = sp.anim.clone();
            sp.player.tick(&anim);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviour::barrel::BarrelState;
    use crate::behaviour::test_support::logic;
    use crate::entity::BARREL;
    use crate::physics::Rect;

    fn barrel(x: i32) -> Entity {
        let mut b = logic(BARREL, x, 0, &[]);
        b.flags = 0x80181;
        b.solid_shapes = true;
        b.behaviour = Behaviour::Barrel(BarrelState::new(1, vec![]));
        b
    }

    #[test]
    fn bolts_set_barrels_off_and_skip_the_eye() {
        let mut eye = logic(LASER_EYE, 0, 0, &[]);
        eye.flags = 0x80809;
        eye.solid_shapes = true;
        let mut bolt = super::super::laser::bolt((10, 10), (8, 0), 0, 0);
        if let Behaviour::Laser(l) = &mut bolt.behaviour {
            l.owner = Some((0, 0));
        }
        let mut ents = vec![eye, bolt, barrel(14)];
        let mut out = Out::default();
        laser_hits(&mut ents, &mut out);
        let Behaviour::Barrel(b) = &ents[2].behaviour else {
            panic!()
        };
        assert!(b.hurt.explosion && b.hurt.amount == 5);
        let Behaviour::Laser(l) = &ents[1].behaviour else {
            panic!()
        };
        assert!(!l.moving());
        assert_eq!(out.effects.len(), 1);
    }

    #[test]
    fn fire_and_fence_hazards_hit_barrels() {
        let mut ents = vec![barrel(0), barrel(500)];
        let fire = Hazard {
            rect: Rect {
                x: 10,
                y: 10,
                w: 5,
                h: 5,
            },
            layer: 0,
            damage: Pending {
                amount: 1,
                fire: true,
                ..Default::default()
            },
            source: None,
        };
        apply_hazards(&mut ents, &[fire]);
        let fired: Vec<bool> = ents
            .iter()
            .map(|e| matches!(&e.behaviour, Behaviour::Barrel(b) if b.hurt.fire))
            .collect();
        assert_eq!(fired, vec![true, false]);
    }

    /// A cerca contra barris segue a regra do Jake (`0x41494b`): centro x a ≤ 8 px da faísca;
    /// com acerto, a cerca entra na recarga de 10 tiques.
    #[test]
    fn fence_hits_barrels_by_centre_and_recharges() {
        use crate::behaviour::fence::{FenceState, RECHARGE};
        let mut fence = logic(crate::entity::FENCE, 500, 0, &[0]);
        fence.behaviour = Behaviour::Fence(FenceState::new(0));
        let zap = Hazard {
            rect: Rect {
                x: 32,
                y: 0,
                w: 17,
                h: 45,
            },
            layer: 0,
            damage: Pending {
                amount: 5,
                electric: true,
                ..Default::default()
            },
            source: Some((500, 0)),
        };
        // caixa cruza a faixa, mas o centro (26) está a 14 px do ponto (40): não pega
        let mut ents = vec![barrel(10), fence];
        apply_hazards(&mut ents, &[zap]);
        let hurt = |e: &Entity| matches!(&e.behaviour, Behaviour::Barrel(b) if b.hurt.amount > 0);
        let recharge = |e: &Entity| match &e.behaviour {
            Behaviour::Fence(f) => f.recharge,
            _ => unreachable!(),
        };
        assert!(!hurt(&ents[0]));
        assert_eq!(recharge(&ents[1]), 0);
        // centrado (40): explode e a cerca recarrega
        ents[0] = barrel(24);
        apply_hazards(&mut ents, &[zap]);
        assert!(hurt(&ents[0]));
        assert_eq!(recharge(&ents[1]), RECHARGE);
    }

    /// Só o olho que disparou fica de fora: o raio de um olho acerta o outro (a `T_J_007` tem
    /// dois).
    #[test]
    fn bolt_hits_another_eye_but_not_its_own() {
        let mk = |x: i32| {
            let mut eye = logic(LASER_EYE, x, 0, &[]);
            eye.flags = 0x80809;
            eye.solid_shapes = true;
            eye.behaviour = Behaviour::LaserEye(super::super::laser_eye::LaserEyeState::new(&[0]));
            eye
        };
        let mut bolt = super::super::laser::bolt((10, 10), (8, 0), 0, 0);
        if let Behaviour::Laser(l) = &mut bolt.behaviour {
            l.owner = Some((0, 0));
        }
        // o raio cruza o próprio olho (0, 0) e o outro (12, 0)
        let mut ents = vec![mk(0), bolt, mk(12)];
        laser_hits(&mut ents, &mut Out::default());
        let hurt =
            |e: &Entity| matches!(&e.behaviour, Behaviour::LaserEye(s) if !s.hurt.is_empty());
        assert!(!hurt(&ents[0]), "acertou o próprio olho");
        assert!(hurt(&ents[2]), "não acertou o outro olho");
    }
}
