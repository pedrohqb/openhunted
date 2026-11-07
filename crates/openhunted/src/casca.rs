// SPDX-License-Identifier: GPL-3.0-or-later
//! A casca da M9 sobre o `App` (`docs/superpowers/specs/2026-11-06-m9-casca-design.md`):
//! as telas restantes do `SMAIN` (`docs/re/menus-restantes.md`), opções gravadas, jogo
//! salvo e lista de heróis (`opcoes-salvamento.md`), peças do carro com `S_CAR`, filmes,
//! créditos e música (`videos-musica.md`).

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;

use openhunted_engine::shell::config::{Config, Film};
use openhunted_engine::shell::credits::Credits;
use openhunted_engine::shell::generic::GenericBox;
use openhunted_engine::shell::heroes::{Car, Heroes, Info, InfoKind, NameEntry};
use openhunted_engine::shell::levels::{Entry, LevelSelect};
use openhunted_engine::shell::options::Options;
use openhunted_engine::shell::results::Results;
use openhunted_engine::shell::{Dir, Input, Page, Placement, Reply, Screen, menu, next_mission};
use openhunted_engine::{TICK_MS, display_palette};
use openhunted_formats::{Palette, SaveGame, Sound, parse_text_list};

use super::{App, Back, Key, State};
use crate::audio::{Pcm, Sfx};
use crate::music::{Music, track_name};
use crate::storage::{self, Storage, clean_name, read_save};
use crate::video::Movie;

/// Textos do `SYSTEM.RES` usados aqui.
const INVALID_SAVE: u16 = 10009;
const LOAD_GAME: u16 = 10041;
const SAVE_GAME_AS: u16 = 10042;
/// Vozes da sequência do meio (`0x444174`) e a espera antes delas.
const VOICE_BEFORE: u16 = 4102;
const VOICE_AFTER: u16 = 4103;
const BLACK_MS: u32 = 1000;
/// Peças para os filmes do meio e do final.
const MIDDLE_PIECE: i32 = 7;
const LAST_PIECE: i32 = 15;
/// Teclas do port na ordem dos botões 17001–17010 do `SKEYB` (códigos VK; o nome é a linha
/// do `KEYS.RES`): cima, baixo, esquerda, direita, Ctrl, espaço, X, PgUp, Z, PgDn.
const PORT_KEYS: [usize; 10] = [38, 40, 37, 39, 17, 32, 88, 33, 90, 34];
/// Cor padrão do jogador 1 (`0x497320 = 35`, `DLS.TBL`).
const PLAYER1_COLORS: u16 = 35;

/// Qual tela da M9 está aberta.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageKind {
    Options,
    Levels,
    Heroes,
    Input,
    Keys,
    Multi,
    About,
    Car,
    /// Nome do herói (`SHIGH`) sobre o resultado.
    HeroName,
    /// Nome do jogo salvo sobre o resultado.
    SaveName,
    /// Nome do jogo a carregar (menu → Restore).
    LoadName,
}

/// Um passo da sequência de filmes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Black(u32),
    Voice(u16),
    Film(Film),
    Credits,
}

enum Running {
    Wait(u32),
    Film(Movie),
    Credits(Box<Credits>, u32),
}

/// O que vem depois da sequência.
enum After {
    Menu,
    Results(Box<Results>),
}

/// Sequência em curso (filmes, vozes, tela preta, créditos).
pub struct Show {
    steps: VecDeque<Step>,
    cur: Option<Running>,
    then: After,
}

/// Fábrica de filmes pelo nome do AVI.
pub type Movies = Box<dyn FnMut(&str) -> Option<Movie>>;

/// Estado da M9 dentro do `App`.
#[derive(Default)]
pub(super) struct Casca {
    storage: Option<Storage>,
    pub(super) config: Config,
    /// Paleta do jogo pelo brilho (só com a configuração ligada).
    pub(super) game_lut: Option<[[u8; 4]; 256]>,
    /// Campanha: nome do jogador, id na lista de heróis, peças, arquivo salvo.
    player: String,
    hero_id: i32,
    pieces: i32,
    save_path: Option<PathBuf>,
    /// O recorde da missão subiu nesta vitória.
    pub(super) improved: bool,
    /// A contagem do resultado já chegou a 100 (nome e gravação feitos).
    count_done: bool,
    /// Resultado à espera sob uma tela (carro, nome) e o filme depois do carro.
    pending: Option<Box<Results>>,
    film: Option<Film>,
    music: Option<Music>,
    movies: Option<Movies>,
    pub(super) track_title: Option<String>,
}

fn nav(k: Key) -> Option<Input> {
    match k {
        Key::Left => Some(Input::Nav(Dir::Left)),
        Key::Right => Some(Input::Nav(Dir::Right)),
        Key::Up => Some(Input::Nav(Dir::Up)),
        Key::Down => Some(Input::Nav(Dir::Down)),
        Key::Enter | Key::Space => Some(Input::Activate),
        Key::Escape => Some(Input::Cancel),
        _ => None,
    }
}

