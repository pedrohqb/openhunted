// SPDX-License-Identifier: GPL-3.0-or-later
//! Grade de células e busca de caminho da IA (`0x42f88c`, `0x42f9a4`;
//! `docs/re/minotauro-ia.md`): bits de passagem de `célula+0x04`, zonas PR e portas. As
//! portas `OH` (tipos 50–52) ficam de fora: a lista `0x4888cc` que `0x42f96a` monta para a
//! busca nunca é lida.

use std::collections::VecDeque;

use crate::jake::{Zone, zone};

pub use crate::scene::CELL;

/// Célula (x, y, camada).
pub type Cell = (i32, i32, i32);

/// Bits de `célula+0x04` (`fields[0]` nas células v ≥ 1006).
pub mod bits {
    /// Fechada embaixo.
    pub const CLOSED_BELOW: i32 = 0x2;
    /// Parede à direita.
    pub const WALL_RIGHT: i32 = 0x8;
    /// Parede à esquerda.
    pub const WALL_LEFT: i32 = 0x10;
    /// Fechada em cima.
    pub const CLOSED_ABOVE: i32 = 0x40;
}

/// Profundidade da busca e limite da distância Manhattan até o alvo.
pub const MAX_DEPTH: i32 = 10;

/// O que a IA sabe de uma célula.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CellInfo {
    /// A célula existe no `.LEV` (`0x4317db` devolve NULL para as vazias).
    pub present: bool,
    /// `célula+0x04`.
    pub bits: i32,
    /// União dos tipos das zonas PR da célula.
    pub zones: u16,
    /// Índice do cenário (o "código" `(c/10) % 100` dos perigos).
    pub code: i32,
    /// `célula+0x40` (espelhada).
    pub flag40: i32,
    /// Porta de célula fechada ou fechando (`célula+0x54` 1/2, `0x40f1f0`).
    pub door_closed: bool,
}

/// Grade `x × y × camadas` (x varia mais rápido, como no `.LEV`).
#[derive(Debug, Clone, Default)]
pub struct CellGrid {
    pub size: (i32, i32, i32),
    cells: Vec<CellInfo>,
}

impl CellGrid {
    /// Grade com todas as células presentes e abertas.
    pub fn new(size: (i32, i32, i32)) -> Self {
        let mut g = Self::empty(size);
        for c in &mut g.cells {
            c.present = true;
        }
        g
    }

    /// Grade só de células vazias (a cena marca as que o `.LEV` tem).
    pub fn empty(size: (i32, i32, i32)) -> Self {
        let size = (size.0.max(1), size.1.max(1), size.2.max(1));
        Self {
            size,
            cells: vec![CellInfo::default(); (size.0 * size.1 * size.2) as usize],
        }
    }

    fn index(&self, c: Cell) -> Option<usize> {
        let (x, y, z) = c;
        let (gx, gy, gz) = self.size;
        ((0..gx).contains(&x) && (0..gy).contains(&y) && (0..gz).contains(&z))
            .then(|| (x + y * gx + z * gx * gy) as usize)
    }

    pub fn get(&self, c: Cell) -> Option<&CellInfo> {
        self.index(c).map(|i| &self.cells[i])
    }

    pub fn get_mut(&mut self, c: Cell) -> Option<&mut CellInfo> {
        self.index(c).map(|i| &mut self.cells[i])
    }

    /// A célula, se existe (`0x4317db`: NULL fora da grade ou vazia).
    pub fn cell(&self, c: Cell) -> Option<&CellInfo> {
        self.get(c).filter(|ci| ci.present)
    }

    /// Célula do ponto `p` (px) na camada `layer`.
    pub fn cell_of(&self, p: (i32, i32), layer: i32) -> Cell {
        (p.0.div_euclid(CELL), p.1.div_euclid(CELL), layer)
    }

