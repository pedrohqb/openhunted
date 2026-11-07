// SPDX-License-Identifier: GPL-3.0-or-later
//! Máquina de telas do jogo, independente da SDL (`docs/re/menu.md`): menu principal →
//! briefing → partida (com a barra de estado e a pausa) → resultado → próxima missão,
//! repetir ou menu.

use std::path::Path;
use std::rc::Rc;

use openhunted_engine::scene::has_controllable_player;
use openhunted_engine::shell::briefing::{self, Briefing};
use openhunted_engine::shell::generic::GenericBox;
use openhunted_engine::shell::results::{self, Remains, Results, Stats};
use openhunted_engine::shell::{
    Dir, Input, Placement, Screen, menu, next_mission, solo_missions, text,
};
use openhunted_engine::sound::Cue;
use openhunted_engine::{
    Assets, Framebuffer, Outcome, SCREEN_H, SCREEN_W, TICK_MS, display_palette,
};
use openhunted_formats::{
    Bitmap, Font, Interface, Level, LevelArchive, MissionInfo, Palette, parse_mission_list,
    parse_text_list,
};

use crate::viewer::{Action, Viewer};

// telas, salvamento, música e filmes da M9 (`docs/superpowers/specs/2026-11-06-m9-casca-design.md`)
#[path = "casca.rs"]
mod casca;
pub use casca::{PageKind, Show};

/// Espera entre o veredito e a tela de resultado (no original, o anúncio do locutor).
pub const END_DELAY_MS: u32 = 2000;

/// Teclas que a máquina de telas entende.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Left,
    Right,
    Up,
    Down,
    Enter,
    Space,
    Escape,
    /// `P` ou Pause.
    Pause,
    Tab,
    PageUp,
    PageDown,
    Home,
    End,
    Backspace,
    Other,
}

/// Teclas seguradas neste quadro (o jogo lê o estado a cada tique).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Held {
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub jump: bool,
    pub shoot: bool,
    /// Furtivo/andar (X, bit 0x400).
    pub stealth: bool,
    /// Soco (Z, bit 0x40000).
    pub punch: bool,
    /// Arma anterior / próxima (PgUp / PgDn, bits 0x800 / 0x1000).
    pub prev_weapon: bool,
    pub next_weapon: bool,
    /// Escolha direta da arma (teclas 1–6; 0 = nenhuma). Extensão do port.
    pub select: u8,
}

/// Para onde o CANCEL do briefing volta (o estado anterior, `0x47f53c`).
enum Back {
    Menu,
    /// O resultado e a missão de antes (índice e fase), restaurados se o jogador cancela.
    Results(Box<Results>, Option<usize>, (String, String, String)),
}

enum State {
    Menu(Box<Screen>),
    Briefing {
        dlg: Box<Briefing>,
        back: Back,
    },
    Play {
        pause: Option<GenericBox>,
        /// Milissegundos desde o veredito.
        end_ms: Option<u32>,
    },
    Results(Box<Results>),
    /// Telas da M9 (opções, missões, heróis, nome, carro…), com a caixa de aviso por cima.
    Page {
        page: Box<dyn openhunted_engine::shell::Page>,
        kind: PageKind,
        notice: Option<GenericBox>,
    },
    /// Filmes, vozes, tela preta e créditos.
    Show(Box<Show>),
}

/// Nome curto da tela corrente (testes e título da janela).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Menu,
    Briefing,
    Play,
    Paused,
    Results,
    /// Uma das telas da M9 (ver `App::page_kind`).
    Page,
    /// Filme ou créditos.
    Show,
}

/// O jogo inteiro: telas, missão corrente e a partida.
pub struct App {
    assets: Assets,
    levels: LevelArchive,
    missions: Vec<MissionInfo>,
    /// Recorde por missão (só nesta sessão: `hunter.fam` ainda não é gravado).
    records: Vec<i32>,
    /// Índice da missão corrente na lista solo; `None` para fase fora da lista (`--level`).
    index: Option<usize>,
    /// Missões com o Jake controlável (as do Garathe ainda não são jogáveis).
    playable: Vec<bool>,
    /// Última posição do ponteiro (para a caixa de pausa realçar o botão já na abertura).
    last_pointer: (i32, i32),
    /// Fase corrente: arquivo, título e descrição.
    level: (String, String, String),
    system: Vec<String>,
    font: Option<Rc<Font>>,
    generic: Option<Rc<Bitmap>>,
    lut: [[u8; 4]; 256],
    fb: Framebuffer,
    state: State,
    viewer: Option<Viewer>,
    pending_ms: u32,
    /// Sons das telas à espera do próximo quadro (ids do `SOUND.RES`).
    sounds: Vec<u16>,
    /// Sons já mandados à saída (para os testes; esvaziado por `take_sounds`).
    heard: Vec<u16>,
    /// Efeitos sonoros quando não há fase aberta (com fase, ficam com o `Viewer`).
    sfx: Option<crate::audio::Sfx>,
    quit: bool,
    /// Botão do menu escolhido por último (o ponteiro volta a ele).
    last_menu_button: u16,
    /// Estado da M9: configuração, campanha, arquivos, música e filmes.
    m9: casca::Casca,
}

