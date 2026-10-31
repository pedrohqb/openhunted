// SPDX-License-Identifier: GPL-3.0-or-later
//! Cena de uma fase: blocos de cenário, decorações e objetos, com as animações em curso.

use std::rc::Rc;

use std::collections::HashMap;

use openhunted_formats::{Animation, Border, BorderBox, Level, LevelObject, Result};

use crate::physics::{Rect, World};
use crate::{AnimPlayer, Assets, Framebuffer, Jake, draw_ops};

/// Duração de um tick de animação, em milissegundos (provisório: `docs/re/cenario.md`).
pub const TICK_MS: u32 = 50;
/// Cores do overlay (índices da paleta).
const OVERLAY_BORDER: u8 = 255;
const OVERLAY_MARKER: u8 = 254;
/// Lado de uma célula da grade, em pixels.
pub const CELL: i32 = 128;

/// Camada de desenho, da mais funda para a mais próxima.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Layer {
    Scenery,
    Decoration,
    Object,
}

/// Animação posicionada na fase.
#[derive(Debug, Clone)]
pub struct Sprite {
    pub layer: Layer,
    pub anim: Rc<Animation>,
    pub player: AnimPlayer,
    /// Origem na fase, em pixels.
    pub x: i32,
    pub y: i32,
    /// Não avança com `tick` (ver `GENERIC_ITEM`).
    pub frozen: bool,
    /// Escala de desenho (numerador, denominador); ver `object_scale`.
    pub scale: (i32, i32),
    /// Espelhado na horizontal (a arte dos personagens olha para a esquerda).
    pub mirror: bool,
}

/// Tipo de objeto cuja animação (`OGF.ANM`) é uma folha de itens num rótulo só: cada objeto
/// mostra um quadro fixo, escolhido por um campo ainda não decifrado. Provisório (M2): parado
/// no quadro `values[0]` quando válido, senão no 0. Ver `docs/re/objetos.md`.
const GENERIC_ITEM: u32 = 6;

/// Escala de desenho de um objeto (`0x00434655`): os dois últimos campos do objeto (A, B)
/// são o tamanho-alvo; se A ≥ B, escala = A / largura do quadro de referência, senão
/// B / altura. Valores ausentes ou inválidos dão escala 1.
pub fn object_scale(a: i32, b: i32, ref_w: i32, ref_h: i32) -> (i32, i32) {
    if a > 0 && ref_w > 0 && a >= b {
        (a, ref_w)
    } else if b > 0 && ref_h > 0 && b > a {
        (b, ref_h)
    } else {
        (1, 1)
    }
}

/// Objeto sem animação (lógica) ou com animação ausente: mostrado só no overlay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Marker {
    pub kind: u32,
    pub x: i32,
    pub y: i32,
}

/// Cena de uma fase.
#[derive(Debug, Clone)]
pub struct Scene {
    pub width: i32,
    pub height: i32,
    /// Em ordem de desenho (camada, depois ordem do arquivo).
    pub sprites: Vec<Sprite>,
    pub markers: Vec<Marker>,
    /// Caixas de colisão das células: origem da célula, camada (z) e caixa.
    pub borders: Vec<(i32, i32, i32, BorderBox)>,
    /// Problemas encontrados ao montar a cena (recursos ausentes), um por linha.
    pub warnings: Vec<String>,
    /// O Jake controlável (objeto de tipo 1), se a fase tiver um.
    pub jake: Option<Jake>,
    /// Sprite do Jake (desenhado depois dos objetos).
    pub jake_sprite: Option<Sprite>,
    /// Geometria sólida da camada do Jake.
    pub world: World,
}

/// Tipo de objeto do Jake.
const JAKE: u32 = 1;
/// Caixa de colisão de reserva do Jake (envolvente do `JK_DEFLT.BRD`).
const JAKE_BOX: Rect = Rect {
    x: 2,
    y: 3,
    w: 18,
    h: 44,
};
/// Tipo de forma "caixa" nos BRD.
const BOX: u16 = 2;

