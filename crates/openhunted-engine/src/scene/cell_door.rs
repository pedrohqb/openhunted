// SPDX-License-Identifier: GPL-3.0-or-later
//! Portas de célula (M7b; `docs/re/itens-campanha.md`): células de cenário com
//! `(índice / 10) % 100 = 38` (`0x0040ecd7`), que abrem e fecham animando o rótulo da célula
//! (`0x0040ed62`) e trocam a colisão `C%d%c%02d%c1.BRD` (fechada) por `…0.BRD` (aberta)
//! (`0x0040eed4`). O Jake abre empurrando numa zona PR 0x20, com uma das chaves pedidas
//! (`0x0040f040`).

use super::Scene;
use crate::behaviour::Event;
use crate::physics::Rect;
use crate::sound::{Cue, PRIORITY_PICKUP};

pub const CLOSED: u8 = 1;
pub const CLOSING: u8 = 2;
pub const OPENING: u8 = 3;
pub const OPEN: u8 = 4;
/// `+0x4c = 0` → 55 tiques até fechar.
pub const DEFAULT_CLOSE: i32 = 55;
/// Falas do Jake: porta trancada (3715 com 3714 sem a chave, com 3719 com o gatilho falso).
pub const SPEECH_LOCKED: u16 = 3715;
/// Pares do inventário das chaves 62–67 (13–18).
const FIRST_KEY_SLOT: usize = 13;

/// Uma porta de célula.
#[derive(Debug, Clone)]
pub struct CellDoor {
    /// Origem da célula (px) e camada.
    pub origin: (i32, i32),
    pub layer: i32,
    /// Índice do sprite de cenário da célula em `Scene::sprites`.
    pub sprite: usize,
    /// Primeiro e último quadro do rótulo; quadro corrente (`+0x30`).
    pub first: usize,
    pub last: usize,
    pub frame: usize,
    /// Estado `+0x54`.
    pub state: u8,
    /// Chaves exigidas `+0x48` (bit i = chave 62 + i).
    pub keys: i32,
    /// Tiques até fechar `+0x4c` (< 0 nunca) e o contador `+0x50`.
    pub close_after: i32,
    pub timer: i32,
    /// Gatilhos da célula (`+0xa0`); só as entradas 3–4 contam para abrir.
    pub triggers: Vec<(i32, Option<i32>)>,
    pub closed: Vec<Rect>,
    pub open: Vec<Rect>,
}

/// A célula de índice `index` é uma porta (`0x0040ecd7`).
pub fn is_door(index: i32) -> bool {
    (index / 10) % 100 == 38
}

/// Nome do BRD da porta: `C%d%c%02d%c%d.BRD` com o índice e 1 = fechada, 0 = aberta.
pub fn border_name(index: i32, closed: bool) -> String {
    let letter = |d: i32| char::from(b'A' + (d.rem_euclid(10)) as u8);
    format!(
        "C{}{}{:02}{}{}.BRD",
        index / 10000,
        letter(index / 1000),
        (index / 10) % 100,
        letter(index),
        u8::from(closed)
    )
}

impl CellDoor {
    /// `table` = os `s16` de `+0x44` em diante (começa aberta, chaves, tempo, …).
    pub fn new(table: &[i32]) -> Self {
        let at = |i: usize| table.get(i).copied().unwrap_or(0);
        let close_after = if at(2) == 0 { DEFAULT_CLOSE } else { at(2) };
        Self {
            origin: (0, 0),
            layer: 0,
            sprite: usize::MAX,
            first: 0,
            last: 0,
            frame: 0,
            state: if at(0) != 0 { OPEN } else { CLOSED },
            keys: at(1),
            close_after,
            timer: close_after,
            triggers: Vec::new(),
            closed: Vec::new(),
            open: Vec::new(),
        }
    }

    /// Caixas sólidas no estado atual (a aberta só depois de terminar de abrir).
    pub fn solids(&self) -> &[Rect] {
        if self.state == OPEN {
            &self.open
        } else {
            &self.closed
        }
    }

    /// Faixa de ocupação (`0x0040ee3b`): na altura toda da célula (128 px), com o x e a
    /// largura da primeira caixa da colisão atual que não é mais larga que alta (a coluna da
    /// porta; os trilhos 128 × 12 ficam de fora); sem ela, x = 58 e largura 12.
    pub fn strip(&self) -> Rect {
        let (x, w) = self
            .solids()
            .iter()
            .find(|b| b.w <= b.h)
            .map_or((self.origin.0 + 58, 12), |b| (b.x, b.w));
        Rect {
            x,
            y: self.origin.1,
            w,
            h: super::CELL,
        }
    }

