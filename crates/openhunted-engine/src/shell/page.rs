// SPDX-License-Identifier: GPL-3.0-or-later
//! Telas cheias e diálogos da casca além do menu, do briefing e do resultado
//! (`docs/re/menus-restantes.md`, `opcoes-salvamento.md`): uma interface comum para a
//! aplicação despachar a entrada e desenhar.

use super::config::Config;
use super::screen::{Input, Screen};
use crate::render::Framebuffer;

/// Texto da caixa dos modos de dois jogadores (decisão do port: não existem).
pub const UNAVAILABLE: &str = "TWO PLAYER MODES ARE NOT AVAILABLE.";

/// O que a tela pede à aplicação.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply {
    /// Fechar (DONE, OK, CANCEL ou Esc).
    Close,
    /// Opções mudaram (gravar e aplicar já).
    Config(Config),
    /// Missão escolhida (índice da lista solo).
    Pick(usize),
    /// Texto confirmado no campo de edição.
    Name(String),
    /// Modo de dois jogadores: caixa "não disponível".
    Unavailable,
    /// CUSTOMIZE do jogador 1 (`SINPU` → `SKEYB`).
    Customize,
}

/// Uma tela aberta.
pub trait Page {
    fn screen(&self) -> &Screen;
    /// Trata uma entrada (`Input::Tick` a cada quadro); `sounds` recebe os sons das telas.
    fn handle(&mut self, input: Input, sounds: &mut Vec<u16>) -> Option<Reply>;
    fn draw(&self, fb: &mut Framebuffer) {
        self.screen().draw(fb);
    }
    /// Texto digitado (só o campo de edição usa).
    fn text(&mut self, _s: &str) {}
    fn backspace(&mut self) {}
    /// Recebe texto (o espaço digita em vez de clicar).
    fn wants_text(&self) -> bool {
        false
    }
}
