// SPDX-License-Identifier: GPL-3.0-or-later
//! Gerador de minas (tipo 103, `0x0041dcc8`/`0x0041de45`; `docs/re/gerador-minas.md`): caixa
//! fixa que se abre e solta minas flutuantes (tipo 102) quando o Jake está ao alcance.

use super::enemy::{self, Clip, Label, Spawn, Wound};
use super::{Blast, Ctx, Out};
use crate::entity::{Behaviour, Entity, MINE};
use crate::physics::Rect;

/// Variante (tabela `0x00472ad0`, registros de 0x38 bytes).
#[derive(Debug, Clone, Copy)]
pub struct Variant {
    pub name: &'static str,
    /// Alcance centro a centro (`+0x14`).
    pub range: i32,
    /// Vida padrão (`+0x18`).
    pub health: i32,
    /// Exige linha de visão (`+0x1c`).
    pub sight: bool,
    /// Intervalo entre levas (`+0x20`), estoque (`+0x24`), minas por leva (`+0x28`).
    pub interval: i32,
    pub stock: i32,
    pub per_wave: usize,
    /// % do dano recebido aplicado (`+0x34`).
    pub applied: i32,
}

pub const VARIANTS: [Variant; 5] = [
    Variant {
        name: "Alpha",
        range: 150,
        health: 15,
        sight: false,
        interval: 20,
        stock: 50,
        per_wave: 1,
        applied: 100,
    },
    Variant {
        name: "Beta",
        range: 200,
        health: 20,
        sight: false,
        interval: 30,
        stock: 40,
        per_wave: 3,
        applied: 70,
    },
    Variant {
        name: "Gamma",
        range: 250,
        health: 25,
        sight: false,
        interval: 40,
        stock: 100,
        per_wave: 4,
        applied: 60,
    },
    Variant {
        name: "Delta",
        range: 300,
        health: 20,
        sight: true,
        interval: 50,
        stock: 100,
        per_wave: 2,
        applied: 40,
    },
    Variant {
        name: "All",
        range: 200,
        health: 20,
        sight: false,
        interval: 60,
        stock: 100,
        per_wave: 5,
        applied: 50,
    },
];

/// Rótulos do `OHG.ANM`: 1 `ohg00` (fechado), 2 `ohg01` (abrindo).
pub const LABELS: [Label; 2] = [(1, 0, 1), (2, 1, 8)];
/// `SOUND.TBL` tipo 103: quadro 2 (abrindo) → 3133, prioridade 7.
pub const SOUNDS: [(usize, u16, u16); 1] = [(2, 3133, 7)];
/// Vagas de mina (`+0x280..+0x28c`).
pub const SLOTS: usize = 4;
/// Mina nasce em (x + 15, y + 8) do gerador, com escala 25 (`0x41e497`).
pub const MINE_OFFSET: (i32, i32) = (15, 8);
pub const MINE_SIZE: i32 = 25;
/// Linha `(103, 0)`: raio 48, empurrão 15, dano de área 10.
pub const BLAST_RADIUS: i32 = 48;
pub const BLAST_PUSH: i32 = 15;
pub const BLAST_DAMAGE: i32 = 10;

pub const CLOSED: i8 = 1;
pub const OPENING: i8 = 2;
pub const CLOSING: i8 = -1;

#[derive(Debug, Clone)]
pub struct MineGeneratorState {
    pub variant: usize,
    pub state: i8,
    pub clip: Clip,
    /// Tiques até a próxima leva (`+0x274`) e estoque de levas (`+0x270`).
    pub countdown: i32,
    pub stock: i32,
    /// Minas vivas soltas por este gerador (contadas pela cena antes do tique).
    pub live: usize,
    /// Marca posta nas minas dele (índice + 1 na lista de objetos).
    pub tag: u32,
    pub wound: Wound,
    pub last_frame: Option<usize>,
}

impl MineGeneratorState {
    /// `values[0]` = variante (negativa → 0; ≥ 5 → 4).
    pub fn new(value: i32) -> Self {
        let variant = value.clamp(0, 4) as usize;
        let v = &VARIANTS[variant];
        Self {
            variant,
            state: CLOSED,
            clip: Clip::new(1, false),
            countdown: v.interval,
            stock: v.stock,
            live: 0,
            tag: 0,
            wound: Wound::default(),
            last_frame: None,
        }
    }

