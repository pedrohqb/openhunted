// SPDX-License-Identifier: GPL-3.0-or-later
//! A casca da M9 sobre o `App` (`docs/superpowers/specs/2026-11-06-m9-casca-design.md`):
//! as telas restantes do `SMAIN` (`docs/re/menus-restantes.md`), opções gravadas, jogo
//! salvo e lista de heróis (`opcoes-salvamento.md`), peças do carro com `S_CAR`, filmes,
//! créditos e música (`videos-musica.md`); da M14 (`2026-11-07-m14-acabamento-design.md`),
//! a abertura em laço, as teclas, os dispositivos e o joystick (`casca-acabamento.md`).

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;

use openhunted_engine::devices::{Device, DeviceState, InputReader, resolve};
use openhunted_engine::display_palette;
use openhunted_engine::shell::config::{Config, Film};
use openhunted_engine::shell::devices::DeviceScreen;
use openhunted_engine::shell::generic::GenericBox;
use openhunted_engine::shell::heroes::{Car, Heroes, Info, InfoKind, NameEntry};
use openhunted_engine::shell::intro::{self, Roll, Scroller, Splash};
use openhunted_engine::shell::keys::{self, KeyScreen};
use openhunted_engine::shell::levels::{Entry, LevelSelect};
use openhunted_engine::shell::options::Options;
use openhunted_engine::shell::results::{Choice, Results};
use openhunted_engine::shell::{Dir, Input, Page, Placement, Reply, Screen, menu, next_mission};
use openhunted_formats::{Palette, SaveGame, Sound, parse_text_list};

use super::{App, Back, JoyRaw, Key, State};
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
/// Cor padrão dos jogadores 1 e 2 (`0x497320`, `DLS.TBL`).
const PLAYER_COLORS: [u16; 2] = [35, 36];
/// Menu sem mexer por 30 s: a abertura em laço (`0x43f641`, estado 12345).
pub const IDLE_MS: u32 = 30_000;
/// Mudança de direção do joystick que conta como atividade no menu (limiar de `0x407e5c`).
const IDLE_AXIS: i16 = 50;
/// Avisos do port (sem texto no `SYSTEM.RES`): sem pasta de dados do usuário (sem `HOME`)
/// e nome de jogo salvo vazio depois de limpo.
pub const NO_STORAGE: &str = "SAVED GAMES ARE NOT AVAILABLE: NO HOME FOLDER.";
pub const EMPTY_NAME: &str = "TYPE A NAME WITH LETTERS OR DIGITS.";

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
    /// Tela preta.
    Black(u32),
    /// Espera com a tela de antes (o título, depois da voz 4101).
    Pause(u32),
    Voice(u16),
    Film(Film),
    Logo,
    Title,
    Story,
    Credits,
}

/// Paleta convertida para RGBA.
type Lut = [[u8; 4]; 256];

enum Running {
    Wait(u32),
    Film(Movie),
    Splash(Box<Splash>, Option<Box<Lut>>),
    Roll(Box<Scroller>),
}

