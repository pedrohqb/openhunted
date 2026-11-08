// SPDX-License-Identifier: GPL-3.0-or-later
//! Desenho de uma vista como `0x405c7c` (`docs/re/camadas-desenho.md`): a camada da vista
//! opaca, as de trás pelos buracos das células, escurecidas (27…33) e no modo "atrás";
//! objetos só na camada em desenho.

use super::view::{self, View, depth_shadow, intersect};
use super::{Layer, Scene, Sprite, light};
use crate::entity::{Behaviour, Entity};
use crate::physics::Rect;
use crate::render::Blend;
use crate::tint::{Dls, Tint};
use crate::{Assets, Framebuffer, draw_ops};

/// Cores do overlay (índices da paleta).
const OVERLAY_BORDER: u8 = 255;
const OVERLAY_MARKER: u8 = 254;
const OVERLAY_OBJECT: u8 = 253;

/// Maior lado, em px do mundo, de um desenho de sprite (defesa, não do original): o maior
/// objeto legítimo das 100 fases mede menos de 400 px; uma escala acima disto é um erro de
/// medida (como o do quadro de 1 px da explosão) e o desenho é pulado.
const MAX_DRAWN: i32 = 2048;

/// Como desenhar numa camada: projeção (`d`), modo e recorte.
#[derive(Clone, Copy)]
struct Pen<'a> {
    view: &'a View,
    d: i32,
    blend: Blend,
    clip: Rect,
}

impl Pen<'_> {
    fn rect(&self, r: Rect) -> Rect {
        self.view.project_rect(r, self.d)
    }
}

/// Desenha um sprite com a projeção da caneta; com `map`, pela tabela de cor.
fn draw_sprite(
    sp: &Sprite,
    assets: &mut Assets,
    fb: &mut Framebuffer,
    pen: &Pen,
    map: Option<&[u8; 256]>,
) {
    for op in draw_ops(&sp.anim, sp.player.frame()) {
        let Ok(bmp) = assets.bitmap(&op.bitmap) else {
            continue;
        };
        let Some(frame) = bmp.frames.get(op.frame as usize) else {
            continue;
        };
        let (num, den) = sp.scale;
        let sc = |v: i32| (i64::from(v) * i64::from(num) / i64::from(den.max(1))) as i32;
        let mut ox = sc(op.x);
        if sp.mirror {
            // espelha em torno do quadro de referência (quadro 0 da animação)
            let rw = sp.anim.frames.first().map_or(0, |f| i32::from(f.width));
            ox = sc(rw) - ox - sc(i32::from(frame.width));
        }
        let world = Rect {
            x: sp.x.saturating_add(ox),
            y: sp.y.saturating_add(sc(op.y)),
            w: sc(i32::from(frame.width)).max(1),
            h: sc(i32::from(frame.height)).max(1),
        };
        if world.w > MAX_DRAWN || world.h > MAX_DRAWN {
            continue;
        }
        // amostragem pela escala composta (a do sprite e a da camada), como `blit_scaled`
        let r = (
            i64::from(num.max(1)) * i64::from(pen.view.reference()),
            i64::from(den.max(1)) * i64::from(pen.d.max(1)),
        );
        let dst = pen.rect(world);
        fb.blit_ratio(frame, dst, (r, r), sp.mirror, map, pen.blend, pen.clip);
    }
}

/// Tabela de cor `T[i] = S[A[i]]` (`0x43becd`) com a sombra `shadow`; sem cor nem sombra,
/// nenhuma.
fn table(dls: Option<&Dls>, tint: Option<Tint>, shadow: usize) -> Option<[u8; 256]> {
    if tint.is_none() && shadow == 0 {
        return None;
    }
    Some(dls?.table(tint.unwrap_or(Tint::new(1, 0)), shadow))
}

/// Sombra de quem está numa célula com luz: a da célula (18 + s) ou a da profundidade.
fn shade(look: Option<light::Look>, depth: i32) -> usize {
    match look {
        Some(light::Look::Shaded(s)) => s,
        _ => depth_shadow(depth),
    }
}

