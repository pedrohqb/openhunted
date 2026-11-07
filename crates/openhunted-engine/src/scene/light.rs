// SPDX-License-Identifier: GPL-3.0-or-later
//! Luz por célula (`docs/re/casca-acabamento.md`): as entradas 1–2 da lista de gatilhos da
//! célula (`+0xa0`) dizem se ela está acesa (`0x433a81`); a célula apagada escurece até a
//! sombra 8 e some (com os objetos dentro), a acesa reaparece e clareia até 0 (`0x432d39`).

use std::collections::HashMap;

use openhunted_formats::Level;

use super::CELL;

/// Sombra máxima (`0x43150f`) e a primeira tabela de sombra do `DLS.TBL` (`0x43be54`:
/// sombra `s` → tabela 18 + `s`).
pub const MAX_SHADE: i32 = 8;
pub const SHADOW_BASE: i32 = 18;
/// Entradas da lista que decidem a luz (as 3–4 são da porta da célula).
pub const ENTRIES: usize = 2;

/// Estado de uma célula com luz.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellLight {
    /// Canto da célula na fase e camada.
    pub origin: (i32, i32),
    pub layer: i32,
    pub triggers: Vec<(i32, Option<i32>)>,
    /// Bit 0 de `+0x38`: a célula aparece.
    pub visible: bool,
    /// `+0x58`: 0 (clara) a 8.
    pub shade: i32,
}

/// Como a célula se desenha.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Look {
    Hidden,
    /// Tabela de sombra do `DLS.TBL` (19–26).
    Shaded(usize),
}

impl CellLight {
    pub fn look(&self) -> Option<Look> {
        if !self.visible {
            Some(Look::Hidden)
        } else if self.shade > 0 {
            Some(Look::Shaded((SHADOW_BASE + self.shade) as usize))
        } else {
            None
        }
    }

    fn lit(&self, on: &dyn Fn(i32) -> Option<bool>) -> bool {
        crate::triggers::satisfied_range(&self.triggers, 0, ENTRIES, on)
    }

    /// Tique da célula (`0x432d39`).
    pub fn tick(&mut self, on: &dyn Fn(i32) -> Option<bool>) {
        if self.lit(on) {
            if !self.visible {
                self.shade = MAX_SHADE;
                self.visible = true;
            }
            if self.shade > 0 {
                self.shade -= 1;
            }
        } else if self.shade < MAX_SHADE {
            self.shade += 1;
        } else {
            self.visible = false;
        }
    }
}

/// As células com luz da fase.
#[derive(Debug, Clone, Default)]
pub struct Lights {
    pub cells: Vec<CellLight>,
    index: HashMap<(i32, i32, i32), usize>,
}

impl Lights {
    /// Células com alguma das entradas 1–2 armada (as outras estão sempre acesas).
    pub fn from_level(level: &Level) -> Self {
        let (gx, gy) = (
            i32::from(level.grid_x).max(1),
            i32::from(level.grid_y).max(1),
        );
        let mut l = Self::default();
        for (i, c) in level.cells.iter().enumerate() {
            let Some(c) = c else { continue };
            if !crate::triggers::armed(&c.triggers[..c.triggers.len().min(ENTRIES)]) {
                continue;
            }
            let i = i as i32;
            let key = (i % gx, (i / gx) % gy, i / (gx * gy));
            l.index.insert(key, l.cells.len());
            l.cells.push(CellLight {
                origin: (key.0 * CELL, key.1 * CELL),
                layer: key.2,
                triggers: c.triggers.clone(),
                visible: true,
                shade: 0,
            });
        }
        l
    }

    /// Estado na carga (`0x4314e7`): acesa → visível e clara; apagada → invisível, sombra 8.
    pub fn init(&mut self, on: &dyn Fn(i32) -> Option<bool>) {
        for c in &mut self.cells {
            let lit = c.lit(on);
            c.visible = lit;
            c.shade = if lit { 0 } else { MAX_SHADE };
        }
    }

    /// Ruling: todas as células com luz andam a cada tique (o original, só as da vista).
    pub fn tick(&mut self, on: &dyn Fn(i32) -> Option<bool>) {
        for c in &mut self.cells {
            c.tick(on);
        }
    }

    pub fn index_of(&self, cell: (i32, i32, i32)) -> Option<usize> {
        self.index.get(&cell).copied()
    }

    /// Como se desenha o que está no ponto (x, y) da camada (`0x435739`: célula de 128 px
    /// pela divisão truncada); `None` = normal.
    pub fn look_at(&self, (x, y): (i32, i32), layer: i32) -> Option<Look> {
        if x < 0 || y < 0 {
            return None;
        }
        let i = self.index_of((x / CELL, y / CELL, layer))?;
        self.cells[i].look()
    }

    pub fn look(&self, i: usize) -> Option<Look> {
        self.cells.get(i).and_then(CellLight::look)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(code: i32) -> CellLight {
        CellLight {
            origin: (0, 0),
            layer: 0,
            triggers: vec![(5, Some(code)), (0, Some(0)), (9, Some(11)), (0, Some(0))],
            visible: true,
            shade: 0,
        }
    }

    #[test]
    fn dark_cell_fades_out_hides_and_comes_back_through_the_shades() {
        let off = |_: i32| Some(false);
        let on = |_: i32| Some(true);
        let mut c = cell(11);
        for s in 1..=MAX_SHADE {
            c.tick(&off);
            assert_eq!((c.visible, c.shade), (true, s));
        }
        assert_eq!(c.look(), Some(Look::Shaded(26)));
        c.tick(&off);
        assert_eq!(c.look(), Some(Look::Hidden));
        // acende: volta com sombra 8 e clareia um passo por tique
        c.tick(&on);
        assert_eq!((c.visible, c.shade), (true, 7));
        for _ in 0..7 {
            c.tick(&on);
        }
        assert_eq!(c.look(), None);
        // a entrada 3 (porta) não conta para a luz
        let mut c = cell(0);
        c.tick(&off);
        assert_eq!(c.look(), None, "sem entradas 1–2: sempre acesa");
    }
}
