// SPDX-License-Identifier: GPL-3.0-or-later
//! Tela `.ITF`: controles desenhados pelos rótulos da animação, botões com os quadros
//! normal/em cima/afundando, áreas de texto e a entrada por mouse e teclado
//! (`docs/re/interface-itf.md`).

use std::collections::HashMap;
use std::rc::Rc;

use openhunted_formats::{Animation, Bitmap, Font, Interface, Result};

use crate::anim::draw_ops;
use crate::assets::Assets;
use crate::physics::Rect;
use crate::render::{Framebuffer, SCREEN_H, SCREEN_W, draw_text, measure_text};

/// Id de comando "cancelar" (Esc ou botão 2 do joystick).
pub const CANCEL: u16 = 2;

/// Sinalizadores de controle (`+0x34`).
pub mod flags {
    /// O tratamento roda a cada quadro.
    pub const UPDATE: u32 = 1;
    /// Oculto: não desenha nem recebe entrada.
    pub const HIDDEN: u32 = 8;
    /// Parada da navegação por teclado.
    pub const NAV: u32 = 0x80;
}

/// Tipo de controle "botão" (`0x461cb9`).
const BUTTON: u16 = 6;

/// Onde a tela fica: tela cheia (sinal 4: o canto do retângulo envolvente vai a (0, 0)) ou
/// diálogo centrado na tela 640×480 (`0x4634d5`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    Full,
    Centered,
}

/// Direção da navegação por teclado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    Left,
    Right,
    Up,
    Down,
}

/// Entrada de uma tela.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input {
    /// Um quadro do laço da tela: anima e trata os botões.
    Tick,
    /// Ponteiro em coordenadas da tela 640×480.
    PointerMove(i32, i32),
    PointerDown,
    PointerUp,
    /// Seta: leva o ponteiro ao controle com sinal 0x80 mais próximo na direção.
    Nav(Dir),
    /// Enter, espaço: clique simulado na posição do ponteiro.
    Activate,
    /// Esc: comando 2.
    Cancel,
}

/// Estado de um controle.
#[derive(Debug, Clone)]
struct Ctl {
    id: u16,
    /// Retângulo de toque, relativo à tela.
    rect: Rect,
    origin: (i32, i32),
    /// Primeiro quadro do rótulo e número de quadros (0 = área de texto).
    start: usize,
    count: usize,
    frame: usize,
    flags: u32,
    /// Avança o quadro em laço a cada atualização (tipo 0 animado).
    looping: bool,
    /// Trata como botão (tipo 6, ou setas instaladas pela tela).
    button: bool,
    /// Item de grupo de opção (`0x4431d6`): clica ao apertar; quadro 2 marcado, 1 realçado,
    /// 0 apagado.
    option: bool,
    marked: bool,
    /// Toca o rótulo uma vez e para no último quadro (`S_CAR`, `0x445ad3`).
    once: bool,
    /// Tabela de cor do `DLS.TBL` aplicada ao desenho (`0x43becd`; retrato da `SLEVL`).
    remap: Option<Rc<[u8; 256]>>,
}

/// Quadros lógicos da animação: com a tabela A, os rótulos contam nela e cada entrada aponta
/// o quadro físico (`docs/formats/anm.md`); sem ela, os quadros são os próprios.
fn logical_len(anim: &Animation) -> usize {
    if anim.table_a.is_empty() {
        anim.frames.len()
    } else {
        anim.table_a.len()
    }
}

/// Quadro físico de um quadro lógico (fora da tabela: nenhum).
pub(crate) fn physical(anim: &Animation, frame: usize) -> Option<usize> {
    if anim.table_a.is_empty() {
        Some(frame)
    } else {
        anim.table_a.get(frame).map(|&f| usize::from(f))
    }
}

/// Primeiro quadro lógico e número de quadros do rótulo: até o quadro que termina a
/// animação (opcode 1) ou o início do rótulo seguinte (como `AnimPlayer`, pela tabela A).
pub(crate) fn label_range(anim: &Animation, label: u16) -> Option<(usize, usize)> {
    let start = usize::from(anim.labels.iter().find(|l| l.id == label)?.frame);
    let next = anim
        .labels
        .iter()
        .map(|l| usize::from(l.frame))
        .filter(|&f| f > start)
        .min()
        .unwrap_or_else(|| logical_len(anim))
        .max(start + 1);
    for f in start..next {
        let ends = physical(anim, f)
            .and_then(|p| anim.frames.get(p))
            .and_then(|fr| {
                anim.script
                    .iter()
                    .position(|i| i.offset == fr.script_offset)
            })
            .is_some_and(|i0| {
                anim.script[i0..]
                    .iter()
                    .find(|i| i.op == 0 || i.op == 1)
                    .is_some_and(|i| i.op == 1)
            });
        if ends {
            return Some((start, f + 1 - start));
        }
    }
    Some((start, next - start))
}

