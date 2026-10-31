// SPDX-License-Identifier: GPL-3.0-or-later
//! Veredito da missão (`0x0043cdf0`; `docs/re/logica-fase.md`): objetivos e fim de partida.

use crate::Jake;
use crate::behaviour::Outcome;
use crate::entity::{Behaviour, Entity};

/// Bits de objetivo (`+0x1ac`).
pub const OBJ_ACTIVATE: i32 = 2;
pub const OBJ_DESTROY: i32 = 4;
pub const OBJ_SURVIVE: i32 = 8;
/// Tiques seguidos que um resultado precisa durar.
pub const STREAK: u32 = 40;

/// Os objetivos estão cumpridos, sem contar a própria saída (`0x0043cdf0(0)`): o jogador
/// vivo e todo objeto "destruir" morto.
pub fn objectives_met(entities: &[Entity], jake: Option<&Jake>) -> bool {
    if jake.is_some_and(Jake::is_dead) {
        return false;
    }
    entities
        .iter()
        .filter(|e| e.objective & OBJ_DESTROY != 0)
        .all(|e| !e.alive || e.health <= 0)
}

/// Resultado do tique: derrota se o jogador morreu (bit "sobreviver"); vitória se a saída
/// está ligada e os objetivos cumpridos; senão nada.
pub fn evaluate(entities: &[Entity], jake: Option<&Jake>) -> Option<Outcome> {
    if jake.is_some_and(Jake::is_dead) {
        return Some(Outcome::Defeat);
    }
    let exit_on = entities
        .iter()
        .any(|e| matches!(e.behaviour, Behaviour::Exit(_)) && e.alive && e.on);
    if exit_on && objectives_met(entities, jake) {
        return Some(Outcome::Victory);
    }
    None
}

/// Acumula o resultado por `STREAK` tiques seguidos (`0x497478`).
#[derive(Debug, Clone, Default)]
pub struct Streak {
    current: Option<Outcome>,
    ticks: u32,
}

impl Streak {
    /// Registra o resultado do tique; devolve o veredito quando ele durou `STREAK` tiques.
    pub fn push(&mut self, r: Option<Outcome>) -> Option<Outcome> {
        if r != self.current {
            self.current = r;
            self.ticks = 0;
        }
        if r.is_some() {
            self.ticks += 1;
            if self.ticks >= STREAK {
                return r;
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streak_needs_forty_consecutive_ticks() {
        let mut s = Streak::default();
        for _ in 0..39 {
            assert_eq!(s.push(Some(Outcome::Victory)), None);
        }
        assert_eq!(s.push(None), None);
        for _ in 0..39 {
            assert_eq!(s.push(Some(Outcome::Victory)), None);
        }
        assert_eq!(s.push(Some(Outcome::Victory)), Some(Outcome::Victory));
        // a derrota no meio reinicia a contagem
        let mut s = Streak::default();
        for _ in 0..30 {
            s.push(Some(Outcome::Victory));
        }
        for i in 0..40 {
            let r = s.push(Some(Outcome::Defeat));
            assert_eq!(r, if i == 39 { Some(Outcome::Defeat) } else { None });
        }
    }
}
