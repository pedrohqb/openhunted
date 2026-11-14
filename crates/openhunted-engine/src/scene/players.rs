// SPDX-License-Identifier: GPL-3.0-or-later
//! Dois jogadores na cena (M15a, `docs/re/multijogador.md`): modos e máscaras (`0x43c95c`),
//! atribuição das vagas (`0x43cbb0`), assentos por jogador, alvo das entidades, crédito,
//! F/S, renascimento da disputa, itens que renascem e fim de partida (`0x43cdf0`).

use openhunted_formats::LevelObject;

use super::{Layer, Scene, Sprite};
use crate::behaviour::{self, Event, Outcome};
use crate::character::Character;
use crate::entity::{Behaviour, Entity};
use crate::physics::{Rect, World};
use crate::rng::Rng;
use crate::{AnimPlayer, Jake};

/// Máscara do modo cooperativo (`0x43cbb0` troca 0x20/0x40/0x80 por 0xe0).
pub const COOP_MASK: i32 = 0xe0;
/// Candidatos às vagas na disputa (`0x43cd91`: qualquer par).
pub const DEATHMATCH_SLOTS: i32 = 0x1c;
/// Cores da `SEGOC` (15012–15015: marrom 36, vermelho 38, preto 35, branco 37) e o padrão
/// (`0x43d33b`): J1 35, J2 36.
pub const COLOURS: [u8; 4] = [36, 38, 35, 37];
pub const DEFAULT_COLOURS: [u8; 2] = [35, 36];
/// Pontos de renascimento: células com o bit 4 de `+0x38` (`fields[14]`), até 16
/// (`0x43d22a`, `0x4973b0`).
pub const SPAWN_BIT: i32 = 4;
pub const MAX_SPAWNS: usize = 16;
/// Tipo do "Regenerate" (`0x412fec`) e a espera antes de animar: 300 tiques (15 s).
pub const REGENERATE: u32 = 43;
pub const REGEN_WAIT: u32 = 300;
/// Quadro do `ORG.ANM` em que o item volta.
pub const REGEN_FRAME: usize = 8;

/// A disputa: personagens e cores da `SEGOC`, limites da `SHVLD` (0 = sem limite).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Duel {
    pub chars: [Character; 2],
    /// Tabelas de cor `+0x280` (35–38).
    pub colours: [u8; 2],
    /// Limite de pontos (`0x497378`): fim quando `F − S ≥` limite.
    pub points_limit: i32,
    /// Limite de tempo real em ms (`0x497374` minutos × 60 000).
    pub time_limit_ms: u64,
}

impl Default for Duel {
    fn default() -> Self {
        Self {
            chars: [Character::Jake, Character::Garathe],
            colours: DEFAULT_COLOURS,
            points_limit: 0,
            time_limit_ms: 0,
        }
    }
}

/// Modo de jogo (`0x497300`/`0x497304`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Solo,
    Coop,
    Deathmatch(Duel),
}

impl Mode {
    /// Máscara do modo `0x4972f8` (`0x43c95c`, coop por `0x43cbb0`).
    pub fn mask(&self) -> i32 {
        match self {
            Mode::Solo => crate::objects::SOLO,
            Mode::Coop => COOP_MASK,
            Mode::Deathmatch(d) => {
                let n: u32 = d.chars.iter().map(|c| c.kind()).sum();
                match n {
                    2 => 0x4,
                    3 => 0x8,
                    _ => 0x10,
                }
            }
        }
    }

    /// Bits de vaga aceitos na atribuição (`0x43cd91`).
    pub fn slot_mask(&self) -> i32 {
        match self {
            Mode::Solo => crate::objects::SOLO,
            Mode::Coop => COOP_MASK,
            Mode::Deathmatch(_) => DEATHMATCH_SLOTS,
        }
    }

    /// Jogadores humanos.
    pub fn humans(&self) -> usize {
        match self {
            Mode::Solo => 1,
            _ => 2,
        }
    }

