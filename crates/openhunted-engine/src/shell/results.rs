// SPDX-License-Identifier: GPL-3.0-or-later
//! Tela de resultado solo (`SMISS.ITF`, `docs/re/fim-de-missao.md`): estatísticas,
//! pontuação com bônus de vitória, contagem progressiva e os botões REPLAY, NEXT e MAIN.

use std::rc::Rc;

use openhunted_formats::Font;

use super::screen::{CANCEL, Input, Screen, flags};
use crate::behaviour::Outcome;
use crate::render::Framebuffer;

pub const REPLAY: u16 = 25001;
pub const NEXT: u16 = 25002;
pub const MAIN: u16 = 25004;
pub const NUM_KILLS: u16 = 25005;
pub const NUM_TREASURE: u16 = 25006;
pub const NUM_SECRETS: u16 = 25007;
pub const SCORE_KILLS: u16 = 25008;
pub const SCORE_TREASURE: u16 = 25009;
pub const SCORE_SECRETS: u16 = 25010;
pub const PERCENT_KILLS: u16 = 25011;
pub const PERCENT_TREASURE: u16 = 25012;
pub const PERCENT_SECRETS: u16 = 25013;
pub const SCORE: u16 = 25014;
pub const CUMUL_SCORE: u16 = 25015;
pub const SUCCESS: u16 = 25016;
pub const FAILURE: u16 = 25017;
pub const PISTONS: u16 = 25018;
pub const BONUS: u16 = 25019;
pub const IMPROVE_TEXT: u16 = 25020;
pub const MISSION_TITLE: u16 = 25021;
/// Cor de todo o texto.
pub const TEXT_COLOR: u8 = 231;
/// Pontos por morto, tesouro e segredo.
pub const KILL_POINTS: i32 = 30;
pub const TREASURE_POINTS: i32 = 65;
pub const SECRET_POINTS: i32 = 215;
/// Som de cada quadro da contagem.
pub const SOUND_COUNT: u16 = 3093;
/// Passo da contagem por quadro (em %).
pub const COUNT_STEP: i32 = 5;

/// Estatísticas da partida.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Stats {
    pub kills: i32,
    pub treasure: i32,
    pub secrets: i32,
    /// Totais da fase (0 → 100 %).
    pub kills_total: i32,
    pub treasure_total: i32,
    pub secrets_total: i32,
}

/// O que sobrou do jogador no fim (para o bônus).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Remains {
    /// Cronômetro em tiques.
    pub timer: i32,
    pub health: i32,
    pub armor: i32,
    /// Soma das munições que entram no bônus (`0x444bf8`).
    pub ammo: i32,
}

/// Bônus de vitória (`0x444bf8`): segundos restantes + vida + armadura + 500 + munição.
pub fn bonus(outcome: Outcome, r: Remains) -> i32 {
    match outcome {
        Outcome::Victory => r.timer.max(0) / 20 + r.health.max(0) + r.armor.max(0) + 500 + r.ammo,
        Outcome::Defeat => 0,
    }
}

/// Pontuação: 30·mortos + 65·tesouros + 215·segredos + bônus.
pub fn score(s: &Stats, bonus: i32) -> i32 {
    KILL_POINTS * s.kills + TREASURE_POINTS * s.treasure + SECRET_POINTS * s.secrets + bonus
}

/// 100 × achados / total (total 0 → 100).
pub fn percent(found: i32, total: i32) -> i32 {
    if total == 0 { 100 } else { 100 * found / total }
}

/// O que o jogador escolheu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    Replay,
    Next,
    Main,
}

/// Tela de resultado aberta.
#[derive(Debug, Clone)]
pub struct Results {
    pub screen: Screen,
    font: Rc<Font>,
    pub outcome: Outcome,
    pub stats: Stats,
    pub bonus: i32,
    pub score: i32,
    /// Soma dos recordes (só na vitória).
    pub cumulative: Option<i32>,
    /// Contagem em % (0…100; acima de 100 = terminou).
    progress: i32,
    /// Veio direto do jogo (Esc ignorado).
    pub from_game: bool,
}

