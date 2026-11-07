// SPDX-License-Identifier: GPL-3.0-or-later
//! Cor de desenho dos sprites (`docs/re/cores-sprites.md`): a tabela que o original monta
//! em `0x481048` com `0x43becd(tabela, fator)` a partir do `DLS.TBL`, antes de desenhar os
//! objetos com rotina `+0x1c4` (Jake, Garathe, Camaleão, Anticorpo, robô de segurança,
//! gerador de minas e mina flutuante), e o piscar dos jogadores (`0x4081ae`/`0x408129`).

use crate::behaviour::chameleon::ChameleonState;
use crate::behaviour::cos_sin14;
use crate::behaviour::security_robot;
use crate::entity::{Behaviour, Entity};

/// Tabelas de 256 bytes no `DLS.TBL`.
pub const TABLES: usize = 48;
/// Primeira tabela "de sprite": daqui em diante o fator desloca a metade alta (`0x43bf87`).
pub const SPRITE_TABLES: i32 = 0x22;
/// Cor padrão do Jake (`0x42315f`) e do Garathe (`0x4282d3`) com `+0x27c = 0`.
pub const JAKE: i32 = 0x23;
pub const GARATHE: i32 = 0x24;
/// Piscares (`0x4081ae`): dano com reação ou explosão, veneno, choque, efeito do especial.
pub const FLASH_HIT: (i32, i32) = (0x2e, 4);
pub const FLASH_POISON: (i32, i32) = (0x2f, 4);
pub const FLASH_SHOCK: (i32, i32) = (0x25, 10);
pub const FLASH_SPECIAL: (i32, i32) = (0x27, 4);
/// Limiar do esmaecer do Camaleão (`0x419391`): visível, invisível e passo por desenho.
pub const FADE_VISIBLE: i32 = 0xf7;
pub const FADE_INVISIBLE: i32 = 0x91;
pub const FADE_STEP: i32 = 7;
/// Camaleão (`0x47256c`, registros de 0x38): cor normal `+0x1c`, fator `+0x20`, cor de
/// clone `+0x34`.
pub const CHAMELEON: [(i32, i32, i32); 5] = [
    (45, -10, 37),
    (42, 0, 45),
    (37, -60, 42),
    (43, -30, 44),
    (38, -30, 39),
];
/// Ciclo de cor da clonagem (`0x47277c`), um passo a cada 3 tiques; depois do último, a cor
/// normal.
pub const CLONE_CYCLE: [i32; 6] = [38, 39, 40, 41, 40, 39];
/// Anticorpo (`0x4720c0`, registros de 72): `(+0x3c, +0x40)`.
pub const ANTIBODY: [(i32, i32); 4] = [(44, -45), (37, -54), (43, -17), (42, 14)];
/// Pulsos `(período, tabela)`: robô (`0x472a18`, `+0x26/+0x28`), gerador (`0x472ad0`,
/// `+0x2c/+0x30`) e mina flutuante (`0x472bfc`, `+0x3c/+0x40`), com a amplitude de cada um.
pub const ROBOT_PULSE: [(i32, i32); 4] = [(20, 40), (15, 41), (10, 39), (7, 38)];
pub const ROBOT_AMPLITUDE: i32 = 32;
/// Fator do robô morrendo (`+0x278 = −2`, `0x41da6d`).
pub const ROBOT_DYING: i32 = -127;
pub const GENERATOR_PULSE: [(i32, i32); 5] = [(30, 40), (50, 41), (30, 39), (20, 38), (10, 42)];
pub const MINE_PULSE: [(i32, i32); 4] = [(30, 40), (50, 41), (30, 39), (20, 38)];
pub const MINE_AMPLITUDE: i32 = 64;

/// O par do `0x43becd` (tabela 0–47 e fator −127…127) e, só no Camaleão, o limiar a partir
/// do qual os índices de origem ficam transparentes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tint {
    pub table: i32,
    pub shift: i32,
    pub cutoff: Option<i32>,
}

impl Tint {
    pub const fn new(table: i32, shift: i32) -> Self {
        Self {
            table,
            shift,
            cutoff: None,
        }
    }
}

/// As 48 tabelas do `DLS.TBL` (`0x43be18` → `0x493c14`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dls {
    rows: Vec<[u8; 256]>,
}

impl Dls {
    /// Lê as 48 tabelas; `None` se o arquivo for curto.
    pub fn parse(data: &[u8]) -> Option<Self> {
        let rows = data
            .get(..TABLES * 256)?
            .chunks_exact(256)
            .map(|c| c.try_into().expect("256 bytes"))
            .collect();
        Some(Self { rows })
    }

