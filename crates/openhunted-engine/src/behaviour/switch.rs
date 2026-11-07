// SPDX-License-Identifier: GPL-3.0-or-later
//! Interruptor (tipo 74, o "Toggle Switch" `0x0040e393` sem objetivo; `logica-fase.md`,
//! `itens-campanha.md`): o Jake o aciona com CIMA (`0x0040e4dd`), invertendo o "ligado" lido
//! pelos gatilhos; `values[1] = 0` → aciona uma vez só.

use crate::entity::Entity;

#[derive(Debug, Clone, Default)]
pub struct SwitchState {
    /// Tique a partir do qual pode inverter de novo (`+0x278`).
    pub rearm: u32,
}

/// Acionado no tique `tick`: inverte se já pode (`+0x278 < tique`) e rearma.
pub fn toggle(e: &mut Entity, tick: u32) -> bool {
    let crate::entity::Behaviour::Switch(st) = &mut e.behaviour else {
        return false;
    };
    if st.rearm > tick {
        return false;
    }
    let again = e.values.get(1).copied().unwrap_or(0) != 0;
    st.rearm = if again {
        tick.saturating_add(1)
    } else {
        u32::MAX
    };
    e.on = !e.on;
    true
}

/// A alavanca anda até o fim do rótulo ligada e volta desligada (como a saída).
pub fn tick(e: &mut Entity) {
    super::exit::tick(e);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviour::test_support::logic;
    use crate::entity::Behaviour;

    #[test]
    fn toggles_back_only_when_allowed() {
        let mut s = logic(crate::entity::SWITCH, 0, 0, &[0, 1]);
        s.behaviour = Behaviour::Switch(SwitchState::default());
        assert!(toggle(&mut s, 5) && s.on);
        assert!(!toggle(&mut s, 5), "no mesmo tique, não");
        assert!(toggle(&mut s, 6) && !s.on);
        let mut once = logic(crate::entity::SWITCH, 0, 0, &[0, 0]);
        once.behaviour = Behaviour::Switch(SwitchState::default());
        assert!(toggle(&mut once, 1) && once.on);
        assert!(!toggle(&mut once, 1000) && once.on, "aciona uma vez só");
    }
}
