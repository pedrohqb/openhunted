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
/// vivo, todo objeto "destruir" morto e todo objeto "acionar" (fora as saídas) ligado — a peça
/// do carro apanhada fica ligada (`+0x178 = 0x40`) mesmo fora do jogo.
pub fn objectives_met(entities: &[Entity], jake: Option<&Jake>) -> bool {
    if jake.is_some_and(Jake::is_dead) {
        return false;
    }
    let destroyed = entities
        .iter()
        .filter(|e| e.objective & OBJ_DESTROY != 0)
        .all(|e| !e.alive || e.health <= 0);
    // Ruling (`logica-fase.md`): só contam os tipos com comportamento implementado; os que
    // ainda são `Static` nunca ligariam e travariam a fase.
    // Desde a M7b, os "acionar" das fases solo (13, 27, 63, 64, 74, 80–85) agem.
    let activated = entities
        .iter()
        .filter(|e| {
            e.objective & OBJ_ACTIVATE != 0
                && !matches!(e.behaviour, Behaviour::Exit(_) | Behaviour::Static)
        })
        .all(|e| e.on);
    destroyed && activated
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

    #[test]
    fn activate_objective_needs_the_object_on() {
        use crate::behaviour::test_support::logic;
        let mut piece = logic(crate::entity::CAR_PIECE, 0, 0, &[]);
        piece.objective = OBJ_ACTIVATE;
        piece.behaviour = Behaviour::CarPiece;
        let mut exit = logic(crate::entity::EXIT, 0, 0, &[]);
        exit.objective = OBJ_ACTIVATE;
        exit.behaviour = Behaviour::Exit(Default::default());
        let mut ents = vec![exit, piece];
        assert!(!objectives_met(&ents, None), "peça não apanhada");
        ents[1].alive = false;
        ents[1].on = true;
        assert!(
            objectives_met(&ents, None),
            "apanhada: cumpre (a saída não conta)"
        );
    }

    /// Ruling: objetos "acionar" de tipos ainda sem comportamento (`Static`) não contam — a
    /// fase se vence só com a saída.
    #[test]
    fn static_activate_objectives_do_not_block_the_win() {
        use crate::behaviour::test_support::logic;
        let mut exit = logic(crate::entity::EXIT, 0, 0, &[]);
        exit.objective = OBJ_ACTIVATE;
        exit.behaviour = Behaviour::Exit(Default::default());
        exit.on = true;
        let mut ents = vec![exit];
        for kind in [13, 27, 63, 64, 74, 80, 85] {
            let mut e = logic(kind, 0, 0, &[]);
            e.objective = OBJ_ACTIVATE;
            assert!(matches!(e.behaviour, Behaviour::Static));
            ents.push(e);
        }
        assert!(objectives_met(&ents, None));
        assert_eq!(evaluate(&ents, None), Some(Outcome::Victory));
    }
}