    pub fn duel(&self) -> Option<&Duel> {
        match self {
            Mode::Deathmatch(d) => Some(d),
            _ => None,
        }
    }
}

/// Estatísticas de um jogador (`0x49732c`, registros de 24 bytes).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Stats {
    pub secrets: i32,
    pub kills: i32,
    /// F: eliminações do outro.
    pub frags: i32,
    /// S: autoeliminações.
    pub suicides: i32,
    pub treasures: i32,
}

impl Stats {
    /// Pontos da disputa: `F − S`.
    pub fn score(&self) -> i32 {
        self.frags - self.suicides
    }
}

/// Um jogador humano.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Player {
    /// `+0x19c` (1 ou 2).
    pub number: u8,
    pub character: Character,
    /// Cor-base `+0x280` (0 = a do personagem).
    pub colour: i32,
    pub stats: Stats,
    /// Crédito `+0x1b0` do corpo: número de quem pôs o último dano com ele vivo.
    pub credit: u8,
    /// Camada inicial (renascer sem pontos de renascimento).
    pub start_layer: i32,
}

/// O estado de um jogador que a cena guarda nos seus campos quando ele é o ativo.
#[derive(Debug, Clone, Default)]
pub(super) struct Seat {
    /// Índice do jogador dono deste assento.
    pub player: usize,
    pub jake: Option<Jake>,
    pub sprite: Option<Sprite>,
    pub layer: i32,
    pub flags: i32,
    pub world: World,
    pub object_solids: Vec<Rect>,
    pub cell_solids: Vec<Rect>,
    pub cell_world: World,
    pub body_checked: Vec<usize>,
    pub prev_input: u32,
    pub input: u32,
    pub confusion: u32,
    pub quake: u32,
}

/// Itens que renascem (disputa): o item apanhado e o efeito "Regenerate".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Regen {
    pub item: usize,
    pub effect: Option<usize>,
    pub ticks: u32,
}

/// Vagas dos jogadores (`0x43cbb0`): índices em `objects`, um por jogador. Candidatos do
/// jogador `i`: bit 0 e `modos & slot_mask`, tipo do personagem escolhido (coop: qualquer),
/// fora os tomados, na lista montada de trás para a frente. Disputa: sorteio; coop: o
/// primeiro da lista (o último dos objetos). `Err(n)` = sem vaga para o jogador `n`.
pub fn assign_slots(
    objects: &[&LevelObject],
    mode: &Mode,
    rng: &mut Rng,
) -> Result<Vec<usize>, u8> {
    let m = mode.slot_mask();
    let mut taken: Vec<usize> = Vec::new();
    for i in 0..mode.humans() {
        let want = mode.duel().map(|d| d.chars[i].kind());
        let cands: Vec<usize> = (0..objects.len())
            .rev()
            .filter(|&k| {
                let o = objects[k];
                o.modes & 1 != 0
                    && o.modes & m != 0
                    && Character::from_kind(o.kind).is_some()
                    && want.is_none_or(|w| o.kind == w)
                    && !taken.contains(&k)
            })
            .collect();
        let pick = match (mode, cands.len()) {
            (_, 0) => return Err(i as u8 + 1),
            (Mode::Deathmatch(_), n) => cands[rng.below(n as u32) as usize],
            _ => cands[0],
        };
        taken.push(pick);
    }
    Ok(taken)
}

/// Pontos de renascimento (`0x43d22a`): células com o bit 4 de `+0x38` (`fields[14]`), na
/// ordem das células, até 16: `(128·cx + 64, 128·cy + 32, camada)`.
pub fn spawn_points(level: &openhunted_formats::Level) -> Vec<(i32, i32, i32)> {
    let (gx, gy) = (
        i32::from(level.grid_x).max(1),
        i32::from(level.grid_y).max(1),
    );
    level
        .cells
        .iter()
        .enumerate()
        .filter_map(|(i, c)| {
            let c = c.as_ref()?;
            (c.fields.get(14).copied().unwrap_or(0) & SPAWN_BIT != 0).then_some(i as i32)
        })
        .take(MAX_SPAWNS)
        .map(|i| {
            let (cx, cy, cz) = (i % gx, (i / gx) % gy, i / (gx * gy));
            (super::CELL * cx + 64, super::CELL * cy + 32, cz)
        })
        .collect()
}