impl Scene {
    /// A vista do jogador `i`: câmera `camera` (canto superior esquerdo da camada da vista,
    /// em 1:1), retângulo `screen` no quadro e tamanho `size` da área de jogo (o centro fica
    /// no meio dela; a barra cobre o resto).
    pub fn view_of(&self, i: usize, camera: (i32, i32), screen: Rect, size: (i32, i32)) -> View {
        let state =
            self.views.get(i).copied().unwrap_or_else(|| {
                view::ViewState::new(self.layer_of(i).unwrap_or(self.layers - 1))
            });
        let mut v = View::new(screen, size, camera, state);
        v.views = self.players.len().max(1) as i32;
        v.player = Some(i);
        v
    }

    /// Tique das vistas depois dos objetos: troca de camada de cada jogador (estado antes,
    /// `prev`), transição (`0x436b20`) e zoom até o `+0x60` da célula sob ele.
    pub(super) fn tick_views(&mut self, prev: &[Option<crate::State>]) {
        let n = self.players.len().max(1);
        self.views.resize_with(n, || view::ViewState::new(0));
        for i in 0..n {
            let Some(j) = self.player_jake(i) else {
                continue;
            };
            let (now, layer) = (Some(j.state()), j.layer());
            let focus = self.focus_of(i);
            let mut v = self.view_of(
                i,
                (0, 0),
                Rect {
                    x: 0,
                    y: 0,
                    w: 0,
                    h: 0,
                },
                (crate::SCREEN_W as i32, crate::shell::hud::VIEW_H),
            );
            v.views = n as i32;
            let start = v.layer_distance(layer);
            let st = &mut self.views[i];
            st.follow(prev.get(i).copied().flatten(), now, layer, start);
            let target = focus.map_or(0, |c| self.cell_map.zoom_at(c, st.layer));
            st.tick(self.layers, target);
        }
    }

    /// Desenha a vista `v` (`0x405c7c` a partir da camada dela). Com `overlay`, as caixas de
    /// colisão, as zonas e os marcadores da camada da vista.
    pub fn draw_view(&self, assets: &mut Assets, fb: &mut Framebuffer, v: &View, overlay: bool) {
        let dls = assets.dls();
        let fbr = Rect {
            x: 0,
            y: 0,
            w: fb.width as i32,
            h: fb.height as i32,
        };
        let clip = intersect(v.screen, fbr);
        if clip.w <= 0 || clip.h <= 0 {
            return;
        }
        self.draw_layer(assets, fb, dls.as_deref(), v, v.state.layer, 0, clip);
        if overlay {
            self.draw_overlay(fb, v, clip);
        }
    }

    /// Uma camada na profundidade `depth`, recortada por `clip` (retângulo do quadro).
    #[allow(clippy::too_many_arguments)]
    fn draw_layer(
        &self,
        assets: &mut Assets,
        fb: &mut Framebuffer,
        dls: Option<&Dls>,
        v: &View,
        layer: i32,
        depth: i32,
        clip: Rect,
    ) {
        let d = v.layer_distance(layer);
        let back = depth > 0;
        let pen = Pen {
            view: v,
            d,
            blend: if back {
                Blend::Behind
            } else {
                Blend::Transparent
            },
            clip,
        };
        let shadow = depth_shadow(depth);
        if !back && self.cell_map.grid.0 > 0 {
            // fora da grade e células vazias ou apagadas: cor 245 (`0x405899`, `0x4061ee`)
            fb.fill_clipped(clip, view::EMPTY_COLOUR, clip);
        }
        if back {
            self.draw_decals(assets, fb, dls, &pen, layer, depth, true);
            self.draw_objects(assets, fb, dls, &pen, layer, depth);
            self.draw_decals(assets, fb, dls, &pen, layer, depth, false);
        }
        // cenário: opaco na camada da vista (o 0 fica 0), atrás nas outras
        let scenery = Pen {
            blend: if back { Blend::Behind } else { Blend::Opaque },
            ..pen
        };
        for (i, sp) in self.sprites.iter().enumerate() {
            if sp.layer != Layer::Scenery || !self.sprite_on(i, layer, v.state.layer) {
                continue;
            }
            let look = self.sprite_look(i);
            if look == Some(light::Look::Hidden) {
                continue;
            }
            // a luz da célula só vale na camada da vista (`0x40613b`)
            let s = if back { shadow } else { shade(look, 0) };
            let m = table(dls, None, s);
            draw_sprite(sp, assets, fb, &scenery, m.as_ref());
        }
        // buracos: a camada de trás, uma profundidade abaixo (`0x40625c`)
        if layer > 0 && layer < self.cell_map.grid.2 {
            let seen = v.unproject(clip, d);
            for &(cell, hole) in &self.cell_map.holes {
                if cell.2 != layer || self.cell_hidden(cell) {
                    continue;
                }
                let h = intersect(hole, seen);
                if h.w <= 0 || h.h <= 0 {
                    continue;
                }
                let grown = Rect {
                    x: h.x - 1,
                    y: h.y - 1,
                    w: h.w + 2,
                    h: h.h + 2,
                };
                let r = intersect(pen.rect(grown), intersect(v.screen, clip_frame(fb)));
                if r.w > 0 && r.h > 0 {
                    self.draw_layer(assets, fb, dls, v, layer - 1, depth + 1, r);
                }
            }
        }
        if !back {
            let pen = Pen { clip, ..pen };
            self.draw_decals(assets, fb, dls, &pen, layer, depth, false);
            self.draw_objects(assets, fb, dls, &pen, layer, depth);
            self.draw_decals(assets, fb, dls, &pen, layer, depth, true);
        }
    }