impl App {
    /// Abre os dados e começa no menu principal; com `level`, direto no briefing dela.
    pub fn load(data: &Path, level: Option<&str>) -> Result<Self, Box<dyn std::error::Error>> {
        let gd = openhunted_formats::open_data(data)?;
        let levels = gd.levels()?;
        let assets = Assets::new(gd.resources()?);
        let mut app = Self::new(assets, levels)?;
        if let Some(l) = level {
            if let Some(i) = app
                .missions
                .iter()
                .position(|m| m.file.eq_ignore_ascii_case(l))
            {
                app.index = Some(i);
            }
            app.start_briefing(l, Back::Menu)?;
        }
        Ok(app)
    }

    /// Monta o jogo a partir dos recursos e do arquivo de fases.
    pub fn new(
        mut assets: Assets,
        levels: LevelArchive,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let missions = solo_missions(&parse_mission_list(&levels.get("ENGLEVEL.TXT")?)?);
        let palette = display_palette(&Palette::parse(assets.archive().get("WORLD.PAL")?)?);
        let mut lut = [[0u8; 4]; 256];
        for (l, c) in lut.iter_mut().zip(palette.0.iter()) {
            *l = [c[0], c[1], c[2], 255];
        }
        let system = assets
            .archive()
            .get("SYSTEM.RES")
            .map(parse_text_list)
            .unwrap_or_default();
        let font = assets.font("SLEVD.PFT").ok();
        let generic = assets.bitmap("SGENERIC.BMP").ok();
        let records = vec![0; missions.len()];
        let playable = missions
            .iter()
            .map(|m| {
                levels
                    .get(&m.file)
                    .ok()
                    .and_then(|d| Level::parse(&d).ok())
                    .is_some_and(|l| has_controllable_player(&l))
            })
            .collect();
        let mut app = Self {
            assets,
            levels,
            missions,
            records,
            index: None,
            playable,
            last_pointer: (SCREEN_W as i32 / 2, SCREEN_H as i32 / 2),
            level: Default::default(),
            system,
            font,
            generic,
            lut,
            fb: Framebuffer::new(SCREEN_W, SCREEN_H),
            state: State::Play {
                pause: None,
                end_ms: None,
            },
            viewer: None,
            pending_ms: 0,
            sounds: Vec::new(),
            heard: Vec::new(),
            sfx: None,
            quit: false,
            last_menu_button: menu::NEW_GAME,
            m9: casca::Casca::default(),
        };
        app.open_menu()?;
        Ok(app)
    }

    fn itf(&mut self, name: &str) -> Result<Interface, Box<dyn std::error::Error>> {
        Ok(Interface::parse(self.assets.archive().get(name)?).map_err(|e| e.in_resource(name))?)
    }

    fn sys_text(&self, id: u16) -> String {
        self.system
            .get(usize::from(id - text::SYSTEM_BASE).wrapping_sub(1))
            .cloned()
            .unwrap_or_default()
    }

    fn font(&self) -> Result<Rc<Font>, Box<dyn std::error::Error>> {
        self.font
            .clone()
            .ok_or_else(|| "fonte SLEVD.PFT ausente".into())
    }

    pub fn mode(&self) -> Mode {
        match &self.state {
            State::Menu(_) => Mode::Menu,
            State::Briefing { .. } => Mode::Briefing,
            State::Play { pause: Some(_), .. } => Mode::Paused,
            State::Play { .. } => Mode::Play,
            State::Results(_) => Mode::Results,
            State::Page { .. } => Mode::Page,
            State::Show(_) => Mode::Show,
        }
    }

    pub fn quit_requested(&self) -> bool {
        self.quit
    }

    /// Missões do modo solo e índice da corrente.
    pub fn missions(&self) -> &[MissionInfo] {
        &self.missions
    }

    pub fn mission_index(&self) -> Option<usize> {
        self.index
    }

    /// Arquivo da fase corrente.
    pub fn level_file(&self) -> &str {
        &self.level.0
    }

    pub fn viewer(&self) -> Option<&Viewer> {
        self.viewer.as_ref()
    }

    pub fn viewer_mut(&mut self) -> Option<&mut Viewer> {
        self.viewer.as_mut()
    }

    pub fn results(&self) -> Option<&Results> {
        match &self.state {
            State::Results(r) => Some(r),
            _ => None,
        }
    }

    pub fn briefing(&self) -> Option<&Briefing> {
        match &self.state {
            State::Briefing { dlg, .. } => Some(dlg),
            _ => None,
        }
    }

    pub fn screen(&self) -> Option<&Screen> {
        match &self.state {
            State::Menu(s) => Some(s),
            State::Briefing { dlg, .. } => Some(&dlg.screen),
            State::Results(r) => Some(&r.screen),
            State::Page { page, .. } => Some(page.screen()),
            State::Play { .. } | State::Show(_) => None,
        }
    }

    /// Sons das telas tocados desde a última chamada (ids do `SOUND.RES`).
    pub fn take_sounds(&mut self) -> Vec<u16> {
        std::mem::take(&mut self.heard)
    }