    /// Um tique (`0x0040ed62`): anda um quadro abrindo ou fechando; aberta, conta e fecha se
    /// nada cruza a faixa da coluna (`strip`, pela consulta `occupied`). Devolve `true` se
    /// a colisão mudou.
    pub fn tick(&mut self, occupied: impl Fn(&[Rect]) -> bool) -> bool {
        match self.state {
            OPENING => {
                self.frame += 1;
                if self.frame > self.last {
                    self.frame = self.last;
                    self.state = OPEN;
                    return true;
                }
            }
            CLOSING => {
                self.frame = self.frame.saturating_sub(1);
                if self.frame <= self.first {
                    self.frame = self.first;
                    self.state = CLOSED;
                }
            }
            OPEN => {
                self.frame = self.last;
                if self.timer > 0 {
                    self.timer -= 1;
                } else if self.timer == 0 && !occupied(&[self.strip()]) {
                    self.timer = self.close_after;
                    self.state = CLOSING;
                    return true;
                }
            }
            _ => self.frame = self.first,
        }
        false
    }

    /// O pedido de quem empurra (`0x0040f040`) com a máscara de chaves e os gatilhos 3–4:
    /// fechada → abrindo (sem chave pedida, ou com alguma) ou a fala; aberta → fechando.
    /// Devolve a fala, se houver.
    pub fn push(&mut self, keys: i32, triggers_ok: bool) -> Option<u16> {
        match self.state {
            CLOSED => {
                if self.keys != 0 && self.keys & keys == 0 {
                    return Some(SPEECH_LOCKED);
                }
                if !triggers_ok {
                    return Some(SPEECH_LOCKED);
                }
                self.state = OPENING;
            }
            OPEN => {
                self.timer = self.close_after;
                self.state = CLOSING;
            }
            _ => {}
        }
        None
    }

    /// Gatilhos da célula só com as entradas 3 e 4 (`0x435bd8(+0xa0, 2, 2)`).
    pub fn door_triggers(&self) -> Vec<(i32, Option<i32>)> {
        self.triggers
            .iter()
            .enumerate()
            .map(|(i, &(id, c))| (id, if i < 2 { None } else { c }))
            .collect()
    }
}

/// Máscara das chaves do Jake: bit i = par 13 + i com quantidade ≠ 0.
pub fn key_mask(inv: &crate::jake::Inventory) -> i32 {
    (0..6)
        .filter(|&i| inv.slots[FIRST_KEY_SLOT + i].1 != 0)
        .fold(0, |m, i| m | (1 << i))
}

impl Scene {
    /// Caixas das portas de célula da camada `layer`, no estado atual.
    pub(super) fn door_solids(&self, layer: i32) -> Vec<Rect> {
        self.cell_doors
            .iter()
            .filter(|d| d.layer == layer)
            .flat_map(|d| d.solids().iter().copied())
            .collect()
    }

