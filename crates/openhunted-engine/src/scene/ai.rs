// SPDX-License-Identifier: GPL-3.0-or-later
//! Personagens da IA na cena (M11, `docs/re/minotauro-ia.md`): Jakes e Garathes sem
//! jogador como entidades `Behaviour::AiPlayer`, com o corpo do jogador e o cérebro de
//! `crate::ai`; tiros com dono, golpes no jogador humano e linhas do `DAMAGE.TBL` por
//! personagem.

use std::rc::Rc;

use openhunted_formats::{Border, Level, LevelObject};

use super::{JAKE_BOX, Layer, Scene, Sprite, blocks_entity, border_bounds, label_lengths};
use crate::ai::{
    self, AiPlayer, Brain, CellGrid, CellInfo, HAZARD_RANGE, Me, Senses, Threat, View,
};
use crate::behaviour::{self, Event};
use crate::character::Character;
use crate::damage::{Geometry, Pending, Strike};
use crate::entity::{Behaviour, Entity, from_object, object_scale, object_start};
use crate::jake::{Env, JumpKind, Skel, State, garathe};
use crate::physics::{Rect, SUB, World};
use crate::sound::{Cue, PRIORITY_FRAME};
use crate::{AnimPlayer, Assets, Jake};

/// O objeto é um personagem da IA: tipo 1 ou 2 sem vaga de jogador (`modos & 1 = 0`;
/// `0x42ec1c`). A presença no modo já foi filtrada pela montagem.
pub fn is_ai(o: &LevelObject) -> bool {
    Character::from_kind(o.kind).is_some() && o.modes & 1 == 0
}

/// Monta o corpo de um personagem (jogador ou IA) a partir do objeto: ANM, BRDs, `.SKE` do
/// perfil, vida/armadura `fields[14]/[15]`, inventário pelos `points` e camada `fields[4]`.
pub(super) fn character(
    o: &LevelObject,
    assets: &mut Assets,
    warnings: &mut Vec<String>,
) -> Option<(Jake, Sprite)> {
    let ch = Character::from_kind(o.kind).unwrap_or_default();
    let prof = ch.profile();
    let a = match assets.animation(&o.names[0]) {
        Ok(a) => a,
        Err(e) => {
            warnings.push(format!("objeto tipo {} ({}): {e}", o.kind, o.names[0]));
            return None;
        }
    };
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
        .get(prof.ske)
        .and_then(openhunted_formats::Skeleton::parse)
    {
        Ok(k) if k.id == prof.ske_id => Some(Skel::new(Rc::new(k), (sc(rw), sc(rh)))),
        Ok(k) => {
            warnings.push(format!(
                "{}: id {} (esperado {})",
                prof.ske, k.id, prof.ske_id
            ));
            None
        }
        Err(e) => {
            warnings.push(format!("{}: {e}", prof.ske));
            None
        }
    };
    let brd = |assets: &mut Assets, name: &str, fallback: Rect| {
        assets
            .archive()
            .get(name)
            .and_then(Border::parse)
            .ok()
            .and_then(|b| border_bounds(&b))
            .unwrap_or(fallback)
    };
    let crawl = brd(assets, prof.crawl_brd, crate::jake::CRAWL_BOX);
    let bar = brd(assets, prof.bar_brd, crate::jake::BAR_BOX);
    let boxr = brd(assets, &o.names[1], JAKE_BOX);
    let layer = o.fields.get(4).copied().unwrap_or(0);
    let player = AnimPlayer::at_label(&a, 1).unwrap_or(player);
    let field = |i: usize| o.fields.get(i).copied().unwrap_or(0);
    let mut j = Jake::new(o.x, o.y, boxr, label_lengths(&a))
        .with_character(ch)
        .with_health(field(14), field(15))
        .with_inventory(crate::jake::Inventory::for_character(&o.points, ch))
        .with_layer(layer)
        .with_boxes(crawl, bar);
    if let Some(k) = skel {
        j = j.with_skeleton(k);
    }
    let sprite = Sprite {
        layer: Layer::Object,
        anim: a,
        player,
        x: o.x,
        y: o.y,
        frozen: false,
        scale,
        mirror: false,
    };
    Some((j, sprite))
}

