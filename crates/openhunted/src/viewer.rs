// SPDX-License-Identifier: GPL-3.0-or-later
//! Estado do visualizador, independente da SDL: câmera, overlay, relógio e conversão do
//! framebuffer indexado para RGBA.

use std::path::Path;

use std::rc::Rc;

use openhunted_engine::shell::hud::{self, Character, HudState, VIEW_H};
use openhunted_engine::{
    Assets, Event, Framebuffer, Outcome, SCREEN_H, SCREEN_W, Scene, TICK_MS, background_index,
    clamp_camera, display_palette, draw_text, measure_text,
};
use openhunted_formats::{Bitmap, Font, Frame, Level, Palette, open_data};

/// Dica do tutorial aberta (modal: o jogo para).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Modal {
    pub text: String,
    pub left_of_jake: bool,
    /// Milissegundos desde a abertura.
    pub open_ms: u32,
}

/// Fecha ao soltar uma tecla depois de 500 ms, ou após 15 s (`docs/re/dicas.md`).
const MODAL_MIN_MS: u32 = 500;
const MODAL_MAX_MS: u32 = 15_000;
/// Cor de fundo da caixa da dica (`0x45ca44`).
const MODAL_FILL: u8 = 211;
/// Cor do HUD de texto (sem `INVBAR01.BMP`) e da moldura de reserva (branco do Windows).
const TEXT_COLOR: u8 = 255;

/// Pixels de rolagem por quadro com uma seta pressionada.
pub const SCROLL_STEP: i32 = 8;

/// Ações do usuário.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    ScrollLeft,
    ScrollRight,
    ScrollUp,
    ScrollDown,
    PageLeft,
    PageRight,
    Home,
    End,
    ToggleOverlay,
}

/// Estado do visualizador.
pub struct Viewer {
    scene: Scene,
    assets: Assets,
    camera: (i32, i32),
    overlay: bool,
    background: u8,
    lut: [[u8; 4]; 256],
    pending_ms: u32,
    fb: Framebuffer,
    input: u32,
    /// Teclas apertadas desde o último tique (um toque curto entre dois tiques não se perde).
    taps: u32,
    font: Option<Font>,
    /// Os 8 quadros de `SWIZARD.BMP`: cantos e bordas da moldura da dica.
    frame_tiles: Vec<Frame>,
    modal: Option<Modal>,
    /// Dicas à espera (duas no mesmo tique: uma de cada vez).
    queued: std::collections::VecDeque<Modal>,
    /// Efeitos sonoros (sem eles, o jogo corre mudo).
    sfx: Option<crate::audio::Sfx>,
    /// `INVBAR01.BMP`: a barra de estado embaixo da vista.
    bar: Option<Rc<Bitmap>>,
}

impl Viewer {
    /// Abre os dados do jogo e monta a cena da fase.
    pub fn load(data: &Path, level: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let gd = open_data(data)?;
        let lv = Level::parse(&gd.levels()?.get(level)?).map_err(|e| e.in_resource(level))?;
        Self::from_level(&lv, Assets::new(gd.resources()?))
    }

    /// Monta a cena de uma fase já lida, com os recursos dados.
    pub fn from_level(lv: &Level, mut assets: Assets) -> Result<Self, Box<dyn std::error::Error>> {
        let palette = display_palette(&Palette::parse(assets.archive().get(&lv.palette)?)?);
        let scene = Scene::build(lv, &mut assets)?;
        for w in &scene.warnings {
            eprintln!("aviso: {w}");
        }
        let mut v = Self::new(scene, assets, &palette);
        v.font = v
            .assets
            .archive()
            .get("SLEVD.PFT")
            .ok()
            .and_then(|d| Font::parse(d).ok());
        v.frame_tiles = v
            .assets
            .archive()
            .get("SWIZARD.BMP")
            .ok()
            .and_then(|d| Bitmap::parse(d).ok())
            .map(|b| b.frames)
            .unwrap_or_default();
        v.bar = v.assets.bitmap("INVBAR01.BMP").ok();
        Ok(v)
    }