    /// Tique das portas: o pedido do Jake que empurra numa zona 0x20 e a animação/fecho.
    pub(super) fn tick_cell_doors(&mut self) {
        if self.cell_doors.is_empty() {
            return;
        }
        let mut changed = false;
        if let Some(j) = self.jake.as_ref().filter(|j| j.door_push) {
            let jb = j.body.bounds();
            let on_zone = self.prop_zones.iter().any(|z| {
                z.layer == self.jake_layer
                    && z.kind & crate::jake::zone::PUSH != 0
                    && z.rect.overlaps(&jb)
            });
            // a célula do Jake pela posição (`+0x14c >> 7`, `+0x150 >> 7`)
            let cell = (
                j.body.x.div_euclid(super::CELL) * super::CELL,
                j.body.y.div_euclid(super::CELL) * super::CELL,
            );
            let mask = key_mask(&j.inventory);
            let snapshot: Vec<(i32, bool)> = self
                .entities
                .iter()
                .filter(|e| e.alive && e.id != 0)
                .map(|e| (e.id, e.on))
                .collect();
            let on = |id: i32| snapshot.iter().find(|(i, _)| *i == id).map(|(_, o)| *o);
            let layer = self.jake_layer;
            if let Some(d) = self
                .cell_doors
                .iter_mut()
                .filter(|_| on_zone)
                .find(|d| d.origin == cell && d.layer == layer)
            {
                let ok = crate::triggers::satisfied(&d.door_triggers(), &on);
                let before = d.state;
                if let Some(id) = d.push(mask, ok) {
                    self.events.push(Event::Sound(Cue::ui(id, PRIORITY_PICKUP)));
                }
                changed |= before != d.state && d.state == CLOSING;
            }
        }
        let jake = self
            .jake
            .as_ref()
            .filter(|j| j.is_active())
            .map(|j| (j.body.bounds(), j.layer()));
        let others: Vec<(Rect, i32)> = self
            .entities
            .iter()
            .filter(|e| e.alive && e.solid_shapes && e.flags & 1 != 0)
            .filter_map(|e| e.bounds().map(|b| (b, e.layer)))
            .collect();
        for d in self.cell_doors.iter_mut() {
            let layer = d.layer;
            let occupied = |boxes: &[Rect]| {
                boxes.iter().any(|b| {
                    jake.is_some_and(|(r, l)| l == layer && r.overlaps(b))
                        || others.iter().any(|(r, l)| *l == layer && r.overlaps(b))
                })
            };
            changed |= d.tick(occupied);
            if let Some(sp) = self.sprites.get_mut(d.sprite) {
                let f = d.frame.min(sp.anim.frames.len().saturating_sub(1));
                sp.player = crate::AnimPlayer::from_frame(&sp.anim, f);
                sp.frozen = true;
            }
        }
        if changed {
            self.set_jake_layer(self.jake_layer);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn border_names_follow_the_index() {
        assert_eq!(border_name(10380, true), "C1A38A1.BRD");
        assert_eq!(border_name(21381, false), "C2B38B0.BRD");
        assert!(is_door(22381) && !is_door(10071));
    }

    fn door(keys: i32) -> CellDoor {
        let mut d = CellDoor::new(&[0, keys, 3, 3, 1]);
        d.first = 10;
        d.last = 13;
        d.frame = 10;
        d
    }

    #[test]
    fn a_key_opens_and_the_door_closes_by_itself() {
        let mut d = door(2);
        assert_eq!(d.push(0, true), Some(SPEECH_LOCKED), "sem a chave");
        assert_eq!(d.push(1, true), Some(SPEECH_LOCKED), "outra chave");
        assert_eq!(d.push(3, false), Some(SPEECH_LOCKED), "gatilho falso");
        assert_eq!(d.push(2, true), None);
        assert_eq!(d.state, OPENING);
        let mut frames = vec![];
        for _ in 0..4 {
            d.tick(|_| false);
            frames.push((d.state, d.frame));
        }
        assert_eq!(
            frames,
            vec![(OPENING, 11), (OPENING, 12), (OPENING, 13), (OPEN, 13)]
        );
        // fica aberta `+0x4c` tiques e não fecha sobre alguém
        for _ in 0..3 {
            d.tick(|_| false);
        }
        assert!(!d.tick(|_| true));
        assert_eq!(d.state, OPEN);
        assert!(d.tick(|_| false));
        assert_eq!(d.state, CLOSING);
        for _ in 0..3 {
            d.tick(|_| false);
        }
        assert_eq!((d.state, d.frame), (CLOSED, 10));
        // nunca fecha com `+0x4c < 0`; sem chave pedida abre sempre
        let mut d = CellDoor::new(&[1, 0, -1, -1, 1]);
        assert_eq!(d.state, OPEN);
        for _ in 0..200 {
            d.tick(|_| false);
        }
        assert_eq!(d.state, OPEN);
        let mut d = door(0);
        assert_eq!(d.push(0, true), None);
    }

    fn r(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }

    /// Porta aberta como `C1A38A0.BRD` na célula (128, 0): trilhos 128 × 12 em cima e
    /// embaixo e a coluna de 20 px encurtada.
    fn open_door() -> CellDoor {
        let mut d = CellDoor::new(&[1, 0, 3]);
        d.origin = (128, 0);
        d.timer = 0;
        let rails = [r(128, 0, 128, 12), r(128, 116, 128, 12)];
        d.closed = [&rails[..], &[r(178, 12, 20, 104)]].concat();
        d.open = [&rails[..], &[r(178, 12, 20, 30)]].concat();
        d
    }

    /// Ocupação (`0x40ee3b`): a faixa da coluna da porta (x e largura da primeira caixa
    /// com largura ≤ altura) na altura toda da célula; sem ela, x = 58 e 12 de largura.
    #[test]
    fn occupancy_strip_is_the_door_column_over_the_whole_cell() {
        let d = open_door();
        assert_eq!(d.strip(), r(178, 0, 20, 128));
        let mut bare = CellDoor::new(&[1]);
        bare.origin = (0, 256);
        assert_eq!(bare.strip(), r(58, 256, 12, 128));
    }

    /// Um barril pousado no trilho de baixo, fora da coluna, não segura a porta aberta;
    /// dentro da coluna, segura.
    #[test]
    fn barrel_on_the_rail_does_not_keep_the_door_open() {
        for (x, stays) in [(140, false), (180, true)] {
            let mut s = Scene::empty(512, 256);
            s.cell_doors = vec![open_door()];
            let mut b = crate::behaviour::test_support::logic(crate::entity::BARREL, x, 90, &[]);
            b.boxes = vec![r(0, 0, 16, 30)]; // cruza o trilho (y 116..128) por 4 px
            b.solid_shapes = true;
            b.flags = 0x80001;
            s.entities.push(b);
            s.tick_cell_doors();
            let st = s.cell_doors[0].state;
            assert_eq!(st == OPEN, stays, "barril em x = {x}: estado {st}");
        }
    }

    #[test]
    fn key_mask_reads_pairs_13_to_18() {
        let mut inv = crate::jake::Inventory::from_points(&[]);
        inv.slots[14] = (1, 1);
        inv.slots[18] = (1, 1);
        assert_eq!(key_mask(&inv), 0b10_0010);
    }
}