    pub fn variant(&self) -> &'static Variant {
        &VARIANTS[self.variant]
    }

    fn set(&mut self, state: i8) {
        self.state = state;
        self.clip = match state {
            CLOSING => Clip::new(2, true),
            s => Clip::new(s as u16, false),
        };
    }
}

/// Antes do tique: cada gerador ganha a sua marca e conta as minas vivas dela
/// (`0x41e62d` esquece as vagas cujas minas morreram). A marca é o índice + 1 na lista de
/// entidades: vale porque a cena só acrescenta entidades no fim e nunca as remove nem
/// reordena (as mortas ficam com `alive = false`); se isso mudar, a marca tem de virar um id
/// estável guardado no gerador.
pub fn sync(entities: &mut [Entity]) {
    let owners: Vec<u32> = entities
        .iter()
        .filter_map(|e| match &e.behaviour {
            Behaviour::Mine(m) if e.alive && e.health > 0 && m.owner != 0 => Some(m.owner),
            _ => None,
        })
        .collect();
    for (i, e) in entities.iter_mut().enumerate() {
        if let Behaviour::MineGenerator(st) = &mut e.behaviour {
            st.tag = i as u32 + 1;
            st.live = owners.iter().filter(|&&o| o == st.tag).count();
        }
    }
}

/// Mina nova de um gerador: tipo 102, sólida e presente em qualquer modo, vida da variante.
pub fn mine(at: (i32, i32), variant: usize, layer: i32, owner: u32) -> Entity {
    let mut st = super::mine::MineState::new(variant as i32);
    st.owner = owner;
    let health = st.variant().health;
    Entity {
        kind: MINE,
        x: at.0,
        y: at.1,
        flags: 0x80811,
        layer,
        id: 0,
        values: vec![variant as i32],
        triggers: vec![],
        objective: 0,
        health,
        armor: 0,
        boxes: vec![Rect {
            x: 0,
            y: 0,
            w: 17,
            h: 17,
        }],
        solid_shapes: true,
        on: false,
        alive: true,
        sprite: None,
        behaviour: Behaviour::Mine(st),
    }
}

/// Alvo (`0x41e0ed`): o Jake na camada, ao alcance; visão só no Delta.
fn sees(e: &Entity, st: &MineGeneratorState, ctx: &Ctx) -> bool {
    let (Some(me), Some(j)) = (e.bounds(), enemy::jake_target(ctx, e.layer)) else {
        return false;
    };
    let (c, jc) = (enemy::center(&me), enemy::center(&j));
    let v = st.variant();
    super::mine::distance(c, jc) <= v.range
        && (!v.sight || enemy::line_of_sight(ctx, c, jc, &[me, j]))
}

