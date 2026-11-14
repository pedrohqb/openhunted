// SPDX-License-Identifier: GPL-3.0-or-later
//! Dois jogadores na casca (M15b, `docs/superpowers/specs/2026-11-08-m15b-multijogador-casca-design.md`):
//! `SMULT` → lista do modo → `SEGOC` × 2 → `SHVLD` (disputa) ou briefing (coop) → partida em
//! tela dividida → `SH2HS` (`docs/re/multijogador.md`). A campanha solo não é tocada.

use openhunted_engine::Outcome;
use openhunted_engine::character::Character;
use openhunted_engine::scene::{Duel, Mode as SceneMode};
use openhunted_engine::shell::choose::{self, Choose};
use openhunted_engine::shell::duo::{DuoResults, Line, Totals};
use openhunted_engine::shell::levels::{Entry, LevelSelect};
use openhunted_engine::shell::limits::Limits;
use openhunted_engine::shell::results::Choice;
use openhunted_engine::shell::{Party, Placement, Reply, Screen, missions_for};
use openhunted_formats::MissionInfo;

use super::{App, Back, PageKind, State};

/// Minutos do limite de tempo em ms (`0x497374` × 60 000).
const MINUTE_MS: u64 = 60_000;
/// Locutor da disputa pelo `F − S` (`0x444a53`): J1 maior, J2 maior, empate.
const ANNOUNCE_DUEL: [u16; 3] = [3790, 3792, 3773];
/// Locutor do coop: vitória, tempo esgotado, outra derrota; depois 3788 (vitória) ou 3787.
const ANNOUNCE_COOP: [u16; 3] = [3780, 3776, 3777];
const ANNOUNCE_COOP_WIN: u16 = 3788;
const ANNOUNCE_COOP_LOSS: u16 = 3787;

/// Estado dos modos de dois jogadores dentro do `App`.
#[derive(Default)]
pub(super) struct Dois {
    /// Modo da casca (`0x497300`/`0x497304`).
    pub(super) party: Party,
    /// A lista inteira do `ENGLEVEL.TXT` (as listas dos modos saem dela).
    pub(super) all: Vec<MissionInfo>,
    /// Missão corrente na lista do modo de dois jogadores.
    pub(super) index: Option<usize>,
    /// Jogador da vez na `SEGOC` (`0x480380`) e as escolhas (`0x497318`, `0x497320`).
    seat: usize,
    pub(super) picks: [(Character, u8); 2],
    /// Limites da `SHVLD` (`0x497374` minutos, `0x497378` pontos).
    pub(super) minutes: i32,
    pub(super) points: i32,
    /// O `SINPU` foi aberto pela `SEGOC` (DONE volta a ela).
    pub(super) input_from_choose: bool,
    /// Para onde o CANCEL da `SHVLD` volta: o `SH2HS` (REPLAY/NEXT) ou, sem ele, a `SEGOC`.
    limits_back: Option<Back>,
}

impl Dois {
    pub(super) fn new(all: Vec<MissionInfo>) -> Self {
        Self {
            all,
            picks: [choose::defaults(0), choose::defaults(1)],
            ..Default::default()
        }
    }
}

impl App {
    /// Modo da casca.
    pub fn party(&self) -> Party {
        self.m15.party
    }

    /// Missões do modo de dois jogadores corrente e o índice da escolhida.
    pub fn duo_missions(&self) -> Vec<MissionInfo> {
        missions_for(&self.m15.all, self.m15.party)
    }

    pub fn duo_index(&self) -> Option<usize> {
        self.m15.index
    }

    /// Escolhas da `SEGOC` e limites da `SHVLD`.
    pub fn picks(&self) -> [(Character, u8); 2] {
        self.m15.picks
    }

    pub fn limits(&self) -> (i32, i32) {
        (self.m15.minutes, self.m15.points)
    }

    /// Jogador da vez na `SEGOC`.
    pub fn choosing(&self) -> usize {
        self.m15.seat
    }