impl Results {
    /// `improve`: texto "MISSION SCORE IMPROVED BY…" já formatado, se houver.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        mut screen: Screen,
        font: Rc<Font>,
        title: &str,
        outcome: Outcome,
        stats: Stats,
        remains: Remains,
        cumulative: Option<i32>,
        improve: Option<String>,
    ) -> Self {
        let bonus = bonus(outcome, remains);
        let score = score(&stats, bonus);
        for id in [REPLAY, NEXT, MAIN] {
            screen.set_flags(id, flags::UPDATE | flags::NAV);
        }
        // a imagem de resultado fica parada; a outra sai
        let (keep, drop) = match outcome {
            Outcome::Victory => (SUCCESS, FAILURE),
            Outcome::Defeat => (FAILURE, SUCCESS),
        };
        screen.remove(drop);
        screen.freeze(keep);
        screen.set_text(MISSION_TITLE, title, font.clone(), TEXT_COLOR);
        if let Some(t) = improve {
            screen.set_text(IMPROVE_TEXT, &t, font.clone(), TEXT_COLOR);
        }
        screen.point_at(match outcome {
            Outcome::Victory => NEXT,
            Outcome::Defeat => REPLAY,
        });
        let mut r = Self {
            screen,
            font,
            outcome,
            stats,
            bonus,
            score,
            cumulative,
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

    /// Salta a contagem para o fim (qualquer tecla).
    pub fn skip_count(&mut self) {
        if self.progress < 100 {
            self.progress = 100;
            self.write_numbers();
        }
    }

    fn write_numbers(&mut self) {
        let p = self.progress.min(100);
        let k = |v: i32| (i64::from(v) * i64::from(p) / 100) as i32;
        let s = &self.stats;
        let mut vals = vec![
            (NUM_KILLS, k(s.kills)),
            (NUM_TREASURE, k(s.treasure)),
            (NUM_SECRETS, k(s.secrets)),
            (SCORE_KILLS, k(KILL_POINTS * s.kills)),
            (SCORE_TREASURE, k(TREASURE_POINTS * s.treasure)),
            (SCORE_SECRETS, k(SECRET_POINTS * s.secrets)),
            (PERCENT_KILLS, k(percent(s.kills, s.kills_total))),
            (PERCENT_TREASURE, k(percent(s.treasure, s.treasure_total))),
            (PERCENT_SECRETS, k(percent(s.secrets, s.secrets_total))),
            (SCORE, k(self.score)),
        ];
        if self.outcome == Outcome::Victory {
            vals.push((BONUS, k(self.bonus)));
            if let Some(c) = self.cumulative {
                vals.push((CUMUL_SCORE, k(c)));
            }
        }
        for (id, v) in vals {
            self.screen
                .set_text(id, &v.to_string(), self.font.clone(), TEXT_COLOR);
        }
    }

    /// Trata a entrada; `Input::Tick` avança a contagem e devolve, em `sounds`, o som de
    /// cada quadro dela.
    pub fn handle(&mut self, input: Input, sounds: &mut Vec<u16>) -> Option<Choice> {
        if input == Input::Tick && self.progress <= 100 {
            self.write_numbers();
            sounds.push(SOUND_COUNT);
            self.progress += COUNT_STEP;
        }
        // durante a contagem, uma tecla só a salta para o fim
        if matches!(input, Input::Activate | Input::Cancel | Input::Nav(_)) && self.progress < 100 {
            self.skip_count();
            return None;
        }
        match self.screen.handle(input)? {
            REPLAY => Some(Choice::Replay),
            NEXT => Some(Choice::Next),
            MAIN => Some(Choice::Main),
            CANCEL if !self.from_game => Some(Choice::Main),
            _ => None,
        }
    }

    pub fn draw(&self, fb: &mut Framebuffer) {
        self.screen.draw(fb);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_and_bonus_follow_the_formula() {
        let s = Stats {
            kills: 3,
            treasure: 1,
            secrets: 2,
            ..Default::default()
        };
        let r = Remains {
            timer: 100 * 20 + 7,
            health: 80,
            armor: 10,
            ammo: 12,
        };
        assert_eq!(bonus(Outcome::Victory, r), 100 + 80 + 10 + 500 + 12);
        assert_eq!(bonus(Outcome::Defeat, r), 0);
        assert_eq!(score(&s, 0), 90 + 65 + 430);
        assert_eq!(score(&s, 702), 585 + 702);
        assert_eq!(percent(1, 0), 100);
        assert_eq!(percent(1, 4), 25);
    }
}