    /// Liga os efeitos sonoros (telas e jogo).
    pub fn set_sound(&mut self, sfx: crate::audio::Sfx) {
        match self.viewer.as_mut() {
            Some(v) => v.set_sound(sfx),
            None => self.sfx = Some(sfx),
        }
    }

    /// Fecha a fase aberta, guardando os efeitos sonoros dela.
    fn drop_viewer(&mut self) {
        if let Some(mut v) = self.viewer.take() {
            if let Some(sfx) = v.take_sound() {
                self.sfx = Some(sfx);
            }
        }
    }

    /// Põe uma fase em jogo, passando a ela os efeitos sonoros.
    fn set_viewer(&mut self, mut v: Viewer) {
        self.drop_viewer();
        if let Some(sfx) = self.sfx.take() {
            v.set_sound(sfx);
        }
        self.viewer = Some(v);
    }

    /// Toca os sons das telas (sem posição: `Cue::ui`).
    fn flush_sounds(&mut self) {
        if self.sounds.is_empty() {
            return;
        }
        let cues: Vec<Cue> = self.sounds.iter().map(|&id| Cue::ui(id, 0)).collect();
        self.heard.append(&mut self.sounds);
        match (self.viewer.as_mut(), self.sfx.as_mut()) {
            (Some(v), _) => v.play_sounds(&cues),
            (None, Some(sfx)) => sfx.tick(self.assets.archive(), &cues, (0, 0), 0),
            (None, None) => {}
        }
    }

    fn open_menu(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let itf = self.itf("SMAIN.ITF")?;
        let mut s = Screen::new(&itf, &mut self.assets, Placement::Full)?;
        for id in menu::BUTTONS {
            s.set_flags(id, 0x81);
        }
        s.point_at(self.last_menu_button);
        self.drop_viewer();
        self.state = State::Menu(Box::new(s));
        Ok(())
    }

    /// Carrega a fase e abre o briefing por cima do primeiro quadro dela.
    fn start_briefing(&mut self, file: &str, back: Back) -> Result<(), Box<dyn std::error::Error>> {
        let lv = Level::parse(&self.levels.get(file)?).map_err(|e| e.in_resource(file))?;
        let mut viewer = Viewer::from_level(&lv, self.assets.clone())?;
        viewer.follow();
        // o título e a descrição vêm da lista de missões; fase fora dela, do cabeçalho
        let (title, desc) = match self
            .missions
            .iter()
            .find(|m| m.file.eq_ignore_ascii_case(file))
        {
            Some(m) => (m.title.clone(), m.description.clone()),
            None => (lv.title.clone(), lv.description.clone()),
        };
        let itf = self.itf("SLEVD.ITF")?;
        let screen = Screen::new(&itf, &mut self.assets, Placement::Centered)?;
        let dlg = Briefing::new(screen, self.font()?, &title, &desc);
        self.level = (file.to_string(), title, desc);
        self.set_viewer(viewer);
        self.state = State::Briefing {
            dlg: Box::new(dlg),
            back,
        };
        Ok(())
    }

    fn start_mission(&mut self, back: Back) {
        let Some(file) = self
            .index
            .and_then(|i| self.missions.get(i))
            .map(|m| m.file.clone())
        else {
            return;
        };
        if let Err(e) = self.start_briefing(&file, back) {
            eprintln!("erro ao abrir {file}: {e}");
        }
    }

    /// Fim da partida: anúncio, recorde e tela de resultado (`0x444174`).
    fn open_results(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let Some(v) = self.viewer.as_ref() else {
            return Ok(());
        };
        let outcome = v.outcome().unwrap_or(Outcome::Defeat);
        let scene = v.scene();
        let jake = scene
            .jake
            .as_ref()
            .is_some_and(|j| j.character() == openhunted_engine::character::Character::Jake);
        // locutor: vitória / tempo esgotado / outra derrota, e o nome do personagem
        self.sounds.push(match (outcome, scene.timed_out()) {
            (Outcome::Victory, _) => 3780,
            (Outcome::Defeat, true) => 3776,
            (Outcome::Defeat, false) => 3777,
        });
        self.sounds.push(match (outcome, jake) {
            (Outcome::Victory, true) => 3782,
            (Outcome::Victory, false) => 3786,
            (Outcome::Defeat, true) => 3781,
            (Outcome::Defeat, false) => 3785,
        });
        let stats = Stats {
            kills: scene.kills,
            kills_total: scene.mines_at_start,
            ..Default::default()
        };
        let remains = match scene.jake.as_ref() {
            Some(j) => Remains {
                timer: scene.timer,
                health: j.health.hp,
                armor: j.health.armor,
                ammo: j.inventory.slots[2..=6].iter().map(|s| s.1).sum(),
            },
            None => Remains {
                timer: scene.timer,
                ..Default::default()
            },
        };
        let score = results::score(&stats, results::bonus(outcome, remains));
        let mut improve = None;
        let mut cumulative = None;
        let mut first_win = false;
        self.m9.improved = false;
        if outcome == Outcome::Victory {
            // fase fora da lista (`--level`): sem recorde nem soma
            if let Some(rec) = self.index.and_then(|i| self.records.get_mut(i)) {
                first_win = *rec == 0;
                if score > *rec {
                    self.m9.improved = true;
                    if *rec != 0 {
                        improve = Some(score - *rec);
                    }
                    *rec = score;
                }
                cumulative = Some(self.records.iter().sum());
            }
        }
        let improve = improve.map(|d| self.sys_text(text::IMPROVED).replace("%d", &d.to_string()));
        let itf = self.itf("SMISS.ITF")?;
        let screen = Screen::new(&itf, &mut self.assets, Placement::Full)?;
        let r = Results::new(
            screen,
            self.font()?,
            &self.level.1,
            outcome,
            stats,
            remains,
            cumulative,
            improve,
        );
        self.after_results(Box::new(r), first_win)
    }