    pub fn new(scene: Scene, assets: Assets, palette: &Palette) -> Self {
        let mut lut = [[0u8; 4]; 256];
        for (l, c) in lut.iter_mut().zip(palette.0.iter()) {
            *l = [c[0], c[1], c[2], 255];
        }
        Self {
            scene,
            assets,
            camera: (0, 0),
            overlay: false,
            background: background_index(palette),
            lut,
            pending_ms: 0,
            fb: Framebuffer::new(SCREEN_W, SCREEN_H),
            input: 0,
            taps: 0,
            font: None,
            frame_tiles: Vec::new(),
            modal: None,
            queued: std::collections::VecDeque::new(),
            sfx: None,
            bar: None,
        }
    }

    /// Põe a câmera no Jake (centrado na vista do jogo), se houver.
    pub fn follow(&mut self) {
        if let Some((fx, fy)) = self.scene.focus() {
            let view = (SCREEN_W as i32, VIEW_H);
            let want = (fx - view.0 / 2, fy - view.1 / 2);
            self.camera = clamp_camera((self.scene.width, self.scene.height), view, want);
        }
    }

    #[cfg(test)]
    pub fn scene_mut(&mut self) -> &mut Scene {
        &mut self.scene
    }

    pub fn scene(&self) -> &Scene {
        &self.scene
    }

    /// O que a barra de estado mostra. Sem o Jake (fases do Garathe, ainda sem personagem
    /// controlável), a barra do Garathe zerada.
    pub fn hud_state(&self) -> HudState {
        let (character, health, armor, slots, weapon) = match self.scene.jake.as_ref() {
            Some(j) => {
                let s = j.inventory.slots;
                // arma corrente: a pistola, se ele a tem; senão os punhos
                let w = if s[openhunted_engine::jake::PISTOL].0 != 0 {
                    openhunted_engine::jake::PISTOL
                } else {
                    1
                };
                (Character::Jake, j.health.hp, j.health.armor, s, w)
            }
            None => (Character::Garathe, 0, 0, [(0, 0); 30], 1),
        };
        HudState {
            character,
            health,
            armor,
            face_alt: false,
            slots,
            weapon,
            timer: self.scene.timer,
        }
    }

    /// Liga os efeitos sonoros: os sons de cada tique vão a `sfx`.
    pub fn set_sound(&mut self, sfx: crate::audio::Sfx) {
        self.sfx = Some(sfx);
    }

    /// Devolve os efeitos sonoros (para passá-los à próxima fase).
    pub fn take_sound(&mut self) -> Option<crate::audio::Sfx> {
        self.sfx.take()
    }

    /// Toca os sons de um tique, ouvidos do centro da vista do jogo na camada do Jake.
    pub fn play_sounds(&mut self, cues: &[openhunted_engine::sound::Cue]) {
        let Some(sfx) = self.sfx.as_mut() else { return };
        if cues.is_empty() {
            return;
        }
        let listener = (
            self.camera.0 + SCREEN_W as i32 / 2,
            self.camera.1 + VIEW_H / 2,
        );
        sfx.tick(self.assets.archive(), cues, listener, self.scene.jake_layer);
    }

    pub fn modal(&self) -> Option<&Modal> {
        self.modal.as_ref()
    }

    pub fn outcome(&self) -> Option<Outcome> {
        self.scene.outcome
    }

    /// Centro do Jake em px da fase, se houver.
    pub fn focus(&self) -> Option<(i32, i32)> {
        self.scene.focus()
    }

    /// Texto do HUD.
    pub fn hud_text(&self) -> Option<String> {
        self.scene.jake.as_ref().map(|j| {
            format!(
                "VIDA {}  ARMADURA {}  PISTOLA {}",
                j.health.hp,
                j.health.armor,
                j.inventory.pistol_ammo()
            )
        })
    }

    /// Uma tecla foi solta: fecha a dica se ela já está aberta há 500 ms.
    pub fn key_released(&mut self) {
        if self
            .modal
            .as_ref()
            .is_some_and(|m| m.open_ms >= MODAL_MIN_MS)
        {
            self.modal = self.queued.pop_front();
        }
    }