    /// Célula do sprite `i` de `sprites`; nenhuma nas cenas montadas ou mexidas à mão (as
    /// listas não batem).
    fn sprite_cell(&self, i: usize) -> Option<view::SpriteCell> {
        if self.sprite_cells.len() != self.sprites.len() {
            return None;
        }
        self.sprite_cells.get(i).copied().flatten()
    }

    /// O sprite `i` é da camada `layer`; sem célula, é da camada da vista `own`.
    fn sprite_on(&self, i: usize, layer: i32, own: i32) -> bool {
        self.sprite_cell(i).map_or(own, |c| c.cell.2) == layer
    }

    fn sprite_look(&self, i: usize) -> Option<light::Look> {
        self.sprite_light
            .get(i)
            .copied()
            .flatten()
            .and_then(|c| self.lights.look(c))
    }

    /// A célula está apagada e invisível (bit 0 de `+0x38`).
    fn cell_hidden(&self, cell: (i32, i32, i32)) -> bool {
        self.lights.index_of(cell).and_then(|c| self.lights.look(c)) == Some(light::Look::Hidden)
    }

    /// Decalques da camada: os da frente (`+0x24 ≠ 0`) ou os de trás.
    #[allow(clippy::too_many_arguments)]
    fn draw_decals(
        &self,
        assets: &mut Assets,
        fb: &mut Framebuffer,
        dls: Option<&Dls>,
        pen: &Pen,
        layer: i32,
        depth: i32,
        front: bool,
    ) {
        for (i, sp) in self.sprites.iter().enumerate() {
            // decalques (e sprites soltos de outra camada de desenho, nas cenas à mão)
            if sp.layer == Layer::Scenery || !self.sprite_on(i, layer, pen.view.state.layer) {
                continue;
            }
            let is_front = self.sprite_cell(i).is_some_and(|c| c.front);
            if is_front != front {
                continue;
            }
            let look = self.sprite_look(i);
            if look == Some(light::Look::Hidden) {
                continue;
            }
            let m = table(dls, None, shade(look, depth));
            draw_sprite(sp, assets, fb, pen, m.as_ref());
        }
    }