    pub fn row(&self, n: usize) -> Option<&[u8; 256]> {
        self.rows.get(n)
    }

    /// Tabela de desenho (`0x43becd`) composta com a tabela de sombra `shadow`
    /// (`0x496c18`; 0 = identidade): `T[i] = S[A[i']]`, com `i' = i`, salvo na metade alta
    /// das tabelas ≥ 0x22 com fator ≠ 0, onde `i' = 128 + clamp(i − 128 − fator, 0, 127)`.
    /// Tabela fora de 0–47: a tabela não muda (fica a da sombra). Com limiar `c`, zera
    /// `T[c..]` (`0x419421`).
    pub fn table(&self, t: Tint, shadow: usize) -> [u8; 256] {
        let ident: [u8; 256] = std::array::from_fn(|i| i as u8);
        let s = self.rows.get(shadow).unwrap_or(&ident);
        let mut out = *s;
        if let Some(a) = usize::try_from(t.table).ok().and_then(|n| self.rows.get(n)) {
            let shift = t.shift.clamp(-127, 127);
            let shifted = t.table >= SPRITE_TABLES && shift != 0;
            for (i, o) in out.iter_mut().enumerate() {
                let src = if shifted && i >= 128 {
                    128 + (i as i32 - 128 - shift).clamp(0, 127) as usize
                } else {
                    i
                };
                *o = s[usize::from(a[src])];
            }
        }
        if let Some(c) = t.cutoff {
            out[c.clamp(0, 256) as usize..].fill(0);
        }
        out
    }
}

/// Fator do pulso: `A · sen(((n mod P) · (0x10000 / P))) >> 14` (`0x41da74`, `0x41e5b2`,
/// `0x41f245`), seno de 14 bits e deslocamento aritmético.
pub fn pulse(n: i32, period: i32, amplitude: i32) -> i32 {
    let p = period.max(1);
    let angle = n.rem_euclid(p) * (0x10000 / p);
    (amplitude * cos_sin14(angle).1) >> 14
}

/// Piscar de um jogador: `+0x28c` (tiques), `+0x290` (cor) e se a cor do piscar está posta
/// em `+0x27c` neste tique.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Flash {
    ticks: i32,
    color: i32,
    on: bool,
}

impl Flash {
    /// `0x4081ae(obj, cor, n)`: só sem piscar em curso; cor igual à base troca 35↔37 e
    /// 36↔38 (`0x4081c0`). `base` = `+0x280` (0 no solo).
    pub fn start(&mut self, (color, ticks): (i32, i32), base: i32) {
        if self.ticks != 0 {
            return;
        }
        self.color = if color == base {
            match color {
                0x23 => 0x25,
                0x24 => 0x26,
                0x25 => 0x23,
                0x26 => 0x24,
                c => c,
            }
        } else {
            color
        };
        self.ticks = ticks;
    }

    /// `0x408129`, no fim do tique: com piscar em curso, conta um tique e alterna (ímpar →
    /// cor do piscar; envenenado, `t mod 4 < 2`). Sem piscar e sem veneno, volta à base: é o
    /// que o antídoto faz (`+0x27c = +0x280`, `0x40b6e4`); só o veneno acaba com a cor do
    /// piscar posta.
    pub fn tick(&mut self, poisoned: bool) {
        if self.ticks == 0 {
            self.on &= poisoned;
            return;
        }
        self.ticks -= 1;
        self.on = if poisoned {
            self.ticks % 4 < 2
        } else {
            self.ticks % 2 == 1
        };
    }

    /// Cor posta em `+0x27c` neste tique, se não a base.
    pub fn color(&self) -> Option<i32> {
        self.on.then_some(self.color)
    }

    pub fn active(&self) -> bool {
        self.ticks > 0
    }
}

/// Cor de um jogador no solo (`+0x280 = 0`): a do piscar ou a padrão do personagem.
pub fn player(garathe: bool, flash: &Flash) -> Tint {
    Tint::new(
        flash
            .color()
            .unwrap_or(if garathe { GARATHE } else { JAKE }),
        0,
    )
}

/// Cor de um jogador com cor-base `colour` (`+0x280`; 0 = a padrão do personagem, como no
/// solo): a do piscar ou a base (`0x42315f`/`0x4282d3`).
pub fn player_coloured(garathe: bool, colour: i32, flash: &Flash) -> Tint {
    if colour == 0 {
        return player(garathe, flash);
    }
    Tint::new(flash.color().unwrap_or(colour), 0)
}