    /// Modo da cena pela casca: solo, coop ou a disputa com as escolhas e os limites.
    pub(super) fn scene_mode(&self) -> SceneMode {
        let d = &self.m15;
        match d.party {
            Party::Solo => SceneMode::Solo,
            Party::Coop => SceneMode::Coop,
            Party::Deathmatch => SceneMode::Deathmatch(Duel {
                chars: [d.picks[0].0, d.picks[1].0],
                colours: [d.picks[0].1, d.picks[1].1],
                points_limit: d.points.max(0),
                time_limit_ms: d.minutes.max(0) as u64 * MINUTE_MS,
            }),
        }
    }

    /// Título e descrição de uma fase pela lista inteira.
    pub(super) fn mission_texts(&self, file: &str) -> Option<(String, String)> {
        self.m15
            .all
            .iter()
            .find(|m| m.file.eq_ignore_ascii_case(file))
            .map(|m| (m.title.clone(), m.description.clone()))
    }

    /// `SLEVL` com a lista do modo `party` (o modo passa a ser ele).
    pub(super) fn open_levels(&mut self, party: Party) {
        self.m15.party = party;
        let (list, won): (Vec<MissionInfo>, Vec<bool>) = match party {
            Party::Solo => (
                self.missions.clone(),
                self.records.iter().map(|&r| r != 0).collect(),
            ),
            _ => {
                let l = self.duo_missions();
                let n = l.len();
                (l, vec![false; n])
            }
        };
        let entries: Vec<Entry> = list
            .iter()
            .zip(won.iter().chain(std::iter::repeat(&false)))
            .map(|(m, &w)| Entry {
                title: m.title.clone(),
                won: w,
                // `0x44143d`: 1 → retratos do Jake, outro → do Garathe
                garathe: m.player != 1,
                colors: self.dls_table(m.extra),
            })
            .collect();
        let sel = match party {
            Party::Solo => self.index,
            _ => self.m15.index,
        }
        .unwrap_or(0);
        let Some(f) = self.font.clone() else { return };
        self.open_page("SLEVL.ITF", Placement::Full, PageKind::Levels, |s, _| {
            Box::new(LevelSelect::new(s, f, entries, sel).with_party(party))
        });
    }

    /// LOAD na lista de dois jogadores: coop → briefing; disputa → `SEGOC` do jogador 1.
    pub(super) fn pick_duo(&mut self, i: usize) {
        let Some(m) = self.duo_missions().get(i).cloned() else {
            return;
        };
        self.m15.index = Some(i);
        self.level = (m.file.clone(), m.title, m.description);
        match self.m15.party {
            Party::Coop => {
                if let Err(e) = self.start_briefing(&m.file, Back::Menu) {
                    eprintln!("erro ao abrir {}: {e}", m.file);
                    self.set_notice(&e.to_string());
                }
            }
            _ => {
                self.m15.seat = 0;
                self.m15.limits_back = None;
                self.open_choose();
            }
        }
    }

    /// `SEGOC` do jogador da vez.
    pub(super) fn open_choose(&mut self) {
        let p = self.m15.seat;
        let device = self.devices()[p];
        let pick = self.m15.picks[p];
        let tables = [35, 36, 37, 38].map(|n| self.dls_table(n));
        let seed = 0x5e60c + p as u32;
        self.open_page("SEGOC.ITF", Placement::Full, PageKind::Choose, |s, _| {
            Box::new(Choose::new(s, p, device, pick, tables, seed))
        });
    }