    pub fn camera(&self) -> (i32, i32) {
        self.camera
    }

    pub fn overlay(&self) -> bool {
        self.overlay
    }

    pub fn apply(&mut self, action: Action) {
        let (mut x, mut y) = self.camera;
        let page = SCREEN_W as i32;
        match action {
            Action::ScrollLeft => x -= SCROLL_STEP,
            Action::ScrollRight => x += SCROLL_STEP,
            Action::ScrollUp => y -= SCROLL_STEP,
            Action::ScrollDown => y += SCROLL_STEP,
            Action::PageLeft => x -= page,
            Action::PageRight => x += page,
            Action::Home => (x, y) = (0, 0),
            Action::End => x = i32::MAX,
            Action::ToggleOverlay => self.overlay = !self.overlay,
        }
        let view = (SCREEN_W as i32, VIEW_H);
        self.camera = clamp_camera((self.scene.width, self.scene.height), view, (x, y));
    }

    /// A fase tem o Jake controlável (as setas o controlam em vez de rolar a tela).
    pub fn has_jake(&self) -> bool {
        self.scene.jake.is_some()
    }

    /// Bits de entrada do jogador (`openhunted_engine::input`) usados nos próximos tiques.
    pub fn set_input(&mut self, input: u32) {
        self.input = input;
    }

    /// Registra um toque de tecla (ex.: `JUMP`): entra no próximo tique mesmo que a tecla já
    /// tenha sido solta.
    pub fn tap(&mut self, bits: u32) {
        self.taps |= bits;
    }

    /// Avança o relógio em `ms` milissegundos; devolve quantos ticks de animação rodaram.
    /// Com uma dica aberta, o jogo não anda (só o relógio da dica).
    pub fn advance(&mut self, ms: u32) -> u32 {
        if let Some(m) = self.modal.as_mut() {
            m.open_ms = m.open_ms.saturating_add(ms);
            if m.open_ms > MODAL_MAX_MS {
                self.modal = self.queued.pop_front();
            }
            return 0;
        }
        self.pending_ms = self.pending_ms.saturating_add(ms);
        let ticks = self.pending_ms / TICK_MS;
        self.pending_ms %= TICK_MS;
        let mut done = 0;
        for _ in 0..ticks {
            self.scene.tick(self.input | self.taps);
            self.taps = 0;
            done += 1;
            if let Some((fx, fy)) = self.scene.focus() {
                // A câmera segue o Jake, centrado na vista do jogo (640×405).
                let view = (SCREEN_W as i32, VIEW_H);
                let want = (fx - view.0 / 2, fy - view.1 / 2);
                self.camera = clamp_camera((self.scene.width, self.scene.height), view, want);
            }
            let mut opened = false;
            let mut cues = Vec::new();
            for ev in self.scene.events.drain(..) {
                if let Event::Sound(c) = ev {
                    cues.push(c);
                } else if let Event::Hint { text, left_of_jake } = ev {
                    let m = Modal {
                        text,
                        left_of_jake,
                        open_ms: 0,
                    };
                    if self.modal.is_none() {
                        self.modal = Some(m);
                    } else {
                        self.queued.push_back(m);
                    }
                    opened = true;
                }
                // veredito: nada a fazer aqui por enquanto
            }
            self.play_sounds(&cues);
            if opened {
                self.pending_ms = 0;
                break;
            }
        }
        done
    }