fn placeholder() -> State {
    State::Play {
        pause: None,
        end_ms: None,
    }
}

impl App {
    // ---- configuração e saídas ----

    /// Liga os arquivos do jogador: lê a configuração e passa a gravar opções, heróis e
    /// jogos salvos.
    pub fn set_storage(&mut self, st: Storage) {
        let c = st.load_config();
        self.m9.storage = Some(st);
        self.apply_config(c, false);
    }

    pub fn config(&self) -> &Config {
        &self.m9.config
    }

    /// Liga a música (saída própria) com a pasta de faixas, se houver.
    pub fn set_music(&mut self, out: Box<dyn crate::audio::SoundOut>, tracks: Vec<PathBuf>) {
        let menu = self
            .assets
            .archive()
            .get("SX_3000.WAV")
            .ok()
            .and_then(|d| Sound::parse(d).ok())
            .and_then(|s| Pcm::decode(&s))
            .map(Arc::new);
        let mut m = Music::new(out, menu, tracks);
        m.set_gain(self.m9.config.music_gain());
        self.m9.music = Some(m);
    }

    /// Liga os filmes (sem isso, eles são pulados).
    pub fn set_movies(&mut self, movies: Movies) {
        self.m9.movies = Some(movies);
    }

    /// Efeitos com o volume da configuração.
    pub fn set_sound_with_volume(&mut self, mut sfx: Sfx) {
        sfx.set_volume(self.m9.config.sfx_gain());
        self.set_sound(sfx);
    }

    fn apply_config(&mut self, c: Config, save: bool) {
        if save {
            if let Some(st) = &self.m9.storage {
                if let Err(e) = st.save_config(&c) {
                    eprintln!("aviso: não gravou {}: {e}", st.config_file.display());
                }
            }
        }
        if let Some(s) = self.sfx.as_mut() {
            s.set_volume(c.sfx_gain());
        }
        if let Some(m) = self.m9.music.as_mut() {
            m.set_gain(c.music_gain());
        }
        if self.m9.storage.is_some() {
            self.m9.game_lut = self
                .assets
                .archive()
                .get(&c.palette())
                .ok()
                .and_then(|d| Palette::parse(d).ok())
                .map(|p| {
                    let p = display_palette(&p);
                    let mut lut = [[0u8; 4]; 256];
                    for (l, c) in lut.iter_mut().zip(p.0.iter()) {
                        *l = [c[0], c[1], c[2], 255];
                    }
                    lut
                });
        }
        self.m9.config = c;
    }

    /// Começa a abertura (o filme `hh*.avi`); sem filmes, fica no menu.
    pub fn play_intro(&mut self) {
        if self.m9.movies.is_some() {
            self.start_show(vec![Step::Film(Film::Intro)], After::Menu);
        }
    }

    /// Zera a campanha (`0x440fe0`): nome, herói, peças e o arquivo salvo.
    pub(super) fn new_campaign(&mut self) {
        let m = &mut self.m9;
        m.player.clear();
        m.hero_id = 0;
        m.pieces = 0;
        m.save_path = None;
    }

    /// Peças do carro ganhas nesta campanha.
    pub fn pieces(&self) -> i32 {
        self.m9.pieces
    }

    /// Caminho do jogo salvo da campanha.
    pub fn save_path(&self) -> Option<&std::path::Path> {
        self.m9.save_path.as_deref()
    }

    pub fn page_kind(&self) -> Option<PageKind> {
        match &self.state {
            State::Page { kind, .. } => Some(*kind),
            _ => None,
        }
    }

    /// Há uma caixa de aviso aberta.
    pub fn notice(&self) -> Option<&str> {
        match &self.state {
            State::Page {
                notice: Some(b), ..
            } => Some(&b.text),
            _ => None,
        }
    }

    /// Passos da sequência que faltam (testes).
    pub fn show_steps(&self) -> Option<Vec<Step>> {
        match &self.state {
            State::Show(s) => Some(s.steps.iter().copied().collect()),
            _ => None,
        }
    }

    // ---- telas ----

    fn open_page(
        &mut self,
        itf: &str,
        placement: Placement,
        kind: PageKind,
        make: impl FnOnce(Screen, &mut App) -> Box<dyn Page>,
    ) {
        let screen = self
            .itf(itf)
            .and_then(|i| Ok(Screen::new(&i, &mut self.assets, placement)?));
        match screen {
            Ok(s) => {
                let page = make(s, self);
                self.drop_viewer();
                self.state = State::Page {
                    page,
                    kind,
                    notice: None,
                };
            }
            Err(e) => eprintln!("erro ao abrir {itf}: {e}"),
        }
    }