    // ---- entrada ----

    /// Uma tecla apertada (sem repetição).
    pub fn key_down(&mut self, key: Key) {
        let nav = |k: Key| match k {
            Key::Left => Some(Input::Nav(Dir::Left)),
            Key::Right => Some(Input::Nav(Dir::Right)),
            Key::Up => Some(Input::Nav(Dir::Up)),
            Key::Down => Some(Input::Nav(Dir::Down)),
            Key::Enter | Key::Space => Some(Input::Activate),
            Key::Escape => Some(Input::Cancel),
            _ => None,
        };
        let (paused_text, okay) = (self.sys_text(text::PAUSED), self.sys_text(text::OKAY));
        if matches!(self.state, State::Page { .. } | State::Show(_)) {
            self.casca_key(key);
            return;
        }
        match &mut self.state {
            State::Menu(_) if key == Key::Escape => {
                // o original ignora o Esc no menu; aqui ele sai do jogo
                self.quit = true;
            }
            State::Menu(s) => {
                if let Some(i) = nav(key) {
                    let r = s.handle(i);
                    self.dispatch_menu(r);
                }
            }
            State::Briefing { dlg, .. } => {
                let r = match key {
                    Key::Up => {
                        dlg.nav(Dir::Up);
                        None
                    }
                    Key::Down => {
                        dlg.nav(Dir::Down);
                        None
                    }
                    k => nav(k).and_then(|i| dlg.handle(i)),
                };
                self.dispatch_briefing(r);
            }
            State::Results(r) => {
                if let Some(i) = nav(key) {
                    let c = r.handle(i, &mut self.sounds);
                    self.dispatch_results(c);
                } else {
                    r.skip_count();
                }
            }
            State::Play { pause: Some(_), .. } => {
                if matches!(key, Key::Enter | Key::Space) {
                    self.resume();
                }
            }
            State::Play { pause, .. } => {
                let Some(v) = self.viewer.as_mut() else {
                    return;
                };
                use openhunted_engine::input::{JUMP, UP};
                match key {
                    Key::Pause if v.modal().is_none() && v.outcome().is_none() => {
                        *pause = self.generic.as_ref().zip(self.font.as_ref()).map(|(b, f)| {
                            let mut g = GenericBox::new(b, f, &paused_text, &okay);
                            g.pointer(self.last_pointer.0, self.last_pointer.1);
                            g
                        });
                    }
                    // sem personagem controlável (fases do Garathe) a fase nunca acaba:
                    // Esc volta ao menu (decisão do port)
                    Key::Escape if !v.has_jake() => {
                        if let Err(e) = self.open_menu() {
                            eprintln!("erro: {e}");
                            self.quit = true;
                        }
                    }
                    Key::Tab => v.apply(Action::ToggleOverlay),
                    // com a dica aberta, a tecla só a fecha (não vira pulo nem CIMA)
                    Key::Space if v.modal().is_none() => v.tap(JUMP),
                    Key::Up if v.modal().is_none() => v.tap(UP),
                    Key::PageUp => v.apply(Action::PageLeft),
                    Key::PageDown => v.apply(Action::PageRight),
                    Key::Home => v.apply(Action::Home),
                    Key::End => v.apply(Action::End),
                    // Esc com o Jake: nada (como o original)
                    _ => {}
                }
            }
            // tratados em `casca_key`, antes do `match`
            State::Page { .. } | State::Show(_) => {}
        }
    }

    /// Uma tecla solta: fecha a dica do tutorial.
    pub fn key_up(&mut self) {
        if let (State::Play { pause: None, .. }, Some(v)) = (&self.state, self.viewer.as_mut()) {
            v.key_released();
        }
    }