/// Caixa envolvente das formas de tipo caixa de um BRD.
fn border_bounds(b: &Border) -> Option<Rect> {
    let boxes: Vec<_> = b.boxes.iter().filter(|x| x.kind == BOX).collect();
    let x0 = boxes.iter().map(|x| i32::from(x.x)).min()?;
    let y0 = boxes.iter().map(|x| i32::from(x.y)).min()?;
    let x1 = boxes
        .iter()
        .map(|x| i32::from(x.x) + i32::from(x.width))
        .max()?;
    let y1 = boxes
        .iter()
        .map(|x| i32::from(x.y) + i32::from(x.height))
        .max()?;
    Some(Rect {
        x: x0,
        y: y0,
        w: x1 - x0,
        h: y1 - y0,
    })
}

/// Número de quadros de cada rótulo (até o rótulo seguinte).
fn label_lengths(a: &Animation) -> HashMap<u16, usize> {
    let mut starts: Vec<usize> = a.labels.iter().map(|l| l.frame as usize).collect();
    starts.sort_unstable();
    a.labels
        .iter()
        .map(|l| {
            let f = l.frame as usize;
            let next = starts
                .iter()
                .copied()
                .find(|&s| s > f)
                .unwrap_or(a.frames.len());
            (l.id, next.saturating_sub(f).max(1))
        })
        .collect()
}

/// Desenha um sprite com a câmera.
fn draw_sprite(sp: &Sprite, assets: &mut Assets, fb: &mut Framebuffer, camera: (i32, i32)) {
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
        let x = sp.x.saturating_add(ox).saturating_sub(camera.0);
        let y = sp.y.saturating_add(sc(op.y)).saturating_sub(camera.1);
        // Índice 0 transparente em todas as camadas (ver docs/re/cenario.md).
        fb.blit_scaled(frame, x, y, sp.scale, sp.mirror, true);
    }
}

impl Scene {
    /// Cena vazia de `width × height` pixels.
    pub fn empty(width: i32, height: i32) -> Self {
        Scene {
            width,
            height,
            sprites: Vec::new(),
            markers: Vec::new(),
            borders: Vec::new(),
            warnings: Vec::new(),
            jake: None,
            jake_sprite: None,
            world: World::default(),
        }
    }