/// Distância de `p` à caixa `r` (0 dentro), ao quadrado.
fn dist2(r: &Rect, p: (i32, i32)) -> i64 {
    let dx = i64::from((r.x - p.0).max(p.0 - (r.x + r.w)).max(0));
    let dy = i64::from((r.y - p.1).max(p.1 - (r.y + r.h)).max(0));
    dx * dx + dy * dy
}

fn center(r: &Rect) -> (i32, i32) {
    (r.x + r.w / 2, r.y + r.h / 2)
}

impl Scene {
    /// Assentos em jogo (ao menos um, mesmo sem jogador).
    pub fn seats(&self) -> usize {
        self.players.len().max(1)
    }

    /// Troca os campos do assento ativo pelos do assento guardado `k`.
    fn swap_seat(&mut self, k: usize) {
        let s = &mut self.parked[k];
        std::mem::swap(&mut self.active, &mut s.player);
        std::mem::swap(&mut self.jake, &mut s.jake);
        std::mem::swap(&mut self.jake_sprite, &mut s.sprite);
        std::mem::swap(&mut self.jake_layer, &mut s.layer);
        std::mem::swap(&mut self.jake_flags, &mut s.flags);
        std::mem::swap(&mut self.world, &mut s.world);
        std::mem::swap(&mut self.object_solids, &mut s.object_solids);
        std::mem::swap(&mut self.cell_solids, &mut s.cell_solids);
        std::mem::swap(&mut self.cell_world, &mut s.cell_world);
        std::mem::swap(&mut self.body_checked, &mut s.body_checked);
        std::mem::swap(&mut self.prev_input, &mut s.prev_input);
        std::mem::swap(&mut self.cur_input, &mut s.input);
        std::mem::swap(&mut self.confusion, &mut s.confusion);
        std::mem::swap(&mut self.quake, &mut s.quake);
    }

    /// Roda `f` com o jogador `i` no assento ativo (os campos `jake`, `jake_layer`, `world`…
    /// passam a ser os dele) e devolve o ativo de antes. O jogador 1 fica sempre no assento
    /// quando não há troca em curso.
    pub(super) fn with_seat<R>(&mut self, i: usize, f: impl FnOnce(&mut Self) -> R) -> R {
        let Some(k) = (i != self.active)
            .then(|| self.parked.iter().position(|s| s.player == i))
            .flatten()
        else {
            return f(self);
        };
        self.swap_seat(k);
        let r = f(self);
        self.swap_seat(k);
        r
    }

    /// O corpo do jogador `i` (0 = jogador 1).
    pub fn player_jake(&self, i: usize) -> Option<&Jake> {
        if i == self.active {
            return self.jake.as_ref();
        }
        self.parked.iter().find(|s| s.player == i)?.jake.as_ref()
    }

    /// O sprite do jogador `i`.
    pub fn player_sprite(&self, i: usize) -> Option<&Sprite> {
        if i == self.active {
            return self.jake_sprite.as_ref();
        }
        self.parked.iter().find(|s| s.player == i)?.sprite.as_ref()
    }

    /// Camada do jogador `i`.
    pub fn layer_of(&self, i: usize) -> Option<i32> {
        self.player_jake(i).map(Jake::layer)
    }

    /// Centro da caixa do jogador `i` (para a câmera da vista dele).
    pub fn focus_of(&self, i: usize) -> Option<(i32, i32)> {
        self.player_jake(i).map(|j| center(&j.body.bounds()))
    }