/// Texto escrito numa área (centrado, como `0x44214c`).
#[derive(Debug, Clone)]
struct Text {
    id: u16,
    text: String,
    font: Rc<Font>,
    color: u8,
}

/// Uma tela `.ITF` montada.
#[derive(Debug, Clone)]
pub struct Screen {
    anim: Rc<Animation>,
    bitmaps: HashMap<String, Rc<Bitmap>>,
    /// Deslocamento da tela (diálogo centrado).
    offset: (i32, i32),
    /// Envolvente dos controles (relativa à tela).
    size: (i32, i32),
    ctls: Vec<Ctl>,
    pointer: (i32, i32),
    down: bool,
    /// O botão acabou de ser apertado (bit 2 de `0x499f3c`).
    pressed: bool,
    /// Clique simulado: solta o botão depois do próximo quadro.
    release_after_tick: bool,
    captured: Option<u16>,
    texts: Vec<Text>,
}

impl Screen {
    /// Monta a tela: resolve os rótulos (campo F ≠ 0) e carrega os bitmaps da animação.
    pub fn new(itf: &Interface, assets: &mut Assets, placement: Placement) -> Result<Self> {
        let anim = assets.animation(&itf.animation)?;
        let mut bitmaps = HashMap::new();
        for (_, name) in &anim.bitmaps {
            if let Ok(b) = assets.bitmap(name) {
                bitmaps.insert(name.to_uppercase(), b);
            }
        }
        Ok(Self::with_bitmaps(itf, anim, bitmaps, placement))
    }

    /// Como `new`, com a animação e os bitmaps já carregados (testes).
    pub fn with_bitmaps(
        itf: &Interface,
        anim: Rc<Animation>,
        bitmaps: HashMap<String, Rc<Bitmap>>,
        placement: Placement,
    ) -> Self {
        let mut ctls: Vec<Ctl> = itf
            .controls
            .iter()
            .map(|c| {
                let (start, count) = match (c.field_f, c.label) {
                    (0, _) | (_, None) => (0, 0),
                    (_, Some(l)) => label_range(&anim, l).unwrap_or((0, 0)),
                };
                Ctl {
                    id: c.id,
                    rect: Rect {
                        x: i32::from(c.x),
                        y: i32::from(c.y),
                        w: i32::from(c.width),
                        h: i32::from(c.height),
                    },
                    origin: (i32::from(c.origin_x), i32::from(c.origin_y)),
                    start,
                    count,
                    frame: 0,
                    // tipos 2 e 6 ligam o sinal 1 na inicialização (`0x461ca0`)
                    flags: if c.kind == 2 || c.kind == BUTTON {
                        flags::UPDATE
                    } else {
                        0
                    },
                    looping: c.kind == 0 && count > 1,
                    button: c.kind == BUTTON,
                    option: false,
                    marked: false,
                    once: false,
                    remap: None,
                }
            })
            .collect();
        // a lista do original é ordenada por id: o fundo (menor id) é desenhado primeiro
        ctls.sort_by_key(|c| c.id);
        let x0 = ctls.iter().map(|c| c.rect.x).min().unwrap_or(0);
        let y0 = ctls.iter().map(|c| c.rect.y).min().unwrap_or(0);
        let x1 = ctls.iter().map(|c| c.rect.x + c.rect.w).max().unwrap_or(0);
        let y1 = ctls.iter().map(|c| c.rect.y + c.rect.h).max().unwrap_or(0);
        let size = (x1 - x0, y1 - y0);
        let offset = match placement {
            Placement::Full => (-x0, -y0),
            Placement::Centered => (
                (SCREEN_W as i32 - size.0) / 2 - x0,
                (SCREEN_H as i32 - size.1) / 2 - y0,
            ),
        };
        Self {
            anim,
            bitmaps,
            offset,
            size,
            ctls,
            pointer: (SCREEN_W as i32 / 2, SCREEN_H as i32 / 2),
            down: false,
            pressed: false,
            release_after_tick: false,
            captured: None,
            texts: Vec::new(),
        }
    }

    fn ctl(&self, id: u16) -> Option<&Ctl> {
        self.ctls.iter().find(|c| c.id == id)
    }