/// Entidade de um personagem da IA: os campos do objeto (`from_object`: id, objetivo,
/// sinalizadores) com o corpo, o cérebro e o sprite do personagem.
pub(super) fn ai_entity(
    o: &LevelObject,
    assets: &mut Assets,
    warnings: &mut Vec<String>,
) -> Option<Entity> {
    let (body, sprite) = character(o, assets, warnings)?;
    let mut e = from_object(o, assets, warnings);
    e.sprite = Some(sprite);
    e.boxes = vec![body.body.boxr];
    e.solid_shapes = true;
    e.layer = body.layer();
    e.health = body.health.hp;
    e.armor = body.health.armor;
    e.behaviour = Behaviour::AiPlayer(Box::new(AiPlayer {
        body,
        brain: Brain::new(),
        body_checked: false,
        rng: crate::rng::Rng::new((o.x as u32).wrapping_mul(31).wrapping_add(o.y as u32) ^ 0x4149),
    }));
    Some(e)
}

/// Grade da IA: as células do `.LEV` (as vazias ficam ausentes), bits de `célula+0x04`
/// (`fields[0]`, células v ≥ 1006), índice, `+0x40` (espelhada) e as zonas PR.
pub(super) fn cell_grid(level: &Level, zones: &[crate::jake::Zone]) -> CellGrid {
    let size = (
        i32::from(level.grid_x).max(1),
        i32::from(level.grid_y).max(1),
        i32::from(level.grid_z).max(1),
    );
    let mut g = CellGrid::empty(size);
    for (i, cell) in level.cells.iter().enumerate() {
        let Some(cell) = cell else { continue };
        let i = i as i32;
        let c = (i % size.0, (i / size.0) % size.1, i / (size.0 * size.1));
        if let Some(ci) = g.get_mut(c) {
            *ci = CellInfo {
                present: true,
                bits: if cell.version >= 1006 {
                    cell.fields.first().copied().unwrap_or(0)
                } else {
                    0
                },
                zones: 0,
                code: cell.index,
                flag40: i32::from(cell.mirrored),
                door_closed: false,
            };
        }
    }
    g.add_zones(zones);
    g
}

/// Velocidade de um projétil (para "vindo para ele"), na unidade do estado.
fn velocity(e: &Entity) -> (i32, i32) {
    match &e.behaviour {
        Behaviour::Bullet(s) => (s.vx, s.vy),
        Behaviour::Pellet(s) => (s.vx, s.vy),
        Behaviour::Rocket(s) => (s.vx, s.vy),
        Behaviour::Star(s) => (s.vx, s.vy),
        Behaviour::Grenade(s) => (s.vx, s.vy),
        Behaviour::EnemyShot(s) => (s.vx, s.vy),
        _ => (0, 0),
    }
}

/// Linha do `DAMAGE.TBL` de um projétil das armas (entidade) contra um personagem (`victim`
/// = tipo 1 ou 2, centro em `at`): `(19, 1)` bala, `(20, 1/2)` chumbo, `(21, 0)` foguete,
/// `(25, 1)` faca, `(26, 0)` granada, `(27, 1)` estrela, `(108, 1)` chicote. A velocidade
/// chega zerada ao contato; `None` se não é projétil das armas.
pub(super) fn hit_on_character(shot: &Entity, victim: u32, at: (i32, i32)) -> Option<Pending> {
    let attacker = match &shot.behaviour {
        Behaviour::Bullet(_)
        | Behaviour::Pellet(_)
        | Behaviour::Rocket(_)
        | Behaviour::Star(_)
        | Behaviour::Grenade(_) => shot.kind,
        _ => return None,
    };
    let c = shot.center();
    let row = crate::damage::row(attacker, Some(victim), 0)?;
    Some(row.pending(Geometry {
        vel: (0, 0),
        toward: (at.0 - c.0, at.1 - c.1),
    }))
}

impl Scene {
    /// Grade de células da IA (bits de passagem, zonas PR, portas).
    pub fn ai_grid(&self) -> &CellGrid {
        &self.ai_grid
    }

    /// Dono (índice do personagem da IA) do tiro `i`.
    pub(super) fn ai_owner(&self, i: usize) -> Option<usize> {
        self.ai_shots.iter().find(|(s, _)| *s == i).map(|(_, o)| *o)
    }

    /// Tipo (1 ou 2) e centro da entidade `i`, se ela é um personagem da IA.
    pub(super) fn ai_victim(&self, i: usize) -> Option<(u32, (i32, i32))> {
        let e = self.entities.get(i)?;
        match &e.behaviour {
            Behaviour::AiPlayer(a) => Some((a.body.character().kind(), e.center())),
            _ => None,
        }
    }

