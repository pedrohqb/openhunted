// SPDX-License-Identifier: GPL-3.0-or-later
//! Cena de uma fase: blocos de cenário, decorações e objetos, com as animações em curso.

mod arsenal;
mod cell_door;
mod switches;

use std::rc::Rc;

use std::collections::HashMap;

use openhunted_formats::{Animation, Border, BorderBox, Level, LevelObject, Result};

use crate::behaviour::{self, Ctx, Event, Out, Outcome};
use crate::entity::{BOX, Behaviour, Entity, from_object, object_scale, object_start};
use crate::jake::{Env, Skel, SpecialBlast, Zone, zone};
use crate::objects::{SOLO, flags, is_player_start, present};
use crate::physics::{Rect, SUB, World};
use crate::rng::Rng;
use crate::sound::Cue;
use crate::verdict::{self, Streak};
use crate::{AnimPlayer, Assets, Framebuffer, Jake, draw_ops};

/// Duração de um tick de animação, em milissegundos (provisório: `docs/re/cenario.md`).
pub const TICK_MS: u32 = 50;
/// Som "Pistol" (SOUND.TBL, primeiro quadro dos estados de tiro).
pub const SOUND_PISTOL: u16 = 3040;
/// Cores do overlay (índices da paleta).
const OVERLAY_BORDER: u8 = 255;
const OVERLAY_MARKER: u8 = 254;
const OVERLAY_OBJECT: u8 = 253;
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

/// A entidade barra o Jake (`objects::blocks` sobre os sinalizadores dela; só caixas do BRD
/// ou própria, e só o que ainda está em jogo).
fn blocks_entity(jake_flags: i32, e: &Entity) -> bool {
    e.alive && e.solid_shapes && crate::objects::blocks_flags(jake_flags, e.flags)
}

/// Parâmetros de um projétil a criar.
struct Projectile {
    kind: u32,
    anim: &'static str,
    label: u16,
    mirror: u8,
    brd: &'static str,
    /// Tamanho-alvo em px (`+0x1f0`).
    target: i32,
    x: i32,
    y: i32,
    layer: i32,
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
    /// O Jake controlável: o início de jogador de tipo 1 no modo solo (ou, sem modo solo, o
    /// primeiro objeto de tipo 1), se a fase tiver um.
    pub jake: Option<Jake>,
    /// Sprite do Jake (desenhado depois dos objetos).
    pub jake_sprite: Option<Sprite>,
    /// Geometria sólida da camada do Jake.
    pub world: World,
    /// Caixas dos objetos sólidos para o Jake (em px da fase), para o overlay.
    pub object_solids: Vec<Rect>,
    /// Os objetos da fase em jogo (sem o Jake), em ordem de desenho.
    pub entities: Vec<Entity>,
    /// Camada do Jake (`+0x17c`) e sinalizadores dele.
    pub jake_layer: i32,
    jake_flags: i32,
    /// Caixas sólidas das células da camada do Jake.
    cell_solids: Vec<Rect>,
    /// Mundo só com as células (para as minas).
    cell_world: World,
    /// `OEX.ANM`, se existir nos dados.
    explosion_anim: Option<Rc<Animation>>,
    /// Animações e BRDs dos projéteis, carregados na montagem.
    projectile_anims: HashMap<String, Rc<Animation>>,
    projectile_borders: HashMap<String, Border>,
    /// Zonas PR das células (tipo em bits: camadas, corda, saída…), em px da fase.
    pub prop_zones: Vec<Zone>,
    /// Número de camadas da fase (`grid_z`).
    pub layers: i32,
    streak: Streak,
    prev_input: u32,
    /// Linhas do `WIZARD.RES` (dicas), se houver.
    pub hints: Vec<String>,
    /// Eventos do último tique (quem mostra a cena esvazia).
    pub events: Vec<Event>,
    /// Fim da partida, quando decidido.
    pub outcome: Option<Outcome>,
    pub tick_count: u32,
    pub rng: Rng,
    /// Inimigos (por ora, minas) destruídos pelo jogador (`0x497348`).
    pub kills: i32,
    /// Inimigos presentes no início da partida (minas e olhos-laser, por ora).
    pub mines_at_start: i32,
    /// Cronômetro em tiques (`0x4972fc` = campo 1004 × 20; decrementado a cada tique).
    pub timer: i32,
    /// Limite de tempo da fase em segundos (campo 1004; 0 = sem limite).
    pub time_limit: i32,
    /// Portas de célula (células de índice `…38…`).
    pub cell_doors: Vec<cell_door::CellDoor>,
    /// Tiques de terremoto que ainda fazem o Jake cambalear (`+0x274`, `0x418229`).
    pub quake: u32,
    /// Último tique de cada som com limite global de repetição (sentinelas, robôs).
    sound_clock: HashMap<u16, u32>,
}

/// Tiques por segundo do cronômetro (`0x43c95c`).
pub const TIMER_TICKS_PER_SECOND: i32 = 20;

/// Tipo de objeto do Jake.
const JAKE: u32 = 1;

/// A fase tem o Jake controlável no modo solo (como em `Scene::build`): um início de jogador
/// de tipo 1 no modo solo ou, sem modo solo, algum objeto de tipo 1. Nas fases do Garathe
/// (ainda sem personagem controlável) é `false`.
pub fn has_controllable_jake(level: &Level) -> bool {
    let objects: Vec<&LevelObject> = level
        .objects
        .iter()
        .chain(level.cells.iter().flatten().flat_map(|c| c.objects.iter()))
        .collect();
    let solo = objects.iter().any(|o| is_player_start(o, SOLO));
    objects
        .iter()
        .any(|o| o.kind == JAKE && (!solo || is_player_start(o, SOLO)))
}
/// Caixa de colisão de reserva do Jake (envolvente do `JK_DEFLT.BRD`).
const JAKE_BOX: Rect = Rect {
    x: 2,
    y: 3,
    w: 18,
    h: 44,
};

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

/// Grafite: um glifo do `OGF.ANM` por caractere, com a escala do texto.
fn draw_graffiti(
    e: &Entity,
    g: &behaviour::graffiti::GraffitiState,
    sp: &Sprite,
    assets: &mut Assets,
    fb: &mut Framebuffer,
    camera: (i32, i32),
) {
    for &(dx, frame) in &g.glyphs {
        let glyph = Sprite {
            player: AnimPlayer::from_frame(&sp.anim, frame),
            x: e.x.saturating_add(dx),
            y: e.y,
            scale: g.scale,
            mirror: false,
            ..sp.clone()
        };
        draw_sprite(&glyph, assets, fb, camera);
    }
}

