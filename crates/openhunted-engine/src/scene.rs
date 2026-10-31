// SPDX-License-Identifier: GPL-3.0-or-later
//! Cena de uma fase: blocos de cenário, decorações e objetos, com as animações em curso.

use std::rc::Rc;

use openhunted_formats::{Animation, Border, BorderBox, Level, LevelObject, Result};

use crate::{AnimPlayer, Assets, Framebuffer, draw_ops};

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
}

/// Tipo de objeto cuja animação (`OGF.ANM`) é uma folha de itens num rótulo só: cada objeto
/// mostra um quadro fixo, escolhido por um campo ainda não decifrado. Provisório (M2): parado
/// no quadro `values[0]` quando válido, senão no 0. Ver `docs/re/objetos.md`.
const GENERIC_ITEM: u32 = 6;

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
    /// Caixas de colisão das células: origem da célula e caixa.
    pub borders: Vec<(i32, i32, BorderBox)>,
    /// Problemas encontrados ao montar a cena (recursos ausentes), um por linha.
    pub warnings: Vec<String>,
}

impl Scene {
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
        };
        let mut objects: Vec<&LevelObject> = level.objects.iter().collect();
        for (i, cell) in level.cells.iter().enumerate() {
            let Some(cell) = cell else { continue };
            let i = i as i32;
            let (cx, cy) = ((i % gx) * CELL, ((i / gx) % gy) * CELL);
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
                    Ok(b) => s.borders.extend(b.boxes.into_iter().map(|bx| (cx, cy, bx))),
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
                    s.sprites.push(Sprite {
                        layer: Layer::Object,
                        anim: a,
                        player,
                        x: o.x,
                        y: o.y,
                        frozen,
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
            for op in draw_ops(&sp.anim, sp.player.frame()) {
                let Ok(bmp) = assets.bitmap(&op.bitmap) else {
                    continue;
                };
                let Some(frame) = bmp.frames.get(op.frame as usize) else {
                    continue;
                };
                let x = sp.x.saturating_add(op.x).saturating_sub(camera.0);
                let y = sp.y.saturating_add(op.y).saturating_sub(camera.1);
                // Índice 0 é transparente em todas as camadas: os blocos de z = 0 não o usam, e os
                // das camadas de cima o usam como área vazada.
                fb.blit(frame, x, y, true);
            }
        }
        if overlay {
            for (bx, by, b) in &self.borders {
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

    /// Avança todas as animações um quadro.
    pub fn tick(&mut self) {
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
        s.tick();
        // mkanm tem um quadro só: continua no mesmo, sem pânico.
        let after: Vec<usize> = s.sprites.iter().map(|p| p.player.frame()).collect();
        assert_eq!(before, after);
    }
}