    fn ctl_mut(&mut self, id: u16) -> Option<&mut Ctl> {
        self.ctls.iter_mut().find(|c| c.id == id)
    }

    /// Ids dos controles, na ordem de desenho.
    pub fn ids(&self) -> Vec<u16> {
        self.ctls.iter().map(|c| c.id).collect()
    }

    /// Canto da tela na tela 640×480 e tamanho do retângulo envolvente.
    pub fn bounds(&self) -> Rect {
        let x0 = self.ctls.iter().map(|c| c.rect.x).min().unwrap_or(0);
        let y0 = self.ctls.iter().map(|c| c.rect.y).min().unwrap_or(0);
        Rect {
            x: x0 + self.offset.0,
            y: y0 + self.offset.1,
            w: self.size.0,
            h: self.size.1,
        }
    }

    /// Deslocamento da tela (coordenadas dos controles → tela 640×480).
    pub fn origin(&self) -> (i32, i32) {
        self.offset
    }

    /// Retângulo do controle na tela 640×480.
    pub fn rect(&self, id: u16) -> Option<Rect> {
        self.ctl(id).map(|c| Rect {
            x: c.rect.x + self.offset.0,
            y: c.rect.y + self.offset.1,
            ..c.rect
        })
    }

    /// Quadro corrente do controle (0…n−1) e número de quadros do rótulo.
    pub fn frame(&self, id: u16) -> Option<(usize, usize)> {
        self.ctl(id).map(|c| (c.frame, c.count))
    }

    pub fn flags(&self, id: u16) -> u32 {
        self.ctl(id).map_or(0, |c| c.flags)
    }

    /// Liga sinalizadores (`0x44a9db`).
    pub fn set_flags(&mut self, id: u16, f: u32) {
        if let Some(c) = self.ctl_mut(id) {
            c.flags |= f;
        }
    }

    pub fn clear_flags(&mut self, id: u16, f: u32) {
        if let Some(c) = self.ctl_mut(id) {
            c.flags &= !f;
        }
    }

    /// Remove o controle (`0x44a89e(id, 0)`).
    pub fn remove(&mut self, id: u16) {
        self.ctls.retain(|c| c.id != id);
        if self.captured == Some(id) {
            self.captured = None;
        }
    }

    /// Para o avanço de quadro do controle (rotina própria que o substitui).
    pub fn freeze(&mut self, id: u16) {
        if let Some(c) = self.ctl_mut(id) {
            c.looping = false;
        }
    }

    /// Trata um controle de tipo 0 como botão (setas do briefing, `0x43e011`).
    pub fn make_button(&mut self, id: u16) {
        if let Some(c) = self.ctl_mut(id) {
            c.button = true;
            c.looping = false;
            c.frame = 0;
            c.flags |= flags::UPDATE;
        }
    }

    /// Torna o controle um item de grupo de opção (`0x443257` liga um e apaga os outros;
    /// quem marca é a tela, com `set_marked`).
    pub fn make_option(&mut self, id: u16) {
        if let Some(c) = self.ctl_mut(id) {
            c.option = true;
            c.button = false;
            c.looping = false;
            c.frame = 0;
            c.flags |= flags::UPDATE | flags::NAV;
        }
    }

    pub fn set_marked(&mut self, id: u16, marked: bool) {
        if let Some(c) = self.ctl_mut(id) {
            c.marked = marked;
            if c.option {
                c.frame = if marked {
                    2.min(c.count.saturating_sub(1))
                } else {
                    0
                };
            }
        }
    }

    /// Troca o rótulo da animação do controle (volta ao primeiro quadro).
    pub fn set_label(&mut self, id: u16, label: u16) {
        let range = label_range(&self.anim, label);
        if let Some(c) = self.ctl_mut(id) {
            let (start, count) = range.unwrap_or((0, 0));
            c.start = start;
            c.count = count;
            c.frame = 0;
            c.looping = !c.button && !c.option && !c.once && count > 1;
        }
    }

    /// Toca o rótulo uma vez, parando no último quadro.
    pub fn play_once(&mut self, id: u16) {
        if let Some(c) = self.ctl_mut(id) {
            c.once = true;
            c.looping = false;
            c.frame = 0;
        }
    }

    /// Desenha o controle recolorido pela tabela (cor `i` → `tabela[i]`).
    pub fn set_remap(&mut self, id: u16, table: Option<Rc<[u8; 256]>>) {
        if let Some(c) = self.ctl_mut(id) {
            c.remap = table;
        }
    }