    /// Teclas seguradas (a cada quadro): controlam o Jake, ou rolam a tela sem ele.
    pub fn set_held(&mut self, h: Held) {
        let (State::Play { pause: None, .. }, Some(v)) = (&self.state, self.viewer.as_mut()) else {
            return;
        };
        if v.has_jake() {
            use openhunted_engine::input::{
                DOWN, JUMP, LEFT, NEXT_WEAPON, PREV_WEAPON, PUNCH, RIGHT, SELECT_MASK,
                SELECT_SHIFT, SHOOT, STEALTH, UP,
            };
            let mut bits = 0;
            for (on, bit) in [
                (h.left, LEFT),
                (h.right, RIGHT),
                (h.up, UP),
                (h.down, DOWN),
                (h.jump, JUMP),
                (h.shoot, SHOOT),
                (h.stealth, STEALTH),
                (h.punch, PUNCH),
                (h.prev_weapon, PREV_WEAPON),
                (h.next_weapon, NEXT_WEAPON),
            ] {
                if on {
                    bits |= bit;
                }
            }
            bits |= (u32::from(h.select) << SELECT_SHIFT) & SELECT_MASK;
            v.set_input(bits);
        } else {
            for (on, act) in [
                (h.left, Action::ScrollLeft),
                (h.right, Action::ScrollRight),
                (h.up, Action::ScrollUp),
                (h.down, Action::ScrollDown),
            ] {
                if on {
                    v.apply(act);
                }
            }
        }
    }

    /// Ponteiro do mouse em coordenadas da tela 640×480.
    pub fn pointer_move(&mut self, x: i32, y: i32) {
        self.last_pointer = (x, y);
        match &mut self.state {
            State::Menu(s) => {
                s.handle(Input::PointerMove(x, y));
            }
            State::Briefing { dlg, .. } => {
                dlg.handle(Input::PointerMove(x, y));
            }
            State::Results(r) => {
                r.handle(Input::PointerMove(x, y), &mut self.sounds);
            }
            State::Play { pause: Some(b), .. } => b.pointer(x, y),
            State::Page { .. } => self.casca_pointer(Some((x, y)), None),
            State::Play { .. } | State::Show(_) => {}
        }
    }

    /// A janela perdeu o foco: solta o botão do mouse e, no jogo, pausa (decisão do port).
    pub fn focus_lost(&mut self) {
        self.pointer_button(false);
        if self.mode() == Mode::Play && self.viewer.as_ref().is_some_and(|v| v.modal().is_none()) {
            self.key_down(Key::Pause);
        }
    }

    /// Botão do mouse apertado (`true`) ou solto.
    pub fn pointer_button(&mut self, down: bool) {
        let i = if down {
            Input::PointerDown
        } else {
            Input::PointerUp
        };
        match &mut self.state {
            State::Menu(s) => {
                s.handle(i);
            }
            State::Briefing { dlg, .. } => {
                dlg.handle(i);
            }
            State::Results(r) => {
                if down {
                    r.skip_count();
                }
                r.handle(i, &mut self.sounds);
            }
            State::Play { pause: Some(b), .. } => {
                if b.click(down) {
                    self.resume();
                }
            }
            State::Page { .. } | State::Show(_) => self.casca_pointer(None, Some(down)),
            State::Play { .. } => {}
        }
    }

    fn resume(&mut self) {
        if let State::Play { pause, .. } = &mut self.state {
            *pause = None;
        }
    }

    // ---- despacho dos comandos ----

    fn dispatch_menu(&mut self, id: Option<u16>) {
        match id {
            Some(menu::NEW_GAME) => {
                self.last_menu_button = menu::NEW_GAME;
                // `0x440fe0`: zera a campanha e começa da primeira missão solo
                self.records.iter_mut().for_each(|r| *r = 0);
                self.new_campaign();
                self.index = Some(0);
                self.start_mission(Back::Menu);
            }
            // sem o diálogo de confirmação 10005
            Some(menu::EXIT) => self.quit = true,
            Some(id) => self.menu_button(id),
            None => {}
        }
    }

    fn dispatch_briefing(&mut self, choice: Option<briefing::Choice>) {
        match choice {
            Some(briefing::Choice::Go) => {
                self.sounds.push(briefing::SOUND_GO);
                self.pending_ms = 0;
                self.state = State::Play {
                    pause: None,
                    end_ms: None,
                };
            }
            Some(briefing::Choice::Cancel) => {
                let old = std::mem::replace(
                    &mut self.state,
                    State::Play {
                        pause: None,
                        end_ms: None,
                    },
                );
                let State::Briefing { back, .. } = old else {
                    return;
                };
                match back {
                    Back::Menu => {
                        if let Err(e) = self.open_menu() {
                            eprintln!("erro: {e}");
                            self.quit = true;
                        }
                    }
                    Back::Results(mut r, index, level) => {
                        // a missão volta a ser a do resultado (o REPLAY repete ela)
                        self.index = index;
                        self.level = level;
                        r.from_game = false;
                        self.state = State::Results(r);
                    }
                }
            }
            None => {}
        }
    }