    /// Botões do menu principal além de New Game e Exit.
    pub(super) fn menu_button(&mut self, id: u16) {
        self.last_menu_button = id;
        let font = self.font.clone();
        match id {
            menu::RESTORE if self.m9.storage.is_some() => {
                let title = self.sys_text(LOAD_GAME);
                let last = self
                    .m9
                    .storage
                    .as_ref()
                    .and_then(Storage::latest_save)
                    .unwrap_or_default();
                if let Some(f) = font {
                    self.open_page(
                        "SHIGH.ITF",
                        Placement::Centered,
                        PageKind::LoadName,
                        |s, _| Box::new(NameEntry::new(s, f, Some(title), &last)),
                    );
                }
            }
            menu::CHOOSE_LEVEL => {
                let entries: Vec<Entry> = self
                    .missions
                    .iter()
                    .zip(&self.records)
                    .map(|(m, &r)| Entry {
                        title: m.title.clone(),
                        won: r != 0,
                        garathe: m.player == 2,
                        colors: self.dls_table(m.extra),
                    })
                    .collect();
                let sel = self.index.unwrap_or(0);
                if let Some(f) = font {
                    self.open_page("SLEVL.ITF", Placement::Full, PageKind::Levels, |s, _| {
                        Box::new(LevelSelect::new(s, f, entries, sel))
                    });
                }
            }
            menu::OPTIONS => {
                let c = self.m9.config.clone();
                self.open_page("SOPTI.ITF", Placement::Full, PageKind::Options, |s, _| {
                    Box::new(Options::new(s, c))
                });
            }
            menu::INPUT_DEVICES => self.open_input(),
            menu::HERO_LIST => {
                let rows = self
                    .m9
                    .storage
                    .as_ref()
                    .map(|st| st.heroes().heroes)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|h| (h.name, h.score))
                    .collect();
                if let Some(f) = font {
                    self.open_page("SHERO.ITF", Placement::Full, PageKind::Heroes, |s, _| {
                        Box::new(Heroes::new(s, f, rows))
                    });
                }
            }
            menu::HELP => {
                let v = env!("CARGO_PKG_VERSION").to_string();
                self.open_page(
                    "SABOUT.ITF",
                    Placement::Centered,
                    PageKind::About,
                    |s, _| Box::new(Info::new(s, InfoKind::About, font, &[v])),
                );
            }
            menu::MULTI_PLAYER => {
                self.open_page("SMULT.ITF", Placement::Full, PageKind::Multi, |s, _| {
                    Box::new(Info::new(s, InfoKind::Multi, None, &[]))
                });
            }
            _ => {}
        }
    }

    fn open_input(&mut self) {
        self.open_page("SINPU.ITF", Placement::Full, PageKind::Input, |s, _| {
            Box::new(Info::new(s, InfoKind::Input, None, &[]))
        });
    }

    fn open_keys(&mut self) {
        let keys = self
            .assets
            .archive()
            .get("KEYS.RES")
            .map(parse_text_list)
            .unwrap_or_default();
        let names: Vec<String> = PORT_KEYS
            .iter()
            .map(|&vk| keys.get(vk - 1).cloned().unwrap_or_default())
            .collect();
        let font = self.font.clone();
        let colors = self.dls_table(PLAYER1_COLORS);
        self.open_page("SKEYB.ITF", Placement::Full, PageKind::Keys, |s, _| {
            let mut i = Info::new(s, InfoKind::Keys, font, &names);
            i.screen.set_remap(
                openhunted_engine::shell::heroes::info_ids::KEYS_CHARACTER,
                colors,
            );
            Box::new(i)
        });
    }

    /// Tabela de cor `n` do `DLS.TBL` (48 × 256 bytes).
    fn dls_table(&self, n: u16) -> Option<std::rc::Rc<[u8; 256]>> {
        let d = self.assets.archive().get("DLS.TBL").ok()?;
        let t: [u8; 256] = d.get(usize::from(n) * 256..)?.get(..256)?.try_into().ok()?;
        Some(std::rc::Rc::new(t))
    }

    fn set_notice(&mut self, text: &str) {
        let okay = self.sys_text(super::text::OKAY);
        let (Some(b), Some(f)) = (self.generic.clone(), self.font.clone()) else {
            return;
        };
        let p = self.last_pointer;
        if let State::Page { notice, .. } = &mut self.state {
            let mut g = GenericBox::new(&b, &f, text, &okay);
            g.pointer(p.0, p.1);
            *notice = Some(g);
        }
    }

    fn go_menu(&mut self) {
        if let Err(e) = self.open_menu() {
            eprintln!("erro: {e}");
            self.quit = true;
        }
    }

    fn dispatch_page(&mut self, reply: Option<Reply>) {
        let (Some(reply), Some(kind)) = (reply, self.page_kind()) else {
            return;
        };
        match (kind, reply) {
            (_, Reply::Unavailable) => self.set_notice(openhunted_engine::shell::page::UNAVAILABLE),
            (PageKind::Options, Reply::Config(c)) => self.apply_config(c, true),
            (PageKind::Levels, Reply::Pick(i)) => {
                self.index = Some(i);
                self.start_mission(Back::Menu);
            }
            (PageKind::Input, Reply::Customize) => self.open_keys(),
            (PageKind::Keys, Reply::Close) => self.open_input(),
            (PageKind::Car, Reply::Close) => match self.m9.film.take() {
                Some(film) => {
                    let steps = match film {
                        Film::Middle => vec![
                            Step::Black(BLACK_MS),
                            Step::Voice(VOICE_BEFORE),
                            Step::Film(Film::Middle),
                            Step::Voice(VOICE_AFTER),
                        ],
                        _ => vec![Step::Film(film), Step::Credits],
                    };
                    let then = match self.m9.pending.take() {
                        Some(r) => After::Results(r),
                        None => After::Menu,
                    };
                    self.start_show(steps, then);
                }
                None => self.back_to_results(),
            },
            (PageKind::HeroName, Reply::Name(n)) => {
                let n = n.trim().to_string();
                if !n.is_empty() {
                    self.m9.player = n;
                    self.record_hero();
                }
                self.save_step();
            }
            (PageKind::HeroName, Reply::Close) => self.save_step(),
            (PageKind::SaveName, Reply::Name(n)) => {
                let n = clean_name(&n);
                if let (false, Some(st)) = (n.is_empty(), &self.m9.storage) {
                    let p = st.save_path(&n);
                    self.write_save(p);
                }
                self.back_to_results();
            }
            (PageKind::SaveName, Reply::Close) => self.back_to_results(),
            (PageKind::LoadName, Reply::Name(n)) => {
                if !self.restore(&n) {
                    let t = self.sys_text(INVALID_SAVE);
                    self.set_notice(&t);
                }
            }
            (_, Reply::Close) => self.go_menu(),
            _ => {}
        }
    }

    // ---- entrada das telas e da sequência ----

    pub(super) fn casca_key(&mut self, key: Key) {
        let reply = match &mut self.state {
            State::Show(s) => {
                s.skip();
                return;
            }
            State::Page {
                notice: notice @ Some(_),
                ..
            } => {
                if matches!(key, Key::Enter | Key::Space | Key::Escape) {
                    *notice = None;
                }
                return;
            }
            State::Page { page, .. } => {
                let input = if page.wants_text() {
                    match key {
                        Key::Backspace => {
                            page.backspace();
                            None
                        }
                        Key::Space => None,
                        k => nav(k),
                    }
                } else {
                    nav(key)
                };
                input.and_then(|i| page.handle(i, &mut self.sounds))
            }
            _ => return,
        };
        self.dispatch_page(reply);
    }

    pub(super) fn casca_pointer(&mut self, pos: Option<(i32, i32)>, button: Option<bool>) {
        let reply = match &mut self.state {
            State::Show(s) => {
                if button == Some(true) {
                    s.skip();
                }
                return;
            }
            State::Page {
                notice: Some(b), ..
            } => {
                if let Some((x, y)) = pos {
                    b.pointer(x, y);
                }
                if let Some(d) = button {
                    if b.click(d) {
                        if let State::Page { notice, .. } = &mut self.state {
                            *notice = None;
                        }
                    }
                }
                return;
            }
            State::Page { page, .. } => {
                let i = match (pos, button) {
                    (Some((x, y)), _) => Input::PointerMove(x, y),
                    (_, Some(true)) => Input::PointerDown,
                    _ => Input::PointerUp,
                };
                page.handle(i, &mut self.sounds)
            }
            _ => return,
        };
        self.dispatch_page(reply);
    }

    pub(super) fn casca_tick(&mut self) {
        let reply = match &mut self.state {
            State::Page {
                page, notice: None, ..
            } => page.handle(Input::Tick, &mut self.sounds),
            _ => None,
        };
        self.dispatch_page(reply);
    }

    /// Texto digitado (entrada de texto da SDL), para o campo de edição.
    pub fn text_input(&mut self, s: &str) {
        if let State::Page {
            page, notice: None, ..
        } = &mut self.state
        {
            page.text(s);
        }
    }

    /// A tela aberta recebe texto (a janela liga a entrada de texto).
    pub fn wants_text(&self) -> bool {
        matches!(&self.state, State::Page { page, notice: None, .. } if page.wants_text())
    }

    pub(super) fn casca_render(&mut self) {
        self.fb.clear(0);
        match &self.state {
            State::Page { page, kind, notice } => {
                if matches!(kind, PageKind::HeroName | PageKind::SaveName) {
                    if let Some(r) = &self.m9.pending {
                        r.draw(&mut self.fb);
                    }
                }
                page.draw(&mut self.fb);
                if let (Some(b), Some(bmp), Some(font)) = (notice, &self.generic, &self.font) {
                    b.draw(&mut self.fb, bmp, font);
                }
            }
            State::Show(s) => {
                if let Some(Running::Credits(c, _)) = &s.cur {
                    c.draw(&mut self.fb);
                }
            }
            _ => {}
        }
    }

    /// Filme na tela: desenha direto em RGBA.
    pub(super) fn render_movie(&mut self, out: &mut [u8]) -> bool {
        if let State::Show(s) = &self.state {
            if let Some(Running::Film(m)) = &s.cur {
                m.render_rgba(out);
                return true;
            }
        }
        false
    }

    // ---- música ----

    pub(super) fn update_music(&mut self) {
        let mode = self.mode();
        let Some(m) = self.m9.music.as_mut() else {
            return;
        };
        use super::Mode;
        match mode {
            Mode::Menu | Mode::Results | Mode::Page => m.menu(),
            Mode::Play | Mode::Paused => {
                if let Some(p) = m.game() {
                    self.m9.track_title = Some(track_name(&p, &self.system));
                }
            }
            Mode::Briefing => m.silence(),
            Mode::Show => {}
        }
    }

    // ---- fim de missão: peças, carro, nome, gravação ----

    /// Depois da partida (`0x444174`): com uma peça nova, `S_CAR` (e o filme da 7ª ou da
    /// 15ª); senão, o resultado.
    pub(super) fn after_results(
        &mut self,
        r: Box<Results>,
        first_win: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.m9.count_done = false;
        let piece = first_win
            && self.viewer.as_ref().is_some_and(|v| {
                v.scene()
                    .entities
                    .iter()
                    .any(|e| e.kind == openhunted_engine::entity::CAR_PIECE && !e.alive && e.on)
            });
        if !piece {
            self.state = State::Results(r);
            return Ok(());
        }
        self.m9.pieces = (self.m9.pieces + 1).min(LAST_PIECE);
        self.m9.film = match self.m9.pieces {
            MIDDLE_PIECE => Some(Film::Middle),
            LAST_PIECE => Some(Film::End),
            _ => None,
        };
        self.m9.pending = Some(r);
        let pieces = self.m9.pieces as u16;
        self.open_page("S_CAR.ITF", Placement::Full, PageKind::Car, |s, _| {
            Box::new(Car::new(s, pieces))
        });
        if self.page_kind() != Some(PageKind::Car) {
            self.back_to_results();
        }
        Ok(())
    }

    fn back_to_results(&mut self) {
        match self.m9.pending.take() {
            Some(r) => self.state = State::Results(r),
            None => self.go_menu(),
        }
    }

    /// A contagem chegou a 100 (uma vez): nome na lista de heróis e gravação do jogo.
    pub(super) fn count_finished(&mut self) {
        if self.m9.count_done || !matches!(self.state, State::Results(_)) {
            return;
        }
        self.m9.count_done = true;
        let (Some(st), Some(_)) = (&self.m9.storage, self.index) else {
            return;
        };
        let total: i32 = self.records.iter().sum();
        // decisão do port: só pergunta quando a soma mudou nesta vitória
        if self.m9.improved && total > 0 && st.heroes().qualifies(total) {
            if self.m9.player.is_empty() {
                let State::Results(r) = std::mem::replace(&mut self.state, placeholder()) else {
                    return;
                };
                self.m9.pending = Some(r);
                let (font, name) = (self.font.clone(), self.m9.player.clone());
                if let Some(f) = font {
                    self.open_page(
                        "SHIGH.ITF",
                        Placement::Centered,
                        PageKind::HeroName,
                        |s, _| Box::new(NameEntry::new(s, f, None, &name)),
                    );
                }
                if self.page_kind() != Some(PageKind::HeroName) {
                    self.back_to_results();
                }
                return;
            }
            self.record_hero();
        }
        if !matches!(self.state, State::Results(_)) {
            return;
        }
        let State::Results(r) = std::mem::replace(&mut self.state, placeholder()) else {
            return;
        };
        self.m9.pending = Some(r);
        self.save_step();
    }

    /// Grava o herói (`0x445098`) com a soma dos recordes.
    fn record_hero(&mut self) {
        let Some(st) = &self.m9.storage else { return };
        let total: i32 = self.records.iter().sum();
        let mut l = st.heroes();
        self.m9.hero_id = l.record(&self.m9.player, total, self.m9.hero_id);
        if let Err(e) = st.save_heroes(&l) {
            eprintln!("aviso: não gravou a lista de heróis: {e}");
        }
    }

    /// Gravação do jogo (`0x440ce4`) com o resultado em `pending`: sem nome, pergunta; com
    /// nome, grava direto. Soma zero: nada.
    fn save_step(&mut self) {
        let total: i32 = self.records.iter().sum();
        if total == 0 || self.m9.storage.is_none() {
            self.back_to_results();
            return;
        }
        if let Some(p) = self.m9.save_path.clone() {
            self.write_save(p);
            self.back_to_results();
            return;
        }
        let title = self.sys_text(SAVE_GAME_AS);
        let initial = clean_name(&self.m9.player);
        let font = self.font.clone();
        if let Some(f) = font {
            self.open_page(
                "SHIGH.ITF",
                Placement::Centered,
                PageKind::SaveName,
                |s, _| Box::new(NameEntry::new(s, f, Some(title), &initial)),
            );
        }
        if self.page_kind() != Some(PageKind::SaveName) {
            self.back_to_results();
        }
    }

    /// O bloco salvo da campanha.
    pub fn save_game(&self, path: &std::path::Path) -> SaveGame {
        let mut records = self.records.clone();
        records.resize(openhunted_formats::save::RECORDS, 0);
        let i = self.index.unwrap_or(0) as i32;
        SaveGame {
            name: self.m9.player.clone(),
            list_len: self.missions.len() as i32,
            solo_count: self.missions.len() as i32,
            current: i,
            last_loaded: i,
            total: self.records.iter().sum(),
            hero_id: self.m9.hero_id,
            pieces: self.m9.pieces,
            path: path.display().to_string(),
            level: format!("HUNTER.HLS|{}", self.level.0),
            records,
        }
    }

    fn write_save(&mut self, path: PathBuf) {
        let g = self.save_game(&path);
        match storage::write(&path, &g.to_bytes()) {
            Ok(()) => self.m9.save_path = Some(path),
            Err(e) => eprintln!("aviso: UNABLE TO SAVE GAME ({}): {e}", path.display()),
        }
    }

    /// Carrega o jogo salvo `name` (`0x440bc6` + `0x43f769`) e abre o briefing.
    fn restore(&mut self, name: &str) -> bool {
        let Some(st) = &self.m9.storage else {
            return false;
        };
        let path = st.save_path(&clean_name(name));
        let Some(g) = read_save(&path) else {
            return false;
        };
        let n = self.missions.len();
        self.records = g.records.iter().copied().take(n).collect();
        self.records.resize(n, 0);
        self.m9.player = g.name;
        self.m9.hero_id = g.hero_id;
        self.m9.pieces = g.pieces.clamp(0, LAST_PIECE);
        self.m9.save_path = Some(path);
        let mut i = usize::try_from(g.current)
            .ok()
            .filter(|&i| i < n)
            .unwrap_or(0);
        if self.records.get(i).is_some_and(|&r| r != 0) {
            i = next_mission(i, &self.records, &self.playable).unwrap_or(i);
        }
        self.index = Some(i);
        self.start_mission(Back::Menu);
        true
    }

    // ---- sequência: filmes, vozes, preto, créditos ----

    fn start_show(&mut self, steps: Vec<Step>, then: After) {
        // a sequência não precisa da fase aberta
        self.drop_viewer();
        self.state = State::Show(Box::new(Show {
            steps: steps.into(),
            cur: None,
            then,
        }));
        self.advance_show(0);
    }

    fn start_step(&mut self, step: Step) -> Option<Running> {
        let music = self.m9.music.as_mut();
        match step {
            Step::Black(ms) => {
                if let Some(m) = music {
                    m.silence();
                }
                Some(Running::Wait(ms))
            }
            Step::Voice(id) => {
                let pcm = self
                    .assets
                    .archive()
                    .get(&format!("SX_{id:04}.WAV"))
                    .ok()
                    .and_then(|d| Sound::parse(d).ok())
                    .and_then(|s| Pcm::decode(&s))?;
                let ms = (pcm.frames() as u64 * 1000 / u64::from(pcm.rate.max(1))) as u32;
                if let Some(m) = self.m9.music.as_mut() {
                    m.play_film(Arc::new(pcm));
                }
                Some(Running::Wait(ms))
            }
            Step::Film(f) => {
                let name = self.m9.config.movie(f);
                let movie = self.m9.movies.as_mut().and_then(|mk| mk(name))?;
                if let Some(m) = self.m9.music.as_mut() {
                    match &movie.audio {
                        Some(a) => m.play_film(a.clone()),
                        None => m.silence(),
                    }
                }
                Some(Running::Film(movie))
            }
            Step::Credits => {
                let lines = self
                    .assets
                    .archive()
                    .get("CREDITS.RES")
                    .map(parse_text_list)
                    .unwrap_or_default();
                Credits::load(&mut self.assets, lines)
                    .ok()
                    .map(|c| Running::Credits(Box::new(c), 0))
            }
        }
    }

    pub(super) fn advance_show(&mut self, ms: u32) {
        let State::Show(mut show) = std::mem::replace(&mut self.state, placeholder()) else {
            return;
        };
        let mut ms = ms;
        loop {
            if show.cur.is_none() {
                let Some(step) = show.steps.pop_front() else {
                    if let Some(m) = self.m9.music.as_mut() {
                        m.end_film();
                    }
                    match show.then {
                        After::Menu => self.go_menu(),
                        After::Results(r) => self.state = State::Results(r),
                    }
                    return;
                };
                show.cur = self.start_step(step);
                continue;
            }
            let finished = match show.cur.as_mut() {
                Some(Running::Wait(left)) => {
                    *left = left.saturating_sub(ms);
                    *left == 0
                }
                Some(Running::Film(m)) => {
                    m.advance(ms);
                    m.done()
                }
                Some(Running::Credits(c, acc)) => {
                    *acc += ms;
                    while *acc >= TICK_MS && !c.done() {
                        *acc -= TICK_MS;
                        c.tick();
                    }
                    c.done()
                }
                None => true,
            };
            if !finished {
                break;
            }
            if let Some(m) = self.m9.music.as_mut() {
                m.end_film();
            }
            show.cur = None;
            ms = 0;
        }
        self.state = State::Show(show);
    }
}