    /// Monta a cena: células (128×128, x varia mais rápido), decorações (relativas à célula)
    /// e objetos (coordenadas da fase), da lista global e das células.
    pub fn build(level: &Level, assets: &mut Assets) -> Result<Self> {
        let (gx, gy) = (
            i32::from(level.grid_x).max(1),
            i32::from(level.grid_y).max(1),
        );
        let mut s = Scene {
            width: gx * CELL,
            height: gy * CELL,
            sprites: Vec::new(),
            markers: Vec::new(),
            borders: Vec::new(),
            warnings: Vec::new(),
            jake: None,
            jake_sprite: None,
            world: World::default(),
        };
        let mut jake_layer = 0;
        let mut objects: Vec<&LevelObject> = level.objects.iter().collect();
        for (i, cell) in level.cells.iter().enumerate() {
            let Some(cell) = cell else { continue };
            let i = i as i32;
            let (cx, cy) = ((i % gx) * CELL, ((i / gx) % gy) * CELL);
            let cz = i / (gx * gy);
            if !cell.animation.is_empty() {
                match assets.animation(&cell.animation) {
                    Ok(a) => match u16::try_from(cell.index)
                        .ok()
                        .and_then(|id| AnimPlayer::at_label(&a, id))
                    {
                        Some(player) => s.sprites.push(Sprite {
                            layer: Layer::Scenery,
                            anim: a,
                            player,
                            x: cx,
                            y: cy,
                            frozen: false,
                            scale: (1, 1),
                            mirror: cell.mirrored,
                        }),
                        None => s.warnings.push(format!(
                            "{}: rótulo {} inexistente",
                            cell.animation, cell.index
                        )),
                    },
                    Err(e) => s.warnings.push(format!("{}: {e}", cell.animation)),
                }
            }
            if !cell.border.is_empty() {
                match assets.archive().get(&cell.border).and_then(Border::parse) {
                    Ok(b) => s.borders.extend(b.boxes.into_iter().map(|mut bx| {
                        if cell.mirrored {
                            // espelha a caixa dentro da célula de 128 px
                            let w = i16::try_from(bx.width).unwrap_or(i16::MAX);
                            bx.x = (CELL as i16).saturating_sub(bx.x).saturating_sub(w);
                        }
                        (cx, cy, cz, bx)
                    })),
                    Err(e) => s.warnings.push(format!("{}: {e}", cell.border)),
                }
            }
            for d in &cell.decorations {
                if d.animation.is_empty() {
                    continue;
                }
                match assets.animation(&d.animation) {
                    Ok(a) => {
                        // O primeiro campo da decoração é o quadro da animação.
                        let frame = d.fields.first().map_or(0, |&f| f.max(0) as usize);
                        let frame = if frame < a.frames.len() { frame } else { 0 };
                        let player = AnimPlayer::from_frame(&a, frame);
                        s.sprites.push(Sprite {
                            layer: Layer::Decoration,
                            anim: a,
                            player,
                            x: cx.saturating_add(d.x),
                            y: cy.saturating_add(d.y),
                            frozen: false,
                            scale: (1, 1),
                            mirror: false,
                        });
                    }
                    Err(e) => s.warnings.push(format!("{}: {e}", d.animation)),
                }
            }
            objects.extend(cell.objects.iter());
        }
        for o in objects {
            let marker = Marker {
                kind: o.kind,
                x: o.x,
                y: o.y,
            };
            if o.names[0].is_empty() {
                s.markers.push(marker);
                continue;
            }
            match assets.animation(&o.names[0]) {
                Ok(a) => {
                    let frozen = o.kind == GENERIC_ITEM;
                    let start = if frozen {
                        o.values
                            .first()
                            .map(|&v| v.max(0) as usize)
                            .filter(|&f| f < a.frames.len())
                            .unwrap_or(0)
                    } else {
                        0
                    };
                    let player = AnimPlayer::from_frame(&a, start);
                    let n = o.fields.len();
                    let (ta, tb) = if n >= 2 {
                        (o.fields[n - 2], o.fields[n - 1])
                    } else {
                        (0, 0)
                    };
                    let (rw, rh) = a
                        .frames
                        .first()
                        .map_or((0, 0), |f| (i32::from(f.width), i32::from(f.height)));
                    let scale = object_scale(ta, tb, rw, rh);
                    if o.kind == JAKE && s.jake.is_none() {
                        let boxr = assets
                            .archive()
                            .get(&o.names[1])
                            .and_then(Border::parse)
                            .ok()
                            .and_then(|b| border_bounds(&b))
                            .unwrap_or(JAKE_BOX);
                        jake_layer = o.fields.get(4).copied().unwrap_or(0);
                        let player = AnimPlayer::at_label(&a, 1).unwrap_or(player);
                        s.jake = Some(Jake::new(o.x, o.y, boxr, label_lengths(&a)));
                        s.jake_sprite = Some(Sprite {
                            layer: Layer::Object,
                            anim: a,
                            player,
                            x: o.x,
                            y: o.y,
                            frozen: false,
                            scale,
                            mirror: false,
                        });
                        continue;
                    }
                    s.sprites.push(Sprite {
                        layer: Layer::Object,
                        anim: a,
                        player,
                        x: o.x,
                        y: o.y,
                        frozen,
                        scale,
                        mirror: false,
                    });
                }
                Err(e) => {
                    s.warnings
                        .push(format!("objeto tipo {} ({}): {e}", o.kind, o.names[0]));
                    s.markers.push(marker);
                }
            }
        }
        s.sprites.sort_by_key(|p| p.layer);
        s.world = World::new(
            s.borders
                .iter()
                .filter(|(_, _, z, b)| *z == jake_layer && b.kind == BOX)
                .map(|(cx, cy, _, b)| Rect {
                    x: cx.saturating_add(i32::from(b.x)),
                    y: cy.saturating_add(i32::from(b.y)),
                    w: i32::from(b.width),
                    h: i32::from(b.height),
                })
                .collect(),
        );
        Ok(s)
    }