    fn dispatch_results(&mut self, choice: Option<results::Choice>) {
        let Some(c) = choice else { return };
        let old = std::mem::replace(
            &mut self.state,
            State::Play {
                pause: None,
                end_ms: None,
            },
        );
        let State::Results(r) = old else { return };
        let back = |r: Box<Results>, app: &App| Back::Results(r, app.index, app.level.clone());
        match c {
            results::Choice::Main => {
                if let Err(e) = self.open_menu() {
                    eprintln!("erro: {e}");
                    self.quit = true;
                }
            }
            results::Choice::Replay => {
                let file = self.level.0.clone();
                let b = back(r, self);
                if let Err(e) = self.start_briefing(&file, b) {
                    eprintln!("erro ao abrir {file}: {e}");
                }
            }
            results::Choice::Next => {
                // fase fora da lista: não há próxima, vai ao menu
                let next = self
                    .index
                    .and_then(|i| next_mission(i, &self.records, &self.playable));
                let file = next
                    .and_then(|i| self.missions.get(i))
                    .map(|m| m.file.clone());
                if let (Some(i), Some(file)) = (next, file) {
                    let b = back(r, self);
                    let prev = self.index;
                    self.index = Some(i);
                    if let Err(e) = self.start_briefing(&file, b) {
                        eprintln!("erro ao abrir {file}: {e}");
                        self.index = prev;
                    }
                }
            }
        }
        // a troca falhou (ou não havia para onde ir): menu
        if matches!(self.state, State::Play { .. }) {
            if let Err(e) = self.open_menu() {
                eprintln!("erro: {e}");
                self.quit = true;
            }
        }
    }

    // ---- relógio e desenho ----

    /// Avança o relógio: as telas e o jogo andam em tiques de 50 ms.
    pub fn advance(&mut self, ms: u32) {
        if matches!(self.state, State::Show(_)) {
            self.advance_show(ms);
        } else {
            self.advance_inner(ms);
        }
        self.update_music();
        self.flush_sounds();
    }

    fn advance_inner(&mut self, ms: u32) {
        if let State::Play { pause, end_ms } = &mut self.state {
            if pause.is_some() {
                return;
            }
            let Some(v) = self.viewer.as_mut() else {
                return;
            };
            v.advance(ms);
            if v.outcome().is_some() {
                let t = end_ms.get_or_insert(0);
                *t = t.saturating_add(ms);
                if *t >= END_DELAY_MS {
                    if let Err(e) = self.open_results() {
                        eprintln!("erro: {e}");
                        self.quit = true;
                    }
                }
            }
            return;
        }
        self.pending_ms = self.pending_ms.saturating_add(ms);
        while self.pending_ms >= TICK_MS {
            self.pending_ms -= TICK_MS;
            match &mut self.state {
                State::Menu(s) => {
                    let r = s.handle(Input::Tick);
                    self.dispatch_menu(r);
                }
                State::Briefing { dlg, .. } => {
                    let r = dlg.handle(Input::Tick);
                    self.dispatch_briefing(r);
                }
                State::Results(r) => {
                    let c = r.handle(Input::Tick, &mut self.sounds);
                    let done = r.progress() >= 100;
                    self.dispatch_results(c);
                    if done {
                        self.count_finished();
                    }
                }
                State::Page { .. } => self.casca_tick(),
                State::Play { .. } | State::Show(_) => {
                    // a tela mudou para o jogo neste quadro: o resto é dele
                    self.pending_ms = 0;
                    return;
                }
            }
        }
    }

    /// Desenha a tela corrente no framebuffer indexado.
    pub fn render(&mut self) -> &Framebuffer {
        match &self.state {
            State::Menu(s) => {
                self.fb.clear(0);
                s.draw(&mut self.fb);
            }
            State::Results(r) => {
                self.fb.clear(0);
                r.draw(&mut self.fb);
            }
            State::Page { .. } | State::Show(_) => self.casca_render(),
            State::Briefing { dlg, .. } => {
                if let Some(v) = self.viewer.as_mut() {
                    self.fb.pixels.copy_from_slice(&v.render().pixels);
                }
                dlg.draw(&mut self.fb);
            }
            State::Play { pause, .. } => {
                if let Some(v) = self.viewer.as_mut() {
                    self.fb.pixels.copy_from_slice(&v.render().pixels);
                }
                if let (Some(b), Some(bmp), Some(font)) = (pause, &self.generic, &self.font) {
                    b.draw(&mut self.fb, bmp, font);
                }
            }
        }
        &self.fb
    }

    /// Desenha e converte para RGBA (640 × 480 × 4 bytes) com a `WORLD.PAL`.
    pub fn render_rgba(&mut self, out: &mut [u8]) {
        if self.render_movie(out) {
            return;
        }
        self.render();
        // o brilho (`WORLD_G%d.PAL`) vale para o briefing e a partida
        let lut = match (&self.state, &self.m9.game_lut) {
            (State::Briefing { .. } | State::Play { .. }, Some(g)) => g,
            _ => &self.lut,
        };
        for (o, &p) in out.chunks_exact_mut(4).zip(&self.fb.pixels) {
            o.copy_from_slice(&lut[p as usize]);
        }
    }