    /// Jogadores humanos em jogo na camada `layer` (pela camada do assento): (índice, caixa).
    fn human_targets(&self, layer: i32) -> Vec<(usize, Rect)> {
        (0..self.seats())
            .filter_map(|i| {
                let seat_layer = if i == self.active {
                    self.jake_layer
                } else {
                    self.parked.iter().find(|s| s.player == i)?.layer
                };
                self.player_jake(i)
                    .filter(|j| j.is_active() && seat_layer == layer)
                    .map(|j| (i, j.body.bounds()))
            })
            .collect()
    }

    /// Alvo de um personagem da IA com dois jogadores (`0x42c5cc`): o escolhido fica enquanto
    /// vive; senão o mais perto dos dois primeiros vivos que está na mesma célula ou que a
    /// busca alcança, senão o outro, senão o mais perto (o cérebro o recusa).
    fn pick_target(&self, ai: &mut AiPlayer, humans: &[(usize, View)]) -> Option<View> {
        if ai.brain.target {
            if let Some(h) = humans.iter().find(|h| h.0 == ai.brain.target_of) {
                return Some(h.1);
            }
            ai.brain.target = false;
        }
        let me = View::of(&ai.body);
        let c = me.center();
        let here = self.ai_grid.cell_of(c, me.layer);
        let mut by_dist: Vec<&(usize, View)> = humans.iter().collect();
        by_dist.sort_by_key(|h| {
            let t = h.1.center();
            let (dx, dy) = (i64::from(t.0 - c.0), i64::from(t.1 - c.1));
            (dx * dx + dy * dy, h.0)
        });
        let reach = |v: &View| {
            let goal = self.ai_grid.cell_of(v.center(), v.layer);
            goal == here || self.ai_grid.search(here, goal, c.0).is_some()
        };
        let Some(pick) = by_dist
            .iter()
            .find(|h| reach(&h.1))
            .or_else(|| by_dist.first())
        else {
            ai.brain.target = false;
            return None;
        };
        ai.brain.target_of = pick.0;
        Some(pick.1)
    }

    /// Mundo de um personagem da IA na camada `layer`: células e portas dessa camada mais os
    /// objetos sólidos dela que o barram (sem os personagens).
    fn ai_world(&self, layer: i32, me: usize, me_flags: i32) -> World {
        let cells = self.layer_cells(layer).solids().to_vec();
        let objects = self
            .entities
            .iter()
            .enumerate()
            .filter(|(k, e)| {
                *k != me
                    && e.layer == layer
                    && blocks_entity(me_flags, e)
                    && !matches!(e.behaviour, Behaviour::AiPlayer(_))
            })
            .flat_map(|(_, e)| {
                e.boxes.iter().map(|b| Rect {
                    x: e.x.saturating_add(b.x),
                    y: e.y.saturating_add(b.y),
                    ..*b
                })
            })
            .collect();
        World::with_objects(cells, objects)
    }

    /// Perigos a menos de `HAZARD_RANGE` px do centro `c`, na camada `layer`.
    fn threats_near(&self, me: usize, c: (i32, i32), layer: i32) -> Vec<Threat> {
        self.entities
            .iter()
            .enumerate()
            .filter(|(k, e)| *k != me && e.alive && e.layer == layer && ai::is_threat(e.kind))
            .map(|(_, e)| Threat {
                kind: e.kind,
                center: e.center(),
                vel: velocity(e),
            })
            .filter(|t| {
                let (dx, dy) = (t.center.0 - c.0, t.center.1 - c.1);
                i64::from(dx).pow(2) + i64::from(dy).pow(2) < i64::from(HAZARD_RANGE).pow(2)
            })
            .collect()
    }

    /// Portas de célula fechadas (ou fechando) na grade da IA.
    fn sync_ai_doors(&mut self) {
        for d in &self.cell_doors {
            let c = self.ai_grid.cell_of(d.origin, d.layer);
            let closed = matches!(
                d.state,
                super::cell_door::CLOSED | super::cell_door::CLOSING
            );
            if let Some(ci) = self.ai_grid.get_mut(c) {
                ci.door_closed = closed;
            }
        }
    }