    /// Objetos da camada (`0x43558e`): as entidades e depois os jogadores.
    fn draw_objects(
        &self,
        assets: &mut Assets,
        fb: &mut Framebuffer,
        dls: Option<&Dls>,
        pen: &Pen,
        layer: i32,
        depth: i32,
    ) {
        for (i, e) in self.entities.iter().enumerate() {
            if !e.visible() || e.layer != layer {
                continue;
            }
            let look = self.lights.look_at(e.center(), e.layer);
            if look == Some(light::Look::Hidden) {
                continue;
            }
            let s = shade(look, depth);
            match (&e.behaviour, &e.sprite) {
                (Behaviour::Graffiti(g), Some(sp)) => {
                    let m = table(dls, None, s);
                    for &(dx, frame) in &g.glyphs {
                        let glyph = Sprite {
                            player: crate::AnimPlayer::from_frame(&sp.anim, frame),
                            x: e.x.saturating_add(dx),
                            y: e.y,
                            scale: g.scale,
                            mirror: false,
                            ..sp.clone()
                        };
                        draw_sprite(&glyph, assets, fb, pen, m.as_ref());
                    }
                }
                (Behaviour::Laser(l), _) => draw_laser(e, l, fb, pen, table(dls, None, s).as_ref()),
                (_, Some(sp)) => {
                    let m = table(dls, crate::tint::entity(e, i, self.tick_count), s);
                    draw_sprite(sp, assets, fb, pen, m.as_ref());
                }
                _ => {}
            }
        }
        // os jogadores: o da vista em transição fica no tamanho do começo (`+0x4c`)
        for i in 0..self.players.len().max(1) {
            let (Some(sp), Some(j)) = (self.player_sprite(i), self.player_jake(i)) else {
                continue;
            };
            if !j.is_visible() || j.layer() != layer {
                continue;
            }
            let look = self
                .focus_of(i)
                .and_then(|c| self.lights.look_at(c, j.layer()));
            if look == Some(light::Look::Hidden) {
                continue;
            }
            let mut p = *pen;
            if pen.view.player == Some(i) && pen.view.state.counter != 0 {
                p.d = pen.view.state.focus_distance.max(1);
            }
            let m = table(dls, Some(j.tint()), shade(look, depth));
            draw_sprite(sp, assets, fb, &p, m.as_ref());
        }
    }

    fn draw_overlay(&self, fb: &mut Framebuffer, v: &View, clip: Rect) {
        let d = v.layer_distance(v.state.layer);
        let pen = Pen {
            view: v,
            d,
            blend: Blend::Transparent,
            clip,
        };
        let mut outline = |r: Rect, c: u8| {
            let r = pen.rect(r);
            for (x, y, w, h) in [
                (r.x, r.y, r.w, 1),
                (r.x, r.y + r.h - 1, r.w, 1),
                (r.x, r.y, 1, r.h),
                (r.x + r.w - 1, r.y, 1, r.h),
            ] {
                fb.fill_clipped(Rect { x, y, w, h }, c, clip);
            }
        };
        for (bx, by, z, b) in &self.borders {
            if *z != v.state.layer && self.cell_map.grid.0 > 0 {
                continue;
            }
            let r = Rect {
                x: bx.saturating_add(i32::from(b.x)),
                y: by.saturating_add(i32::from(b.y)),
                w: i32::from(b.width),
                h: i32::from(b.height),
            };
            outline(r, OVERLAY_BORDER);
        }
        // portas de célula: fora de `borders`, desenhadas no estado atual
        for r in self
            .cell_doors
            .iter()
            .filter(|d| d.layer == v.state.layer)
            .flat_map(|d| d.solids())
        {
            outline(*r, OVERLAY_BORDER);
        }
        for &r in &self.object_solids {
            outline(r, OVERLAY_OBJECT);
        }
        for z in self.prop_zones.iter().filter(|z| z.layer == v.state.layer) {
            outline(z.rect, OVERLAY_MARKER);
        }
        for m in &self.markers {
            let r = Rect {
                x: m.x.saturating_sub(4),
                y: m.y.saturating_sub(4),
                w: 9,
                h: 9,
            };
            outline(r, OVERLAY_MARKER);
        }
    }
}

fn clip_frame(fb: &Framebuffer) -> Rect {
    Rect {
        x: 0,
        y: 0,
        w: fb.width as i32,
        h: fb.height as i32,
    }
}