    /// Texto curto para o título da janela.
    pub fn status(&self) -> String {
        let s = match self.mode() {
            Mode::Menu => "menu".into(),
            Mode::Briefing => format!("briefing — {}", self.level.0),
            Mode::Play => self.level.0.clone(),
            Mode::Paused => format!("{} — pausa", self.level.0),
            Mode::Results => format!("resultado — {}", self.level.0),
            Mode::Page => "menu".into(),
            Mode::Show => "filme".into(),
        };
        match &self.m9.track_title {
            Some(t) if matches!(self.mode(), Mode::Play | Mode::Paused) => format!("{s} — ♪ {t}"),
            _ => s,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> Option<App> {
        app_at(None)
    }

    fn app_at(level: Option<&str>) -> Option<App> {
        let Some(p) = std::env::var_os("OPENHUNTED_DATA") else {
            eprintln!("pulado: OPENHUNTED_DATA não definido");
            return None;
        };
        Some(App::load(Path::new(&p), level).unwrap())
    }

    #[test]
    fn cancelling_next_briefing_restores_the_mission_for_replay() {
        let Some(mut a) = app() else { return };
        enter(&mut a);
        enter(&mut a);
        win(&mut a, 0);
        a.key_down(Key::Enter); // salta a contagem
        enter(&mut a); // NEXT
        assert_eq!(a.level_file(), "T_M_002.lev");
        a.key_down(Key::Escape); // CANCEL: volta ao resultado da T_J_001
        assert_eq!(a.mode(), Mode::Results);
        assert_eq!(a.mission_index(), Some(0));
        assert_eq!(a.level_file(), "T_J_001.lev");
        a.key_down(Key::Left); // de NEXT a REPLAY
        enter(&mut a);
        assert_eq!(a.mode(), Mode::Briefing);
        assert_eq!(
            a.level_file(),
            "T_J_001.lev",
            "REPLAY repete a missão do resultado"
        );
    }

    #[test]
    fn garathe_level_is_playable_and_escape_does_nothing() {
        let Some(mut a) = app_at(Some("T_M_002.LEV")) else {
            return;
        };
        assert!(a.playable[1], "T_M_002: o Garathe é controlável (M8)");
        assert!(a.playable[0]);
        enter(&mut a); // GO
        assert_eq!(a.mode(), Mode::Play);
        a.key_down(Key::Escape);
        assert_eq!(a.mode(), Mode::Play, "com personagem, Esc não faz nada");
    }

    #[test]
    fn level_outside_the_list_keeps_no_record_and_next_goes_to_menu() {
        let Some(mut a) = app_at(Some("C_2_001.LEV")) else {
            return;
        };
        assert_eq!(a.mission_index(), None);
        enter(&mut a);
        win(&mut a, 1);
        assert_eq!(a.mode(), Mode::Results);
        assert!(a.records.iter().all(|&r| r == 0));
        assert_eq!(a.results().unwrap().cumulative, None);
        a.key_down(Key::Enter); // salta a contagem
        enter(&mut a); // NEXT
        assert_eq!(a.mode(), Mode::Menu);
    }

    #[test]
    fn arrow_key_skips_the_count_up() {
        let Some(mut a) = app() else { return };
        enter(&mut a);
        enter(&mut a);
        win(&mut a, 0);
        a.key_down(Key::Down);
        assert_eq!(a.results().unwrap().progress(), 100);
    }

    #[test]
    fn focus_loss_pauses_and_pause_box_hovers_at_once() {
        let Some(mut a) = app() else { return };
        enter(&mut a);
        enter(&mut a);
        a.advance(100);
        a.pointer_button(true);
        a.focus_lost();
        assert_eq!(a.mode(), Mode::Paused);
        let State::Play { pause: Some(b), .. } = &a.state else {
            panic!()
        };
        let r = b.button_rect;
        a.resume();
        a.pointer_move(r.x + 4, r.y + 4);
        a.key_down(Key::Pause);
        // sem mexer o mouse, um clique já pega o OKAY
        a.pointer_button(true);
        a.pointer_button(false);
        assert_eq!(a.mode(), Mode::Play);
    }

    /// Clica com o teclado no controle sob o ponteiro e espera o despacho.
    fn enter(a: &mut App) {
        a.key_down(Key::Enter);
        for _ in 0..6 {
            a.advance(50);
        }
    }

    fn win(a: &mut App, kills: i32) {
        let s = a.viewer.as_mut().unwrap().scene_mut();
        // a cena conta os mortos pelas vidas (`enemy::kills`): mata `kills` inimigos
        for e in s
            .entities
            .iter_mut()
            .filter(|e| openhunted_engine::behaviour::enemy::counted(e))
            .take(kills as usize)
        {
            e.health = 0;
            e.alive = false;
        }
        s.outcome = Some(Outcome::Victory);
        a.advance(END_DELAY_MS + 50);
    }

    #[test]
    fn menu_new_game_briefing_cancel_and_go() {
        let Some(mut a) = app() else { return };
        assert_eq!(a.mode(), Mode::Menu);
        assert_eq!(a.missions()[0].file, "T_J_001.lev");
        assert!(a.missions().iter().all(|m| m.player != 3 && m.player != 4));
        enter(&mut a); // ponteiro em NEW GAME
        assert_eq!(a.mode(), Mode::Briefing);
        assert_eq!(a.level_file(), "T_J_001.lev");
        a.key_down(Key::Escape);
        assert_eq!(a.mode(), Mode::Menu, "CANCEL volta ao menu");
        enter(&mut a);
        enter(&mut a); // ponteiro em GO
        assert_eq!(a.mode(), Mode::Play);
        assert!(a.take_sounds().contains(&briefing::SOUND_GO));
        a.key_down(Key::Escape);
        assert_eq!(a.mode(), Mode::Play, "Esc no jogo: nada");
        assert!(!a.quit_requested());
    }

    #[test]
    fn pause_freezes_the_game() {
        let Some(mut a) = app() else { return };
        enter(&mut a);
        enter(&mut a);
        a.advance(100);
        a.key_down(Key::Pause);
        assert_eq!(a.mode(), Mode::Paused);
        let t = a.viewer().unwrap().scene().tick_count;
        a.advance(1000);
        assert_eq!(a.viewer().unwrap().scene().tick_count, t);
        let mut rgba = vec![0; 640 * 480 * 4];
        a.render_rgba(&mut rgba);
        a.key_down(Key::Enter);
        assert_eq!(a.mode(), Mode::Play);
        a.advance(100);
        assert!(a.viewer().unwrap().scene().tick_count > t);
    }

    #[test]
    fn victory_results_next_replay_and_main() {
        let Some(mut a) = app() else { return };
        enter(&mut a);
        enter(&mut a);
        win(&mut a, 2);
        assert_eq!(a.mode(), Mode::Results);
        let snd = a.take_sounds();
        assert!(snd.windows(2).any(|w| w == [3780, 3782]), "{snd:?}");
        let r = a.results().unwrap();
        assert_eq!(r.outcome, Outcome::Victory);
        assert_eq!(r.stats.kills, 2);
        assert_eq!(r.stats.kills_total, 7, "as minas da fase");
        assert!(r.bonus >= 500);
        assert_eq!(r.score, 60 + r.bonus);
        assert_eq!(a.records[0], r.score);
        // NEXT (o cursor começa nele): a próxima missão solo
        for _ in 0..30 {
            a.advance(50);
        }
        enter(&mut a);
        assert_eq!(a.mode(), Mode::Briefing);
        // a T_M_002 (Garathe, M8) não é mais pulada
        assert_eq!(a.mission_index(), Some(1));
        assert_eq!(a.level_file(), "T_M_002.lev");
        // CANCEL volta ao resultado; aí o Esc vai ao menu
        a.key_down(Key::Escape);
        assert_eq!(a.mode(), Mode::Results);
        a.key_down(Key::Escape);
        a.advance(50);
        assert_eq!(a.mode(), Mode::Menu);
    }

    #[test]
    fn defeat_replays_the_same_level_and_main_goes_to_menu() {
        let Some(mut a) = app() else { return };
        enter(&mut a);
        enter(&mut a);
        a.viewer.as_mut().unwrap().scene_mut().outcome = Some(Outcome::Defeat);
        a.advance(END_DELAY_MS + 50);
        assert_eq!(a.mode(), Mode::Results);
        assert_eq!(a.results().unwrap().bonus, 0);
        assert_eq!(a.records[0], 0, "derrota não grava recorde");
        a.key_down(Key::Escape);
        assert_eq!(
            a.mode(),
            Mode::Results,
            "Esc ignorado vindo do jogo (só salta a contagem)"
        );
        enter(&mut a); // REPLAY
        assert_eq!(a.mode(), Mode::Briefing);
        assert_eq!(a.level_file(), "T_J_001.lev");
        enter(&mut a);
        a.viewer.as_mut().unwrap().scene_mut().outcome = Some(Outcome::Defeat);
        a.advance(END_DELAY_MS + 50);
        // MAIN: a seta para a direita leva de REPLAY a NEXT e depois a MAIN
        a.key_down(Key::Enter);
        a.key_down(Key::Right);
        a.key_down(Key::Right);
        enter(&mut a);
        assert_eq!(a.mode(), Mode::Menu);
    }

    #[test]
    fn second_victory_reports_improvement() {
        let Some(mut a) = app() else { return };
        enter(&mut a);
        enter(&mut a);
        win(&mut a, 0);
        let first = a.results().unwrap().score;
        a.key_down(Key::Enter); // salta a contagem
        a.key_down(Key::Left); // de NEXT a REPLAY
        enter(&mut a);
        assert_eq!(a.mode(), Mode::Briefing);
        enter(&mut a);
        win(&mut a, 3);
        let r = a.results().unwrap();
        assert_eq!(r.score, first + 90);
        assert_eq!(
            r.screen.text(results::IMPROVE_TEXT),
            Some("MISSION SCORE IMPROVED BY 90 POINTS.")
        );
        assert_eq!(r.cumulative, Some(first + 90));
    }

    #[test]
    fn exit_button_and_escape_quit_from_the_menu() {
        let Some(mut a) = app() else { return };
        a.key_down(Key::Escape);
        assert!(a.quit_requested());
        let Some(mut a) = app() else { return };
        let r = a.screen().unwrap().rect(menu::EXIT).unwrap();
        a.pointer_move(r.x + 5, r.y + 5);
        a.pointer_button(true);
        a.pointer_button(false);
        for _ in 0..6 {
            a.advance(50);
        }
        assert!(a.quit_requested());
    }
}