    /// O ponteiro está sobre o controle visível.
    pub fn hit(&self, id: u16) -> bool {
        self.ctl(id)
            .is_some_and(|c| c.flags & flags::HIDDEN == 0 && self.inside(c))
    }

    pub fn pointer(&self) -> (i32, i32) {
        self.pointer
    }

    /// Leva o ponteiro ao centro do controle (`0x44b42a`).
    pub fn point_at(&mut self, id: u16) {
        if let Some(r) = self.rect(id) {
            self.pointer = (r.x + r.w / 2, r.y + r.h / 2);
        }
    }

    /// Escreve `text` centrado na área `id`, recortado no retângulo.
    pub fn set_text(&mut self, id: u16, text: &str, font: Rc<Font>, color: u8) {
        self.texts.retain(|t| t.id != id);
        if !text.is_empty() {
            self.texts.push(Text {
                id,
                text: text.to_string(),
                font,
                color,
            });
        }
    }

    pub fn text(&self, id: u16) -> Option<&str> {
        self.texts
            .iter()
            .find(|t| t.id == id)
            .map(|t| t.text.as_str())
    }

    fn inside(&self, c: &Ctl) -> bool {
        let (px, py) = self.pointer;
        let (x, y) = (c.rect.x + self.offset.0, c.rect.y + self.offset.1);
        px >= x && py >= y && px < x + c.rect.w && py < y + c.rect.h
    }

    /// Trata uma entrada; devolve o id do controle ativado (ou `CANCEL`).
    pub fn handle(&mut self, input: Input) -> Option<u16> {
        match input {
            Input::Tick => return self.tick(),
            Input::PointerMove(x, y) => self.pointer = (x, y),
            Input::PointerDown => {
                self.down = true;
                self.pressed = true;
            }
            Input::PointerUp => self.down = false,
            Input::Nav(d) => self.navigate(d),
            Input::Activate => {
                self.down = true;
                self.pressed = true;
                self.release_after_tick = true;
            }
            Input::Cancel => return Some(CANCEL),
        }
        None
    }

    /// Um quadro: animações em laço e a máquina dos botões (`0x461cb9`).
    fn tick(&mut self) -> Option<u16> {
        let mut activated = None;
        for i in 0..self.ctls.len() {
            let c = &self.ctls[i];
            if c.flags & flags::HIDDEN != 0 {
                continue;
            }
            if c.looping {
                let c = &mut self.ctls[i];
                c.frame = (c.frame + 1) % c.count;
                continue;
            }
            if c.once {
                let c = &mut self.ctls[i];
                c.frame = (c.frame + 1).min(c.count.saturating_sub(1));
                continue;
            }
            if c.option {
                let inside = self.inside(c);
                let id = c.id;
                let pressed = self.pressed;
                let c = &mut self.ctls[i];
                let last = c.count.saturating_sub(1);
                c.frame = if c.marked {
                    2.min(last)
                } else if inside {
                    1.min(last)
                } else {
                    0
                };
                if inside && pressed && self.captured.is_none() {
                    activated = activated.or(Some(id));
                }
                continue;
            }
            if !c.button {
                continue;
            }
            let inside = self.inside(c);
            let id = c.id;
            let last = c.count.saturating_sub(1);
            let captured = self.captured == Some(id);
            let (down, pressed) = (self.down, self.pressed);
            let c = &mut self.ctls[i];
            if !inside {
                c.frame = 0;
                if captured && !down {
                    self.captured = None;
                }
            } else if !captured {
                if pressed && self.captured.is_none() {
                    self.captured = Some(id);
                } else if !down {
                    c.frame = 1.min(last);
                }
            } else if c.frame < last {
                c.frame += 1;
            } else if !down {
                c.frame = 0;
                self.captured = None;
                activated = activated.or(Some(id));
            }
        }
        self.pressed = false;
        if self.release_after_tick {
            self.release_after_tick = false;
            self.down = false;
        }
        activated
    }

    /// Procura o controle com sinal 0x80 mais próximo na direção (`0x44b1c0`) e leva o
    /// ponteiro a ele. Peso: distância no eixo da direção + 2 × distância no outro eixo.
    fn navigate(&mut self, d: Dir) {
        let (px, py) = self.pointer;
        let best = self
            .ctls
            .iter()
            .filter(|c| c.flags & flags::NAV != 0 && c.flags & flags::HIDDEN == 0)
            .filter_map(|c| {
                let cx = c.rect.x + self.offset.0 + c.rect.w / 2;
                let cy = c.rect.y + self.offset.1 + c.rect.h / 2;
                let (dx, dy) = (cx - px, cy - py);
                let (along, across) = match d {
                    Dir::Left => (-dx, dy),
                    Dir::Right => (dx, dy),
                    Dir::Up => (-dy, dx),
                    Dir::Down => (dy, dx),
                };
                (along > 0).then_some((along + 2 * across.abs(), c.id))
            })
            .min();
        if let Some((_, id)) = best {
            self.point_at(id);
        }
    }