    /// O jogador cuja vista está mais perto de `at` (ouvinte do som, `som.md`).
    pub fn nearest_player(&self, at: (i32, i32)) -> Option<usize> {
        (0..self.players.len())
            .filter_map(|i| self.focus_of(i).map(|c| (i, c)))
            .min_by_key(|(i, c)| {
                let (dx, dy) = (i64::from(c.0 - at.0), i64::from(c.1 - at.1));
                (dx * dx + dy * dy, *i)
            })
            .map(|(i, _)| i)
    }

    /// Jogadores em jogo agora: (índice, caixa, camada, ativo).
    pub(super) fn human_boxes(&self) -> Vec<(usize, Rect, i32, bool)> {
        (0..self.players.len())
            .filter_map(|i| {
                let j = self.player_jake(i)?;
                Some((i, j.body.bounds(), j.layer(), j.is_active()))
            })
            .collect()
    }

    /// Contra quem cada entidade age neste tique (índice do jogador). Ruling (M15a): o
    /// jogador ativo mais perto da caixa da entidade, preferindo a mesma camada; empate → o de
    /// menor número; sem ativo, o 1.
    pub(super) fn entity_targets(&self) -> Vec<usize> {
        let humans = self.human_boxes();
        if humans.len() < 2 {
            return vec![0; self.entities.len()];
        }
        self.entities
            .iter()
            .map(|e| {
                let r = e.bounds().unwrap_or(Rect {
                    x: e.x,
                    y: e.y,
                    w: 0,
                    h: 0,
                });
                humans
                    .iter()
                    .filter(|h| h.3)
                    .min_by_key(|(i, b, layer, _)| (*layer != e.layer, dist2(&r, center(b)), *i))
                    .map_or(0, |h| h.0)
            })
            .collect()
    }

    /// Crédito da entidade `i` (`+0x1b0`; 0 = ninguém).
    pub(super) fn credit_of(&self, i: usize) -> u8 {
        self.credits.get(i).copied().unwrap_or(0)
    }

    /// Põe o crédito `c` na entidade `i` (viva).
    pub(super) fn set_credit(&mut self, i: usize, c: u8) {
        if self.players.len() < 2 {
            return;
        }
        if self.credits.len() <= i {
            self.credits.resize(i + 1, 0);
        }
        self.credits[i] = c;
    }

    /// A vítima `victim` (viva) herda o crédito do atacante `attacker` (tiro, explosão).
    pub(super) fn inherit_credit(&mut self, victim: usize, attacker: usize) {
        let alive = self
            .entities
            .get(victim)
            .is_some_and(|e| e.alive && e.health > 0);
        if alive {
            self.set_credit(victim, self.credit_of(attacker));
        }
    }

    /// Fere os outros jogadores humanos (fora o ativo) pela regra `f` (dano para o corpo,
    /// ou nada), com o crédito do ativo. Devolve os centros feridos.
    pub(super) fn hurt_others(
        &mut self,
        mut f: impl FnMut(&Jake) -> Option<crate::damage::Pending>,
    ) -> Vec<(i32, i32)> {
        let c = self.active_number();
        let mut hit = Vec::new();
        let mut at = Vec::new();
        for s in self.parked.iter_mut() {
            let Some(j) = s.jake.as_mut().filter(|j| j.is_active()) else {
                continue;
            };
            if let Some(p) = f(j) {
                j.hurt(p);
                hit.push(s.player);
                at.push(center(&j.body.bounds()));
            }
        }
        for i in hit {
            self.credit_player(i, c);
        }
        at
    }

    /// Põe o jogador `i` com a origem do corpo em `at` na camada `layer`, parado (testes e
    /// ferramentas).
    pub fn place_player(&mut self, i: usize, at: (i32, i32), layer: i32) {
        self.with_seat(i, |s| {
            if let Some(j) = s.jake.as_mut() {
                j.body.x = at.0;
                j.body.y = at.1;
                j.body.vx = 0;
                j.body.vy = 0;
                j.set_layer(layer);
            }
            if layer != s.jake_layer {
                s.set_jake_layer(layer);
            }
        });
    }

