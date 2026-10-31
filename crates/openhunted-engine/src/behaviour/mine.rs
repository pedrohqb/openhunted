// SPDX-License-Identifier: GPL-3.0-or-later
//! Minas flutuantes (tipo 102, `0x0041e7f5`; `docs/re/minas.md`).

/// Variante de mina (tabela `0x00472bfc`).
#[derive(Debug, Clone, Copy)]
pub struct Variant {
    pub name: &'static str,
    /// Período da oscilação vertical (tiques).
    pub period: i32,
    /// Velocidade (1/8 px por tique).
    pub speed: i32,
    /// Alcance de detecção (px, centro a centro).
    pub range: i32,
    pub health: i32,
    /// Amplitude da oscilação (px).
    pub amplitude: i32,
    /// % por tique de começar / parar de pairar.
    pub hover_start: u32,
    pub hover_stop: u32,
}

pub const VARIANTS: [Variant; 4] = [
    Variant {
        name: "Alpha",
        period: 32,
        speed: 25,
        range: 200,
        health: 1,
        amplitude: 24,
        hover_start: 0,
        hover_stop: 100,
    },
    Variant {
        name: "Beta",
        period: 64,
        speed: 15,
        range: 250,
        health: 2,
        amplitude: 32,
        hover_start: 5,
        hover_stop: 20,
    },
    Variant {
        name: "Gamma",
        period: 32,
        speed: 45,
        range: 350,
        health: 3,
        amplitude: 28,
        hover_start: 10,
        hover_stop: 20,
    },
    Variant {
        name: "Delta",
        period: 32,
        speed: 20,
        range: 400,
        health: 4,
        amplitude: 12,
        hover_start: 15,
        hover_stop: 10,
    },
];

#[derive(Debug, Clone)]
pub struct MineState {
    pub variant: usize,
    pub has_target: bool,
    pub hovering: bool,
}

impl MineState {
    /// `values[0]` escolhe a variante (negativo → 0; ≥ 4 → 3).
    pub fn new(value: i32) -> Self {
        Self {
            variant: value.clamp(0, 3) as usize,
            has_target: false,
            hovering: false,
        }
    }

    pub fn variant(&self) -> &'static Variant {
        &VARIANTS[self.variant]
    }
}