    /// Desenha a cena com a câmera em `camera` (canto superior esquerdo, em pixels da fase).
    /// O índice 0 é transparente em todas as camadas (cenário, decorações, objetos). Com
    /// `overlay`, desenha as caixas de colisão das células e os marcadores.
    pub fn draw(
        &self,
        assets: &mut Assets,
        fb: &mut Framebuffer,
        camera: (i32, i32),
        overlay: bool,
    ) {
        for sp in &self.sprites {
            draw_sprite(sp, assets, fb, camera);
        }
        if let Some(j) = &self.jake_sprite {
            draw_sprite(j, assets, fb, camera);
        }
        if overlay {
            for (bx, by, _, b) in &self.borders {
                let x = bx.saturating_add(i32::from(b.x)).saturating_sub(camera.0);
                let y = by.saturating_add(i32::from(b.y)).saturating_sub(camera.1);
                fb.rect(
                    x,
                    y,
                    i32::from(b.width),
                    i32::from(b.height),
                    OVERLAY_BORDER,
                );
            }
            for m in &self.markers {
                fb.rect(
                    m.x.saturating_sub(4).saturating_sub(camera.0),
                    m.y.saturating_sub(4).saturating_sub(camera.1),
                    9,
                    9,
                    OVERLAY_MARKER,
                );
            }
        }
    }

    /// Centro do Jake, para a câmera seguir.
    pub fn focus(&self) -> Option<(i32, i32)> {
        self.jake.as_ref().map(|j| {
            let b = j.body.bounds();
            (b.x.saturating_add(b.w / 2), b.y.saturating_add(b.h / 2))
        })
    }