    /// Tique dos personagens da IA (antes das entidades): o cérebro dá a entrada, o corpo
    /// anda no mundo da camada dele; tiros, golpes, investida, sons e sprite.
    pub(super) fn tick_ai_players(&mut self) {
        let ais: Vec<usize> = (0..self.entities.len())
            .filter(|&i| {
                self.entities[i].alive
                    && matches!(self.entities[i].behaviour, Behaviour::AiPlayer(_))
            })
            .collect();
        if ais.is_empty() {
            return;
        }
        // os tiros mortos saem da lista de donos (as entidades nunca saem da cena)
        let ents = &self.entities;
        self.ai_shots.retain(|(s, _)| ents[*s].alive);
        self.sync_ai_doors();
        // candidatos a alvo: os dois primeiros jogadores humanos vivos (o cérebro escolhe e
        // guarda)
        let humans: Vec<(usize, View)> = (0..self.seats())
            .filter_map(|i| {
                self.player_jake(i)
                    .filter(|j| !j.is_dead())
                    .map(|j| (i, View::of(j)))
            })
            .take(2)
            .collect();
        for i in ais {
            let Behaviour::AiPlayer(mut ai) =
                std::mem::replace(&mut self.entities[i].behaviour, Behaviour::Static)
            else {
                continue;
            };
            let target = if self.players.len() < 2 {
                humans.first().map(|h| h.1)
            } else {
                self.pick_target(&mut ai, &humans)
            };
            self.tick_ai(i, &mut ai, target.as_ref());
            self.entities[i].behaviour = Behaviour::AiPlayer(ai);
        }
    }

    fn tick_ai(&mut self, i: usize, ai: &mut AiPlayer, target: Option<&View>) {
        let layer = ai.body.layer();
        let me_flags = self.entities[i].flags;
        let world = self.ai_world(layer, i, me_flags);
        let input = if ai.body.is_dead() {
            0
        } else {
            let me = Me::of(&ai.body);
            let threats = self.threats_near(i, me.view.center(), layer);
            let mut s = Senses {
                world: &world,
                grid: &self.ai_grid,
                threats: &threats,
                rng: &mut ai.rng,
            };
            ai.brain.think(&me, target, &mut s)
        };
        let env = Env {
            world: &world,
            zones: &self.prop_zones,
            layers: self.layers,
        };
        ai.body.tick_in(input, &env);
        ai.body.respawn_if_outside((self.width, self.height));
        let b = ai.body.body.bounds();
        let center = (b.x + b.w / 2, b.y + b.h / 2);
        let layer = ai.body.layer();
        // tiros: as mesmas entidades das armas do jogador, com dono
        for s in std::mem::take(&mut ai.body.shots) {
            if let Some(mut e) = self.shot_entity(&s, layer) {
                if let Behaviour::Bullet(b) = &mut e.behaviour {
                    b.player_shot = false;
                }
                self.ai_shots.push((self.entities.len(), i));
                self.entities.push(e);
            }
        }
        for id in [ai.body.fire_sound(), ai.body.sound.take()]
            .into_iter()
            .flatten()
        {
            self.events
                .push(Event::Sound(Cue::at(id, PRIORITY_FRAME, center, layer)));
        }
        if let Some(r) = ai.body.punch.take() {
            let strike = ai.body.blow.unwrap_or(Strike {
                attacker: ai.body.character().kind(),
                variant: 0,
            });
            let vel = (ai.body.body.vx / SUB, ai.body.body.vy / SUB);
            self.ai_punch(i, r, layer, strike, center, vel);
        }
        // investida do pulo longo: só no jogador humano, uma vez por pulo
        if ai.body.character() == Character::Garathe
            && ai.body.state() == State::Jump(JumpKind::Long)
        {
            let hit = self
                .human_targets(layer)
                .into_iter()
                .find(|h| h.1.overlaps(&b));
            if let (false, Some((k, h))) = (ai.body_checked, hit) {
                if let Some(j) = self.player_jake_mut(k) {
                    j.hurt(garathe::body_check(b, h));
                    ai.body_checked = true;
                }
                self.credit_player(k, 0);
            }
        } else {
            ai.body_checked = false;
        }
        let e = &mut self.entities[i];
        e.x = ai.body.body.x;
        e.y = ai.body.body.y;
        e.boxes = vec![ai.body.body.boxr];
        e.layer = layer;
        e.health = ai.body.health.hp;
        e.armor = ai.body.health.armor;
        if ai.body.is_dead() {
            // o cadáver fica, sem colisão nem dano (`+0x178 |= 0x8000`)
            e.solid_shapes = false;
            e.flags |= 0x8000;
        }
        if let Some(sp) = e.sprite.as_mut() {
            let (ox, oy) = ai.body.draw_offset();
            sp.x = ai.body.body.x.saturating_add(ox);
            sp.y = ai.body.body.y.saturating_add(oy);
            sp.mirror = ai.body.sprite_mirrored();
            if let Some(mut p) = AnimPlayer::at_label(&sp.anim, ai.body.label()) {
                p.seek(ai.body.frame());
                sp.player = p;
            }
        }
    }