    /// Geometria da caixa da dica (`docs/re/dicas.md`): área do texto × 8/5 × 1,25 / 4,
    /// lado `h = isqrt`, caixa `4h × h`, centrada na vertical e ao lado do Jake.
    fn modal_box(&self, font: &Font, text: &str) -> (i32, i32, i32, i32, Vec<String>) {
        let (tw, th) = measure_text(font, text);
        let area = i64::from(tw.max(1)) * i64::from(th.max(1));
        let s = (area * 8 / 5) * 125 / 100 / 4;
        let mut h = (s as f64).sqrt() as i32;
        h = h.max(th + 8);
        let mut lines;
        loop {
            lines = wrap(font, text, 4 * h - 16);
            if (lines.len() as i32) * (th + 2) <= h - 8 || h >= VIEW_H - 40 {
                break;
            }
            h += th;
        }
        let w = 4 * h;
        // centrada na vista do jogo (640×405), acima da barra de estado
        let y = (VIEW_H - h) / 2;
        let x = match (self.scene.jake.as_ref(), self.modal.as_ref()) {
            (Some(j), Some(m)) => {
                let jb = j.body.bounds();
                let (jl, jr) = (jb.x - self.camera.0, jb.x + jb.w - self.camera.0);
                if m.left_of_jake {
                    (jl - w - 20).max(16)
                } else {
                    (jr + 20).min(SCREEN_W as i32 - w - 16)
                }
            }
            _ => (SCREEN_W as i32 - w) / 2,
        };
        (x, y, w, h, lines)
    }

    /// Moldura de `SWIZARD.BMP` (3 × 3 peças) em volta de (x, y, w, h) com margem de 8 px.
    fn draw_frame(&mut self, x: i32, y: i32, w: i32, h: i32) {
        let (x, y, w, h) = (x - 8, y - 8, w + 16, h + 16);
        self.fb.fill_rect(x, y, w, h, MODAL_FILL);
        let t = &self.frame_tiles;
        if t.len() < 8 {
            self.fb.rect(x, y, w, h, TEXT_COLOR);
            return;
        }
        let (tl, top, tr, left, right, bl, bottom, br) =
            (&t[0], &t[1], &t[2], &t[3], &t[4], &t[5], &t[6], &t[7]);
        let (cw, ch) = (i32::from(tl.width), i32::from(tl.height));
        let (bw, bh) = (i32::from(br.width), i32::from(br.height));
        let mut cx = x + cw;
        while cx < x + w - i32::from(tr.width) {
            self.fb.blit(top, cx, y, true);
            self.fb
                .blit(bottom, cx, y + h - i32::from(bottom.height), true);
            cx += i32::from(top.width);
        }
        let mut cy = y + ch;
        while cy < y + h - bh {
            self.fb.blit(left, x, cy, true);
            self.fb
                .blit(right, x + w - i32::from(right.width), cy, true);
            cy += i32::from(left.height);
        }
        self.fb.blit(tl, x, y, true);
        self.fb.blit(tr, x + w - i32::from(tr.width), y, true);
        self.fb.blit(bl, x, y + h - i32::from(bl.height), true);
        self.fb.blit(br, x + w - bw, y + h - bh, true);
    }

    /// Desenha a vista do jogo (640×405), a barra de estado embaixo e a dica aberta.
    pub fn render(&mut self) -> &Framebuffer {
        self.fb.clear(self.background);
        self.scene
            .draw(&mut self.assets, &mut self.fb, self.camera, self.overlay);
        let font = self.font.clone();
        match (self.bar.clone(), font.as_ref()) {
            (Some(bar), _) => {
                let w = SCREEN_W as i32;
                self.fb.fill_rect(0, VIEW_H, w, SCREEN_H as i32 - VIEW_H, 0);
                let state = self.hud_state();
                hud::draw_hud(&mut self.fb, &bar, 0, VIEW_H, w, &state);
            }
            (None, Some(font)) => {
                if let Some(hud) = self.hud_text() {
                    draw_text(&mut self.fb, font, 8, 8, &hud, Some(TEXT_COLOR));
                }
            }
            (None, None) => {}
        }
        if let (Some(font), Some(m)) = (font, self.modal.clone()) {
            let (x, y, w, h, lines) = self.modal_box(&font, &m.text);
            self.draw_frame(x, y, w, h);
            let th = i32::from(font.height) + 2;
            for (i, line) in lines.iter().enumerate() {
                draw_text(
                    &mut self.fb,
                    &font,
                    x + 8,
                    y + 4 + th * i as i32,
                    line,
                    None,
                );
            }
        }
        &self.fb
    }