    /// Um tique (50 ms): o Jake reage à entrada `input` (bits de `crate::input`) e todas as
    /// animações avançam.
    pub fn tick(&mut self, input: u32) {
        if let (Some(j), Some(sp)) = (self.jake.as_mut(), self.jake_sprite.as_mut()) {
            j.tick(input, &self.world);
            sp.x = j.body.x;
            sp.y = j.body.y;
            // A arte do Jake olha para a esquerda: espelha quando ele olha para a direita.
            sp.mirror = j.facing_right();
            if j.label_changed() {
                if let Some(p) = AnimPlayer::at_label(&sp.anim, j.label()) {
                    sp.player = p;
                }
            } else if j.advance() {
                let anim = sp.anim.clone();
                sp.player.tick(&anim);
            }
        }
        for sp in self.sprites.iter_mut().filter(|sp| !sp.frozen) {
            let anim = sp.anim.clone();
            sp.player.tick(&anim);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openhunted_formats::test_support::{mkanm, mkarchive, mkbmp};
    use openhunted_formats::{Cell, Level, LevelObject, ResourceArchive};

    pub(crate) fn object(kind: u32, x: i32, y: i32, anim: &str) -> LevelObject {
        LevelObject {
            kind,
            x,
            y,
            fields: vec![],
            points: vec![],
            values: vec![],
            triggers: vec![],
            names: [anim.into(), String::new()],
        }
    }

    pub(crate) fn level() -> Level {
        let cell = Cell {
            version: 1010,
            animation: "OAN.ANM".into(),
            border: String::new(),
            index: 1, // rótulo "parado" de mkanm
            mirrored: false,
            fields: vec![],
            triggers: vec![],
            objects: vec![object(5, 10, 20, "OAN.ANM")],
            props: vec![],
            decorations: vec![],
        };
        Level {
            name: "T.LEV".into(),
            palette: "WORLD.PAL".into(),
            version: 1006,
            title: String::new(),
            description: String::new(),
            notes: String::new(),
            field_1004: 0,
            grid_x: 2,
            grid_y: 1,
            grid_z: 1,
            player: 1,
            extra: 36,
            objects: vec![object(102, 200, 30, "OAN.ANM"), object(73, 50, 60, "")],
            cells: vec![None, Some(cell)],
        }
    }

    pub(crate) fn assets() -> Assets {
        let bmp = mkbmp(&[(20, 20, vec![1; 400])]);
        let anm = mkanm();
        let (map, vols) = mkarchive(&[(
            "RESOURCE.001",
            &[("OAN00.BMP", bmp.as_slice()), ("OAN.ANM", anm.as_slice())],
        )]);
        Assets::new(ResourceArchive::from_map(&map, |_| Ok(vols[0].1.clone())).unwrap())
    }

    #[test]
    fn builds_layers_and_markers() {
        let s = Scene::build(&level(), &mut assets()).unwrap();
        assert_eq!((s.width, s.height), (256, 128));
        let scenery: Vec<_> = s
            .sprites
            .iter()
            .filter(|p| p.layer == Layer::Scenery)
            .collect();
        assert_eq!(scenery.len(), 1);
        assert_eq!((scenery[0].x, scenery[0].y), (128, 0));
        let objs: Vec<_> = s
            .sprites
            .iter()
            .filter(|p| p.layer == Layer::Object)
            .collect();
        assert_eq!(objs.len(), 2);
        assert!(objs.iter().any(|p| (p.x, p.y) == (200, 30)));
        assert_eq!(
            s.markers,
            vec![Marker {
                kind: 73,
                x: 50,
                y: 60
            }]
        );
        assert!(s.warnings.is_empty());
    }

    #[test]
    fn missing_animation_becomes_marker_with_warning() {
        let mut l = level();
        l.objects.push(object(9, 1, 2, "NADA.ANM"));
        let s = Scene::build(&l, &mut assets()).unwrap();
        assert!(s.markers.contains(&Marker {
            kind: 9,
            x: 1,
            y: 2
        }));
        assert_eq!(s.warnings.len(), 1);
    }

    #[test]
    fn draw_paints_sprites_with_camera() {
        let s = Scene::build(&level(), &mut assets()).unwrap();
        let mut fb = crate::Framebuffer::new(64, 64);
        let mut a = assets();
        s.draw(&mut a, &mut fb, (190, 20), false);
        // origem (200, 30) + quadro (-7, 1) + opcode (5, 6) = (198, 37)
        assert_eq!(fb.pixels[(37 - 20) * 64 + (198 - 190)], 1);
        assert_eq!(fb.pixels[(37 - 20) * 64 + (198 - 190) - 1], 0);
        let mut empty = crate::Framebuffer::new(64, 64);
        s.draw(&mut a, &mut empty, (10_000, 10_000), true);
    }

    #[test]
    fn object_scale_rule() {
        // Jake: alvo (22, 47) sobre quadro de referência 42×88 → 47/88
        assert_eq!(object_scale(22, 47, 42, 88), (47, 88));
        assert_eq!(object_scale(30, 10, 60, 20), (30, 60));
        assert_eq!(object_scale(0, 0, 42, 88), (1, 1));
        assert_eq!(object_scale(22, 47, 42, 0), (1, 1));
        assert_eq!(object_scale(-5, -3, 42, 88), (1, 1));
    }

    #[test]
    fn extreme_coordinates_never_panic() {
        let mut l = level();
        l.objects.push(object(73, i32::MIN, i32::MAX, ""));
        if let Some(Some(c)) = l.cells.get_mut(1) {
            c.decorations.push(openhunted_formats::Decoration {
                animation: "OAN.ANM".into(),
                x: i32::MAX,
                y: i32::MIN,
                fields: vec![0],
            });
        }
        let s = Scene::build(&l, &mut assets()).unwrap();
        let mut fb = crate::Framebuffer::new(64, 64);
        s.draw(&mut assets(), &mut fb, (i32::MAX, i32::MIN), true);
        s.draw(&mut assets(), &mut fb, (i32::MIN, i32::MAX), true);
    }

    #[test]
    fn tick_advances_every_sprite() {
        let mut s = Scene::build(&level(), &mut assets()).unwrap();
        let before: Vec<usize> = s.sprites.iter().map(|p| p.player.frame()).collect();
        s.tick(0);
        // mkanm tem um quadro só: continua no mesmo, sem pânico.
        let after: Vec<usize> = s.sprites.iter().map(|p| p.player.frame()).collect();
        assert_eq!(before, after);
    }
}
