// SPDX-License-Identifier: GPL-3.0-or-later
//! Cena de uma fase: blocos de cenário, decorações e objetos, com as animações em curso.

use std::rc::Rc;

use std::collections::HashMap;

use openhunted_formats::{Animation, Border, BorderBox, Level, LevelObject, Result};

use crate::behaviour::{self, Ctx, Event, Out, Outcome};
use crate::entity::{BOX, Behaviour, Entity, from_object, object_scale, object_start};
use crate::objects::{SOLO, flags, is_player_start, present};
use crate::physics::{Rect, World};
use crate::rng::Rng;
use crate::verdict::{self, Streak};
use crate::{AnimPlayer, Assets, Framebuffer, Jake, draw_ops};

/// Duração de um tick de animação, em milissegundos (provisório: `docs/re/cenario.md`).
pub const TICK_MS: u32 = 50;
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
    /// Zonas PR de tipo 0x40 (diante das saídas): (camada, retângulo em px da fase).
    exit_zones: Vec<(i32, Rect)>,
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
            object_solids: Vec::new(),
            entities: Vec::new(),
            jake_layer: 0,
            jake_flags: 0,
            cell_solids: Vec::new(),
            cell_world: World::default(),
            explosion_anim: None,
            exit_zones: Vec::new(),
            streak: Streak::default(),
            prev_input: 0,
            hints: Vec::new(),
            events: Vec::new(),
            outcome: None,
            tick_count: 0,
            rng: Rng::new(0x4a4b_4531),
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
            exit_zones: Vec::new(),
            streak: Streak::default(),
            prev_input: 0,
            hints: Vec::new(),
            events: Vec::new(),
            outcome: None,
            tick_count: 0,
            rng: Rng::new(0x4a4b_4531),
        };
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
            for pr in cell
                .props
                .iter()
                .filter(|p| p.kind == behaviour::exit::EXIT_PROP)
            {
                s.exit_zones.push((
                    cz,
                    Rect {
                        x: cx.saturating_add(pr.rect[0]),
                        y: cy.saturating_add(pr.rect[1]),
                        w: pr.rect[2],
                        h: pr.rect[3],
                    },
                ));
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
                        s.jake = Some(
                            Jake::new(o.x, o.y, boxr, label_lengths(&a))
                                .with_health(field(14), field(15))
                                .with_inventory(crate::jake::Inventory::from_points(&o.points)),
                        );
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
        s.jake_layer = jake_layer;
        s.jake_flags = jake_flags;
        s.cell_solids = s
            .borders
            .iter()
            .filter(|(_, _, z, b)| *z == jake_layer && b.kind == BOX)
            .map(|(cx, cy, _, b)| Rect {
                x: cx.saturating_add(i32::from(b.x)),
                y: cy.saturating_add(i32::from(b.y)),
                w: i32::from(b.width),
                h: i32::from(b.height),
            })
            .collect();
        s.cell_world = World::new(s.cell_solids.clone());
        s.rebuild_world();
        if let Ok(res) = assets.archive().get("WIZARD.RES") {
            s.hints = openhunted_formats::parse_text_list(res);
        }
        s.explosion_anim = assets.animation("OEX.ANM").ok();
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
            if let (true, Some(sp)) = (e.alive, &e.sprite) {
                draw_sprite(sp, assets, fb, camera);
            }
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
            for r in &self.object_solids {
                fb.rect(
                    r.x.saturating_sub(camera.0),
                    r.y.saturating_sub(camera.1),
                    r.w,
                    r.h,
                    OVERLAY_OBJECT,
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

    /// Um tique (50 ms): o Jake reage à entrada `input` (bits de `crate::input`), as
    /// entidades agem, e todas as animações avançam. Com a partida decidida, só anima.
    pub fn tick(&mut self, input: u32) {
        let input = if self.outcome.is_some() { 0 } else { input };
        if let (Some(j), Some(sp)) = (self.jake.as_mut(), self.jake_sprite.as_mut()) {
            j.tick(input, &self.world);
            j.respawn_if_outside((self.width, self.height));
            sp.x = j.body.x;
            sp.y = j.body.y;
        }
        if self.outcome.is_none() {
            let up_edge = input & crate::input::UP != 0 && self.prev_input & crate::input::UP == 0;
            if up_edge {
                self.try_exit();
            }
            self.tick_entities();
            let result = verdict::evaluate(&self.entities, self.jake.as_ref());
            if let Some(o) = self.streak.push(result) {
                self.outcome = Some(o);
                self.events.push(Event::Outcome(o));
            }
        }
        self.prev_input = input;
        if let (Some(j), Some(sp)) = (self.jake.as_mut(), self.jake_sprite.as_mut()) {
            sp.x = j.body.x;
            sp.y = j.body.y;
            // A arte do Jake olha para a esquerda: espelha quando ele olha para a direita.
            sp.mirror = j.facing_right();
            if let Some(num) = j.scale_override() {
                sp.scale.0 = num;
            }
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

    /// CIMA (`0x00423b8e`): o Jake parado sobre uma zona PR 0x40 e tocando a caixa de uma
    /// saída aciona-a (`0x0040e4dd`): objetivos cumpridos → a saída liga e ele entra na porta;
    /// senão, choque de 20 e os dois sons.
    fn try_exit(&mut self) {
        let Some(j) = self.jake.as_ref().filter(|j| j.can_use_exit()) else {
            return;
        };
        let jb = j.body.bounds();
        let in_zone = self
            .exit_zones
            .iter()
            .any(|(z, r)| *z == self.jake_layer && r.overlaps(&jb));
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
            self.events
                .push(Event::Sound(behaviour::exit::SOUND_OBJECTIVES));
            self.events.push(Event::Sound(behaviour::exit::SOUND_ZAP));
        }
    }

    /// Entidade visual de uma explosão: `OEX.ANM`, rótulo sorteado 1–4, com o sprite medindo
    /// `2 · raio` px de altura, centrado no ponto. Sem animação disponível, nenhuma.
    fn explosion_entity(&mut self, b: &behaviour::Blast) -> Option<Entity> {
        let anim = self.explosion_anim.clone()?;
        let label = 1 + self.rng.below(4) as u16;
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
            behaviour: Behaviour::Explosion(Default::default()),
        })
    }

    /// Mundo só com as caixas das células da camada do Jake.
    pub fn cell_world(&self) -> &World {
        &self.cell_world
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
        self.object_solids = solids[n..].to_vec();
        self.world = World::new(solids);
    }

    /// Tique das entidades: estado dos gatilhos congelado antes do tique; cada entidade age
    /// em ordem de lista; depois as saídas (dano ao Jake, novas entidades, eventos).
    fn tick_entities(&mut self) {
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
                    Behaviour::Pickup => behaviour::pickup::tick(e, &ctx, &mut out),
                    Behaviour::Mine(_) => behaviour::mine::tick(e, &mut ctx, &mut out),
                    Behaviour::Explosion(_) => behaviour::explosion::tick(e),
                    Behaviour::Exit(_) => behaviour::exit::tick(e),
                    _ => {}
                }
            }
        }
        self.rng = rng;
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
                if let Behaviour::Mine(_) = e.behaviour {
                    if let Some(p) = behaviour::explosion::area_effect(b, e.center(), e.layer) {
                        e.health -= p.amount.max(1);
                    }
                }
            }
            if let Some(e) = self.explosion_entity(b) {
                out.spawn.push(e);
            }
        }
        if let Some(j) = self.jake.as_mut() {
            if let Some(h) = out.heal {
                j.health = h;
            }
            if !out.jake_damage.is_empty() {
                j.hurt(out.jake_damage);
            }
        }
        self.entities.extend(out.spawn);
        self.events.extend(out.events);
        self.rebuild_world();
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
                ("PL.BRD", brd.as_slice()),
            ],
        )]);
        let a = Assets::new(ResourceArchive::from_map(&map, |_| Ok(vols[0].1.clone())).unwrap());
        (l, a)
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
        let j = s.jake.as_ref().unwrap();
        assert!(j.health.hp < 50 && j.health.hp > 0, "vida {}", j.health.hp);
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
        // sem a zona PR, CIMA não faz nada
        let (l, a) = exit_level(false);
        let mut s = settle_scene(&l, a);
        s.tick(UP);
        assert!(!s.entities.iter().find(|e| e.kind == 5).unwrap().on);
        assert_eq!(s.jake.as_ref().unwrap().state(), State::Idle);
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
        assert!(s.events.contains(&Event::Sound(3772)));
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
        // item de tipo 6: mostra values[0], mas a escala é a do quadro 0
        let mut item = object(crate::entity::GENERIC_ITEM, 0, 0, "OGF.ANM");
        item.values = vec![1];
        let (ref_frame, player, frozen) = object_start(&item, &a);
        assert_eq!((ref_frame, player.frame(), frozen), (0, 1, true));
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