    /// Desenha o quadro `frame` da animação da tela deslocado por `(dx, dy)` (relativo à
    /// tela): uso das telas para peças com posição própria (alça da barra de rolagem).
    pub fn draw_anim_frame(&self, fb: &mut Framebuffer, frame: usize, dx: i32, dy: i32) {
        self.draw_frame_mapped(fb, frame, dx, dy, None);
    }

    /// Como `draw_anim_frame`, com cada pixel opaco passado pela tabela `map` (sem
    /// framebuffer intermediário).
    fn draw_frame_mapped(
        &self,
        fb: &mut Framebuffer,
        frame: usize,
        dx: i32,
        dy: i32,
        map: Option<&[u8; 256]>,
    ) {
        let Some(frame) = physical(&self.anim, frame) else {
            return;
        };
        for op in draw_ops(&self.anim, frame) {
            let Some(bmp) = self.bitmaps.get(&op.bitmap.to_uppercase()) else {
                continue;
            };
            let Some(f) = bmp.frames.get(op.frame as usize) else {
                continue;
            };
            let (x, y) = (self.offset.0 + dx + op.x, self.offset.1 + dy + op.y);
            match map {
                None => fb.blit(f, x, y, true),
                Some(t) => blit_mapped(fb, f, x, y, t),
            }
        }
    }

    /// Primeiro quadro do rótulo `label` da animação da tela.
    pub fn label_frame(&self, label: u16) -> Option<usize> {
        label_range(&self.anim, label).map(|(s, _)| s)
    }

    /// Tamanho (largura, altura) do primeiro quadro do rótulo, pelo quadro da animação.
    pub fn label_size(&self, label: u16) -> Option<(i32, i32)> {
        let f = physical(&self.anim, self.label_frame(label)?)?;
        let fr = self.anim.frames.get(f)?;
        Some((i32::from(fr.width), i32::from(fr.height)))
    }

    /// Desenha os controles visíveis em ordem de id (`0x461b8a`: quadro na posição
    /// `(x + origem_x, y + origem_y)` mais o deslocamento do quadro) e depois os textos.
    pub fn draw(&self, fb: &mut Framebuffer) {
        for c in &self.ctls {
            if c.flags & flags::HIDDEN != 0 || c.count == 0 {
                continue;
            }
            let (f, x, y) = (
                c.start + c.frame,
                c.rect.x + c.origin.0,
                c.rect.y + c.origin.1,
            );
            self.draw_frame_mapped(fb, f, x, y, c.remap.as_deref());
        }
        for t in &self.texts {
            if let Some(r) = self.rect(t.id) {
                if self.ctl(t.id).is_some_and(|c| c.flags & flags::HIDDEN == 0) {
                    draw_text_in(fb, &t.font, r, &t.text, t.color, true);
                }
            }
        }
    }
}

/// `Framebuffer::blit` transparente com a tabela de cor `t` nos pixels opacos.
fn blit_mapped(fb: &mut Framebuffer, f: &openhunted_formats::Frame, x: i32, y: i32, t: &[u8; 256]) {
    let (fw, fh) = (i64::from(f.width), i64::from(f.height));
    let (x, y) = (i64::from(x), i64::from(y));
    let (w, h) = (fb.width as i64, fb.height as i64);
    let (x0, y0) = (x.max(0), y.max(0));
    let (x1, y1) = ((x + fw).min(w), (y + fh).min(h));
    if x0 >= x1 || y0 >= y1 || f.pixels.len() < (fw * fh) as usize {
        return;
    }
    let n = (x1 - x0) as usize;
    for dy in y0..y1 {
        let src = ((dy - y) * fw + (x0 - x)) as usize;
        let dst = (dy * w + x0) as usize;
        for (o, &p) in fb.pixels[dst..dst + n]
            .iter_mut()
            .zip(&f.pixels[src..src + n])
        {
            if p != 0 {
                *o = t[usize::from(p)];
            }
        }
    }
}