    /// Soma as zonas PR às células em que começam.
    pub fn add_zones(&mut self, zones: &[Zone]) {
        for z in zones {
            let c = (
                (z.rect.x + z.rect.w / 2).div_euclid(CELL),
                (z.rect.y + z.rect.h / 2).div_euclid(CELL),
                z.layer,
            );
            if let Some(ci) = self.get_mut(c) {
                ci.zones |= z.kind;
            }
        }
    }

    /// Destino da direção `dir` (1 subir, 2 descer, 3 esquerda, 4 direita, 5 camada − 1,
    /// 6 camada + 1) se a célula de origem existe e deixa passar.
    pub fn allowed(&self, from: Cell, dir: u8) -> Option<Cell> {
        self.pass(from, dir, None)
    }

    /// `allowed` com o pensante em `own = (célula, x do centro)`: de lado, a porta fechada na
    /// própria célula também barra se ele está na metade de trás (`(x & 0x7f) > 64` indo à
    /// esquerda, `< 64` à direita; `0x42fc70`/`0x42fd0a`).
    fn pass(&self, from: Cell, dir: u8, own: Option<(Cell, i32)>) -> Option<Cell> {
        let ci = self.cell(from)?;
        let (x, y, z) = from;
        let to = match dir {
            1 if ci.bits & bits::CLOSED_ABOVE == 0 && ci.zones & zone::ROPE != 0 => (x, y - 1, z),
            2 if ci.bits & bits::CLOSED_BELOW == 0 => (x, y + 1, z),
            3 if ci.bits & bits::WALL_LEFT == 0 => (x - 1, y, z),
            4 if ci.bits & bits::WALL_RIGHT == 0 => (x + 1, y, z),
            5 if ci.zones & zone::LAYER_UP != 0 => (x, y, z - 1),
            6 if ci.zones & zone::LAYER_DOWN != 0 => (x, y, z + 1),
            _ => return None,
        };
        // o destino pode ser vazio (`0x40f1f0(NULL)` = aberta), mas não fora da grade
        let dest = self.get(to)?;
        if matches!(dir, 3 | 4) {
            let behind = own.is_some_and(|(c, px)| {
                let ox = px % CELL;
                c == from && if dir == 3 { ox > 64 } else { ox < 64 }
            });
            if dest.door_closed || (behind && ci.door_closed) {
                return None;
            }
        }
        Some(to)
    }

    /// Primeira direção do menor caminho de `from` (a célula do pensante, com o centro em
    /// `x`) a `to` (≤ 10 passos, só com a distância Manhattan ≤ 10); empate na ordem das
    /// direções. Ruling: busca em largura a partir de cada direção, no lugar da busca em
    /// profundidade do original (mesmo resultado).
    pub fn search(&self, from: Cell, to: Cell, x: i32) -> Option<u8> {
        let manhattan = (from.0 - to.0).abs() + (from.1 - to.1).abs() + (from.2 - to.2).abs();
        if from == to || manhattan > MAX_DEPTH || self.get(to).is_none() {
            return None;
        }
        let own = Some((from, x));
        (1..=6u8)
            .filter_map(|d| {
                let n = self.pass(from, d, own)?;
                self.distance(n, to, MAX_DEPTH - 1, own).map(|k| (k, d))
            })
            .min()
            .map(|(_, d)| d)
    }

    /// Passos de `a` a `b` (até `max`), em largura.
    fn distance(&self, a: Cell, b: Cell, max: i32, own: Option<(Cell, i32)>) -> Option<i32> {
        let mut seen = vec![false; self.cells.len()];
        let mut q = VecDeque::from([(a, 0)]);
        seen[self.index(a)?] = true;
        while let Some((c, k)) = q.pop_front() {
            if c == b {
                return Some(k);
            }
            if k >= max {
                continue;
            }
            for d in 1..=6 {
                if let Some(n) = self.pass(c, d, own) {
                    let i = self.index(n)?;
                    if !seen[i] {
                        seen[i] = true;
                        q.push_back((n, k + 1));
                    }
                }
            }
        }
        None
    }
}
