// SPDX-License-Identifier: GPL-3.0-or-later
//! Telas cheias e diálogos da casca além do menu, do briefing e do resultado
//! (`docs/re/menus-restantes.md`, `opcoes-salvamento.md`): uma interface comum para a
//! aplicação despachar a entrada e desenhar.

use super::Party;
use super::config::Config;
use super::results::Choice;
use super::screen::{Input, Screen};
use crate::character::Character;
use crate::render::Framebuffer;

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
    /// Modo de jogo escolhido (`SMULT`, ou os grupos 22016–22018 da `SLEVL`).
    Party(Party),
    /// `SEGOC`: OK com o personagem e a cor (35–38) do jogador da vez.
    Chosen(Character, u8),
    /// `SEGOC`: o botão do dispositivo (15009) leva ao `SINPU`.
    Devices,
    /// `SHVLD`: GO com os limites em minutos e pontos (`0x497374`, `0x497378`).
    Limits { minutes: i32, points: i32 },
    /// Botão do resultado de dois jogadores (`SH2HS`).
    Choice(Choice),
    /// CUSTOMIZE do jogador 0 ou 1 (`SINPU` → `SKEYB` ou `SJOYS`).
    Customize(usize),
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
    /// Espera uma tecla ou um botão (`SKEYB`/`SJOYS`): a aplicação passa as teclas cruas
    /// (`raw_key`, código VK) e os botões do joystick (`joy_buttons`) em vez de navegar.
    fn capturing(&self) -> bool {
        false
    }
    fn raw_key(&mut self, _vk: u8) {}
    fn joy_buttons(&mut self, _pressed: u32) {}
    /// O resultado deixa de vir direto do jogo (volta de um briefing cancelado): o Esc passa
    /// a valer.
    fn leave_game(&mut self) {}
}