    /// Corpo do jogador `i`, mutável.
    pub fn player_jake_mut(&mut self, i: usize) -> Option<&mut Jake> {
        if i == self.active {
            return self.jake.as_mut();
        }
        self.parked
            .iter_mut()
            .find(|s| s.player == i)?
            .jake
            .as_mut()
    }

    /// Crédito do jogador `i` (quem pôs dano nele vivo); com dois jogadores, 0 também
    /// sobrescreve (`0x43d6e3`).
    pub(super) fn credit_player(&mut self, i: usize, c: u8) {
        let alive = self.player_jake(i).is_some_and(|j| !j.is_dead());
        if let (true, Some(p)) = (alive, self.players.get_mut(i)) {
            p.credit = c;
        }
    }

    /// Número do jogador do assento ativo (0 sem jogador).
    pub(super) fn active_number(&self) -> u8 {
        self.players.get(self.active).map_or(0, |p| p.number)
    }

    /// Mortos por jogador: no solo, todos os contados (`enemy::kills`); com dois, os que têm
    /// o crédito dele.
    pub(super) fn count_kills(&mut self) {
        if self.players.len() < 2 {
            if let Some(p) = self.players.first_mut() {
                p.stats.kills = self.kills;
            }
            return;
        }
        for k in 0..self.players.len() {
            let n = self.players[k].number;
            let kills = self
                .entities
                .iter()
                .enumerate()
                .filter(|(i, e)| behaviour::enemy::killed(e) && self.credit_of(*i) == n)
                .count() as i32;
            self.players[k].stats.kills = kills;
        }
    }

    /// Vencedor da disputa pelo `F − S` (anúncio 3790/3792; empate 3773): índice.
    pub fn winner(&self) -> Option<usize> {
        let [a, b] = [0, 1].map(|i| self.players.get(i).map(|p| p.stats.score()));
        match (a?, b?) {
            (a, b) if a > b => Some(0),
            (a, b) if b > a => Some(1),
            _ => None,
        }
    }

    /// Depois do tique do jogador ativo: vivo, o crédito zera (`0x4232ec`, `0x4284b9`); no
    /// fim da morte na disputa, pontos, cadáver e renascimento (`0x4268ac`, `0x42b604`).
    pub(super) fn after_player_tick(&mut self, was_dead: bool) {
        let i = self.active;
        let Some(j) = self.jake.as_ref() else { return };
        let dead = j.state() == crate::State::Dead;
        if !j.is_dead() {
            if let Some(p) = self.players.get_mut(i) {
                p.credit = 0;
            }
        }
        if !dead || was_dead || self.mode.duel().is_none() || self.players.len() < 2 {
            return;
        }
        // pontos (`0x497334/0x497338`)
        let c = self.players[i].credit;
        let victim = self.players[i].number;
        if let Some(k) = self.players.iter().position(|p| p.number == c && c != 0) {
            let st = &mut self.players[k].stats;
            if c == victim {
                st.suicides = (st.suicides + 1).min(st.frags);
            } else {
                st.frags += 1;
            }
        }
        self.players[i].credit = 0;
        self.push_corpse();
        let at = self.spawn_point();
        let start_layer = self.players[i].start_layer;
        if let Some(j) = self.jake.as_mut() {
            match at {
                Some((x, y, l)) => j.respawn_at(Some((x, y)), l),
                None => j.respawn_at(None, start_layer),
            }
        }
        let layer = self.jake.as_ref().map_or(self.jake_layer, Jake::layer);
        if layer != self.jake_layer {
            self.set_jake_layer(layer);
        }
    }

    /// Sorteio uniforme de um ponto de renascimento (sem olhar o rival); sem nenhum, o
    /// início do jogador (`None`).
    fn spawn_point(&mut self) -> Option<(i32, i32, i32)> {
        if self.spawn_points.is_empty() {
            return None;
        }
        let n = self.spawn_points.len() as u32;
        Some(self.spawn_points[self.rng.below(n) as usize])
    }