/// Cor do Camaleão (`0x419391`).
pub fn chameleon(s: &ChameleonState) -> Tint {
    let (normal, shift, clone) = CHAMELEON[s.variant.min(CHAMELEON.len() - 1)];
    if s.clone_of != 0 {
        return Tint::new(clone, shift);
    }
    // ciclo da clonagem: passo de 3 tiques pela contagem restante; 6 = cor normal
    let table = match s.cloning {
        Some(n) => CLONE_CYCLE
            .get((-n.div_euclid(3)).rem_euclid(7) as usize)
            .copied()
            .unwrap_or(normal),
        None => normal,
    };
    let cutoff = (FADE_VISIBLE - FADE_STEP * s.fade).clamp(FADE_INVISIBLE, FADE_VISIBLE);
    Tint {
        table,
        shift,
        cutoff: Some(cutoff),
    }
}

/// Cor de desenho da entidade (a rotina `+0x1c4` do tipo), ou `None` para desenhar sem
/// tabela. `index` faz as vezes do endereço do objeto na fase do pulso.
pub fn entity(e: &Entity, index: usize, tick: u32) -> Option<Tint> {
    let n = (tick as i32).wrapping_add(index as i32);
    match &e.behaviour {
        Behaviour::Chameleon(s) => Some(chameleon(s)),
        // personagens da IA: a rotina do jogador (`+0x27c` 0 → 35 / 36, com os piscares)
        Behaviour::AiPlayer(a) => Some(a.body.tint()),
        Behaviour::Corpse(t) => Some(Tint::new(*t, 0)),
        Behaviour::Flyer(f) if !f.manta => {
            let (t, b) = ANTIBODY[f.variant.min(ANTIBODY.len() - 1)];
            Some(Tint::new(t, b))
        }
        Behaviour::SecurityRobot(s) => {
            let (p, t) = ROBOT_PULSE[s.variant.min(ROBOT_PULSE.len() - 1)];
            let b = if s.orient == security_robot::DYING {
                ROBOT_DYING
            } else {
                pulse((tick as i32).wrapping_add(s.phase), p, ROBOT_AMPLITUDE)
            };
            Some(Tint::new(t, b))
        }
        Behaviour::MineGenerator(s) => {
            let (p, t) = GENERATOR_PULSE[s.variant.min(GENERATOR_PULSE.len() - 1)];
            Some(Tint::new(t, pulse(n, p, MINE_AMPLITUDE)))
        }
        Behaviour::Mine(s) => {
            let (p, t) = MINE_PULSE[s.variant.min(MINE_PULSE.len() - 1)];
            Some(Tint::new(t, pulse(n, p, MINE_AMPLITUDE)))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tabelas de teste: 0 identidade; `n` soma `n` (mod 256); a 40 (≥ 0x22) é a rampa
    /// identidade com a metade baixa invertida.
    fn dls() -> Dls {
        let mut rows: Vec<[u8; 256]> = (0..TABLES)
            .map(|n| std::array::from_fn(|i| (i + n) as u8))
            .collect();
        rows[1] = std::array::from_fn(|i| i as u8);
        rows[40] = std::array::from_fn(|i| if i < 128 { 127 - i as u8 } else { i as u8 });
        Dls { rows }
    }

    #[test]
    fn parse_needs_48_tables() {
        assert!(Dls::parse(&[0; 48 * 256 - 1]).is_none());
        let d = Dls::parse(&[7; 48 * 256 + 3]).unwrap();
        assert_eq!(d.row(47).unwrap()[255], 7);
        assert!(d.row(48).is_none());
    }

    #[test]
    fn plain_table_maps_every_index() {
        let t = dls().table(Tint::new(3, 0), 0);
        assert_eq!(t[0], 3);
        assert_eq!(t[200], 203);
        // fator ignorado abaixo de 0x22
        assert_eq!(dls().table(Tint::new(3, 50), 0), t);
    }

    #[test]
    fn composes_with_the_shadow() {
        // S[A[i]]: A soma 2, S soma 5
        let t = dls().table(Tint::new(2, 0), 5);
        assert_eq!(t[10], 17);
    }

    #[test]
    fn positive_shift_lightens_the_high_half() {
        let t = dls().table(Tint::new(40, 10), 0);
        assert_eq!(t[5], 122, "metade baixa intacta");
        assert_eq!(t[128], 128);
        assert_eq!(t[137], 128, "repete A[128] nos 10 primeiros");
        assert_eq!(t[138], 128);
        assert_eq!(t[139], 129);
        assert_eq!(t[255], 245);
    }

    #[test]
    fn negative_shift_darkens_and_clamps() {
        let t = dls().table(Tint::new(40, -10), 0);
        assert_eq!(t[128], 138);
        assert_eq!(t[245], 255);
        assert_eq!(t[255], 255, "repete A[255] no fim");
        let dark = dls().table(Tint::new(40, -1000), 0);
        assert!(
            dark[128..].iter().all(|&v| v == 255),
            "fator limitado a −127"
        );
    }

    #[test]
    fn table_out_of_range_keeps_the_shadow() {
        let id: [u8; 256] = std::array::from_fn(|i| i as u8);
        assert_eq!(dls().table(Tint::new(48, 0), 0), id);
        assert_eq!(dls().table(Tint::new(-1, 0), 0), id);
    }

    #[test]
    fn cutoff_zeroes_the_tail() {
        let mut t = Tint::new(40, 0);
        t.cutoff = Some(0x91);
        let m = dls().table(t, 0);
        assert_eq!(m[0x90], 0x90);
        assert!(m[0x91..].iter().all(|&v| v == 0));
    }

    #[test]
    fn pulse_follows_integer_formula() {
        // P = 20: 0x10000 / 20 = 3276; n = 5 → ângulo 16380 (quase 90°)
        assert_eq!(pulse(5, 20, 32), 32);
        // P = 7: 0x10000 / 7 = 9362; n = 2 → 18724 (≈ 103°): 32 · 15973 >> 14
        assert_eq!(pulse(2, 7, 32), 31);
        assert_eq!(pulse(0, 20, 32), 0);
        assert_eq!(
            pulse(15, 20, 32),
            -32,
            "deslocamento aritmético arredonda para baixo"
        );
        assert_eq!(pulse(25, 20, 32), pulse(5, 20, 32));
        assert_eq!(pulse(-15, 20, 32), pulse(5, 20, 32));
        for n in 0..200 {
            assert!((-64..=64).contains(&pulse(n, 30, 64)));
        }
    }

    #[test]
    fn flash_alternates_and_does_not_restart() {
        let mut f = Flash::default();
        f.start(FLASH_HIT, 0);
        let mut seen = vec![];
        for _ in 0..6 {
            f.tick(false);
            seen.push(f.color());
        }
        let h = Some(0x2e);
        assert_eq!(seen, vec![h, None, h, None, None, None]);
        f.start(FLASH_SHOCK, 0);
        f.tick(false);
        f.start(FLASH_HIT, 0);
        f.tick(false);
        f.tick(false);
        assert!(f.active());
        assert_eq!(f.color(), Some(0x25), "o piscar em curso não é trocado");
    }

    #[test]
    fn poisoned_flash_holds_two_ticks() {
        let mut f = Flash::default();
        f.start(FLASH_POISON, 0);
        let v: Vec<_> = (0..4)
            .map(|_| {
                f.tick(true);
                f.color().is_some()
            })
            .collect();
        // t = 3, 2, 1, 0 → 3 % 4 = 3, 2: base; 1, 0: piscar
        assert_eq!(v, vec![false, false, true, true]);
        f.tick(true);
        assert!(
            f.color().is_some(),
            "acabado, a última cor fica até a próxima troca"
        );
        f.tick(false);
        assert!(f.color().is_none(), "curado, volta à base");
    }

    #[test]
    fn flash_equal_to_base_swaps() {
        let mut f = Flash::default();
        f.start((0x25, 2), 0x25);
        f.tick(false);
        assert_eq!(f.color(), Some(0x23));
    }

    #[test]
    fn player_default_colors() {
        let f = Flash::default();
        assert_eq!(player(false, &f), Tint::new(35, 0));
        assert_eq!(player(true, &f), Tint::new(36, 0));
    }

    #[test]
    fn chameleon_color_clone_cycle_and_fade() {
        let mut s = ChameleonState::new(2);
        let t = chameleon(&s);
        assert_eq!((t.table, t.shift, t.cutoff), (37, -60, Some(0xf7)));
        s.fade = 3;
        assert_eq!(chameleon(&s).cutoff, Some(0xf7 - 21));
        s.fade = 15;
        assert_eq!(chameleon(&s).cutoff, Some(0x91));
        // ciclo: passos de 3 tiques pela contagem restante, 7 estados (6 cores e a normal)
        let cyc: Vec<i32> = (0..21)
            .rev()
            .step_by(3)
            .map(|n| {
                s.cloning = Some(n);
                chameleon(&s).table
            })
            .collect();
        let mut sorted = cyc.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, vec![37, 38, 39, 39, 40, 40, 41]);
        s.cloning = None;
        s.clone_of = 1;
        let c = chameleon(&s);
        assert_eq!((c.table, c.shift, c.cutoff), (42, -60, None));
    }
}