    /// Desenha e converte para RGBA (640 × 480 × 4 bytes).
    pub fn render_rgba(&mut self, out: &mut [u8]) {
        self.render();
        for (o, &p) in out.chunks_exact_mut(4).zip(&self.fb.pixels) {
            o.copy_from_slice(&self.lut[p as usize]);
        }
    }
}

/// Quebra `text` em linhas de até `width` px, por palavras.
fn wrap(font: &Font, text: &str, width: i32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let candidate = if line.is_empty() {
            word.to_string()
        } else {
            format!("{line} {word}")
        };
        if measure_text(font, &candidate).0 <= width || line.is_empty() {
            line = candidate;
        } else {
            lines.push(std::mem::replace(&mut line, word.to_string()));
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use openhunted_engine::{Assets, Scene};
    use openhunted_formats::{Palette, ResourceArchive, test_support::mkarchive};

    fn viewer(w: i32, h: i32) -> Viewer {
        let (map, vols) = mkarchive(&[("RESOURCE.001", &[])]);
        let assets =
            Assets::new(ResourceArchive::from_map(&map, |_| Ok(vols[0].1.clone())).unwrap());
        let scene = Scene::empty(w, h);
        let mut pal = [[100u8, 100, 100]; 256];
        pal[3] = [0, 0, 0];
        Viewer::new(scene, assets, &Palette(pal))
    }

    #[test]
    fn scrolling_is_clamped_to_scene() {
        let mut v = viewer(1792, 128);
        v.apply(Action::End);
        assert_eq!(v.camera(), (1152, 0));
        v.apply(Action::PageRight);
        assert_eq!(v.camera(), (1152, 0));
        v.apply(Action::Home);
        v.apply(Action::ScrollLeft);
        v.apply(Action::ScrollUp);
        assert_eq!(v.camera(), (0, 0));
        v.apply(Action::ScrollRight);
        assert_eq!(v.camera(), (SCROLL_STEP, 0));
        v.apply(Action::PageLeft);
        assert_eq!(v.camera(), (0, 0));
    }

    #[test]
    fn overlay_toggles_and_clock_ticks() {
        let mut v = viewer(640, 480);
        assert!(!v.overlay());
        v.apply(Action::ToggleOverlay);
        assert!(v.overlay());
        assert_eq!(v.advance(120), 2); // 50 ms por tick: 2 ticks e 20 ms guardados
        assert_eq!(v.advance(30), 1);
    }

    #[test]
    fn input_is_kept_for_next_ticks() {
        let mut v = viewer(640, 480);
        assert!(!v.has_jake());
        v.set_input(openhunted_engine::input::LEFT);
        assert_eq!(v.advance(50), 1);
    }

    #[test]
    fn taps_reach_the_next_tick_only() {
        let mut v = viewer(640, 480);
        v.tap(openhunted_engine::input::JUMP);
        assert_eq!(v.advance(20), 0);
        assert_eq!(
            v.taps,
            openhunted_engine::input::JUMP,
            "guardado até haver tique"
        );
        assert_eq!(v.advance(30), 1);
        assert_eq!(v.taps, 0);
    }

    #[test]
    fn hint_pauses_clock_and_closes_on_release() {
        let mut v = viewer(640, 480);
        v.modal = Some(Modal {
            text: "Oi".into(),
            left_of_jake: true,
            open_ms: 0,
        });
        assert_eq!(v.advance(200), 0, "com a dica aberta o jogo não anda");
        v.key_released();
        assert!(v.modal().is_some(), "antes de 500 ms não fecha");
        v.advance(400);
        v.key_released();
        assert!(v.modal().is_none());
        v.modal = Some(Modal {
            text: "Oi".into(),
            left_of_jake: true,
            open_ms: 0,
        });
        v.advance(16_000);
        assert!(v.modal().is_none(), "fecha sozinha após 15 s");
        assert_eq!(v.advance(50), 1, "e o jogo volta a andar");
    }

    #[test]
    fn two_hints_in_one_tick_open_one_after_the_other() {
        let mut v = viewer(640, 480);
        v.scene.events.push(Event::Hint {
            text: "um".into(),
            left_of_jake: true,
        });
        v.scene.events.push(Event::Hint {
            text: "dois".into(),
            left_of_jake: false,
        });
        v.advance(50);
        assert_eq!(v.modal().map(|m| m.text.as_str()), Some("um"));
        v.advance(600);
        v.key_released();
        assert_eq!(v.modal().map(|m| m.text.as_str()), Some("dois"));
        v.advance(600);
        v.key_released();
        assert!(v.modal().is_none());
    }

    #[test]
    fn sounds_reach_the_sink_heard_from_the_camera() {
        use crate::audio::tests::{Log, wav_bytes};
        use openhunted_engine::sound::Cue;
        let wav = wav_bytes();
        let (map, vols) = mkarchive(&[(
            "RESOURCE.001",
            &[("SX_3774.WAV", &wav), ("SX_3040.WAV", &wav)],
        )]);
        let assets =
            Assets::new(ResourceArchive::from_map(&map, |_| Ok(vols[0].1.clone())).unwrap());
        let mut v = Viewer::new(Scene::empty(2000, 480), assets, &Palette([[0u8; 3]; 256]));
        v.advance(50); // sem destino: nada acontece
        let log = Log::default();
        v.set_sound(crate::audio::Sfx::new(Box::new(log.clone())));
        v.apply(Action::PageRight);
        let (cx, cy) = (v.camera().0 + SCREEN_W as i32 / 2, SCREEN_H as i32 / 2);
        v.scene.events.push(Event::Hint {
            text: "Oi".into(),
            left_of_jake: true,
        });
        v.scene.events.push(Event::Sound(Cue::ui(3774, 0)));
        v.scene
            .events
            .push(Event::Sound(Cue::at(3040, 9, (cx - 384, cy), 0)));
        v.scene
            .events
            .push(Event::Sound(Cue::at(3040, 9, (cx, cy), 1)));
        assert_eq!(v.advance(50), 1);
        assert!(v.modal().is_some());
        assert_eq!(
            *log.0.borrow(),
            vec![(3774, 0, 100, 0), (3040, 9, 50, -64)],
            "a dica abre e o som dela toca; o tiro à esquerda da câmera, pela metade; \
             outra camada não se ouve"
        );
    }

    #[test]
    fn hint_box_is_centred_on_the_game_view() {
        use openhunted_formats::test_support::mkpft;
        let glyphs: Vec<(u32, Vec<u8>)> = (0..3).map(|_| (2, vec![1; 4])).collect();
        let font = Font::parse(&mkpft('a' as u32, &glyphs)).unwrap();
        let v = viewer(640, 480);
        let (_, y, _, h, _) = v.modal_box(&font, "aa bb cc");
        assert_eq!(y, (VIEW_H - h) / 2);
    }

    #[test]
    fn wrap_breaks_by_words() {
        use openhunted_formats::test_support::mkpft;
        // glifos de 2 px para 'a'..'c'; sem glifo de espaço, ele vale 4 px
        let glyphs: Vec<(u32, Vec<u8>)> = (0..3).map(|_| (2, vec![1; 4])).collect();
        let font = Font::parse(&mkpft('a' as u32, &glyphs)).unwrap();
        // "aa" = 5 px; "aa bb" = 5 + 1 + 4 + 1 + 5 = 16
        let lines = wrap(&font, "aa bb cc", 16);
        assert_eq!(lines, vec!["aa bb", "cc"]);
        assert_eq!(wrap(&font, "aa bb cc", 10), vec!["aa", "bb", "cc"]);
    }

    #[test]
    fn render_maps_background_through_palette() {
        let mut v = viewer(640, 480);
        let mut out = vec![9u8; 640 * 480 * 4];
        v.render_rgba(&mut out);
        assert_eq!(&out[..4], &[0, 0, 0, 255]);
    }
}