    /// Golpe de um personagem da IA (`0x0043afe1`): o jogador humano e os objetos da camada
    /// (fora ele) cuja caixa cruza a do golpe levam a linha do golpe contra o tipo deles.
    fn ai_punch(
        &mut self,
        me: usize,
        r: Rect,
        layer: i32,
        strike: Strike,
        from: (i32, i32),
        vel: (i32, i32),
    ) {
        let geometry = |c: (i32, i32)| Geometry {
            vel,
            toward: (c.0 - from.0, c.1 - from.1),
        };
        let mut sounds = Vec::new();
        for (k, h) in self.human_targets(layer) {
            if !h.overlaps(&r) {
                continue;
            }
            if let Some(j) = self.player_jake_mut(k) {
                let b = j.body.bounds();
                let c = (b.x + b.w / 2, b.y + b.h / 2);
                if let Some(row) = strike.row(j.character().kind()) {
                    j.hurt(row.pending(geometry(c)));
                    sounds.extend(row.sound().map(|id| (id, c)));
                }
            }
            self.credit_player(k, 0);
        }
        for (k, e) in self.entities.iter_mut().enumerate() {
            if k == me
                || !e.alive
                || e.layer != layer
                || e.flags & 0x80001 != 0x80001
                || e.flags & 0x8000 != 0
                || !e.bounds().is_some_and(|b| b.overlaps(&r))
            {
                continue;
            }
            let Some(row) = strike.row(e.kind) else {
                continue;
            };
            let c = e.center();
            let p = row.pending(geometry(c));
            if self.players.len() >= 2 && e.health > 0 {
                // a IA não tem jogador: crédito 0, que também sobrescreve
                if self.credits.len() > k {
                    self.credits[k] = 0;
                }
            }
            if behaviour::enemy::hurt(e, &p, false) || behaviour::interact::hurt(e, &p) {
                sounds.extend(row.sound().map(|id| (id, c)));
            }
        }
        for (id, c) in sounds {
            self.events
                .push(Event::Sound(Cue::at(id, PRIORITY_FRAME, c, layer)));
        }
    }

    /// Tiros da IA nos jogadores humanos e, com dois jogadores, os de um humano no outro
    /// (antes dos acertos nos objetos): balas e projéteis das armas cuja caixa varrida cruza a
    /// de um jogador levam a linha da vítima jogador e param.
    pub(super) fn ai_shots_hit_human(&mut self, out: &mut behaviour::Out) {
        if self.jake.is_none() && self.parked.is_empty() {
            return;
        }
        let mut shots: Vec<(usize, Option<usize>)> =
            self.ai_shots.iter().map(|(s, _)| (*s, None)).collect();
        if self.players.len() >= 2 {
            let ents = &self.entities;
            self.human_shots.retain(|(s, _)| ents[*s].alive);
            shots.extend(self.human_shots.iter().map(|(s, p)| (*s, Some(*p))));
        }
        for (si, owner) in shots {
            let e = &self.entities[si];
            if !e.alive {
                continue;
            }
            let swept = match &e.behaviour {
                Behaviour::Bullet(_) => behaviour::bullet::swept_bounds(e),
                _ => super::arsenal::flight(e)
                    .filter(|(_, spent)| !spent)
                    .and_then(|(prev, _)| behaviour::projectile::swept(e, prev)),
            };
            let Some(sw) = swept else { continue };
            let Some((k, h)) = self
                .human_targets(e.layer)
                .into_iter()
                .find(|(k, h)| Some(*k) != owner && h.overlaps(&sw))
            else {
                continue;
            };
            let victim = self.player_jake(k).map_or(1, |j| j.character().kind());
            let p = hit_on_character(e, victim, (h.x + h.w / 2, h.y + h.h / 2));
            if let (Some(p), Some(j)) = (p, self.player_jake_mut(k)) {
                j.hurt(p);
            }
            if self.players.len() >= 2 {
                let c = self.credit_of(si);
                self.credit_player(k, c);
            }
            if matches!(self.entities[si].behaviour, Behaviour::Bullet(_)) {
                let c = self.entities[si].center();
                self.entities[si].alive = false;
                out.impacts.push((c.0, c.1, self.entities[si].layer));
            } else {
                self.weapon_impact(si, None, out);
            }
        }
    }
}

#[cfg(test)]
mod tests;