    /// Cadáver clonado do jogador ativo (`0x433d80`): o sprite no último quadro da morte,
    /// na cor dele, sem colisão nem rotinas (`+0x178 |= 0x8038`).
    fn push_corpse(&mut self) {
        let (Some(j), Some(sp)) = (self.jake.as_ref(), self.jake_sprite.as_ref()) else {
            return;
        };
        let mut sp = sp.clone();
        sp.frozen = true;
        sp.layer = Layer::Object;
        // o último quadro da morte, como o desenho do corpo (`sync_jake_sprite`)
        let (ox, oy) = j.draw_offset();
        sp.x = j.body.x.saturating_add(ox);
        sp.y = j.body.y.saturating_add(oy);
        sp.mirror = j.sprite_mirrored();
        if let Some(mut p) = AnimPlayer::at_label(&sp.anim, j.label()) {
            p.seek(j.frame());
            sp.player = p;
        }
        let table = j.tint().table;
        self.entities.push(Entity {
            kind: j.character().kind(),
            x: sp.x,
            y: sp.y,
            flags: 0x8038,
            layer: j.layer(),
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
            sprite: Some(sp),
            behaviour: Behaviour::Corpse(table),
        });
    }

    /// Itens apanhados neste tique que renascem na disputa (`0x412fec`): a coleta genérica,
    /// salvo `0x10`, chaves, bandeiras e a peça do carro.
    pub(super) fn schedule_regen(&mut self, before: &[bool]) {
        if self.mode.duel().is_none() {
            return;
        }
        for (i, was) in before.iter().enumerate() {
            let e = &self.entities[i];
            if !*was
                || e.alive
                || !e.on
                || e.flags & 0x10 != 0
                || !matches!(e.behaviour, Behaviour::Pickup(_))
                || behaviour::pickup::KEYS.contains(&e.kind)
                || behaviour::pickup::FLAGS.contains(&e.kind)
                || e.kind == crate::entity::CAR_PIECE
                || self.regens.iter().any(|r| r.item == i)
            {
                continue;
            }
            let effect = self.regen_entity(i);
            let effect = effect.map(|fx| {
                self.entities.push(fx);
                self.entities.len() - 1
            });
            self.regens.push(Regen {
                item: i,
                effect,
                ticks: 0,
            });
        }
    }

    /// O "Regenerate" (tipo 43, `ORG.ANM` rótulo 1) do tamanho do item, centrado nele,
    /// escondido durante a espera.
    fn regen_entity(&self, item: usize) -> Option<Entity> {
        let anim = self.assets_anim("ORG.ANM")?;
        let it = &self.entities[item];
        let b = it.bounds()?;
        let player =
            AnimPlayer::at_label(&anim, 1).unwrap_or_else(|| AnimPlayer::from_frame(&anim, 0));
        let fh = anim
            .frames
            .first()
            .map_or(1, |f| i32::from(f.height).max(1));
        let (x, y) = (b.x + b.w / 2, b.y + b.h / 2);
        Some(Entity {
            kind: REGENERATE,
            x,
            y,
            flags: 0x8038,
            layer: it.layer,
            id: 0,
            values: vec![],
            triggers: vec![],
            objective: 0,
            health: 0,
            armor: 0,
            boxes: vec![],
            solid_shapes: false,
            on: false,
            alive: false,
            sprite: Some(Sprite {
                layer: Layer::Object,
                anim,
                player,
                x,
                y,
                frozen: true,
                scale: (b.h.max(b.w).max(1), fh),
                mirror: false,
            }),
            behaviour: Behaviour::Static,
        })
    }