/// Raio laser (`0x00410dae`): traço da cauda à cabeça, até 25 px, nas cores da variante.
fn draw_laser(
    e: &Entity,
    l: &crate::behaviour::laser::LaserState,
    fb: &mut Framebuffer,
    pen: &Pen,
    map: Option<&[u8; 256]>,
) {
    use crate::behaviour::{laser, mine};
    let head = e.center();
    let (mut dx, mut dy) = (l.tail.0 - head.0, l.tail.1 - head.1);
    let d = mine::distance((0, 0), (dx, dy));
    if d > laser::TRAIL {
        dx = dx * laser::TRAIL / d;
        dy = dy * laser::TRAIL / d;
    }
    let colors = laser::COLORS[l.variant.min(3)];
    let c = |i: usize| map.map_or(colors[i], |m| m[usize::from(colors[i])]);
    let n = dx.abs().max(dy.abs()).max(1);
    for i in 0..=n {
        let (x, y) = pen
            .view
            .project((head.0 + dx * i / n, head.1 + dy * i / n), pen.d);
        // o miolo por cima das bordas (`fill_rect` de 3 px e depois 1 px)
        fb.plot(x, y, c(0), pen.blend, pen.clip);
        fb.plot(x - 1, y, c(2), pen.blend, pen.clip);
        fb.plot(x + 1, y, c(2), pen.blend, pen.clip);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AnimPlayer;
    use crate::scene::view::{SpriteCell, ViewState};
    use openhunted_formats::ResourceArchive;
    use openhunted_formats::test_support::{mkanm, mkarchive, mkbmp};

    /// Arte 20×20 de índice 1 com um miolo 10×10 de índice 0 (colunas e linhas 5…14).
    fn assets() -> Assets {
        let px: Vec<u8> = (0..400)
            .map(|i| {
                let (x, y) = (i % 20, i / 20);
                u8::from(!((5..15).contains(&x) && (5..15).contains(&y)))
            })
            .collect();
        let bmp = mkbmp(&[(20, 20, px)]);
        let anm = mkanm();
        let (map, vols) = mkarchive(&[(
            "RESOURCE.001",
            &[("OAN00.BMP", bmp.as_slice()), ("OAN.ANM", anm.as_slice())],
        )]);
        Assets::new(ResourceArchive::from_map(&map, |_| Ok(vols[0].1.clone())).unwrap())
    }

    /// Cena de uma célula e três camadas: cenário (o quadro com miolo 0) na camada 1, um
    /// decalque cheio só onde a camada 0 aparece e outro igual na camada 2 (à frente).
    /// O `mkanm` desenha a arte em (x − 2, y + 7).
    fn scene(a: &mut Assets, hole: bool) -> Scene {
        let anim = a.animation("OAN.ANM").unwrap();
        let mut s = Scene::empty(128, 128);
        let sprite = |layer, x, y| Sprite {
            layer,
            anim: anim.clone(),
            player: AnimPlayer::from_frame(&anim, 0),
            x,
            y,
            frozen: false,
            scale: (1, 1),
            mirror: false,
        };
        let cell = |z, front| {
            Some(SpriteCell {
                cell: (0, 0, z),
                front,
            })
        };
        // cenário da camada 1: arte em (40, 40)…(60, 60), miolo 0 em (45, 45)…(55, 55)
        s.sprites.push(sprite(Layer::Scenery, 42, 33));
        s.sprite_cells.push(cell(1, false));
        // camada 0: o mesmo quadro (a borda de índice 1 cai no miolo da frente, menor)
        s.sprites.push(sprite(Layer::Decoration, 42, 33));
        s.sprite_cells.push(cell(0, false));
        // camada 2, à frente da vista: nunca desenhada
        s.sprites.push(sprite(Layer::Decoration, 2, 3));
        s.sprite_cells.push(cell(2, true));
        s.sprite_light = vec![None; 3];
        s.cell_map.grid = (1, 1, 3);
        if hole {
            // buraco (46, 46, 4×4), dentro do miolo
            s.cell_map.holes.push((
                (0, 0, 1),
                Rect {
                    x: 46,
                    y: 46,
                    w: 4,
                    h: 4,
                },
            ));
        }
        s.layers = 3;
        s.views = vec![ViewState::new(1)];
        s
    }

    fn render(s: &Scene, a: &mut Assets) -> Framebuffer {
        let screen = Rect {
            x: 0,
            y: 0,
            w: 100,
            h: 100,
        };
        // vista 640×405 (distâncias 23 e 34) centrada no meio do miolo (50, 50)
        let mut v = View::new(screen, (640, 405), (50 - 320, 50 - 202), s.views[0]);
        v.anchor = (50, 50);
        let mut fb = Framebuffer::new(100, 100);
        fb.clear(77);
        s.draw_view(a, &mut fb, &v, false);
        fb
    }

    #[test]
    fn view_layer_is_opaque_and_back_layers_show_only_through_holes() {
        let mut a = assets();
        let shut = render(&scene(&mut a, false), &mut a);
        let px = |fb: &Framebuffer, x: usize, y: usize| fb.pixels[y * 100 + x];
        // a camada da vista em 1:1, opaca: a borda (1), o miolo (0) e as células em volta
        // não: a cena é de uma célula de 128 px; fora dela, fora da grade
        assert_eq!(px(&shut, 41, 41), 1);
        assert_eq!(px(&shut, 50, 50), 0);
        assert_eq!(px(&shut, 30, 30), view::EMPTY_COLOUR);
        // nada da camada 2 (à frente) nem da 0 (sem buraco): o miolo fica 0
        assert!((45..55).all(|y| (45..55).all(|x| px(&shut, x, y) == 0)));
        assert!((0..40).all(|y| (0..100).all(|x| px(&shut, x, y) == view::EMPTY_COLOUR)));
        // com o buraco (46…50, aumentado de 1 px: 45…51): a camada 0, menor (23/34), aparece
        // só dentro dele, e sem a tabela de sombra (o arquivo de teste não tem `DLS.TBL`)
        let open = render(&scene(&mut a, true), &mut a);
        let mut seen = 0;
        for y in 0..100 {
            for x in 0..100 {
                let (p, q) = (px(&shut, x, y), px(&open, x, y));
                if p != q {
                    assert!((45..51).contains(&x) && (45..51).contains(&y), "({x}, {y})");
                    assert_eq!((p, q), (0, 1));
                    seen += 1;
                }
            }
        }
        // a borda da camada 0 (largura 5 px no mundo dela → 3–4 na tela) cai no buraco
        assert!(seen > 0);
        // o centro do buraco é o miolo da camada 0 também: fica 0
        assert_eq!(px(&open, 50, 50), 0);
    }

    #[test]
    fn the_transition_enlarges_the_view_layer() {
        let mut a = assets();
        let mut s = scene(&mut a, false);
        assert_eq!(render(&s, &mut a).pixels[50 * 100 + 20], view::EMPTY_COLOUR);
        // descendo, contador 2: a camada da vista a `d` = 6 (escala 23/6), a arte de 20 px
        // vai de 12 a 88 na tela e o miolo de 31 a 69
        s.views[0].counter = 2;
        s.views[0].dir = 1;
        let fb = render(&s, &mut a);
        assert_eq!(fb.pixels[50 * 100 + 20], 1);
        assert_eq!(fb.pixels[50 * 100 + 35], 0);
        assert_eq!(fb.pixels[50 * 100 + 8], view::EMPTY_COLOUR);
    }

    /// Defesa: um desenho de sprite com mais de `MAX_DRAWN` px de lado no mundo (escala
    /// absurda) não sai; até o limite, sai normalmente.
    #[test]
    fn absurd_sprite_scales_are_not_drawn() {
        let mut a = assets();
        let mut s = scene(&mut a, false);
        // arte de 20 px a `k`: o `mkanm` a desloca (−2, +7); começa em (−10, −10) no mundo,
        // com borda de índice 1 de `5·k` px
        let place = |s: &mut Scene, k: i32| {
            let sp = &mut s.sprites[0];
            sp.scale = (k, 1);
            sp.x = 2 * k - 10;
            sp.y = -7 * k - 10;
        };
        place(&mut s, MAX_DRAWN / 20);
        assert_eq!(render(&s, &mut a).pixels[41 * 100 + 41], 1);
        place(&mut s, MAX_DRAWN / 20 + 1);
        assert_ne!(render(&s, &mut a).pixels[41 * 100 + 41], 1);
    }
}