    /// Respostas das telas de dois jogadores (as outras seguem em `dispatch_page`).
    pub(super) fn dispatch_duo(&mut self, kind: PageKind, reply: Reply) -> bool {
        match (kind, reply) {
            (PageKind::Multi | PageKind::Levels, Reply::Party(p)) => self.open_levels(p),
            (PageKind::Levels, Reply::Pick(i)) if self.m15.party.two() => self.pick_duo(i),
            (PageKind::Choose, Reply::Chosen(c, colour)) => {
                self.m15.picks[self.m15.seat] = (c, colour);
                if self.m15.seat == 0 {
                    self.m15.seat = 1;
                    self.open_choose();
                } else {
                    // depois do 2º: carrega a fase e abre a `SHVLD` (`0x443809`)
                    self.m15.seat = 0;
                    let file = self.level.0.clone();
                    self.start_limits(&file, None);
                }
            }
            (PageKind::Choose, Reply::Close) => {
                if self.m15.seat == 1 {
                    self.m15.seat = 0;
                    self.open_choose();
                } else {
                    self.open_levels(Party::Deathmatch);
                }
            }
            (PageKind::Choose, Reply::Devices) => {
                self.m15.input_from_choose = true;
                self.open_input();
            }
            (PageKind::Input, Reply::Close) if self.m15.input_from_choose => {
                self.m15.input_from_choose = false;
                self.open_choose();
            }
            (PageKind::Limits, Reply::Limits { minutes, points }) => {
                self.m15.minutes = minutes;
                self.m15.points = points;
                self.m15.limits_back = None;
                let mode = self.scene_mode();
                if let Some(v) = self.viewer.as_mut() {
                    v.set_mode(mode);
                }
                self.pending_ms = 0;
                self.state = State::Play {
                    pause: None,
                    end_ms: None,
                };
            }
            (PageKind::Limits, Reply::Close) => match self.m15.limits_back.take() {
                Some(back) => self.go_back(back),
                None => {
                    self.m15.seat = 0;
                    self.open_choose();
                }
            },
            (PageKind::Duo, Reply::Choice(c)) => self.duo_choice(c),
            _ => return false,
        }
        true
    }

