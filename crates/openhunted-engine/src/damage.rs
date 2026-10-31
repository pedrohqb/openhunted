// SPDX-License-Identifier: GPL-3.0-or-later
//! Vida, armadura e aplicação do dano (`0x00407862`; `docs/re/dano-vida.md`).

/// Vida (`+0x258`) e armadura (`+0x25c`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Health {
    pub hp: i32,
    pub armor: i32,
}

impl Health {
    pub fn is_dead(&self) -> bool {
        self.hp <= 0
    }
}

/// Dano pendente do tique (`+0x24c` e vizinhos), somado por quem bate.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Pending {
    pub amount: i32,
    /// Natureza (`+0x252`): veneno (1), explosão (2), elétrico (0x20).
    pub poison: bool,
    pub explosion: bool,
    pub electric: bool,
    /// Empurrão (`+0x244`/`+0x248`), em px/tique.
    pub push: (i32, i32),
    /// Reações (`+0x250`): derrubada (4), empurrão (2).
    pub knockdown: bool,
    pub shove: bool,
}

impl Pending {
    pub fn is_empty(&self) -> bool {
        *self == Pending::default()
    }

    /// Soma outro dano pendente (como os campos do original, que acumulam no tique).
    pub fn add(&mut self, o: &Pending) {
        self.amount = self.amount.saturating_add(o.amount);
        self.poison |= o.poison;
        self.explosion |= o.explosion;
        self.electric |= o.electric;
        self.push.0 = self.push.0.saturating_add(o.push.0);
        self.push.1 = self.push.1.saturating_add(o.push.1);
        self.knockdown |= o.knockdown;
        self.shove |= o.shove;
    }
}

/// Aplica o dano pendente (`0x00407862`) e devolve a vida resultante. Fora do veneno, a
/// armadura absorve metade (arredondada para cima) e impede o golpe letal enquanto durar;
/// a exceção do original (dano 1 em tique par com vida ≠ 1 passa direto) está incluída.
pub fn apply(h: &mut Health, p: &Pending, tick: u32) -> i32 {
    if p.amount <= 0 || h.is_dead() {
        return h.hp;
    }
    let mut dmg = p.amount;
    let skip_armor = dmg == 1 && tick % 2 == 0 && h.hp != 1;
    if !p.poison && h.armor > 0 && !skip_armor {
        let a = (dmg + 1) / 2;
        h.armor -= a;
        dmg -= a;
        if dmg >= h.hp {
            h.armor -= dmg - (h.hp - 1);
            dmg = h.hp - 1;
        }
        if h.armor < 0 {
            dmg += -h.armor;
            h.armor = 0;
        }
    }
    h.hp = (h.hp - dmg).max(0);
    h.hp
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dmg(amount: i32) -> Pending {
        Pending {
            amount,
            ..Default::default()
        }
    }

    #[test]
    fn armor_absorbs_half_and_blocks_lethal_blow() {
        // dano 30 contra vida 10, armadura 10: a = 15 → armadura −5, dano 15 ≥ vida →
        // excedente 15 − 9 = 6 sai da armadura (−11), dano 9; armadura negativa devolve 11
        // ao dano → 20; vida 10 − 20 → 0. Ou seja: sem armadura suficiente, o golpe mata.
        let mut h = Health { hp: 10, armor: 10 };
        assert_eq!(apply(&mut h, &dmg(30), 1), 0);
        assert_eq!(h, Health { hp: 0, armor: 0 });
        // armadura farta: dano 30 contra vida 10, armadura 100 → a = 15, dano 15 ≥ 10 →
        // armadura −6, dano 9 → vida 1, armadura 79
        let mut h = Health { hp: 10, armor: 100 };
        assert_eq!(apply(&mut h, &dmg(30), 1), 1);
        assert_eq!(h, Health { hp: 1, armor: 79 });
        // metade arredondada para cima: dano 5, armadura 10 → armadura 7, vida −2
        let mut h = Health { hp: 50, armor: 10 };
        apply(&mut h, &dmg(5), 1);
        assert_eq!(h, Health { hp: 48, armor: 7 });
    }

    #[test]
    fn poison_ignores_armor_and_dead_stay_dead() {
        let mut h = Health { hp: 50, armor: 10 };
        let p = Pending {
            amount: 4,
            poison: true,
            ..Default::default()
        };
        apply(&mut h, &p, 1);
        assert_eq!(h, Health { hp: 46, armor: 10 });
        let mut h = Health { hp: 0, armor: 10 };
        assert_eq!(apply(&mut h, &dmg(4), 1), 0);
        assert_eq!(h.armor, 10);
    }

    #[test]
    fn single_point_on_even_tick_skips_armor() {
        let mut h = Health { hp: 50, armor: 10 };
        apply(&mut h, &dmg(1), 2);
        assert_eq!(h, Health { hp: 49, armor: 10 });
        apply(&mut h, &dmg(1), 3);
        assert_eq!(h, Health { hp: 49, armor: 9 });
    }
}
