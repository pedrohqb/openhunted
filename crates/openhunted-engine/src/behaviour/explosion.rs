// SPDX-License-Identifier: GPL-3.0-or-later
//! Explosão (tipo 86, `0x0040d20c`/`0x0040d499`; `docs/re/dano-vida.md`).

use super::Blast;
use crate::damage::Pending;
use crate::entity::Entity;

#[derive(Debug, Clone, Default)]
pub struct ExplosionState {
    pub applied: bool,
}

/// Dano e empurrão que a explosão causa a um objeto com centro em `victim` e na mesma camada
/// (`0x0040d499`): `R = 1,5 · raio`; a `d < R`, dano `dano·(R − d)/R` e empurrão
/// `empurrão·(R − d)/R` na direção centro → vítima; empurrão > 7 derruba.
pub fn area_effect(b: &Blast, victim: (i32, i32), layer: i32) -> Option<Pending> {
    if layer != b.layer {
        return None;
    }
    let r = b.radius * 3 / 2;
    let d = super::mine::distance(b.center, victim);
    if d >= r {
        return None;
    }
    let amount = b.damage * (r - d) / r;
    let push = b.push * (r - d) / r;
    let (dx, dy) = (victim.0 - b.center.0, victim.1 - b.center.1);
    let dir = if d == 0 { (0, -1) } else { (dx, dy) };
    let len = d.max(1);
    Some(Pending {
        amount,
        explosion: true,
        push: (push * dir.0 / len, push * dir.1 / len),
        knockdown: push > 7,
        ..Default::default()
    })
}

/// Avança a animação; no fim, a explosão some.
pub fn tick(e: &mut Entity) {
    if let Some(sp) = e.sprite.as_mut() {
        if sp.player.tick_once() {
            e.alive = false;
        }
    } else {
        e.alive = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn area_damage_falls_linearly_and_knocks_down_when_close() {
        let b = Blast {
            center: (100, 100),
            layer: 0,
            radius: 28,
            push: 15,
            damage: 10,
        };
        let at0 = area_effect(&b, (100, 100), 0).unwrap();
        assert_eq!((at0.amount, at0.knockdown), (10, true));
        let at21 = area_effect(&b, (121, 100), 0).unwrap();
        assert_eq!((at21.amount, at21.knockdown), (5, false));
        assert_eq!(at21.push, (7, 0));
        assert!(area_effect(&b, (142, 100), 0).is_none());
        assert!(area_effect(&b, (100, 100), 1).is_none());
        let left = area_effect(&b, (90, 100), 0).unwrap();
        assert!(left.push.0 < 0 && left.knockdown);
    }
}