/// A abertura (`0x4399e0(1)`): logotipo, preto, voz 4100, filme, título, voz 4101, história,
/// créditos.
fn intro_steps() -> VecDeque<Step> {
    VecDeque::from([
        Step::Logo,
        Step::Black(intro::logo::BLACK_MS),
        Step::Voice(intro::logo::VOICE),
        Step::Film(Film::Intro),
        Step::Title,
        Step::Voice(intro::title::VOICE),
        Step::Pause(intro::title::AFTER_MS),
        Step::Story,
        Step::Credits,
    ])
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
    /// A abertura: uma interrupção encerra tudo; sem ela, recomeça (`0x4399e0(0)`).
    intro: bool,
    /// Última tela da abertura (fica durante a voz e a espera que a seguem) e a paleta dela.
    backdrop: Option<(Box<Splash>, Option<Box<Lut>>)>,
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
    /// Escolha do resultado feita antes da contagem acabar, à espera do nome e da gravação.
    after_count: Option<Choice>,
    /// Resultado à espera sob uma tela (carro, nome) e o filme depois do carro.
    pending: Option<Box<Results>>,
    film: Option<Film>,
    music: Option<Music>,
    movies: Option<Movies>,
    pub(super) track_title: Option<String>,
    /// Tempo sem atividade no menu principal.
    pub(super) idle_ms: u32,
    /// Joysticks da SDL neste quadro, a leitura do jogador 1 (espera da combinação) e o
    /// joystick da tela `SJOYS` aberta.
    joys: Vec<JoyRaw>,
    reader: InputReader,
    joy_page: usize,
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

    /// Começa a abertura em laço (`0x4399e0(1)` e `(0)`): repete até uma tecla, um clique
    /// ou um botão de joystick, e volta ao menu. Sem filme, o passo dele é pulado.
    pub fn play_intro(&mut self) {
        self.start_show(intro_steps().into(), After::Menu);
        if let State::Show(s) = &mut self.state {
            s.intro = true;
        }
    }

    /// Paleta de um recurso `.PAL`, convertida para RGBA.
    fn lut_of(&self, name: &str) -> Option<Box<Lut>> {
        let p = Palette::parse(self.assets.archive().get(name).ok()?).ok()?;
        let p = display_palette(&p);
        let mut lut = [[0u8; 4]; 256];
        for (l, c) in lut.iter_mut().zip(p.0.iter()) {
            *l = [c[0], c[1], c[2], 255];
        }
        Some(Box::new(lut))
    }

    /// Paleta da sequência na tela (logotipo e título têm a própria).
    pub(super) fn show_lut(&self) -> Option<&Lut> {
        let State::Show(s) = &self.state else {
            return None;
        };
        match &s.cur {
            Some(Running::Splash(_, l)) => l.as_deref(),
            Some(Running::Wait(_)) => s.backdrop.as_ref().and_then(|(_, l)| l.as_deref()),
            _ => None,
        }
    }

    /// Duração de um som do `SOUND.RES` em ms (0 sem ele).
    fn sound_ms(&self, id: u16) -> u32 {
        self.assets
            .archive()
            .get(&format!("SX_{id:04}.WAV"))
            .ok()
            .and_then(|d| Sound::parse(d).ok())
            .and_then(|s| Pcm::decode(&s))
            .map_or(0, |p| {
                (p.frames() as u64 * 1000 / u64::from(p.rate.max(1))) as u32
            })
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
            menu::RESTORE => {
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
                if self.m9.storage.is_none() {
                    self.set_notice(NO_STORAGE);
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

    /// Joysticks ligados (os da SDL, até 4).
    pub fn joysticks(&self) -> usize {
        self.m9.joys.len().min(4)
    }

    /// Dispositivos em vigor dos dois jogadores (`0x43edc1`).
    pub fn devices(&self) -> [Device; 2] {
        resolve(self.m9.config.devices, self.joysticks())
    }

    fn open_input(&mut self) {
        let (c, n) = (self.m9.config.clone(), self.joysticks());
        self.open_page("SINPU.ITF", Placement::Full, PageKind::Input, |s, _| {
            Box::new(DeviceScreen::new(s, c, n))
        });
    }

    /// `SKEYB` (teclado) ou `SJOYS` (joystick) do jogador `p` (CUSTOMIZE).
    fn open_keys(&mut self, p: usize) {
        let text = |a: &App, n: &str| {
            a.assets
                .archive()
                .get(n)
                .map(parse_text_list)
                .unwrap_or_default()
        };
        let Some(font) = self.font.clone() else {
            return;
        };
        let colors = self.dls_table(PLAYER_COLORS[p.min(1)]);
        let (c, garathe) = (self.m9.config.clone(), p == 1);
        let (itf, kind, joy, names, descs) = match self.devices()[p.min(1)] {
            Device::Joystick(n) => {
                let j = usize::from(n - 1);
                let count = self.m9.joys.get(j).map_or(0, |r| r.count);
                let t = text(self, "JOYSTICK.RES");
                ("SJOYS.ITF", keys::Kind::Joystick(count), j, t.clone(), t)
            }
            _ => (
                "SKEYB.ITF",
                keys::Kind::Keyboard,
                0,
                text(self, "KEYS.RES"),
                text(self, "SHELL.RES"),
            ),
        };
        self.m9.joy_page = joy;
        self.open_page(itf, Placement::Full, PageKind::Keys, |s, _| {
            let mut k = KeyScreen::new(s, font, kind, p, joy, c, names, descs, garathe);
            k.screen.set_remap(keys::ids::CHARACTER, colors);
            Box::new(k)
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
            (PageKind::Options | PageKind::Input, Reply::Config(c)) => self.apply_config(c, true),
            (PageKind::Keys, Reply::Config(c)) => {
                self.apply_config(c, true);
                self.open_input();
            }
            (PageKind::Levels, Reply::Pick(i)) => {
                self.index = Some(i);
                self.start_mission(Back::Menu);
            }
            (PageKind::Input, Reply::Customize(p)) => self.open_keys(p),
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
                match &self.m9.storage {
                    None => self.set_notice(NO_STORAGE),
                    Some(_) if n.is_empty() => self.set_notice(EMPTY_NAME),
                    Some(st) => {
                        let p = st.save_path(&n);
                        self.write_save(p);
                        self.back_to_results();
                    }
                }
            }
            (PageKind::SaveName, Reply::Close) => self.back_to_results(),
            (PageKind::LoadName, Reply::Name(_)) if self.m9.storage.is_none() => {
                self.set_notice(NO_STORAGE)
            }
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
            // na espera de tecla, as teclas chegam por `vk_down`
            State::Page { page, .. } if page.capturing() => return,
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
            State::Show(s) => match &s.cur {
                Some(Running::Roll(r)) => r.draw(&mut self.fb),
                Some(Running::Splash(sp, _)) => sp.draw(&mut self.fb),
                Some(Running::Wait(_)) => {
                    if let Some((sp, _)) = &s.backdrop {
                        sp.draw(&mut self.fb);
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }

    /// Filme na tela: desenha direto em RGBA. No primeiro quadro desenhado, o relógio do
    /// filme e o áudio dele começam juntos (a abertura é criada antes da janela).
    pub(super) fn render_movie(&mut self, out: &mut [u8]) -> bool {
        if let State::Show(s) = &mut self.state {
            if let Some(Running::Film(m)) = &mut s.cur {
                if m.start() {
                    if let (Some(a), Some(mu)) = (&m.audio, self.m9.music.as_mut()) {
                        mu.play_film(a.clone());
                    }
                }
                m.render_rgba(out);
                return true;
            }
        }
        false
    }

    // ---- música ----

    pub(super) fn update_music(&mut self, ms: u32) {
        let mode = self.mode();
        let rolling =
            matches!(&self.state, State::Show(s) if matches!(s.cur, Some(Running::Roll(_))));
        let Some(m) = self.m9.music.as_mut() else {
            return;
        };
        use super::Mode;
        match mode {
            Mode::Menu | Mode::Results | Mode::Page => m.menu(),
            Mode::Play | Mode::Paused => {
                if let Some(p) = m.game(ms) {
                    self.m9.track_title = Some(track_name(&p, &self.system));
                }
            }
            Mode::Briefing => m.silence(),
            // a história e os créditos rolam com a música dos menus (`0x402d48` no laço)
            Mode::Show if rolling => m.menu(),
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
        self.m9.after_count = None;
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
            Some(r) => {
                self.state = State::Results(r);
                if let Some(c) = self.m9.after_count.take() {
                    self.dispatch_results(Some(c));
                }
            }
            None => self.go_menu(),
        }
    }

    /// Antes de seguir uma escolha do resultado: se a contagem não acabou, acaba agora (nome
    /// do herói e gravação). Devolve se a escolha segue já; senão ela fica à espera da tela
    /// aberta e segue quando o resultado volta.
    pub(super) fn finish_count_before(&mut self, c: Choice) -> bool {
        if self.m9.count_done {
            return true;
        }
        if let State::Results(r) = &mut self.state {
            r.skip_count();
        }
        self.count_finished();
        if matches!(self.state, State::Results(_)) {
            return true;
        }
        self.m9.after_count = Some(c);
        false
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
            intro: false,
            backdrop: None,
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
            Step::Pause(ms) => Some(Running::Wait(ms)),
            Step::Logo => {
                // o logotipo para a música (`0x4042c4`)
                if let Some(m) = music {
                    m.silence();
                }
                let ms = self.sound_ms(intro::logo::SOUND);
                let lut = self.lut_of(intro::logo::PALETTE);
                let sp = Splash::logo(&mut self.assets, ms).ok()?;
                self.sounds.push(intro::logo::SOUND);
                Some(Running::Splash(Box::new(sp), lut))
            }
            Step::Title => {
                let lut = self.lut_of(intro::title::PALETTE);
                let sp = Splash::title(&mut self.assets).ok()?;
                self.sounds.extend(intro::title::SOUNDS);
                Some(Running::Splash(Box::new(sp), lut))
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
                // o áudio começa com o primeiro quadro na tela (`render_movie`)
                if let Some(m) = self.m9.music.as_mut() {
                    m.silence();
                }
                Some(Running::Film(movie))
            }
            Step::Story | Step::Credits => {
                let (roll, res) = match step {
                    Step::Story => (Roll::Story, "STORY.RES"),
                    _ => (Roll::Credits, "CREDITS.RES"),
                };
                let lines = self
                    .assets
                    .archive()
                    .get(res)
                    .map(parse_text_list)
                    .unwrap_or_default();
                Scroller::load(&mut self.assets, roll, &lines)
                    .ok()
                    .map(|c| Running::Roll(Box::new(c)))
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
                if show.intro && show.steps.is_empty() {
                    // vista inteira: recomeça (`0x4399e0(0)`)
                    show.steps = intro_steps();
                }
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
                if matches!(step, Step::Black(_) | Step::Story | Step::Credits) {
                    show.backdrop = None;
                }
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
                Some(Running::Splash(sp, _)) => {
                    sp.advance(ms);
                    sp.done()
                }
                Some(Running::Roll(r)) => {
                    r.advance(ms);
                    r.done()
                }
                None => true,
            };
            if !finished {
                break;
            }
            if let Some(Running::Splash(sp, lut)) = show.cur.take() {
                show.backdrop = Some((sp, lut));
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
    /// Tecla, clique ou botão de joystick: pula o passo corrente; na abertura, encerra a
    /// sequência inteira (`0x43a123`: a primeira interrupção acaba tudo).
    fn skip(&mut self) {
        if self.intro {
            self.intro = false;
            self.steps.clear();
            self.backdrop = None;
        }
        match self.cur.as_mut() {
            Some(Running::Wait(left)) => *left = 0,
            Some(Running::Film(m)) => m.skip(),
            Some(Running::Splash(..)) | Some(Running::Roll(_)) => self.cur = None,
            None => {}
        }
    }
}

impl App {
    /// Uma tecla com o código VK do Windows (antes de `key_down`): na espera do `SKEYB` vai
    /// para a tela (e devolve `true`: a tecla não segue); na partida, as teclas de pular e de
    /// CIMA do jogador 1 viram a batida rápida (`tap`).
    pub fn vk_down(&mut self, vk: u8) -> bool {
        self.m9.idle_ms = 0;
        if let State::Page {
            page, notice: None, ..
        } = &mut self.state
        {
            if page.capturing() {
                page.raw_key(vk);
                return true;
            }
        }
        let keys = self.m9.config.keys[0];
        let keyboard = self.devices()[0].is_keyboard();
        if let (State::Play { pause: None, .. }, Some(v)) = (&self.state, self.viewer.as_mut()) {
            use openhunted_engine::devices::slot;
            use openhunted_engine::input::{JUMP, UP};
            // com a dica aberta, a tecla só a fecha (não vira pulo nem CIMA)
            if keyboard && v.modal().is_none() {
                if vk == keys[slot::JUMP] {
                    v.tap(JUMP);
                } else if vk == keys[slot::UP] {
                    v.tap(UP);
                }
            }
        }
        false
    }

    /// Os joysticks da SDL neste quadro (na ordem: Joystick 1–4). Botão novo interrompe a
    /// abertura e, na espera do `SJOYS`, vai para a tela; direção nova conta como atividade.
    pub fn set_joysticks(&mut self, joys: Vec<JoyRaw>) {
        let dir = |v: i16| (v > IDLE_AXIS) as i8 - (v < -IDLE_AXIS) as i8;
        let mut fresh = Vec::with_capacity(joys.len());
        for (i, j) in joys.iter().enumerate() {
            let old = self.m9.joys.get(i).copied().unwrap_or_default();
            fresh.push(j.buttons & !old.buttons);
            if (dir(j.x), dir(j.y)) != (dir(old.x), dir(old.y)) {
                self.m9.idle_ms = 0;
            }
        }
        self.m9.joys = joys;
        if fresh.iter().any(|&b| b != 0) {
            self.m9.idle_ms = 0;
        }
        match &mut self.state {
            State::Show(s) if fresh.iter().any(|&b| b != 0) => s.skip(),
            State::Page {
                page, notice: None, ..
            } if page.capturing() => {
                if let Some(&b) = fresh.get(self.m9.joy_page) {
                    page.joy_buttons(b);
                }
            }
            _ => {}
        }
    }

    /// Entrada do jogador 1 neste quadro pelo dispositivo dele (`0x407e2d`): o teclado pelas
    /// teclas do `HUNTER.INI` (`pressed(vk)`), ou o joystick escolhido no `SINPU`.
    pub fn read_input(&mut self, pressed: &dyn Fn(u8) -> bool, select: u8) {
        let st = match self.devices()[0] {
            Device::Joystick(n) => match self.m9.joys.get(usize::from(n - 1)) {
                Some(j) => {
                    DeviceState::joystick(j.x, j.y, j.buttons, j.count, &self.m9.config.buttons[0])
                }
                None => DeviceState::keyboard(&self.m9.config.keys[0], pressed),
            },
            _ => DeviceState::keyboard(&self.m9.config.keys[0], pressed),
        };
        let bits = self.m9.reader.bits(&st);
        self.set_input(bits, select);
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
        // a dificuldade gravada vale na partida (`0x487854`)
        let mut b = b;
        b.key_down(Key::Enter); // New Game
        run(&mut b, 6);
        assert_eq!(
            b.viewer().map(|v| v.scene().difficulty),
            Some(openhunted_engine::Difficulty::Easy)
        );
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
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[0xa0..0xa4], &[0; 4], "o original remonta a lista");
        assert_eq!(&bytes[0x7a0..0x7a8], &[0; 8]);
        assert_eq!(saved.records[0], a.records[0]);
        assert!(saved.hero_id > 0);
        // carregar noutra sessão: a T_J_001 já tem recorde → a próxima sem recorde
        let mut b = app(&root, None).unwrap();
        click(&mut b, menu::RESTORE);
        assert_eq!(b.page_kind(), Some(PageKind::LoadName));
        b.key_down(Key::Enter); // o nome do mais recente já vem preenchido
        assert_eq!(b.mode(), super::super::Mode::Briefing);
        assert_eq!(b.records, a.records);
        assert_eq!(b.mission_index(), Some(1), "a do Garathe (M8) é jogável");
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
    fn a_results_choice_before_the_count_ends_still_asks_and_saves() {
        use openhunted_engine::shell::results;
        let root = temp("app-early");
        let Some(mut a) = app(&root, None) else {
            return;
        };
        a.key_down(Key::Enter); // New Game
        run(&mut a, 6);
        a.key_down(Key::Enter); // GO
        run(&mut a, 6);
        win(&mut a, false);
        assert_eq!(a.mode(), super::super::Mode::Results);
        // clique em NEXT com a contagem no começo: o botão ativa sem a contagem fechar
        let r = a.screen().unwrap().rect(results::NEXT).unwrap();
        a.pointer_move(r.x + r.w / 2, r.y + r.h / 2);
        a.pointer_button(true);
        a.pointer_button(false);
        a.dispatch_results(Some(results::Choice::Next));
        assert_eq!(
            a.page_kind(),
            Some(PageKind::HeroName),
            "o nome antes do NEXT"
        );
        a.text_input("ANA");
        a.key_down(Key::Enter);
        assert_eq!(a.page_kind(), Some(PageKind::SaveName));
        a.key_down(Key::Enter); // o nome do herói já vem
        let st = Storage::at(&root);
        assert_eq!(st.heroes().heroes[0].name, "ANA");
        assert!(read_save(&st.save_path("ANA")).is_some(), "gravado");
        // e então o NEXT pedido
        assert_eq!(a.mode(), super::super::Mode::Briefing);
        assert_eq!(a.mission_index(), Some(1));
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn restore_without_storage_and_an_empty_save_name_explain_themselves() {
        let Some(d) = data() else {
            return;
        };
        let mut a = App::load(&d, None).unwrap();
        click(&mut a, menu::RESTORE);
        assert_eq!(
            a.page_kind(),
            Some(PageKind::LoadName),
            "o botão não fica morto"
        );
        assert_eq!(a.notice(), Some(NO_STORAGE));
        a.key_down(Key::Enter);
        assert_eq!(a.notice(), None);
        a.key_down(Key::Enter);
        assert_eq!(a.notice(), Some(NO_STORAGE));
        a.key_down(Key::Enter);
        a.key_down(Key::Escape);
        assert_eq!(a.mode(), super::super::Mode::Menu);
        // nome que a limpeza esvazia: aviso e o campo continua aberto
        let root = temp("app-empty-name");
        let mut b = app(&root, None).unwrap();
        b.key_down(Key::Enter); // New Game
        run(&mut b, 6);
        b.key_down(Key::Enter); // GO
        run(&mut b, 6);
        win(&mut b, false);
        b.key_down(Key::Enter);
        run(&mut b, 2);
        assert_eq!(b.page_kind(), Some(PageKind::HeroName));
        b.text_input("!!!");
        b.key_down(Key::Enter);
        assert_eq!(b.page_kind(), Some(PageKind::SaveName));
        b.key_down(Key::Enter);
        assert_eq!(b.notice(), Some(EMPTY_NAME));
        b.key_down(Key::Enter);
        assert_eq!(b.page_kind(), Some(PageKind::SaveName), "ainda pede o nome");
        assert_eq!(b.save_path(), None);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn the_prefilled_restore_name_finds_the_same_file() {
        let root = temp("app-latest");
        let Some(mut a) = app(&root, None) else {
            return;
        };
        let st = Storage::at(&root);
        // um nome de 31 caracteres (o máximo do campo) e um arquivo que o port não nomearia
        let long = "A".repeat(storage::SAVE_NAME_MAX);
        let g = a.save_game(&st.save_path(&long));
        storage::write(&st.save_path(&long), &g.to_bytes()).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        storage::write(&st.saves_dir().join("x.y.hhg"), &g.to_bytes()).unwrap();
        assert_eq!(st.latest_save().as_deref(), Some(long.as_str()));
        click(&mut a, menu::RESTORE);
        a.key_down(Key::Enter);
        assert_eq!(
            a.mode(),
            super::super::Mode::Briefing,
            "carregou o pré-preenchido"
        );
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

    /// Saída de música que registra o que tocou.
    struct Rec(Rc<RefCell<Vec<String>>>);

    impl crate::audio::SoundOut for Rec {
        fn play(&mut self, id: u16, _: Arc<Pcm>, _: u16, _: u8, _: i16) {
            self.0.borrow_mut().push(format!("play {id}"));
        }
    }

    fn movie_frames(a: &App) -> Option<u64> {
        match &a.state {
            State::Show(s) => match &s.cur {
                Some(Running::Film(m)) => Some(m.frames_shown()),
                _ => None,
            },
            _ => None,
        }
    }

    #[test]
    fn film_audio_and_clock_start_with_the_first_frame_on_screen() {
        let root = temp("app-film-clock");
        let Some(mut a) = app(&root, None) else {
            return;
        };
        let log = Rc::new(RefCell::new(Vec::new()));
        a.set_music(Box::new(Rec(log.clone())), vec![]);
        a.set_movies(Box::new(|_| {
            let pcm = Pcm {
                rate: 22_050,
                channels: 2,
                samples: vec![0; 4],
            };
            Some(Movie::new(Box::new(Fake { n: 30, i: 0 }), Some(pcm)))
        }));
        let film = format!("play {}", crate::music::FILM_ID);
        // a abertura é criada antes da janela: nada toca nem anda até o primeiro quadro
        a.start_show(vec![Step::Film(Film::Intro)], After::Menu);
        a.advance(400);
        assert!(
            !log.borrow().contains(&film),
            "sem quadro na tela, sem áudio"
        );
        assert_eq!(movie_frames(&a), Some(1));
        let mut rgba = vec![0u8; 640 * 480 * 4];
        a.render_rgba(&mut rgba);
        assert!(
            log.borrow().contains(&film),
            "o áudio com o primeiro quadro"
        );
        // o avanço seguinte traz o tempo de abrir o filme (~0,4 s): descartado
        a.advance(450);
        assert_eq!(
            movie_frames(&a),
            Some(1),
            "o vídeo não corre na frente do áudio"
        );
        a.advance(140);
        assert_eq!(movie_frames(&a), Some(3));
        a.render_rgba(&mut rgba);
        assert_eq!(
            log.borrow().iter().filter(|l| **l == film).count(),
            1,
            "o áudio começa uma vez"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Avança em passos de 50 ms até a sequência chegar a `step` (ou desistir).
    fn run_to(a: &mut App, step: Option<Step>, max_ms: u32) -> u32 {
        let mut t = 0;
        while t < max_ms {
            let cur = a.show_steps().map(|s| s.first().copied());
            if cur == Some(step) {
                return t;
            }
            a.advance(50);
            t += 50;
        }
        panic!("não chegou a {step:?} em {max_ms} ms");
    }

    #[test]
    fn intro_runs_logo_film_title_story_credits_in_a_loop_until_a_key() {
        let root = temp("app-intro");
        let Some(mut a) = app(&root, None) else {
            return;
        };
        let asked = Rc::new(RefCell::new(Vec::new()));
        let log = asked.clone();
        a.set_movies(Box::new(move |name| {
            log.borrow_mut().push(name.to_string());
            Some(Movie::new(Box::new(Fake { n: 30, i: 0 }), None))
        }));
        a.play_intro();
        assert_eq!(a.mode(), super::super::Mode::Show);
        a.advance(50);
        assert!(a.take_sounds().contains(&intro::logo::SOUND));
        // o logotipo com a paleta dele
        let mut rgba = vec![0u8; 640 * 480 * 4];
        a.render_rgba(&mut rgba);
        assert_eq!(a.show_lut().copied(), a.lut_of("SIERRA.PAL").map(|l| *l));
        // som 3403, 193 quadros do "tp" e 500 ms
        let logo = run_to(&mut a, Some(Step::Voice(intro::logo::VOICE)), 60_000);
        let want = a.sound_ms(intro::logo::SOUND) + 193 * 50 + 500;
        assert!(
            logo.abs_diff(want) <= 100,
            "logotipo {logo} ms, esperado {want}"
        );
        run_to(&mut a, Some(Step::Title), 60_000);
        a.render_rgba(&mut rgba); // o primeiro quadro do filme começa o relógio dele
        run_to(&mut a, Some(Step::Voice(intro::title::VOICE)), 60_000);
        let heard = a.take_sounds();
        assert!(
            intro::title::SOUNDS.iter().all(|s| heard.contains(s)),
            "{heard:?}"
        );
        assert_eq!(*asked.borrow(), ["HHDARK.AVI"]);
        run_to(&mut a, Some(Step::Credits), 120_000);
        // a história subiu inteira; os créditos acabam e a abertura recomeça
        run_to(&mut a, Some(Step::Black(intro::logo::BLACK_MS)), 120_000);
        assert_eq!(a.mode(), super::super::Mode::Show, "em laço");
        // uma tecla encerra tudo
        a.key_down(Key::Other);
        a.advance(50);
        assert_eq!(a.mode(), super::super::Mode::Menu);
        // sem filmes: o passo do filme é pulado
        let mut b = app(&root, None).unwrap();
        b.play_intro();
        // com o filme pulado, o título já é o passo corrente
        run_to(&mut b, Some(Step::Voice(intro::title::VOICE)), 60_000);
        assert_eq!(b.mode(), super::super::Mode::Show);
        // um botão de joystick também interrompe
        b.set_joysticks(vec![JoyRaw::default()]);
        b.set_joysticks(vec![JoyRaw {
            buttons: 1,
            count: 4,
            ..Default::default()
        }]);
        b.advance(50);
        assert_eq!(b.mode(), super::super::Mode::Menu);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn thirty_idle_seconds_in_the_menu_start_the_intro() {
        let root = temp("app-idle");
        let Some(mut a) = app(&root, None) else {
            return;
        };
        for _ in 0..590 {
            a.advance(50);
        }
        a.pointer_move(10, 10);
        for _ in 0..590 {
            a.advance(50);
        }
        assert_eq!(
            a.mode(),
            super::super::Mode::Menu,
            "o ponteiro zerou o relógio"
        );
        for _ in 0..20 {
            a.advance(50);
        }
        assert_eq!(a.mode(), super::super::Mode::Show);
        a.pointer_button(true);
        a.advance(50);
        assert_eq!(a.mode(), super::super::Mode::Menu);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn rebinding_keys_and_the_joystick_persists_and_drives_the_game() {
        use openhunted_engine::devices::{DEFAULT_KEYS, slot};
        use openhunted_engine::input::{JUMP, RIGHT, SHOOT, STEALTH};
        use openhunted_engine::shell::devices::ids as dev;
        use openhunted_engine::shell::keys::ids as k;
        let root = temp("app-keys");
        let Some(mut a) = app(&root, None) else {
            return;
        };
        click(&mut a, menu::INPUT_DEVICES);
        assert_eq!(a.page_kind(), Some(PageKind::Input));
        click(&mut a, dev::CUSTOMIZE[0]);
        assert_eq!(a.page_kind(), Some(PageKind::Keys));
        click(&mut a, k::ACTION + slot::JUMP as u16);
        // na espera, a tecla crua vai para a tela e não navega
        assert!(a.vk_down(32));
        click(&mut a, k::DONE);
        assert_eq!(a.page_kind(), Some(PageKind::Input));
        assert_eq!(a.config().keys[0][slot::JUMP], 32, "espaço no pular");
        let st = Storage::at(&root);
        assert_eq!(st.load_config().keys[0][slot::JUMP], 32, "gravado");
        let txt = std::fs::read_to_string(&st.config_file).unwrap();
        assert!(
            txt.contains("InputKeys1=38,40,37,39,17,32,88,33,90,34"),
            "{txt}"
        );
        // um joystick: o Joystick 1 do jogador 1, botões trocados no SJOYS
        a.set_joysticks(vec![JoyRaw {
            count: 6,
            ..Default::default()
        }]);
        a.key_down(Key::Escape);
        run(&mut a, 1);
        click(&mut a, menu::INPUT_DEVICES);
        click(&mut a, dev::FIRST[0] + 1);
        assert_eq!(a.devices()[0], Device::Joystick(1));
        click(&mut a, dev::CUSTOMIZE[0]);
        assert_eq!(a.page_kind(), Some(PageKind::Keys));
        click(&mut a, k::ACTION + slot::SHOOT as u16);
        a.set_joysticks(vec![JoyRaw {
            buttons: 1 << 2,
            count: 6,
            ..Default::default()
        }]);
        a.set_joysticks(vec![JoyRaw {
            count: 6,
            ..Default::default()
        }]);
        click(&mut a, k::DONE);
        assert_eq!(a.config().buttons[0], [3, 2, 1, 4, 5, 6]);
        let b = app(&root, None).unwrap();
        assert_eq!(b.config().devices[0], Some(Device::Joystick(1)));
        assert_eq!(b.config().buttons[0], [3, 2, 1, 4, 5, 6]);
        // no jogo: meia inclinação para a direita anda; o botão 3 atira
        a.key_down(Key::Escape);
        run(&mut a, 1);
        click(&mut a, menu::NEW_GAME);
        a.key_down(Key::Enter); // GO
        run(&mut a, 6);
        assert_eq!(a.mode(), super::super::Mode::Play);
        a.set_joysticks(vec![JoyRaw {
            x: 80,
            buttons: 1 << 2,
            count: 6,
            ..Default::default()
        }]);
        a.read_input(&|_| false, 0);
        assert_eq!(a.viewer().unwrap().input().0, RIGHT | STEALTH | SHOOT);
        // sem o joystick, o teclado com as teclas gravadas
        a.set_joysticks(vec![]);
        a.read_input(&|vk| vk == 32, 0);
        assert_eq!(a.viewer().unwrap().input().0, JUMP);
        assert!(!a.vk_down(32));
        assert_eq!(
            a.viewer().unwrap().input().1 & JUMP,
            JUMP,
            "o toque rápido do pulo"
        );
        assert_ne!(DEFAULT_KEYS[0][slot::JUMP], 32);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn two_player_modes_stay_unavailable() {
        let root = temp("app-multi");
        let Some(mut a) = app(&root, None) else {
            return;
        };
        let mut rgba = vec![0u8; 640 * 480 * 4];
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
