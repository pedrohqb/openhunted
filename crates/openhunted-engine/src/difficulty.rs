// SPDX-License-Identifier: GPL-3.0-or-later
//! Dificuldade (`0x487854`, `docs/re/opcoes-salvamento.md`): não muda o dano que o jogador
//! recebe; escala o dano que certos inimigos recebem, a munição apanhada e o ritmo da Morte.

/// 0 fácil, 1 médio (padrão), 2 difícil (`Difficulty` do `HUNTER.INI`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Difficulty {
    Easy,
    #[default]
    Normal,
    Hard,
}

impl Difficulty {
    /// Pelo valor do `HUNTER.INI` (`0x43dfef`: fora de 0–2 vira médio).
    pub fn from_index(i: i32) -> Self {
        match i {
            0 => Difficulty::Easy,
            2 => Difficulty::Hard,
            _ => Difficulty::Normal,
        }
    }

    /// O valor do `HUNTER.INI` (0, 1, 2).
    pub fn index(self) -> i32 {
        match self {
            Difficulty::Easy => 0,
            Difficulty::Normal => 1,
            Difficulty::Hard => 2,
        }
    }

    /// Escala do dano recebido pelos inimigos que usam a dificuldade: fácil ×2, difícil ÷2
    /// (sem mínimo; ver [`Difficulty::hit`]).
    pub fn damage(self, d: i32) -> i32 {
        match self {
            Difficulty::Easy => d.saturating_mul(2),
            Difficulty::Normal => d,
            Difficulty::Hard => d / 2,
        }
    }

    /// Dano que um inimigo leva nos seis pontos que leem a dificuldade (Morte `0x416979`,
    /// Camaleão `0x41a002`, gerador `0x41dfaf`, Manta `0x42231b`, Bruiser `0x417b31`, robô
    /// `0x41ca37`): dano 0 não faz nada (`None`); escalado, **só um negativo vira 1**
    /// (`test ecx,ecx; jge`), logo no difícil um dano de 1 vira 0. O Anticorpo (`0x412189`)
    /// não lê a dificuldade.
    pub fn hit(self, d: i32) -> Option<i32> {
        (d != 0).then(|| {
            let v = self.damage(d);
            if v < 0 { 1 } else { v }
        })
    }

    /// Munição apanhada (`0x40a36d`): ×1,5 / ×1 / ×0,75 (no mínimo 1 se havia alguma).
    pub fn ammo(self, n: i32) -> i32 {
        let v = match self {
            Difficulty::Easy => n.saturating_mul(3) / 2,
            Difficulty::Normal => n,
            Difficulty::Hard => n.saturating_mul(3) / 4,
        };
        if n > 0 { v.max(1) } else { v }
    }

    /// Espalhamento vertical do tiro da Morte (`0x416696`): `max(10, (2 − d)·15/2)`.
    pub fn death_spread(self) -> i32 {
        ((2 - self.index()) * 15 / 2).max(10)
    }

    /// Base da espera da Morte entre ataques (`0x416753`): `(2 − d)·75` tiques (somam-se
    /// `rand % 100`).
    pub fn death_wait_base(self) -> i32 {
        (2 - self.index()) * 75
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn damage_doubles_and_halves() {
        assert_eq!(Difficulty::Easy.damage(2), 4);
        assert_eq!(Difficulty::Normal.damage(2), 2);
        assert_eq!(Difficulty::Hard.damage(2), 1);
        assert_eq!(
            Difficulty::Hard.damage(1),
            0,
            "o mínimo fica com quem chama"
        );
        assert_eq!(Difficulty::Easy.damage(15), 30);
        assert_eq!(Difficulty::from_index(7), Difficulty::Normal);
        assert_eq!(Difficulty::from_index(0), Difficulty::Easy);
        assert_eq!(Difficulty::default().index(), 1);
    }

    #[test]
    fn hit_skips_zero_and_only_a_negative_becomes_one() {
        for d in [Difficulty::Easy, Difficulty::Normal, Difficulty::Hard] {
            assert_eq!(d.hit(0), None, "dano 0 não faz nada");
            assert_eq!(d.hit(-3), Some(1), "negativo → 1");
        }
        assert_eq!(
            Difficulty::Hard.hit(1),
            Some(0),
            "difícil: 1 / 2 = 0, sem mínimo"
        );
        assert_eq!(Difficulty::Easy.hit(1), Some(2));
        assert_eq!(Difficulty::Normal.hit(5), Some(5));
    }

    #[test]
    fn ammo_and_death_pace_scale() {
        assert_eq!(Difficulty::Easy.ammo(30), 45);
        assert_eq!(Difficulty::Normal.ammo(30), 30);
        assert_eq!(Difficulty::Hard.ammo(30), 22);
        assert_eq!(Difficulty::Hard.ammo(1), 1);
        assert_eq!(Difficulty::Easy.ammo(0), 0);
        assert_eq!(
            [Difficulty::Easy, Difficulty::Normal, Difficulty::Hard].map(Difficulty::death_spread),
            [15, 10, 10]
        );
        assert_eq!(
            [Difficulty::Easy, Difficulty::Normal, Difficulty::Hard]
                .map(Difficulty::death_wait_base),
            [150, 75, 0]
        );
    }
}
