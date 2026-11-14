// SPDX-License-Identifier: GPL-3.0-or-later
//! Resultado de dois jogadores (`SH2HS.ITF`, `0x443c30` + `0x443c9c`, números `0x443e7a`;
//! `docs/re/multijogador.md`): por jogador mortos, tesouros e segredos (número, pontos e %),
//! na disputa as mortes sofridas (S) e dadas (F), e a pontuação; contagem como a do `SMISS`.

use std::rc::Rc;

use openhunted_formats::Font;

use super::page::{Page, Reply};
use super::results::{
    COUNT_STEP, Choice, KILL_POINTS, SECRET_POINTS, SOUND_COUNT, TEXT_COLOR, TREASURE_POINTS,
    percent,
};
use super::screen::{CANCEL, Input, Screen, flags};
use crate::behaviour::Outcome;
use crate::render::Framebuffer;

pub mod ids {
    pub const BACKGROUND: u16 = 30000;
    pub const REPLAY: u16 = 30001;
    pub const MAIN: u16 = 30003;
    /// Primeiro número de cada jogador (`MH2H_NKILLS1`, `MH2H_NKILLS2`); os outros seguem:
    /// +1 pontos dos mortos, +2 %, +3…+5 tesouros, +6…+8 segredos, +9 `KILLS`, +10
    /// `KILLO`, +11 `SCORE`.
    pub const FIRST: [u16; 2] = [30004, 30017];
    pub const NEXT: u16 = 30030;
    pub const SUCCESS: u16 = 30031;
    pub const FAILURE: u16 = 30032;
    pub const MISSION_TITLE: u16 = 30033;
    /// Botões navegáveis (`0x48045c`).
    pub const BUTTONS: [u16; 3] = [REPLAY, NEXT, MAIN];
}

/// Números de um jogador (`0x49732c`, registro `k`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Line {
    pub kills: i32,
    pub treasures: i32,
    pub secrets: i32,
    /// F (eliminações do outro) e S (autoeliminações), só mostrados na disputa.
    pub frags: i32,
    pub suicides: i32,
}

impl Line {
    /// `SCORE` = 30·mortos + 65·tesouros + 215·segredos (sem F nem bônus).
    pub fn score(&self) -> i32 {
        KILL_POINTS * self.kills + TREASURE_POINTS * self.treasures + SECRET_POINTS * self.secrets
    }
}

/// Totais da fase (registro 0: `0x497330`, `0x49733c`, `0x49732c`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Totals {
    pub kills: i32,
    pub treasures: i32,
    pub secrets: i32,
}

#[derive(Debug, Clone)]
pub struct DuoResults {
    pub screen: Screen,
    font: Rc<Font>,
    pub outcome: Outcome,
    pub deathmatch: bool,
    pub lines: [Line; 2],
    pub totals: Totals,
    progress: i32,
    from_game: bool,
}

impl DuoResults {
    pub fn new(
        mut screen: Screen,
        font: Rc<Font>,
        title: &str,
        outcome: Outcome,
        deathmatch: bool,
        lines: [Line; 2],
        totals: Totals,
    ) -> Self {
        for id in ids::BUTTONS {
            screen.set_flags(id, flags::UPDATE | flags::NAV);
        }
        // `0x443d31`: vitória (1) tira 30032, senão 30031
        let (keep, drop) = match outcome {
            Outcome::Victory => (ids::SUCCESS, ids::FAILURE),
            Outcome::Defeat => (ids::FAILURE, ids::SUCCESS),
        };
        screen.remove(drop);
        screen.freeze(keep);
        screen.set_text(ids::MISSION_TITLE, title, font.clone(), TEXT_COLOR);
        screen.point_at(ids::NEXT);
        let mut r = Self {
            screen,
            font,
            outcome,
            deathmatch,
            lines,
            totals,
            progress: 0,
            from_game: true,
        };
        r.write_numbers();
        r
    }

    /// Contagem em % (0…100).
    pub fn progress(&self) -> i32 {
        self.progress.min(100)
    }

    pub fn skip_count(&mut self) {
        if self.progress < 100 {
            self.progress = 100;
            self.write_numbers();
        }
    }

    /// Valores de cada controle com a contagem em `p` %.
    pub fn values(&self, p: i32) -> Vec<(u16, i32)> {
        let k = |v: i32| (i64::from(v) * i64::from(p.min(100)) / 100) as i32;
        let t = &self.totals;
        let mut out = Vec::new();
        for (base, l) in ids::FIRST.into_iter().zip(&self.lines) {
            out.extend([
                (base, k(l.kills)),
                (base + 1, k(KILL_POINTS * l.kills)),
                (base + 2, k(percent(l.kills, t.kills))),
                (base + 3, k(l.treasures)),
                (base + 4, k(TREASURE_POINTS * l.treasures)),
                (base + 5, k(percent(l.treasures, t.treasures))),
                (base + 6, k(l.secrets)),
                (base + 7, k(SECRET_POINTS * l.secrets)),
                (base + 8, k(percent(l.secrets, t.secrets))),
            ]);
            // só na disputa (`0x443f52`): KILLS = S, KILLO = F
            if self.deathmatch {
                out.extend([(base + 9, k(l.suicides)), (base + 10, k(l.frags))]);
            }
            out.push((base + 11, k(l.score())));
        }
        out
    }

    fn write_numbers(&mut self) {
        for (id, v) in self.values(self.progress) {
            self.screen
                .set_text(id, &v.to_string(), self.font.clone(), TEXT_COLOR);
        }
    }
}

impl Page for DuoResults {
    fn screen(&self) -> &Screen {
        &self.screen
    }

    fn handle(&mut self, input: Input, sounds: &mut Vec<u16>) -> Option<Reply> {
        if input == Input::Tick && self.progress <= 100 {
            self.write_numbers();
            sounds.push(SOUND_COUNT);
            self.progress += COUNT_STEP;
        }
        // durante a contagem, uma tecla ou um clique só a salta para o fim
        let key = matches!(input, Input::Activate | Input::Cancel | Input::Nav(_));
        if (key || input == Input::PointerDown) && self.progress < 100 {
            self.skip_count();
            if key {
                return None;
            }
        }
        match self.screen.handle(input)? {
            ids::REPLAY => Some(Reply::Choice(Choice::Replay)),
            ids::NEXT => Some(Reply::Choice(Choice::Next)),
            ids::MAIN => Some(Reply::Choice(Choice::Main)),
            CANCEL if !self.from_game => Some(Reply::Choice(Choice::Main)),
            _ => None,
        }
    }

    fn leave_game(&mut self) {
        self.from_game = false;
    }

    fn draw(&self, fb: &mut Framebuffer) {
        self.screen.draw(fb);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_score_has_no_frags() {
        let l = Line {
            kills: 2,
            treasures: 1,
            secrets: 1,
            frags: 9,
            suicides: 3,
        };
        assert_eq!(l.score(), 60 + 65 + 215);
    }
}