impl Show {
    /// Tecla ou clique: pula o passo corrente.
    fn skip(&mut self) {
        match self.cur.as_mut() {
            Some(Running::Wait(left)) => *left = 0,
            Some(Running::Film(m)) => m.skip(),
            Some(Running::Credits(c, _)) => c.skip(),
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::tests::temp;
    use crate::video::tests::Fake;
    use openhunted_engine::Outcome;
    use openhunted_engine::shell::options;
    use std::cell::RefCell;
    use std::path::Path;
    use std::rc::Rc;

    fn data() -> Option<PathBuf> {
        let p = std::env::var_os("OPENHUNTED_DATA").map(PathBuf::from);
        if p.is_none() {
            eprintln!("pulado: OPENHUNTED_DATA não definido");
        }
        p
    }

    fn app(root: &Path, level: Option<&str>) -> Option<App> {
        let mut a = App::load(&data()?, level).unwrap();
        a.set_storage(Storage::at(root));
        Some(a)
    }

    fn run(a: &mut App, ticks: u32) {
        for _ in 0..ticks {
            a.advance(50);
        }
    }

    /// Clica no centro do controle da tela corrente.
    fn click(a: &mut App, id: u16) {
        let r = a.screen().unwrap().rect(id).unwrap();
        a.pointer_move(r.x + r.w / 2, r.y + r.h / 2);
        a.pointer_button(true);
        run(a, 1);
        a.pointer_button(false);
        run(a, 6);
    }

    fn win(a: &mut App, piece: bool) {
        let s = a.viewer.as_mut().unwrap().scene_mut();
        if piece {
            let p = s
                .entities
                .iter_mut()
                .find(|e| e.kind == openhunted_engine::entity::CAR_PIECE)
                .expect("peça do carro na fase");
            p.alive = false;
            p.on = true;
        }
        s.outcome = Some(Outcome::Victory);
        a.advance(super::super::END_DELAY_MS + 50);
    }

    #[test]
    fn options_persist_and_reload() {
        let root = temp("app-options");
        let Some(mut a) = app(&root, None) else {
            return;
        };
        click(&mut a, menu::OPTIONS);
        assert_eq!(a.page_kind(), Some(PageKind::Options));
        click(&mut a, options::MUSIC_UP);
        click(&mut a, options::EASY);
        click(&mut a, options::LEVEL1 + 2);
        assert_eq!(a.config().music_volume, 35);
        a.key_down(Key::Escape);
        assert_eq!(a.mode(), super::super::Mode::Menu);
        let b = app(&root, None).unwrap();
        assert_eq!(
            (
                b.config().music_volume,
                b.config().difficulty,
                b.config().brightness
            ),
            (35, 0, 1404)
        );
        assert!(b.m9.game_lut.is_some(), "paleta do brilho");
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn choose_level_opens_the_briefing_of_the_pick() {
        let root = temp("app-levels");
        let Some(mut a) = app(&root, None) else {
            return;
        };
        click(&mut a, menu::CHOOSE_LEVEL);
        assert_eq!(a.page_kind(), Some(PageKind::Levels));
        a.key_down(Key::Down);
        a.key_down(Key::Down);
        a.key_down(Key::Enter); // o ponteiro começa em LOAD
        run(&mut a, 6);
        assert_eq!(a.mode(), super::super::Mode::Briefing);
        assert_eq!(a.mission_index(), Some(2));
        assert_eq!(a.level_file(), a.missions()[2].file);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn victory_asks_names_saves_and_restores_equal() {
        let root = temp("app-save");
        let Some(mut a) = app(&root, None) else {
            return;
        };
        a.key_down(Key::Enter); // New Game
        run(&mut a, 6);
        a.key_down(Key::Enter); // GO
        run(&mut a, 6);
        win(&mut a, false);
        a.key_down(Key::Enter); // salta a contagem
        run(&mut a, 2);
        assert_eq!(a.page_kind(), Some(PageKind::HeroName));
        assert!(a.wants_text());
        a.text_input("JAKE R");
        a.key_down(Key::Space); // o espaço não clica
        a.key_down(Key::Backspace);
        a.key_down(Key::Enter);
        assert_eq!(a.page_kind(), Some(PageKind::SaveName));
        let st = Storage::at(&root);
        assert_eq!(
            st.heroes().heroes[0].name,
            "JAKE",
            "sem os espaços das pontas"
        );
        a.text_input("1");
        a.key_down(Key::Enter);
        assert_eq!(a.mode(), super::super::Mode::Results);
        let path = st.save_path("JAKE1");
        assert_eq!(a.save_path(), Some(path.as_path()));
        let saved = read_save(&path).unwrap();
        assert_eq!(saved, a.save_game(&path), "gravado e relido igual");
        assert_eq!(&saved.to_bytes()[0x7a0..0x7a8], &[0; 8]);
        assert_eq!(saved.records[0], a.records[0]);
        assert!(saved.hero_id > 0);
        // carregar noutra sessão: a T_J_001 já tem recorde → a próxima sem recorde
        let mut b = app(&root, None).unwrap();
        click(&mut b, menu::RESTORE);
        assert_eq!(b.page_kind(), Some(PageKind::LoadName));
        b.key_down(Key::Enter); // o nome do mais recente já vem preenchido
        assert_eq!(b.mode(), super::super::Mode::Briefing);
        assert_eq!(b.records, a.records);
        assert_eq!(b.mission_index(), Some(2), "pula a do Garathe");
        // nome inválido: caixa 10009
        let mut c = app(&root, None).unwrap();
        click(&mut c, menu::RESTORE);
        c.text_input("x");
        c.key_down(Key::Enter);
        assert_eq!(c.notice(), Some("UNABLE TO LOAD. INVALID GAME FORMAT."));
        c.key_down(Key::Enter);
        assert_eq!(c.notice(), None);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn car_piece_shows_s_car_and_the_seventh_plays_the_middle_film() {
        let root = temp("app-car");
        let Some(mut a) = app(&root, Some("T_J_007.LEV")) else {
            return;
        };
        let asked = Rc::new(RefCell::new(Vec::new()));
        let log = asked.clone();
        a.set_movies(Box::new(move |name| {
            log.borrow_mut().push(name.to_string());
            Some(Movie::new(Box::new(Fake { n: 3, i: 0 }), None))
        }));
        a.m9.pieces = 6;
        a.key_down(Key::Enter); // GO
        run(&mut a, 6);
        win(&mut a, true);
        assert_eq!(a.page_kind(), Some(PageKind::Car));
        assert_eq!(a.pieces(), 7);
        click(&mut a, openhunted_engine::shell::heroes::car_ids::DONE);
        assert_eq!(a.mode(), super::super::Mode::Show);
        // preto, voz, filme, voz: cada tecla pula um passo
        for _ in 0..10 {
            if a.mode() != super::super::Mode::Show {
                break;
            }
            a.key_down(Key::Enter);
            a.advance(50);
        }
        assert_eq!(*asked.borrow(), ["MIDDARK.AVI"]);
        assert_eq!(a.mode(), super::super::Mode::Results);
        // a mesma missão de novo não dá peça
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn intro_film_then_menu_and_unavailable_two_player_modes() {
        let root = temp("app-intro");
        let Some(mut a) = app(&root, None) else {
            return;
        };
        a.play_intro();
        assert_eq!(
            a.mode(),
            super::super::Mode::Menu,
            "sem filmes, fica no menu"
        );
        a.set_movies(Box::new(|_| {
            Some(Movie::new(Box::new(Fake { n: 30, i: 0 }), None))
        }));
        a.play_intro();
        assert_eq!(a.show_steps(), Some(vec![]));
        let mut rgba = vec![0u8; 640 * 480 * 4];
        a.render_rgba(&mut rgba);
        a.advance(1000);
        assert_eq!(a.mode(), super::super::Mode::Show);
        a.advance(2000);
        assert_eq!(a.mode(), super::super::Mode::Menu, "o filme acabou");
        click(&mut a, menu::MULTI_PLAYER);
        assert_eq!(a.page_kind(), Some(PageKind::Multi));
        click(
            &mut a,
            openhunted_engine::shell::heroes::info_ids::MULTI_H2H,
        );
        assert_eq!(
            a.notice(),
            Some(openhunted_engine::shell::page::UNAVAILABLE)
        );
        a.key_down(Key::Escape);
        a.key_down(Key::Escape);
        assert_eq!(a.mode(), super::super::Mode::Menu);
        for id in [menu::HERO_LIST, menu::HELP, menu::INPUT_DEVICES] {
            click(&mut a, id);
            assert_eq!(a.mode(), super::super::Mode::Page, "{id}");
            a.render_rgba(&mut rgba);
            a.key_down(Key::Escape);
            run(&mut a, 1);
        }
        let _ = std::fs::remove_dir_all(&root);
    }
}