/// Escreve uma linha de texto no retângulo `r`, centrada na vertical (e na horizontal, com
/// `center`), recortada no retângulo.
pub fn draw_text_in(
    fb: &mut Framebuffer,
    font: &Font,
    r: Rect,
    text: &str,
    color: u8,
    center: bool,
) {
    if r.w <= 0 || r.h <= 0 {
        return;
    }
    let (tw, th) = measure_text(font, text);
    let mut tmp = Framebuffer::new(r.w as usize, r.h as usize);
    let x = if center { (r.w - tw) / 2 } else { 0 };
    draw_text(&mut tmp, font, x, (r.h - th) / 2, text, Some(color));
    let frame = openhunted_formats::Frame {
        width: r.w as u16,
        height: r.h as u16,
        pixels: tmp.pixels,
    };
    fb.blit(&frame, r.x, r.y, true);
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use openhunted_formats::test_support::mkitf_controls;
    use openhunted_formats::{AnimFrame, AnimLabel, Frame, Instruction};

    /// Animação sintética: quadro `i` desenha o quadro `i` de `T.BMP` (4×4 pintado com a
    /// cor `i + 1`) no deslocamento do quadro. Rótulos: 1 = fundo (quadro 0), 2 = botão A
    /// (1–3), 3 = laço (4–5, fim no 5), 4 = botão B (6–8), 5 = seta (9–10).
    pub(crate) fn synth_anim() -> (Rc<Animation>, HashMap<String, Rc<Bitmap>>) {
        let pos = [
            (0, 0),
            (10, 10),
            (10, 10),
            (10, 10),
            (40, 0),
            (40, 0),
            (10, 30),
            (10, 30),
            (10, 30),
            (50, 50),
            (50, 50),
        ];
        let mut script = Vec::new();
        let mut frames = Vec::new();
        for (i, &(x, y)) in pos.iter().enumerate() {
            let off = (script.len() * 6) as u16;
            frames.push(AnimFrame {
                x,
                y,
                width: 4,
                height: 4,
                script_offset: off,
                extra: 0,
            });
            script.push(Instruction {
                offset: off,
                op: 2,
                args: vec![1, i as i16, 0, 0, 0],
            });
            script.push(Instruction {
                offset: off + 6,
                op: if i == 5 { 1 } else { 0 },
                args: vec![],
            });
        }
        // offsets: cada par (op 2 de 6 palavras + op 0/1 de 1) ocupa 7 palavras
        for (k, ins) in script.iter_mut().enumerate() {
            ins.offset = (k / 2 * 7 + if k % 2 == 1 { 6 } else { 0 }) as u16;
        }
        for (i, f) in frames.iter_mut().enumerate() {
            f.script_offset = (i * 7) as u16;
        }
        let labels = [(0, 1), (1, 2), (4, 3), (6, 4), (9, 5)]
            .iter()
            .map(|&(frame, id)| AnimLabel {
                frame,
                id,
                name: None,
            })
            .collect();
        let anim = Animation {
            version: 1004,
            width: 64,
            height: 64,
            extra: [0; 3],
            table_a: vec![],
            frames,
            labels,
            script,
            bitmaps: vec![(1, "T.BMP".into())],
            palette: None,
        };
        let bmp = Bitmap {
            frames: (0..pos.len())
                .map(|i| Frame {
                    width: 4,
                    height: 4,
                    pixels: vec![i as u8 + 1; 16],
                })
                .collect(),
        };
        let mut m = HashMap::new();
        m.insert("T.BMP".into(), Rc::new(bmp));
        (Rc::new(anim), m)
    }

    /// Tela 64×64: fundo 100, botões 101 (10,10) e 102 (10,30), laço 103, área de texto
    /// 104 (F = 0) e seta 105 (tipo 0).
    pub(crate) fn synth_screen(placement: Placement) -> Screen {
        let itf = Interface::parse(&mkitf_controls(
            "t.anm",
            &[
                (6, 102, 10, 30, 4, 4, 1, 4),
                (0, 100, 0, 0, 64, 64, 1, 1),
                (6, 101, 10, 10, 4, 4, 1, 2),
                (0, 103, 40, 0, 4, 4, 1, 3),
                (0, 104, 20, 40, 30, 10, 0, 1),
                (0, 105, 50, 50, 4, 4, 1, 5),
            ],
        ))
        .unwrap();
        let (anim, bmps) = synth_anim();
        Screen::with_bitmaps(&itf, anim, bmps, placement)
    }

    fn px(fb: &Framebuffer, x: i32, y: i32) -> u8 {
        fb.pixels[y as usize * fb.width + x as usize]
    }

    #[test]
    fn controls_are_sorted_and_labels_resolved() {
        let s = synth_screen(Placement::Full);
        assert_eq!(s.ids(), [100, 101, 102, 103, 104, 105]);
        assert_eq!(s.frame(101), Some((0, 3)));
        assert_eq!(s.frame(103), Some((0, 2)), "laço termina no opcode 1");
        assert_eq!(s.frame(104), Some((0, 0)), "campo F = 0: área de texto");
        assert_eq!(s.flags(101), flags::UPDATE, "tipo 6 liga o sinal 1");
        assert_eq!(s.flags(103), 0);
    }

    #[test]
    fn centered_dialog_is_offset() {
        let s = synth_screen(Placement::Centered);
        let b = s.bounds();
        assert_eq!((b.x, b.y, b.w, b.h), (288, 208, 64, 64));
        let r = s.rect(101).unwrap();
        assert_eq!((r.x, r.y), (298, 218));
        let mut fb = Framebuffer::new(640, 480);
        s.draw(&mut fb);
        assert_eq!(px(&fb, 288, 208), 1, "fundo");
        assert_eq!(px(&fb, 298, 218), 2, "botão no quadro normal");
    }

    #[test]
    fn button_hover_press_and_release_activates() {
        let mut s = synth_screen(Placement::Full);
        assert_eq!(s.handle(Input::PointerMove(11, 11)), None);
        s.handle(Input::Tick);
        assert_eq!(s.frame(101), Some((1, 3)), "mouse em cima");
        s.handle(Input::PointerDown);
        assert_eq!(s.handle(Input::Tick), None, "captura");
        assert_eq!(s.handle(Input::Tick), None);
        assert_eq!(s.frame(101), Some((2, 3)), "afundou até o último");
        assert_eq!(s.handle(Input::Tick), None, "segurando não ativa");
        s.handle(Input::PointerUp);
        assert_eq!(s.handle(Input::Tick), Some(101));
        assert_eq!(s.frame(101), Some((0, 3)));
    }

    #[test]
    fn releasing_outside_cancels_the_click() {
        let mut s = synth_screen(Placement::Full);
        s.handle(Input::PointerMove(11, 11));
        s.handle(Input::PointerDown);
        s.handle(Input::Tick);
        s.handle(Input::PointerMove(60, 5));
        s.handle(Input::PointerUp);
        for _ in 0..5 {
            assert_eq!(s.handle(Input::Tick), None);
        }
        assert_eq!(s.frame(101), Some((0, 3)));
    }

    #[test]
    fn keyboard_moves_pointer_between_nav_stops_and_enter_clicks() {
        let mut s = synth_screen(Placement::Full);
        s.set_flags(101, flags::NAV);
        s.set_flags(102, flags::NAV);
        s.point_at(101);
        s.handle(Input::Nav(Dir::Down));
        assert_eq!(s.pointer(), (12, 32));
        s.handle(Input::Nav(Dir::Down));
        assert_eq!(s.pointer(), (12, 32), "nada abaixo: fica");
        s.handle(Input::Nav(Dir::Up));
        assert_eq!(s.pointer(), (12, 12));
        s.handle(Input::Activate);
        let got: Vec<_> = (0..5).filter_map(|_| s.handle(Input::Tick)).collect();
        assert_eq!(got, [101]);
        assert_eq!(s.handle(Input::Cancel), Some(CANCEL));
    }

    #[test]
    fn loops_animate_and_hidden_or_removed_controls_vanish() {
        let mut s = synth_screen(Placement::Full);
        s.handle(Input::Tick);
        assert_eq!(s.frame(103), Some((1, 2)));
        s.handle(Input::Tick);
        assert_eq!(s.frame(103), Some((0, 2)));
        s.freeze(103);
        s.handle(Input::Tick);
        assert_eq!(s.frame(103), Some((0, 2)));
        s.set_flags(101, flags::HIDDEN);
        s.remove(102);
        let mut fb = Framebuffer::new(64, 64);
        s.draw(&mut fb);
        assert_eq!(px(&fb, 10, 10), 0, "oculto");
        assert_eq!(px(&fb, 10, 30), 0, "removido");
        assert_eq!(px(&fb, 40, 0), 5, "laço no quadro 4");
        assert!(s.rect(102).is_none());
    }

    #[test]
    fn options_click_on_press_and_show_marked_or_hover_frames() {
        let mut s = synth_screen(Placement::Full);
        s.make_option(101);
        s.make_option(102);
        s.set_marked(102, true);
        assert_eq!(s.frame(102), Some((2, 3)));
        s.handle(Input::PointerMove(11, 11));
        assert_eq!(s.handle(Input::Tick), None);
        assert_eq!(s.frame(101), Some((1, 3)), "realce");
        s.handle(Input::PointerDown);
        assert_eq!(s.handle(Input::Tick), Some(101), "clica ao apertar");
        s.handle(Input::PointerUp);
        assert_eq!(s.handle(Input::Tick), None);
        assert!(s.hit(101) && !s.hit(102));
        s.set_marked(101, true);
        s.set_marked(102, false);
        s.handle(Input::Tick);
        assert_eq!((s.frame(101), s.frame(102)), (Some((2, 3)), Some((0, 3))));
    }

    #[test]
    fn play_once_stops_on_the_last_frame_and_labels_change() {
        let mut s = synth_screen(Placement::Full);
        s.set_label(103, 4);
        assert_eq!(s.frame(103), Some((0, 3)));
        s.play_once(103);
        for _ in 0..5 {
            s.handle(Input::Tick);
        }
        assert_eq!(s.frame(103), Some((2, 3)), "parado no último");
        assert_eq!(s.label_frame(4), Some(6));
        assert_eq!(s.label_size(4), Some((4, 4)));
    }

    #[test]
    fn table_a_maps_logical_frames_to_physical() {
        let (anim, bmps) = synth_anim();
        let mut a = (*anim).clone();
        // rótulo 2 passa a começar no quadro lógico 20, que aponta o físico 1
        a.table_a = (0..30)
            .map(|i| if i >= 20 { (i - 19) as u16 } else { 0 })
            .collect();
        a.labels.iter_mut().find(|l| l.id == 2).unwrap().frame = 20;
        let itf = Interface::parse(&mkitf_controls(
            "t.anm",
            &[(0, 100, 0, 0, 64, 64, 1, 1), (6, 101, 10, 10, 4, 4, 1, 2)],
        ))
        .unwrap();
        let s = Screen::with_bitmaps(&itf, Rc::new(a), bmps, Placement::Full);
        assert_eq!(s.frame(101), Some((0, 5)), "até o opcode 1 do físico 5");
        let mut fb = Framebuffer::new(64, 64);
        s.draw(&mut fb);
        assert_eq!(px(&fb, 10, 10), 2, "lógico 20 → físico 1");
    }

    #[test]
    fn remap_recolors_only_the_control() {
        let mut s = synth_screen(Placement::Full);
        let mut t = [0u8; 256];
        for (i, v) in t.iter_mut().enumerate() {
            *v = i as u8;
        }
        t[2] = 99;
        s.set_remap(101, Some(Rc::new(t)));
        let mut fb = Framebuffer::new(64, 64);
        s.draw(&mut fb);
        assert_eq!(px(&fb, 10, 10), 99, "botão recolorido");
        assert_eq!(px(&fb, 0, 0), 1, "fundo intacto");
    }

    #[test]
    fn type0_made_button_clicks() {
        let mut s = synth_screen(Placement::Full);
        s.make_button(105);
        s.handle(Input::PointerMove(51, 51));
        s.handle(Input::Activate);
        let got: Vec<_> = (0..4).filter_map(|_| s.handle(Input::Tick)).collect();
        assert_eq!(got, [105]);
    }

    #[test]
    fn text_is_centered_and_clipped_in_its_area() {
        use openhunted_formats::test_support::mkpft;
        let font = Rc::new(
            Font::parse(&mkpft('a' as u32, &[(2, vec![255; 4]), (2, vec![255; 4])])).unwrap(),
        );
        let mut s = synth_screen(Placement::Full);
        s.set_text(104, "ab", font.clone(), 77);
        assert_eq!(s.text(104), Some("ab"));
        let mut fb = Framebuffer::new(64, 64);
        s.draw(&mut fb);
        // "ab" = 5 px, área 30×10 em (20, 40): x = 20 + 12, y = 40 + 4
        assert_eq!(px(&fb, 32, 44), 77);
        assert_eq!(px(&fb, 31, 44), 0);
        let long = "ab".repeat(40);
        s.set_text(104, &long, font, 77);
        let mut fb = Framebuffer::new(64, 64);
        s.draw(&mut fb);
        assert_eq!(px(&fb, 19, 44), 0, "recortado à esquerda");
        assert_eq!(px(&fb, 50, 44), 0, "recortado à direita");
    }
}