    /// Conta a espera dos itens que renascem: depois de 300 tiques o efeito aparece e anima
    /// um quadro por tique; no quadro 8 o item volta; no fim o efeito some.
    pub(super) fn tick_regens(&mut self) {
        let mut done = Vec::new();
        for (k, r) in self.regens.iter_mut().enumerate() {
            r.ticks += 1;
            if r.ticks < REGEN_WAIT {
                continue;
            }
            let step = (r.ticks - REGEN_WAIT) as usize;
            let len = r
                .effect
                .and_then(|fx| self.entities[fx].sprite.as_ref())
                .map_or(REGEN_FRAME + 1, |sp| sp.anim.frames.len());
            if let Some(fx) = r.effect {
                let e = &mut self.entities[fx];
                e.alive = step < len;
                if let Some(sp) = e.sprite.as_mut() {
                    sp.player = AnimPlayer::from_frame(&sp.anim, step.min(len.saturating_sub(1)));
                }
            }
            if step == REGEN_FRAME.min(len.saturating_sub(1)) {
                let it = &mut self.entities[r.item];
                it.alive = true;
                it.on = false;
            }
            if step + 1 >= len.max(REGEN_FRAME + 1) {
                done.push(k);
            }
        }
        for k in done.into_iter().rev() {
            self.regens.remove(k);
        }
    }

    /// Resultado do tique com dois jogadores (`0x43cdf0`).
    pub(super) fn evaluate_players(&self) -> Option<Outcome> {
        let jakes: Vec<&Jake> = (0..self.players.len())
            .filter_map(|i| self.player_jake(i))
            .collect();
        let all_dead = !jakes.is_empty() && jakes.iter().all(|j| j.is_dead());
        let exit_on = self
            .entities
            .iter()
            .any(|e| matches!(e.behaviour, Behaviour::Exit(_)) && e.alive && e.on);
        // a saída do laço (`edi`): a última da lista; sem ela, nada a esperar
        let last_exit = self
            .entities
            .iter()
            .rev()
            .find(|e| matches!(e.behaviour, Behaviour::Exit(_)) && e.alive);
        match &self.mode {
            Mode::Solo => None,
            Mode::Coop => {
                if all_dead || self.timed_out() {
                    return Some(Outcome::Defeat);
                }
                let alive = jakes.iter().copied().find(|j| !j.is_dead());
                (exit_on && crate::verdict::objectives_met(&self.entities, alive))
                    .then_some(Outcome::Victory)
            }
            Mode::Deathmatch(d) => {
                // o laço dos objetos: todos mortos → "2"; senão "1" com os objetivos
                // ("acionar" ligado, "destruir" morto) cumpridos, "0" sem eles
                let mut r = if all_dead {
                    Some(Outcome::Defeat)
                } else if crate::verdict::objectives_met(&self.entities, None) {
                    Some(Outcome::Victory)
                } else {
                    None
                };
                // com limite de pontos, só o `F − S` decide
                if d.points_limit != 0 {
                    let reached = self
                        .players
                        .iter()
                        .any(|p| p.stats.score() >= d.points_limit);
                    r = reached.then_some(Outcome::Defeat);
                }
                if d.time_limit_ms != 0 && self.elapsed_ms > d.time_limit_ms {
                    r = Some(Outcome::Defeat);
                }
                if r == Some(Outcome::Defeat) {
                    return r;
                }
                // `0x43cf36`: com a saída desligada, nada; ligada (ou sem saída), sem limite
                // de pontos, objetivos pendentes → "2" (`0x43cf68`)
                if last_exit.is_some_and(|e| !e.on) {
                    return None;
                }
                if d.points_limit == 0 && r.is_none() {
                    return Some(Outcome::Defeat);
                }
                r
            }
        }
    }

    /// Fim de partida pelo resultado do tique (40 tiques seguidos).
    pub(super) fn push_outcome(&mut self, result: Option<Outcome>) {
        if let Some(o) = self.streak.push(result) {
            self.outcome = Some(o);
            self.events.push(Event::Outcome(o));
        }
    }
}

#[cfg(test)]
mod tests;
