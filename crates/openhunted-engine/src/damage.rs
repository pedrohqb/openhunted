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
    /// Natureza (`+0x252`): veneno (1), explosão (2), fogo (4), elétrico (0x20).
    pub poison: bool,
    pub explosion: bool,
    pub electric: bool,
    /// Fogo (`0x4`): só o barril reage a ela (explode).
    pub fire: bool,
    /// Empurrão (`+0x244`/`+0x248`), em px/tique.
    pub push: (i32, i32),
    /// Reações (`+0x250`): derrubada (4), empurrão (2).
    pub knockdown: bool,
    pub shove: bool,
    /// Tranco (`+0x250 = 1`): só o Garathe reage (estado 59).
    pub jolt: bool,
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
        self.fire |= o.fire;
        self.push.0 = self.push.0.saturating_add(o.push.0);
        self.push.1 = self.push.1.saturating_add(o.push.1);
        self.knockdown |= o.knockdown;
        self.shove |= o.shove;
        self.jolt |= o.jolt;
    }
}

pub mod table;

/// Uma linha do `DAMAGE.TBL` (campos em `docs/re/dano-vida.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row {
    pub attacker: i32,
    /// 0 qualquer (com ou sem vítima); 1 só jogador (tipo ≤ 2); > 1 tipo exato; < 0 só sem
    /// vítima.
    pub victim: i32,
    /// 0 = qualquer; senão igual à variante pedida.
    pub variant: i32,
    /// `+0x252 |=`: 1 veneno, 2 explosão, 4 fogo, 0x20 elétrico (8 e 0x10 sem efeito aqui).
    pub nature: i32,
    /// `+0x250`: 1 tranco, 2 empurrão, 4 derrubada.
    pub reaction: i32,
    pub damage: i32,
    /// Empurrão fixo (vx com sinal pelo lado, vy).
    pub push: (i32, i32),
    /// Módulo do empurrão da derrubada (reescala o vetor pela direção).
    pub magnitude: i32,
    /// Efeito (rótulo do `oex.anm`; −1 nenhum) e raio.
    pub effect: i32,
    pub radius: i32,
    pub area_push: i32,
    pub area_damage: i32,
    pub bits: i32,
    /// Som posicional (0 = nenhum).
    pub sound: i32,
}

impl Row {
    const fn from_raw(r: &[i32; 15]) -> Row {
        Row {
            attacker: r[0],
            victim: r[1],
            variant: r[2],
            nature: r[3],
            reaction: r[4],
            damage: r[5],
            push: (r[6], r[7]),
            magnitude: r[8],
            effect: r[9],
            radius: r[10],
            area_push: r[11],
            area_damage: r[12],
            bits: r[13],
            sound: r[14],
        }
    }

    /// A linha casa com o atacante, a vítima (tipo, ou nenhuma) e a variante
    /// (`0x0043d629`, laço `0x43d648`).
    fn matches(&self, attacker: u32, victim: Option<u32>, variant: u32) -> bool {
        let victim_ok = match (self.victim, victim) {
            (0, _) => true,
            (1, Some(t)) => t <= 2,
            (v, Some(t)) if v > 1 => t == v as u32,
            (v, None) => v < 0,
            _ => false,
        };
        self.attacker == attacker as i32
            && victim_ok
            && (self.variant == 0 || self.variant == variant as i32)
    }

    /// Dano pendente que a linha põe na vítima (`0x43d6c5…0x43d8d2`). `g` dá a velocidade
    /// do atacante (`+0x15c/+0x160`; os projéteis chegam ao contato com ela zerada) e o vetor
    /// centro do atacante → centro da vítima, em px.
    pub fn pending(&self, g: Geometry) -> Pending {
        let mut p = Pending {
            amount: self.damage,
            poison: self.nature & 1 != 0,
            explosion: self.nature & 2 != 0,
            fire: self.nature & 4 != 0,
            electric: self.nature & 0x20 != 0,
            ..Default::default()
        };
        let toward_sign = if g.toward.0 < 0 { -1 } else { 1 };
        match self.reaction {
            4 => {
                p.knockdown = true;
                p.push = if self.push != (0, 0) {
                    // lado pela velocidade do atacante; parado, pelos centros
                    let s = match g.vel.0 {
                        v if v < 0 => -1,
                        0 => toward_sign,
                        _ => 1,
                    };
                    (self.push.0 * s, self.push.1)
                } else {
                    g.toward
                };
                if self.magnitude != 0 {
                    let a = crate::behaviour::angle16(p.push.0, p.push.1);
                    let (c, s) = crate::behaviour::cos_sin14(a);
                    p.push = ((c * self.magnitude) >> 14, (s * self.magnitude) >> 14);
                }
            }
            2 => {
                p.shove = true;
                p.push = if self.push != (0, 0) {
                    (self.push.0 * toward_sign, self.push.1)
                } else {
                    g.vel
                };
            }
            1 => p.jolt = true,
            _ => {}
        }
        p
    }

    /// Som posicional da linha.
    pub fn sound(&self) -> Option<u16> {
        u16::try_from(self.sound).ok().filter(|&s| s != 0)
    }
}