    /// Carrega a fase da disputa e abre a `SHVLD` sobre o primeiro quadro dela (estado 6).
    pub(super) fn start_limits(&mut self, file: &str, back: Option<Back>) {
        let viewer = match self.load_viewer(file) {
            Ok((v, _)) => v,
            Err(e) => {
                eprintln!("erro ao abrir {file}: {e}");
                self.open_levels(Party::Deathmatch);
                self.set_notice(&e.to_string());
                return;
            }
        };
        let (Some(font), Ok(itf)) = (self.font.clone(), self.itf("SHVLD.ITF")) else {
            return;
        };
        let screen = match Screen::new(&itf, &mut self.assets, Placement::Centered) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("erro ao abrir SHVLD.ITF: {e}");
                return;
            }
        };
        let bar = self.assets.bitmap("INVBAR01.BMP").ok();
        let (minutes, points) = (self.m15.minutes, self.m15.points);
        let page = Limits::new(screen, font, &self.level.1, bar, minutes, points);
        self.set_viewer(viewer);
        self.m15.limits_back = back;
        self.state = State::Page {
            page: Box::new(page),
            kind: PageKind::Limits,
            notice: None,
        };
    }

    /// Volta a uma tela guardada (o resultado de antes do REPLAY/NEXT).
    pub(super) fn go_back(&mut self, back: Back) {
        match back {
            Back::Menu => self.go_menu(),
            Back::Results(mut r, index, level) => {
                self.index = index;
                self.level = level;
                r.from_game = false;
                self.state = State::Results(r);
            }
            Back::Page(mut page, kind, index, level) => {
                self.m15.index = index;
                self.level = level;
                page.leave_game();
                self.drop_viewer();
                self.state = State::Page {
                    page,
                    kind,
                    notice: None,
                };
            }
        }
    }

    /// Fim da partida de dois jogadores (`0x444174` → estado 16): locutor e `SH2HS`.
    pub(super) fn open_duo_results(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let Some(v) = self.viewer.as_ref() else {
            return Ok(());
        };
        let scene = v.scene();
        let outcome = v.outcome().unwrap_or(Outcome::Defeat);
        let deathmatch = self.m15.party == Party::Deathmatch;
        if deathmatch {
            self.sounds.push(match scene.winner() {
                Some(0) => ANNOUNCE_DUEL[0],
                Some(_) => ANNOUNCE_DUEL[1],
                None => ANNOUNCE_DUEL[2],
            });
        } else {
            let (first, then) = match (outcome, scene.timed_out()) {
                (Outcome::Victory, _) => (ANNOUNCE_COOP[0], ANNOUNCE_COOP_WIN),
                (Outcome::Defeat, true) => (ANNOUNCE_COOP[1], ANNOUNCE_COOP_LOSS),
                (Outcome::Defeat, false) => (ANNOUNCE_COOP[2], ANNOUNCE_COOP_LOSS),
            };
            self.sounds.extend([first, then]);
        }
        let line = |k: usize| {
            scene.players.get(k).map_or(Line::default(), |p| Line {
                kills: p.stats.kills,
                treasures: p.stats.treasures,
                secrets: p.stats.secrets,
                frags: p.stats.frags,
                suicides: p.stats.suicides,
            })
        };
        let lines = [line(0), line(1)];
        let totals = Totals {
            kills: scene.kills_total,
            ..Default::default()
        };
        let (title, font) = (self.level.1.clone(), self.font()?);
        self.open_page("SH2HS.ITF", Placement::Full, PageKind::Duo, |s, _| {
            Box::new(DuoResults::new(
                s, font, &title, outcome, deathmatch, lines, totals,
            ))
        });
        Ok(())
    }

    /// REPLAY, NEXT e MAIN do `SH2HS` (`0x443dd9`).
    fn duo_choice(&mut self, c: Choice) {
        if c == Choice::Main {
            self.go_menu();
            return;
        }
        let list = self.duo_missions();
        let index = match (c, self.m15.index) {
            // NEXT: a entrada seguinte da lista do modo, voltando ao início (Ruling)
            (Choice::Next, Some(i)) if !list.is_empty() => Some((i + 1) % list.len()),
            (_, i) => i,
        };
        let file = match (c, index.and_then(|i| list.get(i))) {
            (Choice::Next, Some(m)) => m.file.clone(),
            _ => self.level.0.clone(),
        };
        let State::Page { page, kind, .. } = std::mem::replace(
            &mut self.state,
            State::Play {
                pause: None,
                end_ms: None,
            },
        ) else {
            return;
        };
        let back = Back::Page(page, kind, self.m15.index, self.level.clone());
        if let Some(m) = index.and_then(|i| list.get(i)) {
            self.level = (m.file.clone(), m.title.clone(), m.description.clone());
        }
        self.m15.index = index;
        match self.m15.party {
            Party::Deathmatch => self.start_limits(&file, Some(back)),
            _ => {
                if let Err(e) = self.start_briefing(&file, back) {
                    eprintln!("erro ao abrir {file}: {e}");
                }
            }
        }
        if matches!(self.state, State::Play { .. }) {
            self.go_menu();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{END_DELAY_MS, JoyRaw, Key, Mode};
    use super::*;
    use crate::storage::{Storage, tests::temp};
    use openhunted_engine::Framebuffer;
    use openhunted_engine::devices::Device;
    use openhunted_engine::input::{JUMP, LEFT, RIGHT, SHOOT};
    use openhunted_engine::shell::choose::ids as segoc;
    use openhunted_engine::shell::duo::ids as h2h;
    use openhunted_engine::shell::heroes::info_ids;
    use openhunted_engine::shell::levels;
    use openhunted_engine::shell::limits::ids as dmm;
    use openhunted_engine::shell::menu;
    use openhunted_engine::shell::views;

    fn app(tag: &str) -> Option<(App, std::path::PathBuf)> {
        let Some(p) = std::env::var_os("OPENHUNTED_DATA") else {
            eprintln!("pulado: OPENHUNTED_DATA não definido");
            return None;
        };
        let root = temp(tag);
        let mut a = App::load(std::path::Path::new(&p), None).unwrap();
        a.set_storage(Storage::at(&root));
        Some((a, root))
    }

    fn run(a: &mut App, ticks: u32) {
        for _ in 0..ticks {
            a.advance(50);
        }
    }

    fn click(a: &mut App, id: u16) {
        let r = a.screen().unwrap().rect(id).unwrap();
        a.pointer_move(r.x + r.w / 2, r.y + r.h / 2);
        a.pointer_button(true);
        run(a, 1);
        a.pointer_button(false);
        run(a, 6);
    }

    /// Menu → `SMULT` → o modo → `SLEVL` → LOAD da 1ª missão da lista.
    fn load_first(a: &mut App, button: u16) {
        click(a, menu::MULTI_PLAYER);
        assert_eq!(a.page_kind(), Some(PageKind::Multi));
        click(a, button);
        assert_eq!(a.page_kind(), Some(PageKind::Levels));
        click(a, levels::LOAD);
    }

    /// Coop pronto para jogar (briefing → GO).
    fn coop(a: &mut App) {
        load_first(a, info_ids::MULTI_COOP);
        assert_eq!(a.mode(), Mode::Briefing);
        a.key_down(Key::Enter);
        run(a, 6);
        assert_eq!(a.mode(), Mode::Play);
    }

    #[test]
    fn deathmatch_flow_from_menu_to_results() {
        let Some((mut a, root)) = app("duo-dm") else {
            return;
        };
        let records = a.records.clone();
        load_first(&mut a, info_ids::MULTI_H2H);
        assert_eq!(a.party(), Party::Deathmatch);
        let list = a.duo_missions();
        assert_eq!(list.len(), 20);
        assert!(list.iter().all(|m| m.player == 3));
        assert_eq!(a.level_file(), list[0].file);
        // `SEGOC`: jogador 1 com o Garathe branco; CANCEL no 2º volta ao 1º
        assert_eq!(a.page_kind(), Some(PageKind::Choose));
        assert_eq!(a.choosing(), 0);
        click(&mut a, segoc::PICK[1]);
        click(&mut a, segoc::WHITE);
        click(&mut a, segoc::OK);
        assert_eq!((a.page_kind(), a.choosing()), (Some(PageKind::Choose), 1));
        a.key_down(Key::Escape);
        run(&mut a, 1);
        assert_eq!(a.choosing(), 0, "CANCEL no 2º volta ao 1º");
        assert_eq!(a.picks()[0], (Character::Garathe, 37));
        // o botão do dispositivo leva ao `SINPU`, que volta à `SEGOC`
        click(&mut a, segoc::INPUT);
        assert_eq!(a.page_kind(), Some(PageKind::Input));
        a.key_down(Key::Escape);
        run(&mut a, 1);
        assert_eq!((a.page_kind(), a.choosing()), (Some(PageKind::Choose), 0));
        click(&mut a, segoc::OK);
        click(&mut a, segoc::OK); // jogador 2: Garathe marrom (padrão)
        assert_eq!(a.picks()[1], (Character::Garathe, 36));
        // `SHVLD` sobre a fase já carregada, com os dois jogadores
        assert_eq!(a.page_kind(), Some(PageKind::Limits));
        let v = a.viewer().expect("fase carregada");
        assert!(v.split());
        assert_eq!(v.scene().players.len(), 2);
        let mut rgba = vec![0u8; 640 * 480 * 4];
        a.render_rgba(&mut rgba);
        click(&mut a, dmm::TIME_UP);
        click(&mut a, dmm::GO);
        assert_eq!(a.mode(), Mode::Play);
        assert_eq!(a.limits(), (1, 0));
        let duel = *a.viewer().unwrap().scene().mode.duel().unwrap();
        assert_eq!(duel.time_limit_ms, 60_000);
        assert_eq!(duel.chars, [Character::Garathe; 2]);
        assert_eq!(duel.colours, [37, 36]);
        // a pausa congela o relógio real
        run(&mut a, 20);
        a.key_down(Key::Pause);
        let t = a.viewer().unwrap().clock_ms();
        run(&mut a, 200);
        assert_eq!(a.viewer().unwrap().clock_ms(), t);
        a.focus_lost();
        a.key_down(Key::Enter);
        assert_eq!(a.mode(), Mode::Play);
        // um minuto de relógio real encerra a partida
        let mut n = 0;
        while a.mode() == Mode::Play && n < 2000 {
            a.advance(50);
            n += 1;
        }
        let v = a.viewer().map(|v| v.clock_ms());
        assert_eq!(a.page_kind(), Some(PageKind::Duo), "relógio {v:?}");
        let snd = a.take_sounds();
        assert!(
            snd.iter().any(|s| [3790, 3792, 3773].contains(s)),
            "{snd:?}"
        );
        assert_eq!(a.records, records, "a campanha solo não muda");
        // NEXT: a próxima da lista, direto na `SHVLD`; CANCEL volta ao resultado
        a.key_down(Key::Enter); // salta a contagem
        click(&mut a, h2h::NEXT);
        assert_eq!(a.page_kind(), Some(PageKind::Limits));
        assert_eq!(a.duo_index(), Some(1));
        assert_eq!(a.level_file(), list[1].file);
        a.key_down(Key::Escape);
        run(&mut a, 1);
        assert_eq!(a.page_kind(), Some(PageKind::Duo));
        assert_eq!(a.duo_index(), Some(0));
        a.key_down(Key::Escape);
        run(&mut a, 1);
        assert_eq!(a.mode(), Mode::Menu, "Esc vale depois de voltar");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn coop_flow_briefing_split_screen_and_results() {
        let Some((mut a, root)) = app("duo-coop") else {
            return;
        };
        coop(&mut a);
        assert_eq!(a.party(), Party::Coop);
        let list = a.duo_missions();
        assert_eq!(list.len(), 15);
        let v = a.viewer().unwrap();
        assert!(v.split());
        // geometria: duas vistas, cada uma com a câmera no seu jogador
        assert_eq!(v.views(), views::layout(2));
        for i in 0..2 {
            let f = v.scene().focus_of(i).unwrap();
            let c = v.camera_of(i);
            let g = v.views()[i].game;
            assert!(
                f.0 >= c.0 && f.0 < c.0 + g.w && f.1 >= c.1 && f.1 < c.1 + g.h,
                "jogador {i} na vista dele: foco {f:?}, câmera {c:?}"
            );
            assert_eq!(v.hud_state_of(i).score, None, "coop: o cronômetro");
        }
        let fb = a.render().clone();
        for bar in views::layout(2).iter().map(|v| v.bar) {
            let rows = &fb.pixels[bar.y as usize * 640..(bar.y + bar.h) as usize * 640];
            assert!(
                rows.iter().filter(|&&p| p != 0).count() > 10_000,
                "barra {bar:?}"
            );
        }
        a.viewer_mut().unwrap().scene_mut().outcome = Some(Outcome::Victory);
        a.advance(END_DELAY_MS + 50);
        assert_eq!(a.page_kind(), Some(PageKind::Duo));
        let snd = a.take_sounds();
        assert!(snd.windows(2).any(|w| w == [3780, 3788]), "{snd:?}");
        // REPLAY: a mesma fase pelo briefing; CANCEL volta ao resultado; MAIN vai ao menu
        a.key_down(Key::Enter);
        click(&mut a, h2h::REPLAY);
        assert_eq!(a.mode(), Mode::Briefing);
        assert_eq!(a.level_file(), list[0].file);
        a.key_down(Key::Escape);
        assert_eq!(a.page_kind(), Some(PageKind::Duo));
        click(&mut a, h2h::MAIN);
        assert_eq!(a.mode(), Mode::Menu);
        assert!(a.records.iter().all(|&r| r == 0));
        assert!(a.save_path().is_none(), "sem jogo salvo");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Encerra a partida corrente com `outcome` e espera o `SH2HS`.
    fn finish(a: &mut App, outcome: Outcome) {
        a.viewer_mut().unwrap().scene_mut().outcome = Some(outcome);
        a.advance(END_DELAY_MS + 50);
        assert_eq!(a.page_kind(), Some(PageKind::Duo));
        a.key_down(Key::Enter); // salta a contagem
    }

    #[test]
    fn limits_cancel_replay_and_next_wrap_in_the_deathmatch() {
        let Some((mut a, root)) = app("duo-wrap") else {
            return;
        };
        load_first(&mut a, info_ids::MULTI_H2H);
        click(&mut a, segoc::OK);
        click(&mut a, segoc::OK);
        assert_eq!(a.page_kind(), Some(PageKind::Limits));
        // CANCEL da `SHVLD` sem resultado antes: a `SEGOC` do jogador 1
        a.key_down(Key::Escape);
        run(&mut a, 1);
        assert_eq!((a.page_kind(), a.choosing()), (Some(PageKind::Choose), 0));
        click(&mut a, segoc::OK);
        click(&mut a, segoc::OK);
        click(&mut a, dmm::GO);
        assert_eq!(a.mode(), Mode::Play);
        finish(&mut a, Outcome::Victory);
        // REPLAY na disputa: a mesma fase, direto na `SHVLD`
        click(&mut a, h2h::REPLAY);
        assert_eq!(a.page_kind(), Some(PageKind::Limits));
        assert_eq!(a.duo_index(), Some(0));
        let list = a.duo_missions();
        assert_eq!(a.level_file(), list[0].file);
        click(&mut a, dmm::GO);
        finish(&mut a, Outcome::Victory);
        // NEXT na última da lista volta à primeira (Ruling)
        a.m15.index = Some(list.len() - 1);
        click(&mut a, h2h::NEXT);
        assert_eq!(a.page_kind(), Some(PageKind::Limits));
        assert_eq!(a.duo_index(), Some(0));
        assert_eq!(a.level_file(), list[0].file);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn coop_time_out_announces_and_keeps_the_campaign() {
        let Some((mut a, root)) = app("duo-timeout") else {
            return;
        };
        coop(&mut a);
        let s = a.viewer_mut().unwrap().scene_mut();
        (s.time_limit, s.timer) = (60, 0);
        a.take_sounds();
        finish(&mut a, Outcome::Defeat);
        let snd = a.take_sounds();
        assert!(snd.windows(2).any(|w| w == [3776, 3787]), "{snd:?}");
        assert!(a.records.iter().all(|&r| r == 0));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn split_views_draw_each_half_through_the_scene_view() {
        let Some((mut a, root)) = app("duo-views") else {
            return;
        };
        coop(&mut a);
        run(&mut a, 4);
        let v = a.viewer_mut().unwrap();
        // afasta as câmeras para que as metades mostrem pontos diferentes da fase
        let s = v.scene_mut();
        let f0 = s.focus_of(0).unwrap();
        let (w, layer) = (s.width, s.layer_of(1).unwrap());
        s.place_player(1, (if f0.0 > w / 2 { 200 } else { w - 200 }, f0.1), layer);
        v.follow();
        let fb = v.render().clone();
        for (i, r) in views::layout(2).iter().enumerate() {
            // a vista de cada metade: 640×203 no lugar dela, duas vistas (referência 240)
            let view = v.view(i);
            assert_eq!(view.screen, r.game);
            assert_eq!(view.size, (640, 203));
            assert_eq!((view.views, view.player), (2, Some(i)));
            let l = view.state.layer;
            if view.state.counter == 0 && l > 0 {
                let raw = i64::from(view.state.raw_distance(l - 1));
                let want = (raw * 240 / 203) as i32;
                assert_eq!(view.layer_distance(l - 1), if want == 0 { 1 } else { want });
            }
            // o quadro da metade é o da `Scene::draw_view` direto no quadro, sem cópia
            let mut want = Framebuffer::new(640, 480);
            want.fill_rect(r.game.x, r.game.y, r.game.w, r.game.h, v.background());
            let (scene, assets) = v.parts();
            scene.draw_view(assets, &mut want, &view, false);
            for y in r.game.y..r.game.y + r.game.h {
                let row = y as usize * 640;
                assert_eq!(
                    fb.pixels[row..row + 640],
                    want.pixels[row..row + 640],
                    "vista {i}, linha {y}"
                );
            }
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn two_devices_read_the_same_keyboard_and_a_joystick() {
        let Some((mut a, root)) = app("duo-input") else {
            return;
        };
        coop(&mut a);
        // teclado 1 (setas) e teclado 2 (I M J K…) no mesmo estado do teclado
        a.m9.config.devices = [Some(Device::Keyboard1), Some(Device::Keyboard2)];
        let keys = |vk: u8| vk == 39 || vk == 74;
        a.read_input(&keys, 0);
        let v = a.viewer().unwrap();
        assert_eq!(v.input().0, RIGHT);
        assert_eq!(v.input2().0, LEFT);
        // o toque rápido do pulo do jogador 2 (Enter, `InputKeys2`)
        assert!(!a.vk_down(13));
        assert_eq!(a.viewer().unwrap().input2().1 & JUMP, JUMP);
        assert_eq!(a.viewer().unwrap().input().1 & JUMP, 0);
        // jogador 2 no joystick 1: eixo à direita e o botão 1 (atirar)
        a.m9.config.devices[1] = Some(Device::Joystick(1));
        a.set_joysticks(vec![JoyRaw {
            x: 127,
            y: 0,
            buttons: 1,
            count: 6,
        }]);
        a.read_input(&keys, 0);
        let v = a.viewer().unwrap();
        assert_eq!(v.input().0, RIGHT);
        assert_eq!(v.input2().0, RIGHT | SHOOT);
        // as entradas chegam aos dois corpos: o 2 anda pelo eixo do joystick (o 1 parado)
        a.set_joysticks(vec![JoyRaw {
            x: -128,
            count: 6,
            ..Default::default()
        }]);
        a.read_input(&|_| false, 0);
        let at = |a: &App| {
            let s = a.viewer().unwrap().scene();
            (s.focus_of(0).unwrap().0, s.focus_of(1).unwrap().0)
        };
        let (p0, p1) = at(&a);
        run(&mut a, 10);
        let (q0, q1) = at(&a);
        assert_eq!(q0, p0, "o jogador 1 parado");
        assert!(q1 < p1, "o jogador 2 andou para a esquerda ({p1} → {q1})");
        // a câmera da vista 2 persegue o jogador com o passo máximo de 18 px por tique
        let start = a.viewer().unwrap().camera_of(1);
        let mut c = start;
        for _ in 0..40 {
            a.read_input(&|_| false, 0);
            a.advance(50);
            let d = a.viewer().unwrap().camera_of(1);
            assert!(
                (d.0 - c.0).abs() <= 18 && (d.1 - c.1).abs() <= 18,
                "{c:?} → {d:?}"
            );
            c = d;
        }
        assert_ne!(c, start, "a câmera andou");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn sounds_are_heard_from_the_nearest_view() {
        use crate::audio::tests::Log;
        use openhunted_engine::Event;
        use openhunted_engine::sound::Cue;
        let Some((mut a, root)) = app("duo-sound") else {
            return;
        };
        coop(&mut a);
        let log = Log::default();
        let v = a.viewer_mut().unwrap();
        v.set_sound(crate::audio::Sfx::new(Box::new(log.clone())));
        // afasta o jogador 2 e espera as câmeras chegarem
        let s = v.scene_mut();
        let (w, layer) = (s.width, s.layer_of(1).unwrap());
        let f0 = s.focus_of(0).unwrap();
        let far = if f0.0 > w / 2 { 200 } else { w - 200 };
        s.place_player(1, (far, f0.1 - 60), layer);
        v.advance(50);
        let g = v.views()[1].game;
        let c2 = v.camera_of(1);
        let ear2 = (c2.0 + g.w / 2, c2.1 + g.h / 2);
        let l2 = v.scene().layer_of(1).unwrap();
        v.scene_mut()
            .events
            .push(Event::Sound(Cue::at(3040, 9, ear2, l2)));
        log.0.borrow_mut().clear();
        v.advance(50);
        assert!(
            log.0.borrow().contains(&(3040, 9, 100, 0)),
            "ouvido da vista 2: {:?}",
            log.0.borrow()
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}