pub fn tick(e: &mut Entity, ctx: &mut Ctx, out: &mut Out) {
    let center = e.center();
    let seen = match &e.behaviour {
        Behaviour::MineGenerator(st) => sees(e, st, ctx),
        _ => return,
    };
    let Behaviour::MineGenerator(st) = &mut e.behaviour else {
        return;
    };
    // dano (`0x41dfaf`): dificuldade, mínimo 1, vezes o % da variante
    let w = st.wound.take();
    if !w.is_empty() {
        e.health -= ctx.difficulty.damage(w.pending.amount).max(1) * st.variant().applied / 100;
        if e.health <= 0 {
            out.blasts.push(Blast {
                center,
                layer: e.layer,
                radius: BLAST_RADIUS,
                push: BLAST_PUSH,
                damage: BLAST_DAMAGE,
            });
            e.alive = false;
            e.on = true;
            return;
        }
    }
    // próxima leva
    let mut opened = false;
    if st.state != OPENING && seen && st.stock > 0 && st.live < SLOTS {
        st.countdown -= 1;
        if st.countdown <= 0 {
            let iv = st.variant().interval.max(1);
            st.stock -= 1;
            st.countdown = iv + ctx.rng.below(iv as u32) as i32 / 2 - iv / 4;
            st.set(OPENING);
            opened = true;
        }
    }
    let (_, _, len) = enemy::label(&LABELS, st.clip.label);
    if !opened && st.clip.advance(len) {
        // fim da animação (`0x41e33b`)
        if st.state == OPENING {
            let n = st.variant().per_wave.min(SLOTS.saturating_sub(st.live));
            for _ in 0..n {
                let variant = if st.variant < 4 {
                    st.variant
                } else {
                    ctx.rng.below(4) as usize
                };
                out.enemy_spawns.push(Spawn::Mine {
                    at: (e.x + MINE_OFFSET.0, e.y + MINE_OFFSET.1),
                    variant,
                    layer: e.layer,
                    owner: st.tag,
                });
            }
            st.live += n;
            st.set(CLOSING);
        } else {
            st.set(CLOSED);
        }
    }
    let Behaviour::MineGenerator(st) = &e.behaviour else {
        return;
    };
    let clip = st.clip;
    let frame = enemy::show(e, &LABELS, &clip);
    let Behaviour::MineGenerator(st) = &mut e.behaviour else {
        return;
    };
    enemy::sound_frame(&SOUNDS, frame, &mut st.last_frame, center, e.layer, out);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviour::test_support::{ctx, ctx_with_jake, logic};
    use crate::entity::MINE_GENERATOR;

    fn generator(x: i32, y: i32, variant: i32) -> Entity {
        let mut e = logic(MINE_GENERATOR, x, y, &[variant]);
        e.flags = 0x80809;
        e.health = 35;
        e.solid_shapes = true;
        e.behaviour = Behaviour::MineGenerator(MineGeneratorState::new(variant));
        e
    }

    fn mines(out: &Out) -> usize {
        out.enemy_spawns
            .iter()
            .filter(|s| matches!(s, Spawn::Mine { .. }))
            .count()
    }

    #[test]
    fn generator_opens_and_releases_mines_up_to_four() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(150, 0);
        let mut c = ctx(&j, &w, &on);
        // Beta: 3 por leva, intervalo 30
        let mut g = generator(100, 0, 1);
        let mut out = Out::default();
        let mut t = 0;
        while mines(&out) == 0 && t < 200 {
            tick(&mut g, &mut c, &mut out);
            t += 1;
        }
        assert_eq!(t, 30 + 8, "intervalo e abertura");
        assert_eq!(mines(&out), 3);
        let Spawn::Mine { at, owner, .. } = out.enemy_spawns[0].clone() else {
            panic!()
        };
        assert_eq!(at, (115, 8));
        assert_eq!(owner, 0, "sem sync, marca 0");
        let Behaviour::MineGenerator(s) = &g.behaviour else {
            panic!()
        };
        assert_eq!(s.stock, 39, "uma leva gasta um do estoque");
        // com as 3 vivas, a próxima leva só solta 1 (4 vagas)
        for _ in 0..200 {
            tick(&mut g, &mut c, &mut out);
        }
        assert_eq!(mines(&out), 4);
        // sync esquece as mortas
        let mut ents = vec![g, mine((0, 0), 1, 0, 1), mine((0, 0), 1, 0, 1)];
        ents[2].alive = false;
        sync(&mut ents);
        let Behaviour::MineGenerator(s) = &ents[0].behaviour else {
            panic!()
        };
        assert_eq!((s.tag, s.live), (1, 1));
    }

    #[test]
    fn generator_damage_percent_and_death() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(5000, 0);
        let mut c = ctx(&j, &w, &on);
        let mut g = generator(100, 0, 3); // Delta: 40 %
        let p = crate::damage::Pending {
            amount: 10,
            ..Default::default()
        };
        let mut out = Out::default();
        enemy::hurt(&mut g, &p, true);
        tick(&mut g, &mut c, &mut out);
        assert_eq!(g.health, 31);
        g.health = 2;
        enemy::hurt(&mut g, &p, true);
        tick(&mut g, &mut c, &mut out);
        assert!(!g.alive);
        assert!(enemy::killed(&g));
        assert_eq!(out.blasts.len(), 1);
        assert_eq!(out.blasts[0].radius, BLAST_RADIUS);
    }

    #[test]
    fn only_delta_needs_line_of_sight() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(250, 0);
        let walled = crate::World::new(
            w.solids()
                .iter()
                .copied()
                .chain([Rect {
                    x: 180,
                    y: -50,
                    w: 20,
                    h: 100,
                }])
                .collect(),
        );
        let c = ctx(&j, &walled, &on);
        let alpha = generator(100, 0, 1);
        let delta = generator(100, 0, 3);
        let s = |e: &Entity| match &e.behaviour {
            Behaviour::MineGenerator(s) => s.clone(),
            _ => panic!(),
        };
        assert!(sees(&alpha, &s(&alpha), &c));
        assert!(!sees(&delta, &s(&delta), &c));
    }
}