/// Geometria de um acerto para `Row::pending`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Geometry {
    /// Velocidade do atacante no contato.
    pub vel: (i32, i32),
    /// Centro da vítima menos o centro do atacante (px).
    pub toward: (i32, i32),
}

/// Primeira linha do `DAMAGE.TBL` para `(atacante, vítima, variante)` (`0x0043d629`);
/// `victim` é o tipo do objeto atingido (`None` = sem vítima, ex.: a parede).
pub fn row(attacker: u32, victim: Option<u32>, variant: u32) -> Option<Row> {
    table::RAW
        .iter()
        .map(Row::from_raw)
        .find(|r| r.matches(attacker, victim, variant))
}

/// Golpe corpo a corpo armado no tique (`0x43d629(atacante, vítima, variante)` para cada
/// objeto na caixa): a linha sai por vítima.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Strike {
    /// Tipo do atacante (1 Jake, 2 Garathe).
    pub attacker: u32,
    pub variant: u32,
}

impl Strike {
    /// Linha do golpe contra um objeto do tipo `victim`.
    pub fn row(&self, victim: u32) -> Option<Row> {
        row(self.attacker, Some(victim), self.variant)
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

    fn hit(attacker: u32, victim: u32, variant: u32, toward: (i32, i32)) -> (Pending, Option<u16>) {
        let r = row(attacker, Some(victim), variant).expect("linha");
        (
            r.pending(Geometry {
                vel: (0, 0),
                toward,
            }),
            r.sound(),
        )
    }

    #[test]
    fn garathe_rows_depend_on_the_victim() {
        // barril (28): todas as variantes corpo a corpo o explodem (linhas 14…70)
        for v in 0..=7 {
            let (p, s) = hit(2, 28, v, (10, 0));
            assert!(p.explosion && p.knockdown && p.amount == 5, "variante {v}");
            assert_eq!(s, Some(5062));
        }
        // mina saltadora (11), camaleão (4), manta (89), anticorpo (105), formiga (109)
        for t in [11, 4, 89, 105, 109] {
            let (p, s) = hit(2, t, 0, (10, 0));
            assert_eq!(
                (p.amount, p.shove, p.push, s),
                (8, true, (2, 0), Some(3038))
            );
            assert!(!p.explosion && !p.knockdown);
        }
        // os demais: garras 15 com empurrão e 5062; contra o jogador, 3038
        assert_eq!(hit(2, 7, 0, (-10, 0)).0.push, (-2, 0), "lado pelos centros");
        assert_eq!(hit(2, 7, 0, (10, 0)).1, Some(5062));
        assert_eq!(hit(2, 1, 0, (10, 0)).1, Some(3038));
        // clava 2: derrubada (20, 10); chicote 7: (20, −20)
        let (p, _) = hit(2, 7, 2, (-5, 0));
        assert_eq!((p.amount, p.knockdown, p.push), (15, true, (-20, 10)));
        assert_eq!(hit(2, 7, 7, (5, 0)).0.push, (20, -20));
    }

    #[test]
    fn jake_punch_rows_change_only_the_sound() {
        let (p, s) = hit(1, 11, 0, (5, 0));
        assert_eq!(
            (p.amount, p.shove, p.push, s),
            (8, true, (2, 0), Some(3038))
        );
        let (p, s) = hit(1, 28, 0, (5, 0));
        assert_eq!(
            (p.amount, p.explosion, s),
            (8, false, Some(5062)),
            "barril sem linha 0"
        );
        assert_eq!(hit(1, 7, 0, (5, 0)).1, Some(5062));
    }

    #[test]
    fn projectile_rows_for_the_garathe_weapons() {
        // chicote (108): em qualquer vítima, explosão e derrubada de módulo 20 na direção
        let (p, _) = hit(108, 7, 0, (30, 0));
        assert_eq!(
            (p.amount, p.explosion, p.knockdown, p.push),
            (20, true, true, (20, 0))
        );
        let (p, _) = hit(108, 7, 0, (0, 9));
        assert_eq!(p.push, (0, 20));
        // faca (25): no barril explode; nos demais 12 com empurrão (0, 0)
        let (p, s) = hit(25, 28, 0, (5, 0));
        assert_eq!(
            (p.amount, p.explosion, p.knockdown, s),
            (5, true, true, Some(5056))
        );
        let (p, s) = hit(25, 7, 0, (5, 0));
        assert_eq!((p.amount, p.shove, p.push, s), (12, true, (0, 0), None));
        // sem vítima: só as linhas 0 e < 0
        assert_eq!(row(25, None, 0).map(|r| r.damage), Some(12));
        assert_eq!(row(3, None, 0), None, "a linha (3, 1) exige o jogador");
    }

    #[test]
    fn knockdown_side_follows_the_attacker_velocity() {
        let r = row(2, Some(7), 1).unwrap();
        let p = |vel, toward| r.pending(Geometry { vel, toward }).push;
        assert_eq!(p((-3, 0), (10, 0)), (-20, 30), "vx < 0 nega");
        assert_eq!(p((3, 0), (-10, 0)), (20, 30), "vx > 0 mantém");
        assert_eq!(p((0, 0), (-10, 0)), (-20, 30), "parado: pelos centros");
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