/// Raio laser (`0x00410dae`): traço da cauda à cabeça, até 25 px, nas cores da variante.
fn draw_laser(
    e: &Entity,
    l: &behaviour::laser::LaserState,
    fb: &mut Framebuffer,
    camera: (i32, i32),
) {
    let head = e.center();
    let (mut dx, mut dy) = (l.tail.0 - head.0, l.tail.1 - head.1);
    let d = behaviour::mine::distance((0, 0), (dx, dy));
    if d > behaviour::laser::TRAIL {
        dx = dx * behaviour::laser::TRAIL / d;
        dy = dy * behaviour::laser::TRAIL / d;
    }
    let colors = behaviour::laser::COLORS[l.variant.min(3)];
    let n = dx.abs().max(dy.abs()).max(1);
    for i in 0..=n {
        let x = head.0 + dx * i / n - camera.0;
        let y = head.1 + dy * i / n - camera.1;
        fb.fill_rect(x - 1, y, 3, 1, colors[2]);
        fb.fill_rect(x, y, 1, 1, colors[0]);
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
            object_solids: Vec::new(),
            entities: Vec::new(),
            jake_layer: 0,
            jake_flags: 0,
            cell_solids: Vec::new(),
            cell_world: World::default(),
            explosion_anim: None,
            projectile_anims: HashMap::new(),
            projectile_borders: HashMap::new(),
            prop_zones: Vec::new(),
            layers: 1,
            streak: Streak::default(),
            prev_input: 0,
            hints: Vec::new(),
            events: Vec::new(),
            outcome: None,
            tick_count: 0,
            rng: Rng::new(0x4a4b_4531),
            kills: 0,
            mines_at_start: 0,
            timer: 0,
            time_limit: 0,
            cell_doors: Vec::new(),
            quake: 0,
            sound_clock: HashMap::new(),
        }
    }

    /// Monta a cena: células (128×128, x varia mais rápido), decorações (relativas à célula)
    /// e objetos (coordenadas da fase), da lista global e das células. Nas fases com início de
    /// jogador solo, só entram os objetos presentes no modo solo (`objects::present`).
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
            object_solids: Vec::new(),
            entities: Vec::new(),
            jake_layer: 0,
            jake_flags: 0,
            cell_solids: Vec::new(),
            cell_world: World::default(),
            explosion_anim: None,
            projectile_anims: HashMap::new(),
            projectile_borders: HashMap::new(),
            prop_zones: Vec::new(),
            layers: 1,
            streak: Streak::default(),
            prev_input: 0,
            hints: Vec::new(),
            events: Vec::new(),
            outcome: None,
            tick_count: 0,
            rng: Rng::new(0x4a4b_4531),
            kills: 0,
            mines_at_start: 0,
            timer: 0,
            time_limit: 0,
            cell_doors: Vec::new(),
            quake: 0,
            sound_clock: HashMap::new(),
        };
        let mut scenery = 0usize;
        let mut jake_layer = 0;
        let mut jake_flags = 0;
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
                        Some(player) => {
                            if cell_door::is_door(cell.index) {
                                let id = cell.index as u16;
                                let first = a
                                    .labels
                                    .iter()
                                    .find(|l| l.id == id)
                                    .map_or(0, |l| l.frame as usize);
                                let next = a
                                    .labels
                                    .iter()
                                    .map(|l| l.frame as usize)
                                    .filter(|&f| f > first)
                                    .min()
                                    .unwrap_or(a.frames.len());
                                let fx = if cell.version >= 1006 { 16 } else { 15 };
                                let mut d =
                                    cell_door::CellDoor::new(cell.fields.get(fx..).unwrap_or(&[]));
                                d.origin = (cx, cy);
                                d.layer = cz;
                                d.sprite = scenery;
                                d.first = first;
                                d.last = next.saturating_sub(1).max(first);
                                d.frame = if d.state == cell_door::OPEN {
                                    d.last
                                } else {
                                    first
                                };
                                d.triggers = cell.triggers.clone();
                                s.cell_doors.push(d);
                            }
                            scenery += 1;
                            s.sprites.push(Sprite {
                                layer: Layer::Scenery,
                                anim: a,
                                player,
                                x: cx,
                                y: cy,
                                frozen: false,
                                scale: (1, 1),
                                mirror: cell.mirrored,
                            })
                        }
                        None => s.warnings.push(format!(
                            "{}: rótulo {} inexistente",
                            cell.animation, cell.index
                        )),
                    },
                    Err(e) => s.warnings.push(format!("{}: {e}", cell.animation)),
                }
            }
            let door = s
                .cell_doors
                .last_mut()
                .filter(|d| d.origin == (cx, cy) && d.layer == cz);
            if let Some(d) = door {
                // a colisão da porta fica fora das caixas fixas: as duas versões do BRD
                for closed in [true, false] {
                    let name = cell_door::border_name(cell.index, closed);
                    let boxes: Vec<Rect> = assets
                        .archive()
                        .get(&name)
                        .and_then(Border::parse)
                        .map(|b| {
                            b.boxes
                                .into_iter()
                                .filter(|bx| bx.kind == BOX)
                                .map(|bx| {
                                    let w = i32::from(bx.width);
                                    let x = if cell.mirrored {
                                        CELL - i32::from(bx.x) - w
                                    } else {
                                        i32::from(bx.x)
                                    };
                                    Rect {
                                        x: cx.saturating_add(x),
                                        y: cy.saturating_add(i32::from(bx.y)),
                                        w,
                                        h: i32::from(bx.height),
                                    }
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    if closed {
                        d.closed = boxes;
                    } else {
                        d.open = boxes;
                    }
                }
            } else if !cell.border.is_empty() {
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
            for pr in &cell.props {
                // espelhada com a célula, como as caixas do BRD (`0x00405775`)
                let x = if cell.mirrored {
                    CELL.saturating_sub(pr.rect[0]).saturating_sub(pr.rect[2])
                } else {
                    pr.rect[0]
                };
                s.prop_zones.push(Zone {
                    layer: cz,
                    kind: pr.kind as u16,
                    rect: Rect {
                        x: cx.saturating_add(x),
                        y: cy.saturating_add(pr.rect[1]),
                        w: pr.rect[2],
                        h: pr.rect[3],
                    },
                });
            }
            objects.extend(cell.objects.iter());
        }
        // Modo solo quando a fase tem um início de jogador nele: só os objetos presentes no
        // modo, e o Jake é esse início. Sem ele (fases de vários jogadores), tudo, e o Jake é o
        // primeiro objeto de tipo 1.
        let solo = objects.iter().any(|o| is_player_start(o, SOLO));
        if solo {
            objects.retain(|o| present(o, SOLO) || is_player_start(o, SOLO));
        }
        let is_jake = |o: &LevelObject| o.kind == JAKE && (!solo || is_player_start(o, SOLO));
        for o in objects {
            if is_jake(o) && s.jake.is_none() {
                match assets.animation(&o.names[0]) {
                    Ok(a) => {
                        let n = o.fields.len();
                        let (ta, tb) = if n >= 2 {
                            (o.fields[n - 2], o.fields[n - 1])
                        } else {
                            (0, 0)
                        };
                        let (ref_frame, player, _) = object_start(o, &a);
                        let (rw, rh) = a
                            .frames
                            .get(ref_frame)
                            .map_or((0, 0), |f| (i32::from(f.width), i32::from(f.height)));
                        let scale = object_scale(ta, tb, rw, rh);
                        let sc = |v: i32| v * scale.0 / scale.1.max(1);
                        // o esqueleto do Jake tem id 1 (o do Garathe, 2)
                        let skel = match assets
                            .archive()
                            .get("JAKE.SKE")
                            .and_then(openhunted_formats::Skeleton::parse)
                        {
                            Ok(k) if k.id == 1 => Some(Skel::new(Rc::new(k), (sc(rw), sc(rh)))),
                            Ok(k) => {
                                s.warnings
                                    .push(format!("JAKE.SKE: id {} (esperado 1)", k.id));
                                None
                            }
                            Err(e) => {
                                s.warnings.push(format!("JAKE.SKE: {e}"));
                                None
                            }
                        };
                        let brd = |name: &str, fallback: Rect| {
                            assets
                                .archive()
                                .get(name)
                                .and_then(Border::parse)
                                .ok()
                                .and_then(|b| border_bounds(&b))
                                .unwrap_or(fallback)
                        };
                        let crawl = brd("JK_CRAWL.BRD", crate::jake::CRAWL_BOX);
                        let bar = brd("JK_BARUP.BRD", crate::jake::BAR_BOX);
                        let boxr = assets
                            .archive()
                            .get(&o.names[1])
                            .and_then(Border::parse)
                            .ok()
                            .and_then(|b| border_bounds(&b))
                            .unwrap_or(JAKE_BOX);
                        jake_layer = o.fields.get(4).copied().unwrap_or(0);
                        jake_flags = flags(o);
                        let player = AnimPlayer::at_label(&a, 1).unwrap_or(player);
                        let field = |i: usize| o.fields.get(i).copied().unwrap_or(0);
                        let mut j = Jake::new(o.x, o.y, boxr, label_lengths(&a))
                            .with_health(field(14), field(15))
                            .with_inventory(crate::jake::Inventory::from_points(&o.points))
                            .with_layer(jake_layer)
                            .with_boxes(crawl, bar);
                        if let Some(k) = skel {
                            j = j.with_skeleton(k);
                        }
                        s.jake = Some(j);
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
                    Err(e) => {
                        s.warnings
                            .push(format!("objeto tipo {} ({}): {e}", o.kind, o.names[0]));
                    }
                }
            }
            let e = from_object(o, assets, &mut s.warnings);
            if e.sprite.is_none() {
                s.markers.push(Marker {
                    kind: o.kind,
                    x: o.x,
                    y: o.y,
                });
            }
            s.entities.push(e);
        }
        s.sprites.sort_by_key(|p| p.layer);
        s.jake_flags = jake_flags;
        s.layers = i32::from(level.grid_z).max(1);
        s.set_jake_layer(jake_layer);
        s.time_limit = i32::from(level.field_1004.max(0));
        s.timer = s.time_limit * TIMER_TICKS_PER_SECOND;
        if let Ok(res) = assets.archive().get("WIZARD.RES") {
            s.hints = openhunted_formats::parse_text_list(res);
        }
        s.explosion_anim = assets.animation("OEX.ANM").ok();
        for name in ["OBU.ANM", "OBS00.ANM"]
            .into_iter()
            .chain(arsenal::ANIMS)
            .chain(behaviour::enemy::ANIMS)
        {
            if let Ok(a) = assets.animation(name) {
                s.projectile_anims.insert(name.to_string(), a);
            }
        }
        for name in ["OBU00.BRD", "OBU01.BRD", "OBU02.BRD", "OBS.BRD"]
            .into_iter()
            .chain(arsenal::BORDERS)
            .chain(behaviour::enemy::BORDERS)
        {
            if let Ok(b) = assets.archive().get(name).and_then(Border::parse) {
                s.projectile_borders.insert(name.to_string(), b);
            }
        }
        s.mines_at_start = s
            .entities
            .iter()
            .filter(|e| {
                e.alive
                    && (matches!(e.behaviour, Behaviour::Mine(_) | Behaviour::LaserEye(_))
                        || crate::entity::ENEMIES.contains(&e.kind))
            })
            .count() as i32;
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
        for e in &self.entities {
            if !e.visible() {
                continue;
            }
            match (&e.behaviour, &e.sprite) {
                (Behaviour::Graffiti(g), Some(sp)) => draw_graffiti(e, g, sp, assets, fb, camera),
                (Behaviour::Laser(l), _) => draw_laser(e, l, fb, camera),
                (_, Some(sp)) => draw_sprite(sp, assets, fb, camera),
                _ => {}
            }
        }
        if let Some(j) = self
            .jake_sprite
            .as_ref()
            .filter(|_| self.jake.as_ref().is_none_or(crate::jake::Jake::is_visible))
        {
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
            // portas de célula: fora de `borders`, desenhadas no estado atual
            for r in self.cell_doors.iter().flat_map(|d| d.solids()) {
                fb.rect(
                    r.x.saturating_sub(camera.0),
                    r.y.saturating_sub(camera.1),
                    r.w,
                    r.h,
                    OVERLAY_BORDER,
                );
            }
            for r in &self.object_solids {
                fb.rect(
                    r.x.saturating_sub(camera.0),
                    r.y.saturating_sub(camera.1),
                    r.w,
                    r.h,
                    OVERLAY_OBJECT,
                );
            }
            for z in self
                .prop_zones
                .iter()
                .filter(|z| z.layer == self.jake_layer)
            {
                fb.rect(
                    z.rect.x.saturating_sub(camera.0),
                    z.rect.y.saturating_sub(camera.1),
                    z.rect.w,
                    z.rect.h,
                    OVERLAY_MARKER,
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

    /// A fase tem limite de tempo e o cronômetro chegou a 0.
    pub fn timed_out(&self) -> bool {
        self.time_limit > 0 && self.timer <= 0
    }

    /// Centro do Jake, para a câmera seguir.
    pub fn focus(&self) -> Option<(i32, i32)> {
        self.jake.as_ref().map(|j| {
            let b = j.body.bounds();
            (b.x.saturating_add(b.w / 2), b.y.saturating_add(b.h / 2))
        })
    }

    /// Um tique (50 ms): o Jake reage à entrada `input` (bits de `crate::input`), as
    /// entidades agem, e todas as animações avançam. Com a partida decidida, só anima.
    pub fn tick(&mut self, input: u32) {
        let input = if self.outcome.is_some() { 0 } else { input };
        if let (Some(j), Some(sp)) = (self.jake.as_mut(), self.jake_sprite.as_mut()) {
            let env = Env {
                world: &self.world,
                zones: &self.prop_zones,
                layers: self.layers,
            };
            j.tick_in(input, &env);
            j.respawn_if_outside((self.width, self.height));
            sp.x = j.body.x;
            sp.y = j.body.y;
        }
        let layer = self.jake.as_ref().map(|j| j.layer());
        if let Some(l) = layer.filter(|&l| l != self.jake_layer) {
            self.set_jake_layer(l);
        }
        if let Some(b) = self.jake.as_mut().and_then(|j| j.special.take()) {
            self.apply_special(b);
        }
        if let Some(r) = self.jake.as_mut().and_then(|j| j.punch.take()) {
            self.apply_punch(r);
        }
        self.tick_cell_doors();
        self.spawn_shots();
        if self.outcome.is_none() {
            // balas à queima-roupa: acertam já na boca, antes de voar
            let mut pre = Out::default();
            self.bullet_hits(&mut pre);
            self.resolve_impacts(&mut pre);
            self.entities.extend(pre.spawn);
            self.events.extend(pre.events);
            let up_edge = input & crate::input::UP != 0 && self.prev_input & crate::input::UP == 0;
            if up_edge {
                self.try_switch();
                self.try_exit();
            }
            self.tick_entities();
            if self.timer > 0 {
                self.timer -= 1;
            }
            let mut result = verdict::evaluate(&self.entities, self.jake.as_ref());
            // tempo esgotado numa fase com limite → derrota (`0x43cdf0`)
            if result.is_none() && self.timed_out() {
                result = Some(Outcome::Defeat);
            }
            if let Some(o) = self.streak.push(result) {
                self.outcome = Some(o);
                self.events.push(Event::Outcome(o));
            }
        }
        self.prev_input = input;
        if let (Some(j), Some(sp)) = (self.jake.as_mut(), self.jake_sprite.as_mut()) {
            // deslocamento do desenho pelo esqueleto (`+0x16c/+0x170`)
            let (ox, oy) = j.draw_offset();
            sp.x = j.body.x.saturating_add(ox);
            sp.y = j.body.y.saturating_add(oy);
            // A arte do Jake olha para a esquerda: espelha quando ele olha para a direita.
            sp.mirror = j.sprite_mirrored();
            if let Some(num) = j.scale_override() {
                sp.scale.0 = num;
            }
            if let Some(mut p) = AnimPlayer::at_label(&sp.anim, j.label()) {
                p.seek(j.frame());
                sp.player = p;
            }
        }
        for sp in self.sprites.iter_mut().filter(|sp| !sp.frozen) {
            let anim = sp.anim.clone();
            sp.player.tick(&anim);
        }
        for e in self.entities.iter_mut().filter(|e| e.alive) {
            if let Some(sp) = e.sprite.as_mut() {
                sp.x = e.x;
                sp.y = e.y;
                if !sp.frozen
                    && !matches!(e.behaviour, Behaviour::Exit(_) | Behaviour::Explosion(_))
                {
                    let anim = sp.anim.clone();
                    sp.player.tick(&anim);
                }
            }
        }
        self.tick_count = self.tick_count.saturating_add(1);
    }

    fn assets_anim(&self, name: &str) -> Option<Rc<Animation>> {
        self.projectile_anims.get(name).cloned()
    }

    fn assets_border(&self, name: &str) -> Option<Border> {
        self.projectile_borders.get(name).cloned()
    }

    /// Projéteis e cápsulas que o Jake disparou neste tique viram entidades; som do tiro ao
    /// entrar num estado de tiro; aviso de sem munição e de troca de arma (3090).
    fn spawn_shots(&mut self) {
        let focus = self.focus();
        let Some(j) = self.jake.as_mut() else { return };
        let shots: Vec<_> = j.shots.drain(..).collect();
        if j.no_ammo || j.switched {
            self.events
                .push(Event::Sound(Cue::ui(crate::jake::SOUND_NO_AMMO, 0)));
        }
        if let (Some(c), Some(id)) = (focus, j.fire_sound()) {
            self.events.push(Event::Sound(Cue::at(
                id,
                crate::sound::PRIORITY_FRAME,
                c,
                self.jake_layer,
            )));
        }
        let layer = self.jake_layer;
        for s in shots {
            if let Some(e) = self.shot_entity(&s, layer) {
                self.entities.push(e);
            }
        }
    }

    /// Entidade de um `Shot` do Jake pelo tipo.
    fn shot_entity(&mut self, s: &crate::jake::Shot, layer: i32) -> Option<Entity> {
        use crate::jake::ShotKind;
        match s.kind {
            ShotKind::Bullet => self.bullet_entity(s, layer),
            ShotKind::Casing => self.casing_entity(s, layer),
            _ => self.weapon_entity(s, layer),
        }
    }

    /// Entidade de projétil: animação, rótulo, espelho, caixa do BRD e tamanho-alvo.
    fn projectile_entity(&mut self, p: Projectile, behaviour: Behaviour) -> Option<Entity> {
        let Projectile {
            kind,
            anim: anim_name,
            label,
            mirror,
            brd,
            target,
            x,
            y,
            layer,
        } = p;
        let anim = self.assets_anim(anim_name)?;
        // rótulo da direção; sem ele, o primeiro rótulo ou o quadro 0 (animações parciais)
        let player = AnimPlayer::at_label(&anim, label)
            .or_else(|| AnimPlayer::at_label(&anim, 1))
            .unwrap_or_else(|| AnimPlayer::from_frame(&anim, 0));
        let f = anim.frames.get(player.frame())?;
        let (fw, fh) = (i32::from(f.width).max(1), i32::from(f.height).max(1));
        let scale = if fw >= fh { (target, fw) } else { (target, fh) };
        let boxes = self
            .assets_border(brd)
            .map(|b| crate::entity::border_boxes(&b))
            .unwrap_or_else(|| {
                vec![Rect {
                    x: 0,
                    y: 0,
                    w: 4,
                    h: 4,
                }]
            });
        Some(Entity {
            kind,
            x,
            y,
            flags: if kind == crate::entity::CASING || kind == crate::entity::SHELL {
                0x2031
            } else {
                0x2019
            },
            layer,
            id: 0,
            values: vec![],
            triggers: vec![],
            objective: 0,
            health: 1,
            armor: 0,
            boxes,
            solid_shapes: false,
            on: false,
            alive: true,
            sprite: Some(Sprite {
                layer: Layer::Object,
                anim,
                player,
                x,
                y,
                frozen: false,
                scale,
                mirror: mirror & 1 != 0,
            }),
            behaviour,
        })
    }

    fn bullet_entity(&mut self, s: &crate::jake::Shot, layer: i32) -> Option<Entity> {
        let (label, mirror) = behaviour::bullet::label_and_mirror(s.vx, s.vy);
        let brd = ["OBU00.BRD", "OBU00.BRD", "OBU01.BRD", "OBU02.BRD"][label as usize];
        self.projectile_entity(
            Projectile {
                kind: crate::entity::BULLET,
                anim: "OBU.ANM",
                label,
                mirror,
                brd,
                target: 10,
                x: s.x,
                y: s.y,
                layer,
            },
            Behaviour::Bullet(behaviour::bullet::BulletState {
                vx: s.vx * SUB,
                vy: s.vy * SUB,
                prev: None,
                spent: false,
                // `spawn_shots` só cria as balas do Jake
                player_shot: true,
            }),
        )
    }

    fn casing_entity(&mut self, s: &crate::jake::Shot, layer: i32) -> Option<Entity> {
        self.projectile_entity(
            Projectile {
                kind: crate::entity::CASING,
                anim: "OBS00.ANM",
                label: 1,
                mirror: 0,
                brd: "OBS.BRD",
                target: 3,
                x: s.x,
                y: s.y,
                layer,
            },
            Behaviour::Casing(behaviour::casing::CasingState {
                vx: s.vx * SUB,
                vy: s.vy * SUB,
                slow: 0,
                clinked: false,
            }),
        )
    }

    /// Impactos de bala → faísca `oex15` e som.
    fn resolve_impacts(&mut self, out: &mut Out) {
        for (x, y, layer) in std::mem::take(&mut out.impacts) {
            let b = behaviour::Blast {
                center: (x, y),
                layer,
                radius: behaviour::bullet::IMPACT_RADIUS,
                push: 0,
                damage: 0,
            };
            if let Some(e) = self.effect_entity(&b, behaviour::bullet::IMPACT_LABEL) {
                out.spawn.push(e);
            }
            out.events.push(Event::Sound(Cue::at(
                behaviour::bullet::SOUND_WALL,
                crate::sound::PRIORITY_FRAME,
                (x, y),
                layer,
            )));
        }
    }

    /// Balas contra as entidades sólidas da camada (minas, inimigos…): vítima leva o dano
    /// da linha (19, qualquer); a bala vira faísca.
    fn bullet_hits(&mut self, out: &mut Out) {
        let hits: Vec<(usize, usize)> = {
            let bullets: Vec<(usize, Rect, i32)> = self
                .entities
                .iter()
                .enumerate()
                .filter(|(_, e)| e.alive && matches!(e.behaviour, Behaviour::Bullet(_)))
                .filter_map(|(i, e)| behaviour::bullet::swept_bounds(e).map(|b| (i, b, e.layer)))
                .collect();
            let mut v = Vec::new();
            for (bi, bb, layer) in bullets {
                let victim = self.entities.iter().position(|e| {
                    e.alive
                        && e.solid_shapes
                        && e.layer == layer
                        && e.flags & 0x80003 == 0x80001
                        && e.flags & 0x20 == 0
                        && !matches!(e.behaviour, Behaviour::Bullet(_) | Behaviour::Casing(_))
                        && e.bounds().is_some_and(|b| b.overlaps(&bb))
                });
                if let Some(vi) = victim {
                    v.push((bi, vi));
                }
            }
            v
        };
        for (bi, vi) in hits {
            if !self.entities[bi].alive {
                continue;
            }
            // só quem trata a própria morte perde vida (minas, olhos-laser, barris); os
            // demais só param a bala. Contam para o jogador as minas e os olhos que a bala
            // dele mata.
            let by_player = matches!(
                self.entities[bi].behaviour,
                Behaviour::Bullet(behaviour::bullet::BulletState {
                    player_shot: true,
                    ..
                })
            );
            if let Behaviour::Mine(_) = self.entities[vi].behaviour {
                if self.entities[vi].health > 0 && by_player {
                    self.kills += 1;
                }
                self.entities[vi].health -= behaviour::bullet::DAMAGE;
                self.entities[vi].health = 0;
            } else if behaviour::enemy::hurt(
                &mut self.entities[vi],
                &crate::damage::Pending {
                    amount: behaviour::bullet::DAMAGE,
                    ..Default::default()
                },
                by_player,
            ) {
                // linha (19, 8): "Bullet hits metal" no robô de segurança
                if self.entities[vi].kind == crate::entity::SECURITY_ROBOT {
                    let c = self.entities[vi].center();
                    out.events.push(Event::Sound(Cue::at(
                        behaviour::security_robot::SOUND_BULLET,
                        crate::sound::PRIORITY_FRAME,
                        c,
                        self.entities[vi].layer,
                    )));
                }
            } else {
                let fresh = matches!(&self.entities[vi].behaviour,
                    Behaviour::LaserEye(s) if s.hurt.is_empty());
                let p = crate::damage::Pending {
                    amount: behaviour::bullet::DAMAGE,
                    explosion: true,
                    ..Default::default()
                };
                if behaviour::interact::hurt(&mut self.entities[vi], &p) && fresh && by_player {
                    self.kills += 1;
                }
            }
            let c = self.entities[bi].center();
            self.entities[bi].alive = false;
            out.impacts.push((c.0, c.1, self.entities[bi].layer));
        }
        for e in self.entities.iter_mut() {
            behaviour::bullet::finish(e, out);
        }
    }

    /// Interações entre objetos depois do tique deles (`behaviour::interact`): raios contra
    /// objetos, fogo e cerca contra barris, o par da cerca, os efeitos `oex` fixos e o
    /// teletransporte do Jake.
    fn object_interactions(&mut self, out: &mut Out) {
        behaviour::interact::laser_hits(&mut self.entities, out);
        behaviour::interact::apply_hazards(&mut self.entities, &out.hazards);
        behaviour::interact::sync_fences(&mut self.entities);
        for (b, label) in std::mem::take(&mut out.effects) {
            if let Some(e) = self.effect_entity(&b, label) {
                if let Some(c) = behaviour::explosion::frame_cue(&e) {
                    out.events.push(Event::Sound(c));
                }
                out.spawn.push(e);
            }
        }
        if let Some(req) = out.teleport.take() {
            let dest = behaviour::teleporter::destination(&self.entities, &req, &mut self.rng);
            let mut to = None;
            if let (Some((delta, i)), Some(j)) = (dest, self.jake.as_mut()) {
                j.start_teleport(delta);
                to = Some(i);
            }
            behaviour::teleporter::settle(&mut self.entities, to, self.tick_count);
        }
    }

    /// CIMA (`0x00423b8e`): o Jake parado sobre uma zona PR 0x40 e tocando a caixa de uma
    /// saída aciona-a (`0x0040e4dd`): objetivos cumpridos → a saída liga e ele entra na porta;
    /// senão, choque de 20 e os dois sons.
    fn try_exit(&mut self) {
        let Some(j) = self.jake.as_ref().filter(|j| j.can_use_exit()) else {
            return;
        };
        let jb = j.body.bounds();
        let in_zone = self.prop_zones.iter().any(|z| {
            z.layer == self.jake_layer && z.kind & zone::EXIT != 0 && z.rect.overlaps(&jb)
        });
        if !in_zone {
            return;
        }
        let touching = |e: &Entity| {
            matches!(e.behaviour, Behaviour::Exit(_))
                && e.alive
                && e.layer == self.jake_layer
                && e.bounds().is_some_and(|b| {
                    // desigualdades não estritas (`0x0040e4dd`)
                    jb.x <= b.x + b.w
                        && jb.x + jb.w >= b.x
                        && jb.y <= b.y + b.h
                        && jb.y + jb.h >= b.y
                })
        };
        let Some(i) = self.entities.iter().position(touching) else {
            return;
        };
        if verdict::objectives_met(&self.entities, self.jake.as_ref()) {
            self.entities[i].on = true;
            let num = self.jake_sprite.as_ref().map_or(1, |sp| sp.scale.0);
            if let Some(j) = self.jake.as_mut() {
                j.enter_exit(num);
            }
        } else {
            if let Some(j) = self.jake.as_mut() {
                j.hurt(crate::damage::Pending {
                    amount: behaviour::exit::ZAP_DAMAGE,
                    electric: true,
                    ..Default::default()
                });
            }
            self.events.push(Event::Sound(Cue::ui(
                behaviour::exit::SOUND_OBJECTIVES,
                crate::sound::PRIORITY_PICKUP,
            )));
            if let Some(c) = self.focus() {
                self.events.push(Event::Sound(Cue::at(
                    behaviour::exit::SOUND_ZAP,
                    0,
                    c,
                    self.jake_layer,
                )));
            }
        }
    }

    /// Entidade visual de uma explosão: `OEX.ANM`, rótulo sorteado 1–4, com o sprite medindo
    /// `2 · raio` px de altura, centrado no ponto. Sem animação disponível, nenhuma.
    fn explosion_entity(&mut self, b: &behaviour::Blast) -> Option<Entity> {
        let label = 1 + self.rng.below(4) as u16;
        self.effect_entity(b, label)
    }

    /// Efeito do `OEX.ANM` com um rótulo dado (faísca de bala: 15).
    fn effect_entity(&mut self, b: &behaviour::Blast, label: u16) -> Option<Entity> {
        let anim = self.explosion_anim.clone()?;
        let player = AnimPlayer::at_label(&anim, label)?;
        let f = anim.frames.get(player.frame())?;
        let (fw, fh) = (i32::from(f.width).max(1), i32::from(f.height).max(1));
        let scale = (2 * b.radius.max(1), fh);
        let (w, h) = (fw * scale.0 / scale.1, 2 * b.radius.max(1));
        Some(Entity {
            kind: crate::entity::EXPLOSION,
            x: b.center.0 - w / 2,
            y: b.center.1 - h / 2,
            flags: 0x99,
            layer: b.layer,
            id: 0,
            values: vec![],
            triggers: vec![],
            objective: 0,
            health: 0,
            armor: 0,
            boxes: vec![],
            solid_shapes: false,
            on: false,
            alive: true,
            sprite: Some(Sprite {
                layer: Layer::Object,
                anim,
                player,
                x: b.center.0 - w / 2,
                y: b.center.1 - h / 2,
                frozen: true,
                scale,
                mirror: false,
            }),
            behaviour: Behaviour::Explosion(behaviour::explosion::ExplosionState {
                applied: false,
                center: b.center,
            }),
        })
    }

    /// Mundo só com as caixas das células da camada do Jake.
    pub fn cell_world(&self) -> &World {
        &self.cell_world
    }

    /// Põe o Jake na camada `layer` (`+0x17c`): as caixas das células passam a ser as dessa
    /// camada e o mundo é refeito.
    pub fn set_jake_layer(&mut self, layer: i32) {
        self.jake_layer = layer;
        self.cell_solids = self
            .borders
            .iter()
            .filter(|(_, _, z, b)| *z == layer && b.kind == BOX)
            .map(|(cx, cy, _, b)| Rect {
                x: cx.saturating_add(i32::from(b.x)),
                y: cy.saturating_add(i32::from(b.y)),
                w: i32::from(b.width),
                h: i32::from(b.height),
            })
            .collect();
        let doors = self.door_solids(layer);
        self.cell_solids.extend(doors);
        self.cell_world = World::new(self.cell_solids.clone());
        self.rebuild_world();
    }

    /// Especial BAIXO+ATIRAR (`0x00427643`): objetos vivos da camada a menos de 70 px do
    /// centro do Jake. Projéteis somem; os demais levam o dano da variante 9 (15 por tique).
    fn apply_special(&mut self, b: SpecialBlast) {
        for e in self.entities.iter_mut().filter(|e| {
            e.alive && e.layer == b.layer && e.flags & 0x80003 == 0x80001 && e.present()
        }) {
            if !b.reaches(e.center()) {
                continue;
            }
            match SpecialBlast::effect(e.kind) {
                crate::jake::SpecialEffect::Destroy => e.alive = false,
                crate::jake::SpecialEffect::Knockdown => {}
                crate::jake::SpecialEffect::Damage => {
                    // pelo mesmo caminho das balas: mina (vida), barril (natureza 0: nada) e
                    // olho-laser (qualquer dano mata); a cerca não trata dano (como antes)
                    let before = e.health;
                    let fresh = matches!(&e.behaviour,
                        Behaviour::LaserEye(s) if s.hurt.is_empty());
                    let eye = matches!(e.behaviour, Behaviour::LaserEye(_));
                    let dmg = b.damage(e.center());
                    // os inimigos da M7a creditam o jogador na própria morte
                    let hit =
                        behaviour::enemy::hurt(e, &dmg, true) || behaviour::interact::hurt(e, &dmg);
                    let mine_died =
                        matches!(e.behaviour, Behaviour::Mine(_)) && before > 0 && e.health <= 0;
                    if mine_died || (hit && eye && fresh) {
                        self.kills += 1;
                    }
                }
            }
        }
    }

    /// Refaz o mundo do Jake: caixas das células mais as dos objetos sólidos vivos da camada
    /// dele (formas relativas à posição atual do objeto).
    pub fn rebuild_world(&mut self) {
        let mut solids = self.cell_solids.clone();
        let n = solids.len();
        if self.jake.is_some() {
            for e in self.entities.iter().filter(|e| e.layer == self.jake_layer) {
                if !blocks_entity(self.jake_flags, e) {
                    continue;
                }
                solids.extend(e.boxes.iter().map(|b| Rect {
                    x: e.x.saturating_add(b.x),
                    y: e.y.saturating_add(b.y),
                    ..*b
                }));
            }
        }
        self.object_solids = solids.split_off(n);
        self.world = World::with_objects(solids, self.object_solids.clone());
    }

    /// Tique das entidades: estado dos gatilhos congelado antes do tique; cada entidade age
    /// em ordem de lista; depois as saídas (dano ao Jake, novas entidades, eventos).
    fn tick_entities(&mut self) {
        behaviour::mine_generator::sync(&mut self.entities);
        let snapshot: Vec<(i32, bool)> = self
            .entities
            .iter()
            .filter(|e| e.alive && e.id != 0)
            .map(|e| (e.id, e.on))
            .collect();
        let on = |id: i32| snapshot.iter().find(|(i, _)| *i == id).map(|(_, o)| *o);
        let mut out = Out::default();
        let mut rng = self.rng.clone();
        {
            let mut ctx = Ctx {
                tick: self.tick_count,
                jake: self.jake.as_ref(),
                jake_layer: self.jake_layer,
                world: &self.world,
                cells: &self.cell_world,
                on: &on,
                rng: &mut rng,
                hints: &self.hints,
                level: (self.width, self.height),
            };
            for e in self.entities.iter_mut().filter(|e| e.alive) {
                match e.behaviour {
                    Behaviour::Zone(_) => behaviour::zone::tick(e, &ctx, &mut out),
                    Behaviour::Hint(_) => behaviour::hint::tick(e, &ctx, &mut out),
                    Behaviour::Pickup(_) => behaviour::pickup::tick(e, &ctx, &mut out),
                    Behaviour::LaserEye(_) => behaviour::laser_eye::tick(e, &mut ctx, &mut out),
                    Behaviour::Laser(_) => behaviour::laser::tick(e, &ctx, &mut out),
                    Behaviour::Barrel(_) => behaviour::barrel::tick(e, &ctx, &mut out),
                    Behaviour::Fire(_) => behaviour::fire::tick(e, &ctx, &mut out),
                    Behaviour::Fence(_) => behaviour::fence::tick(e, &mut ctx, &mut out),
                    Behaviour::Graffiti(_) => behaviour::graffiti::tick(e, &ctx, &mut out),
                    Behaviour::CarPiece => behaviour::car_piece::tick(e, &ctx, &mut out),
                    Behaviour::Teleporter(_) => behaviour::teleporter::tick(e, &ctx, &mut out),
                    Behaviour::Door(_) => behaviour::door::tick(e, &ctx, &mut out),
                    Behaviour::Mine(_) => behaviour::mine::tick(e, &mut ctx, &mut out),
                    Behaviour::Explosion(_) => behaviour::explosion::tick(e, &mut out),
                    Behaviour::Exit(_) => behaviour::exit::tick(e),
                    Behaviour::Bullet(_) => behaviour::bullet::tick(e, &ctx, &mut out),
                    Behaviour::Casing(_) => behaviour::casing::tick(e, &mut ctx, &mut out),
                    Behaviour::Pellet(_) => behaviour::pellet::tick(e, &ctx, &mut out),
                    Behaviour::Rocket(_) => behaviour::rocket::tick(e, &ctx, &mut out),
                    Behaviour::Star(_) => behaviour::star::tick(e, &ctx, &mut out),
                    Behaviour::Grenade(_) => behaviour::grenade::tick(e, &ctx, &mut out),
                    Behaviour::Piston(_) => behaviour::piston::tick(e, &ctx, &mut out),
                    Behaviour::Switch(_) => behaviour::switch::tick(e),
                    Behaviour::Sentinel(_) => behaviour::sentinel::tick(e, &mut ctx, &mut out),
                    Behaviour::HopMine(_) => behaviour::hop_mine::tick(e, &mut ctx, &mut out),
                    Behaviour::MineGenerator(_) => {
                        behaviour::mine_generator::tick(e, &mut ctx, &mut out)
                    }
                    Behaviour::SecurityRobot(_) => {
                        behaviour::security_robot::tick(e, &mut ctx, &mut out)
                    }
                    Behaviour::Bruiser(_) => behaviour::bruiser::tick(e, &mut ctx, &mut out),
                    Behaviour::EnemyShot(_) => behaviour::enemy_shot::tick(e, &mut ctx, &mut out),
                    _ => {}
                }
            }
        }
        self.rng = rng;
        self.bullet_hits(&mut out);
        self.weapon_hits(&mut out);
        self.resolve_impacts(&mut out);
        self.object_interactions(&mut out);
        // Explosões: dano de área ao Jake e às minas, e o efeito visual (`OEX.ANM`).
        for b in &out.blasts {
            if let Some(j) = self.jake.as_mut().filter(|j| j.is_active()) {
                let jb = j.body.bounds();
                let jc = (jb.x + jb.w / 2, jb.y + jb.h / 2);
                if let Some(p) = behaviour::explosion::area_effect(b, jc, self.jake_layer) {
                    out.jake_damage.add(&p);
                }
            }
            for e in self.entities.iter_mut().filter(|e| e.alive) {
                behaviour::interact::blast(b, e);
            }
            if let Some(e) = self.explosion_entity(b) {
                if let Some(c) = behaviour::explosion::frame_cue(&e) {
                    out.events.push(Event::Sound(c));
                }
                out.spawn.push(e);
            }
        }
        if let Some(j) = self.jake.as_mut() {
            if let Some(h) = out.heal {
                j.health = h;
            }
            if let Some(p) = out.poison {
                j.poison = p;
            }
            if let Some(inv) = out.inventory.take() {
                j.inventory = inv;
            }
            if !out.jake_damage.is_empty() {
                j.hurt(out.jake_damage);
            }
        }
        self.kills += out.kills;
        self.apply_quake(out.quake);
        self.spawn_enemy_objects(&mut out);
        self.entities.extend(out.spawn);
        let events = self.throttle_sounds(out.events);
        self.events.extend(events);
        self.rebuild_world();
    }

    /// Terremoto do Bruiser: enquanto durar, o Jake no chão cambaleia (choque sem dano,
    /// estado 15).
    fn apply_quake(&mut self, quake: Option<u32>) {
        if let Some(q) = quake {
            self.quake = self.quake.max(q);
        }
        if self.quake == 0 {
            return;
        }
        self.quake -= 1;
        if let Some(j) = self
            .jake
            .as_mut()
            .filter(|j| j.is_active() && j.body.on_ground)
        {
            j.hurt(crate::damage::Pending {
                electric: true,
                ..Default::default()
            });
        }
    }

    /// Sons com intervalo mínimo global entre todos os objetos do tipo (o zumbido da
    /// sentinela, 200 ms em `0x488820`; o "security hovering", 250 ms em `0x488834`).
    fn throttle_sounds(&mut self, events: Vec<Event>) -> Vec<Event> {
        let tick = self.tick_count;
        events
            .into_iter()
            .filter(|ev| {
                let Some(id) = ev.sound_id() else {
                    return true;
                };
                let Some(gap) = behaviour::enemy::throttle(id) else {
                    return true;
                };
                match self.sound_clock.get(&id) {
                    Some(&t) if tick.saturating_sub(t) < gap => false,
                    _ => {
                        self.sound_clock.insert(id, tick);
                        true
                    }
                }
            })
            .collect()
    }

    /// Cria os objetos que os inimigos pediram (projéteis, minas do gerador, fogo do
    /// Bruiser), com as animações e BRDs carregados na montagem.
    fn spawn_enemy_objects(&mut self, out: &mut Out) {
        for sp in std::mem::take(&mut out.enemy_spawns) {
            let e = match sp {
                behaviour::enemy::Spawn::Shot {
                    kind,
                    at,
                    vel,
                    layer,
                } => {
                    let mut e = behaviour::enemy_shot::entity(kind, at, vel, layer);
                    let (anim, brd, target) = behaviour::enemy_shot::art(kind);
                    let (label, mirror) = behaviour::bullet::label_and_mirror(vel.0, vel.1);
                    // o foguete tem os rótulos por direção; as balas, um só
                    let label = if kind == behaviour::enemy_shot::ROCKET {
                        label
                    } else {
                        1
                    };
                    let brd = if kind == behaviour::enemy_shot::ROCKET && label == 2 {
                        "ORO01.BRD"
                    } else {
                        brd
                    };
                    self.attach_sprite(&mut e, anim, label, mirror & 1 != 0, target, Some(brd));
                    e
                }
                behaviour::enemy::Spawn::Mine {
                    at,
                    variant,
                    layer,
                    owner,
                } => {
                    let mut e = behaviour::mine_generator::mine(at, variant, layer, owner);
                    self.attach_sprite(&mut e, "OHM00.ANM", 1, false, 25, Some("OHM00.BRD"));
                    e
                }
                behaviour::enemy::Spawn::Fire {
                    base,
                    height,
                    layer,
                } => {
                    let fh = self
                        .assets_anim("OFI.ANM")
                        .and_then(|a| a.frames.first().map(|f| i32::from(f.height)))
                        .unwrap_or(1);
                    let mut e = behaviour::bruiser::fire(base, height, layer, fh);
                    if let Some(anim) = self.assets_anim("OFI.ANM") {
                        let player = AnimPlayer::from_frame(&anim, 0);
                        e.sprite = Some(Sprite {
                            layer: Layer::Object,
                            anim,
                            player,
                            x: e.x,
                            y: e.y,
                            frozen: false,
                            scale: (height, fh.max(1)),
                            mirror: false,
                        });
                    }
                    e
                }
            };
            out.spawn.push(e);
        }
    }

    /// Põe o sprite (rótulo, espelho, tamanho-alvo `+0x1f0` sobre a medida do quadro) e as
    /// caixas do BRD num objeto criado pela cena; sem a animação, fica sem sprite.
    fn attach_sprite(
        &mut self,
        e: &mut Entity,
        anim: &str,
        label: u16,
        mirror: bool,
        target: i32,
        brd: Option<&str>,
    ) {
        if let Some(b) = brd.and_then(|n| self.assets_border(n)) {
            let boxes = crate::entity::border_boxes(&b);
            if !boxes.is_empty() {
                e.boxes = boxes;
            }
        }
        let Some(anim) = self.assets_anim(anim) else {
            return;
        };
        let player = AnimPlayer::at_label(&anim, label)
            .or_else(|| AnimPlayer::at_label(&anim, 1))
            .unwrap_or_else(|| AnimPlayer::from_frame(&anim, 0));
        let Some(f) = anim.frames.get(player.frame()) else {
            return;
        };
        let (fw, fh) = (i32::from(f.width).max(1), i32::from(f.height).max(1));
        let scale = (target, fw.max(fh));
        e.sprite = Some(Sprite {
            layer: Layer::Object,
            anim,
            player,
            x: e.x,
            y: e.y,
            frozen: false,
            scale,
            mirror,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::State;
    use openhunted_formats::test_support::{mkanm, mkarchive, mkbmp};
    use openhunted_formats::{Cell, Level, LevelObject, ResourceArchive};

    pub(crate) fn object(kind: u32, x: i32, y: i32, anim: &str) -> LevelObject {
        LevelObject {
            kind,
            x,
            y,
            fields: vec![],
            start_label: None,
            modes: 0xfe,
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

    /// Fase sem células com o Jake em (50, 0) e uma peça sólida (0x80089, `PL.BRD`: caixa de
    /// 60 × 10) em (40, 60) na camada `layer`.
    fn platform_level(layer: i32, flags: i32) -> (Level, Assets) {
        let mut jake = object(JAKE, 50, 0, "OAN.ANM");
        jake.fields = vec![0, 0, 0, 0x94801, 0];
        jake.fields.resize(21, 0);
        jake.fields[14] = 50; // vivo: senão o veredito dá derrota em 40 tiques
        let mut piece = object(56, 40, 60, "OAN.ANM");
        piece.fields = vec![0, 0, 0, flags, layer];
        piece.names[1] = "PL.BRD".into();
        let mut l = level();
        l.grid_y = 8; // fase alta: o Jake que cai não sai dela (voltaria ao início)
        l.cells = vec![None; 16];
        l.objects = vec![jake, piece];
        let bmp = mkbmp(&[(20, 20, vec![1; 400])]);
        let anm = mkanm();
        let brd = openhunted_formats::test_support::mkbrd(&[(2, 0, 0, 60, 10)]);
        let (map, vols) = mkarchive(&[(
            "RESOURCE.001",
            &[
                ("OAN00.BMP", bmp.as_slice()),
                ("OAN.ANM", anm.as_slice()),
                ("OBU.ANM", anm.as_slice()),
                ("OBS00.ANM", anm.as_slice()),
                ("OEX.ANM", anm.as_slice()),
                ("PL.BRD", brd.as_slice()),
            ],
        )]);
        let a = Assets::new(ResourceArchive::from_map(&map, |_| Ok(vols[0].1.clone())).unwrap());
        (l, a)
    }

    /// O `JAKE.SKE` precisa ter id 1 (o do Garathe é 2): senão o Jake fica sem esqueleto e
    /// a cena avisa.
    #[test]
    fn jake_skeleton_must_have_id_1() {
        let (l, _) = platform_level(0, 0x80089);
        for (id, warned) in [(1, false), (2, true)] {
            let ske = openhunted_formats::test_support::mkske(id, (20, 57), &[]);
            let bmp = mkbmp(&[(20, 20, vec![1; 400])]);
            let anm = mkanm();
            let brd = openhunted_formats::test_support::mkbrd(&[(2, 0, 0, 60, 10)]);
            let (map, vols) = mkarchive(&[(
                "RESOURCE.001",
                &[
                    ("OAN00.BMP", bmp.as_slice()),
                    ("OAN.ANM", anm.as_slice()),
                    ("PL.BRD", brd.as_slice()),
                    ("JAKE.SKE", ske.as_slice()),
                ],
            )]);
            let mut a =
                Assets::new(ResourceArchive::from_map(&map, |_| Ok(vols[0].1.clone())).unwrap());
            let s = Scene::build(&l, &mut a).unwrap();
            let w = s.warnings.iter().any(|w| w.starts_with("JAKE.SKE"));
            assert_eq!(w, warned, "id {id}: {:?}", s.warnings);
        }
    }

    fn settle(l: &Level, mut a: Assets) -> crate::Body {
        let mut s = Scene::build(l, &mut a).unwrap();
        for _ in 0..20 {
            s.tick(0);
        }
        s.jake.unwrap().body
    }

    #[test]
    fn solid_object_holds_jake_on_his_layer_only() {
        let (l, a) = platform_level(0, 0x80089);
        let b = settle(&l, a);
        assert!(b.on_ground);
        assert_eq!(b.y, 60 - 47); // pés (caixa de reserva: y 3 + 44) no topo da peça
        let (l, mut a) = platform_level(0, 0x80089);
        let sc = Scene::build(&l, &mut a).unwrap();
        assert_eq!(
            sc.object_solids,
            vec![Rect {
                x: 40,
                y: 60,
                w: 60,
                h: 10
            }]
        );
        let (l, a) = platform_level(1, 0x80089);
        assert!(settle(&l, a).y > 60, "outra camada não sustenta");
        let (l, a) = platform_level(0, 0x800a9);
        assert!(
            settle(&l, a).y > 60,
            "objeto de contato (0x20) não sustenta"
        );
    }

    /// Cronômetro (`0x4972fc`): conta para baixo; esgotado numa fase com limite, derrota.
    #[test]
    fn timer_counts_down_and_running_out_loses() {
        let mut s = Scene::empty(640, 480);
        s.time_limit = 2;
        s.timer = 2 * TIMER_TICKS_PER_SECOND;
        for _ in 0..39 {
            s.tick(0);
        }
        assert_eq!(s.timer, 1);
        assert!(!s.timed_out());
        s.tick(0);
        assert!(s.timed_out());
        // o veredito ainda precisa de 40 tiques seguidos
        for _ in 0..38 {
            s.tick(0);
        }
        assert_eq!(s.outcome, None);
        s.tick(0);
        assert_eq!(s.outcome, Some(Outcome::Defeat));
        let mut free = Scene::empty(640, 480);
        for _ in 0..100 {
            free.tick(0);
        }
        assert_eq!(
            (free.timer, free.outcome),
            (0, None),
            "sem limite: 0000 e nada"
        );
    }

    /// Duas minas a 20 px uma da outra e o Jake encostado na primeira: a primeira explode no
    /// contato, a explosão fere o Jake e mata a segunda em cadeia.
    #[test]
    fn mine_contact_hurts_jake_and_chains() {
        use crate::behaviour::mine::MineState;
        let mut jake = object(JAKE, 50, 0, "OAN.ANM");
        jake.fields = vec![0, 0, 0, 0x94801, 0];
        jake.fields.resize(16, 0);
        jake.fields[14] = 50;
        let mine = |x: i32| {
            let mut m = object(crate::entity::MINE, x, 10, "OAN.ANM");
            m.fields = vec![0, 0, 0, 0x80801, 0];
            m.fields.resize(16, 0);
            m.fields[14] = 1;
            m.names[1] = "PL.BRD".into();
            m
        };
        let (mut l, mut a) = platform_level(0, 0x80089);
        l.objects = vec![jake, mine(60), mine(80)];
        let mut s = Scene::build(&l, &mut a).unwrap();
        for e in s.entities.iter_mut() {
            e.behaviour = Behaviour::Mine(MineState::new(0));
            e.boxes = vec![Rect {
                x: 0,
                y: 0,
                w: 17,
                h: 17,
            }];
        }
        s.rebuild_world();
        s.tick(0);
        s.tick(0);
        let dead: Vec<_> = s
            .entities
            .iter()
            .filter(|e| e.kind == 102)
            .map(|e| e.alive)
            .collect();
        assert_eq!(
            dead,
            vec![false, false],
            "as duas minas explodem (em cadeia)"
        );
        assert_eq!(s.kills, 0, "explosão no contato não conta");
        let j = s.jake.as_ref().unwrap();
        assert!(j.health.hp < 50 && j.health.hp > 0, "vida {}", j.health.hp);
    }

    /// Especial BAIXO+ATIRAR: a mina a menos de 70 px do centro do Jake morre (conta como
    /// abatida), a de longe não; o Jake perde 3 de vida por tique do efeito (5 tiques).
    #[test]
    fn down_shoot_special_kills_near_mines() {
        use crate::behaviour::mine::MineState;
        use crate::input::{DOWN, SHOOT};
        let (mut l, mut a) = platform_level(0, 0x80089);
        let mine = |x: i32| {
            let mut m = object(crate::entity::MINE, x, 10, "OAN.ANM");
            m.fields = vec![0, 0, 0, 0x80801, 0];
            m.fields.resize(16, 0);
            m.fields[14] = 1;
            m
        };
        l.objects.push(mine(100));
        l.objects.push(mine(900));
        let mut s = Scene::build(&l, &mut a).unwrap();
        for _ in 0..30 {
            s.tick(0);
        }
        let hp = s.jake.as_ref().unwrap().health.hp;
        let jc = s.focus().unwrap();
        for e in s
            .entities
            .iter_mut()
            .filter(|e| e.kind == crate::entity::MINE)
        {
            e.behaviour = Behaviour::Mine(MineState::new(0));
            e.y = jc.1;
        }
        s.entities[1].x = jc.0 + 40;
        s.entities[1].health = 20;
        // linha (1, qualquer, 9) do `DAMAGE.TBL`: 15 por tique do efeito
        s.tick(DOWN | SHOOT);
        assert_eq!(s.entities[1].health, 5);
        s.tick(DOWN | SHOOT);
        let alive: Vec<bool> = s
            .entities
            .iter()
            .filter(|e| e.kind == crate::entity::MINE)
            .map(|e| e.alive && e.health > 0)
            .collect();
        assert_eq!(alive, vec![false, true]);
        assert_eq!(s.kills, 1);
        for _ in 0..5 {
            s.tick(0);
        }
        assert_eq!(
            s.jake.as_ref().unwrap().health.hp,
            hp - 15,
            "3 por tique, 5 tiques"
        );
    }

    /// A camada da cena acompanha a do Jake: os sólidos das células passam a ser os da
    /// camada nova.
    #[test]
    fn jake_layer_change_switches_cell_solids() {
        let mut s = Scene::empty(256, 128);
        s.borders = vec![
            (
                0,
                0,
                0,
                BorderBox {
                    kind: BOX,
                    x: 0,
                    y: 100,
                    width: 128,
                    height: 10,
                },
            ),
            (
                0,
                0,
                1,
                BorderBox {
                    kind: BOX,
                    x: 0,
                    y: 50,
                    width: 128,
                    height: 10,
                },
            ),
            (
                128,
                0,
                1,
                BorderBox {
                    kind: BOX,
                    x: 0,
                    y: 50,
                    width: 128,
                    height: 10,
                },
            ),
        ];
        s.set_jake_layer(0);
        assert_eq!(s.cell_world().solids().len(), 1);
        s.set_jake_layer(1);
        assert_eq!((s.jake_layer, s.cell_world().solids().len()), (1, 2));
        assert_eq!(s.world.solids()[0].y, 50);
    }

    /// Fase sintética com o Jake, uma saída (OAN.ANM como animação, rótulo 1) à esquerda
    /// dele e uma zona PR 0x40 diante dela.
    fn exit_level(zone: bool) -> (Level, Assets) {
        let mut jake = object(JAKE, 50, 0, "OAN.ANM");
        jake.fields = vec![0, 0, 0, 0x94801, 0];
        jake.fields.resize(16, 0);
        jake.fields[14] = 50;
        let mut exit = object(crate::entity::EXIT, 40, 10, "OAN.ANM");
        exit.fields = vec![0, 0, 0, 0x80029, 0];
        exit.fields.resize(21, 0); // os dois últimos campos são o tamanho (0 = escala 1)
        exit.fields[16] = 2;
        let (mut l, a) = platform_level(0, 0x80089);
        l.objects[0] = jake; // mantém a peça que sustenta o Jake
        l.objects.push(exit);
        if zone {
            let cell = l.cells[0].get_or_insert_with(|| Cell {
                version: 1010,
                animation: String::new(),
                border: String::new(),
                index: 0,
                mirrored: false,
                fields: vec![],
                triggers: vec![],
                objects: vec![],
                props: vec![],
                decorations: vec![],
            });
            cell.props.push(openhunted_formats::Prop {
                kind: 0x40,
                rect: [30, 0, 60, 60],
            });
        }
        (l, a)
    }

    fn settle_scene(l: &Level, mut a: Assets) -> Scene {
        let mut s = Scene::build(l, &mut a).unwrap();
        for _ in 0..30 {
            s.tick(0);
        }
        s
    }

    #[test]
    fn up_at_the_exit_wins_after_forty_ticks() {
        use crate::input::UP;
        let (l, a) = exit_level(true);
        let mut s = settle_scene(&l, a);
        let j = s.jake.as_ref().unwrap();
        assert!(
            j.can_use_exit(),
            "{:?} y={} chão={}",
            j.state(),
            j.body.y,
            j.body.on_ground
        );
        s.tick(UP);
        let exit = s.entities.iter().find(|e| e.kind == 5).unwrap();
        assert!(exit.on);
        assert_eq!(s.jake.as_ref().unwrap().state(), State::EnteringExit(0));
        // o tique do CIMA já conta: vitória no 40º tique com a saída ligada
        for t in 1..39 {
            s.tick(0);
            assert_eq!(s.outcome, None, "tique {t}");
        }
        s.tick(0);
        assert_eq!(s.outcome, Some(Outcome::Victory));
        assert!(s.events.contains(&Event::Outcome(Outcome::Victory)));
        // sem a zona PR, CIMA não usa a saída: o Jake se esconde (164, M6b)
        let (l, a) = exit_level(false);
        let mut s = settle_scene(&l, a);
        s.tick(UP);
        assert!(!s.entities.iter().find(|e| e.kind == 5).unwrap().on);
        assert_eq!(s.jake.as_ref().unwrap().state(), State::Hide);
    }

    #[test]
    fn exit_refused_without_objectives_zaps_and_death_defeats() {
        use crate::input::UP;
        let (mut l, a) = exit_level(true);
        // um alvo a destruir, vivo
        let mut target = object(9, 400, 0, "OAN.ANM");
        target.fields = vec![0, 0, 0, 0x80001, 0];
        target.fields.resize(21, 0);
        target.fields[14] = 5;
        target.fields[16] = 4;
        l.objects.push(target);
        let mut s = settle_scene(&l, a);
        s.tick(UP);
        s.tick(0);
        let j = s.jake.as_ref().unwrap();
        assert_eq!(j.health.hp, 30);
        assert_eq!(j.state(), State::Shocked(9));
        assert!(s.events.iter().any(|e| e.sound_id() == Some(3772)));
        // morte → derrota 40 tiques depois (mesmo que a saída ligue no meio)
        if let Some(j) = s.jake.as_mut() {
            j.hurt(crate::damage::Pending {
                amount: 99,
                ..Default::default()
            });
        }
        s.tick(0);
        assert!(s.jake.as_ref().unwrap().is_dead());
        s.entities.iter_mut().find(|e| e.kind == 5).unwrap().on = true;
        for _ in 0..39 {
            s.tick(0);
        }
        assert_eq!(s.outcome, Some(Outcome::Defeat));
    }

    /// Jake com pistola na peça; atira para a direita contra uma parede a 100 px e, depois,
    /// contra uma mina na linha de tiro.
    #[test]
    fn bullets_fly_spark_on_walls_and_kill_mines() {
        use crate::behaviour::mine::MineState;
        use crate::input::SHOOT;
        let (mut l, mut a) = platform_level(0, 0x80089);
        l.grid_x = 4;
        l.cells = vec![None; 32];
        l.objects[0].points = vec![(0, 0), (1, 20), (1, 80)];
        l.objects[0].fields[4] = 0;
        // parede a 100 px à direita do Jake (como objeto sólido com BRD)
        let mut wall = object(56, 160, 0, "OAN.ANM");
        wall.fields = vec![0, 0, 0, 0x80089, 0];
        wall.names[1] = "PL.BRD".into(); // caixa 60×10 em (0,0): fica em y 0..10
        l.objects.push(wall);
        let mut s = Scene::build(&l, &mut a).unwrap();
        for _ in 0..30 {
            s.tick(0);
        }
        s.tick(crate::input::RIGHT);
        s.tick(SHOOT);
        s.tick(SHOOT); // t = 1: 1ª bala + cápsula
        let bullets = |s: &Scene| {
            s.entities
                .iter()
                .filter(|e| e.alive && e.kind == crate::entity::BULLET)
                .map(|e| (e.x, e.y))
                .collect::<Vec<_>>()
        };
        let b0 = bullets(&s);
        assert_eq!(b0.len(), 1);
        assert!(
            s.entities
                .iter()
                .any(|e| e.alive && e.kind == crate::entity::CASING)
        );
        s.tick(0);
        let b1 = bullets(&s);
        assert_eq!(b1[0].0 - b0[0].0, 20, "20 px por tique");
        // some ao sair da fase (sem parede na altura do tiro, y ≈ 15) e nenhuma faísca
        let mut trace = vec![];
        for _ in 0..40 {
            s.tick(0);
            trace.push(bullets(&s));
        }
        assert!(bullets(&s).is_empty(), "{:?} {:?}", trace.last(), s.outcome);
        // mina à direita, na altura, fora do alcance dela (fica parada): morre com a bala,
        // e a faísca aparece
        let mut mine = Entity {
            kind: crate::entity::MINE,
            x: 300,
            y: 10,
            flags: 0x80801,
            layer: 0,
            id: 0,
            values: vec![],
            triggers: vec![],
            objective: 0,
            health: 1,
            armor: 0,
            boxes: vec![Rect {
                x: 0,
                y: 0,
                w: 17,
                h: 17,
            }],
            solid_shapes: true,
            on: false,
            alive: true,
            sprite: None,
            behaviour: Behaviour::Mine(MineState::new(0)),
        };
        mine.values = vec![0];
        let mine2 = mine.clone();
        s.entities.push(mine);
        s.tick(SHOOT);
        for _ in 0..20 {
            s.tick(0);
        }
        assert!(
            !s.entities
                .iter()
                .any(|e| e.alive && e.kind == crate::entity::MINE)
        );
        assert_eq!(s.kills, 1, "mina morta pela bala conta para o jogador");
        // bala que não é do jogador (inimigos atirando, no futuro): mata, mas não conta
        s.entities.push(mine2);
        let not_mine = |s: &mut Scene| {
            for e in s.entities.iter_mut() {
                if let Behaviour::Bullet(b) = &mut e.behaviour {
                    b.player_shot = false;
                }
            }
        };
        for _ in 0..30 {
            s.tick(SHOOT);
            not_mine(&mut s);
        }
        for _ in 0..20 {
            s.tick(0);
        }
        assert!(
            !s.entities
                .iter()
                .any(|e| e.alive && e.kind == crate::entity::MINE)
        );
        assert_eq!(s.kills, 1);
        // (a animação sintética só tem o rótulo 1: a faísca `oex15` fica para o teste real)
        assert!(
            s.events
                .iter()
                .any(|e| e.sound_id() == Some(crate::behaviour::bullet::SOUND_WALL))
        );
        let pistol = s.events.iter().find_map(|e| match e {
            Event::Sound(c) if c.id == SOUND_PISTOL => Some(*c),
            _ => None,
        });
        let at = pistol.and_then(|c| c.at).expect("tiro posicional");
        assert_eq!((at.layer, pistol.unwrap().priority), (s.jake_layer, 9));
    }

    /// Tiro na diagonal para baixo: a bala toca a peça em que o Jake está e faísca (não
    /// desliza pelo chão); uma zona invisível no caminho não a para; uma mina colada no cano
    /// morre no primeiro tique.
    #[test]
    fn bullets_spark_on_floors_ignore_zones_and_hit_point_blank() {
        use crate::behaviour::mine::MineState;
        use crate::input::{DOWN, RIGHT, SHOOT};
        let (mut l, mut a) = platform_level(0, 0x80089);
        l.grid_x = 4;
        l.cells = vec![None; 32];
        l.objects[0].points = vec![(0, 0), (1, 20), (1, 80)];
        // chão largo de células: uma caixa em y = 100 (abaixo da peça)
        let mut s = Scene::build(&l, &mut a).unwrap();
        s.cell_solids.push(Rect {
            x: -1000,
            y: 100,
            w: 3000,
            h: 20,
        });
        s.cell_world = World::new(s.cell_solids.clone());
        s.rebuild_world();
        for _ in 0..30 {
            s.tick(0);
        }
        s.tick(RIGHT);
        s.tick(DOWN | RIGHT | SHOOT);
        let mut sparks = 0;
        for _ in 0..12 {
            s.tick(DOWN | RIGHT);
            sparks += s
                .events
                .drain(..)
                .filter(|e| e.sound_id() == Some(crate::behaviour::bullet::SOUND_WALL))
                .count();
        }
        assert_eq!(
            sparks, 3,
            "as três balas da diagonal tocam o chão e faíscam"
        );
        assert!(
            !s.entities
                .iter()
                .any(|e| e.alive && e.kind == crate::entity::BULLET)
        );
        // zona invisível na linha de tiro: a bala passa
        let mut zone = crate::behaviour::test_support::zone(120, 0, 64, 64, &[64, 64, 0, 0, 0, 0]);
        zone.layer = 0;
        s.entities.push(zone);
        for _ in 0..30 {
            s.tick(0);
        }
        s.tick(SHOOT);
        s.tick(SHOOT);
        for _ in 0..4 {
            s.tick(0);
        }
        let flying = s
            .entities
            .iter()
            .filter(|e| e.alive && e.kind == crate::entity::BULLET)
            .count();
        assert!(flying >= 2, "balas paradas pela zona: só {flying} voando");
        // mina colada no cano (a 10 px): morre no mesmo tique do disparo
        for _ in 0..60 {
            s.tick(0);
        }
        let (jx, jy) = (
            s.jake.as_ref().unwrap().body.x,
            s.jake.as_ref().unwrap().body.y,
        );
        let mut mine =
            crate::behaviour::test_support::logic(crate::entity::MINE, jx + 30, jy - 5, &[0]);
        mine.flags = 0x80801;
        mine.boxes = vec![Rect {
            x: 0,
            y: 0,
            w: 17,
            h: 17,
        }];
        mine.solid_shapes = true;
        mine.health = 1;
        mine.behaviour = Behaviour::Mine(MineState::new(0));
        s.entities.push(mine);
        s.tick(SHOOT);
        s.tick(SHOOT);
        assert!(
            !s.entities
                .iter()
                .any(|e| e.alive && e.kind == crate::entity::MINE),
            "mina à queima-roupa sobreviveu"
        );
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
        let objs: Vec<_> = s.entities.iter().filter(|e| e.sprite.is_some()).collect();
        assert_eq!((objs.len(), s.entities.len()), (2, 3));
        assert!(objs.iter().any(|e| (e.x, e.y) == (200, 30)));
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

    /// Animação com dois rótulos: 68 (quadro 0, 78×10) e 272 (quadro 1, 18×64).
    fn two_label_anim() -> Animation {
        let frame = |width, height| openhunted_formats::AnimFrame {
            x: 0,
            y: 0,
            width,
            height,
            script_offset: 0,
            extra: 0,
        };
        let label = |frame, id| openhunted_formats::AnimLabel {
            frame,
            id,
            name: None,
        };
        Animation {
            version: 1004,
            width: 78,
            height: 64,
            extra: [0; 3],
            table_a: vec![],
            frames: vec![frame(78, 10), frame(18, 64)],
            labels: vec![label(0, 68), label(1, 272)],
            script: vec![],
            bitmaps: vec![],
            palette: None,
        }
    }

    #[test]
    fn start_label_picks_shown_and_reference_frame() {
        let a = two_label_anim();
        let mut o = object(72, 0, 0, "OPO.ANM");
        o.start_label = Some(272);
        let (ref_frame, player, frozen) = object_start(&o, &a);
        assert_eq!((ref_frame, player.frame(), frozen), (1, 1, false));
        // sem rótulo inicial (ou rótulo inexistente): quadro 0
        o.start_label = None;
        assert_eq!(object_start(&o, &a).0, 0);
        o.start_label = Some(999);
        assert_eq!(object_start(&o, &a).0, 0);
        // grafite (tipo 6): o OGF.ANM é uma fonte; o sprite fica parado no quadro 0 e o
        // desenho é letra a letra
        let mut item = object(crate::entity::GRAFFITI, 0, 0, "OGF.ANM");
        item.values = vec![1];
        let (ref_frame, player, frozen) = object_start(&item, &a);
        assert_eq!((ref_frame, player.frame(), frozen), (0, 0, true));
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
