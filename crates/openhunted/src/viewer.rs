// SPDX-License-Identifier: GPL-3.0-or-later
//! Estado do visualizador, independente da SDL: câmera, overlay, relógio e conversão do
//! framebuffer indexado para RGBA.

use std::path::Path;

use openhunted_engine::{
    Assets, Framebuffer, SCREEN_H, SCREEN_W, Scene, TICK_MS, background_index, clamp_camera,
};
use openhunted_formats::{Level, Palette, open_data};

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
}

impl Viewer {
    /// Abre os dados do jogo e monta a cena da fase.
    pub fn load(data: &Path, level: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let gd = open_data(data)?;
        let lv = Level::parse(&gd.levels()?.get(level)?).map_err(|e| e.in_resource(level))?;
        let mut assets = Assets::new(gd.resources()?);
        let palette = Palette::parse(assets.archive().get(&lv.palette)?)?;
        let scene = Scene::build(&lv, &mut assets)?;
        for w in &scene.warnings {
            eprintln!("aviso: {w}");
        }
        Ok(Self::new(scene, assets, &palette))
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
        let view = (SCREEN_W as i32, SCREEN_H as i32);
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
    pub fn advance(&mut self, ms: u32) -> u32 {
        self.pending_ms = self.pending_ms.saturating_add(ms);
        let ticks = self.pending_ms / TICK_MS;
        self.pending_ms %= TICK_MS;
        for _ in 0..ticks {
            self.scene.tick(self.input | self.taps);
            self.taps = 0;
            if let Some((fx, fy)) = self.scene.focus() {
                // A câmera segue o Jake, centrado na tela.
                let view = (SCREEN_W as i32, SCREEN_H as i32);
                let want = (fx - view.0 / 2, fy - view.1 / 2);
                self.camera = clamp_camera((self.scene.width, self.scene.height), view, want);
            }
        }
        ticks
    }

    /// Desenha a cena e converte para RGBA (640 × 480 × 4 bytes).
    pub fn render_rgba(&mut self, out: &mut [u8]) {
        self.fb.clear(self.background);
        self.scene
            .draw(&mut self.assets, &mut self.fb, self.camera, self.overlay);
        for (o, &p) in out.chunks_exact_mut(4).zip(&self.fb.pixels) {
            o.copy_from_slice(&self.lut[p as usize]);
        }
    }
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
    fn render_maps_background_through_palette() {
        let mut v = viewer(640, 480);
        let mut out = vec![9u8; 640 * 480 * 4];
        v.render_rgba(&mut out);
        assert_eq!(&out[..4], &[0, 0, 0, 255]);
    }
}
