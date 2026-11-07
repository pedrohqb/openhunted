// SPDX-License-Identifier: GPL-3.0-or-later
//! Comportamentos por tipo de objeto (`docs/re/logica-fase.md`, `dicas.md`, `itens.md`,
//! `minas.md`, `dano-vida.md`).

pub mod ant;
pub mod barrel;
pub mod bruiser;
pub mod bullet;
pub mod car_piece;
pub mod casing;
pub mod catapult;
pub mod chameleon;
pub mod dart_generator;
pub mod death;
pub mod door;
pub mod enemy;
pub mod enemy_shot;
pub mod exit;
pub mod explosion;
pub mod fence;
pub mod fire;
pub mod flyer;
pub mod graffiti;
pub mod grenade;
pub mod hint;
pub mod hop_mine;
pub mod interact;
pub mod laser;
pub mod laser_eye;
pub mod mine;
pub mod mine_generator;
pub mod pellet;
pub mod pickup;
pub mod piston;
pub mod projectile;
pub mod rocket;
pub mod security_robot;
pub mod sentinel;
pub mod star;
pub mod switch;
pub mod teleporter;
pub mod the_entity;
pub mod zone;

use crate::damage::Pending;
use crate::entity::Entity;
use crate::physics::World;
use crate::rng::Rng;
use crate::{Jake, Rect};

/// O que a cena emite num tique para quem a mostra.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// Dica do tutorial: texto e se o mago está à esquerda do Jake.
    Hint {
        text: String,
        left_of_jake: bool,
    },
    /// Som do `SOUND.RES`, com prioridade e, se posicional, o ponto de origem.
    Sound(crate::sound::Cue),
    Outcome(Outcome),
}

impl Event {
    /// Id do som, se o evento é um som.
    pub fn sound_id(&self) -> Option<u16> {
        match self {
            Event::Sound(c) => Some(c.id),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Victory,
    Defeat,
}

/// Explosão a aplicar aos demais objetos depois do tique das entidades.
#[derive(Debug, Clone, Copy)]
pub struct Blast {
    pub center: (i32, i32),
    pub layer: i32,
    pub radius: i32,
    pub push: i32,
    pub damage: i32,
}

/// Contexto de leitura de um tique.
pub struct Ctx<'a> {
    pub tick: u32,
    pub jake: Option<&'a Jake>,
    pub jake_layer: i32,
    pub world: &'a World,
    /// Só as caixas das células da camada do Jake.
    pub cells: &'a World,
    /// Só as caixas das células de cada camada (índice = camada), para quem se move sem ser
    /// o Jake: cada objeto anda contra as células da própria camada (`cells_for`).
    pub layer_cells: &'a [World],
    /// Estado "ligado" de um id de gatilho antes do tique (`None` = id inexistente).
    pub on: &'a dyn Fn(i32) -> Option<bool>,
    pub rng: &'a mut Rng,
    /// Linhas do `WIZARD.RES`.
    pub hints: &'a [String],
    /// Tamanho da fase em px.
    pub level: (i32, i32),
    /// Dificuldade (`0x487854`).
    pub difficulty: crate::Difficulty,
    /// Qualquer personagem pega qualquer item (disputa, `itens.md` passo 2).
    pub any_taker: bool,
}

impl Ctx<'_> {
    /// Células da camada `layer` (a do Jake é `cells`; camada sem mundo próprio também).
    pub fn cells_for(&self, layer: i32) -> &World {
        if layer == self.jake_layer {
            return self.cells;
        }
        usize::try_from(layer)
            .ok()
            .and_then(|l| self.layer_cells.get(l))
            .unwrap_or(self.cells)
    }

    /// Caixa do Jake, se ele está vivo e em jogo.
    pub fn jake_bounds(&self) -> Option<Rect> {
        self.jake.filter(|j| j.is_active()).map(|j| j.body.bounds())
    }
}

/// Saídas de um tique das entidades.
#[derive(Default)]
pub struct Out {
    pub events: Vec<Event>,
    pub spawn: Vec<Entity>,
    pub jake_damage: Pending,
    pub blasts: Vec<Blast>,
    /// Vida do Jake depois de apanhar itens neste tique (acumulada item a item).
    pub heal: Option<crate::damage::Health>,
    /// Veneno do Jake (`+0x270`) depois de apanhar veneno ou antídoto neste tique.
    pub poison: Option<i32>,
    /// Impactos de bala (centro x, y, camada): faísca `oex15` e som.
    pub impacts: Vec<(i32, i32, i32)>,
    /// Efeitos do `OEX.ANM` com rótulo fixo e sem dano (o `oex01` do raio laser).
    pub effects: Vec<(Blast, u16)>,
    /// Dano de contato a outros objetos (fogo, cerca): a cena aplica a quem cruza a caixa.
    pub hazards: Vec<Hazard>,
    /// Teletransporte pedido neste tique.
    pub teleport: Option<teleporter::Request>,
    /// Inventário do Jake depois de apanhar armas e munição neste tique.
    pub inventory: Option<crate::jake::Inventory>,
    /// Objetos que os inimigos pedem (tiros, minas, fogo): a cena os cria com os recursos.
    pub enemy_spawns: Vec<enemy::Spawn>,
    /// Terremoto do Bruiser (`0x418229`): tiques de cambaleio para o Jake, se perto.
    pub quake: Option<u32>,
    /// Uma Entidade (107) confunde o jogador neste tique (`+0x264 = 3`).
    pub confuse: bool,
    /// Lançamentos da catapulta sobre barris e formigas (a cena aplica).
    pub launches: Vec<catapult::Launch>,
    /// Crédito (`+0x1b0`) de explosões de `blasts`: (índice, número do jogador), herdado de
    /// quem a criou; as ausentes têm crédito 0.
    pub blast_credits: Vec<(usize, u8)>,
    /// Crédito dos objetos de `spawn` criados por quem tem crédito (o chumbo que se divide).
    pub spawn_credits: Vec<(usize, u8)>,
}

/// Área que fere quem a cruza na mesma camada (`0x43d629` de fogo e cerca sobre os objetos).
#[derive(Debug, Clone, Copy)]
pub struct Hazard {
    pub rect: Rect,
    pub layer: i32,
    pub damage: Pending,
    /// Faísca de cerca (posição do poste principal): pega pelo centro x da vítima e, com
    /// acerto, põe a cerca na recarga (`0x41494b`). `None` = fogo (cruzar a área basta).
    pub source: Option<(i32, i32)>,
}

/// Ângulo de 16 bits de (dx, dy) com y para baixo (`0x46718e`, atan2): 0 = direita,
/// 16384 = baixo, 32768 = esquerda.
pub fn angle16(dx: i32, dy: i32) -> i32 {
    let a = f64::from(dy).atan2(f64::from(dx));
    ((a * 65536.0 / std::f64::consts::TAU).round() as i64).rem_euclid(65536) as i32
}

/// Diferença `a − b` de ângulos de 16 bits, reduzida a −32768…32767.
pub fn angle_diff(a: i32, b: i32) -> i32 {
    (a.wrapping_sub(b) as i16) as i32
}

/// Cosseno e seno em escala 2^14 de um ângulo de 16 bits (`0x467154`/`0x46717a`).
pub fn cos_sin14(a: i32) -> (i32, i32) {
    let r = f64::from(a) * std::f64::consts::TAU / 65536.0;
    (
        (r.cos() * 16384.0).round() as i32,
        (r.sin() * 16384.0).round() as i32,
    )
}

/// Caixa `r` alargada em `d` px de cada lado (contato de quem encosta num sólido).
pub fn grow(r: Rect, d: i32) -> Rect {
    Rect {
        x: r.x - d,
        y: r.y - d,
        w: r.w + 2 * d,
        h: r.h + 2 * d,
    }
}

/// Põe o sprite da entidade no quadro `offset` do rótulo `label` (sem o rótulo, nada).
pub fn show(e: &mut Entity, label: u16, offset: usize) {
    if let Some(sp) = e.sprite.as_mut() {
        if let Some(l) = sp.anim.labels.iter().find(|l| l.id == label) {
            let f = (l.frame as usize).saturating_add(offset);
            if f < sp.anim.frames.len() {
                sp.player = crate::AnimPlayer::from_frame(&sp.anim, f);
            }
        }
    }
}

/// Rótulo em que está o quadro atual do sprite (o de maior início ≤ quadro).
pub fn label_of(sp: &crate::Sprite) -> Option<u16> {
    let f = sp.player.frame();
    sp.anim
        .labels
        .iter()
        .filter(|l| l.frame as usize <= f)
        .max_by_key(|l| l.frame)
        .map(|l| l.id)
}

/// Recomeça o sprite no rótulo `label`, se ele existir.
pub fn restart(e: &mut Entity, label: u16) {
    if let Some(sp) = e.sprite.as_mut() {
        if let Some(p) = crate::AnimPlayer::at_label(&sp.anim, label) {
            sp.player = p;
        }
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::Ctx;
    use crate::behaviour::zone::ZoneState;
    use crate::entity::{Behaviour, Entity, ZONE};
    use crate::physics::Rect;
    use crate::rng::Rng;
    use crate::{Jake, World};

    /// Objeto de lógica sem sprite, com caixa própria 32 × 32.
    pub fn logic(kind: u32, x: i32, y: i32, values: &[i32]) -> Entity {
        Entity {
            kind,
            x,
            y,
            flags: 0x80089,
            layer: 0,
            id: 0,
            values: values.to_vec(),
            triggers: vec![],
            objective: 0,
            health: 0,
            armor: 0,
            boxes: vec![Rect {
                x: 0,
                y: 0,
                w: 32,
                h: 32,
            }],
            solid_shapes: false,
            on: false,
            alive: true,
            sprite: None,
            behaviour: match kind {
                ZONE => Behaviour::Zone(ZoneState::default()),
                crate::entity::HINT => Behaviour::Hint(Default::default()),
                _ => Behaviour::Static,
            },
        }
    }

    pub fn zone(x: i32, y: i32, w: i32, h: i32, values: &[i32]) -> Entity {
        let mut e = logic(ZONE, x, y, values);
        e.boxes = vec![Rect { x: 0, y: 0, w, h }];
        e
    }

    /// Jake parado em (x, y) sobre um chão infinito em y + 100.
    pub fn ctx_with_jake(x: i32, y: i32) -> (Jake, World) {
        let w = World::new(vec![Rect {
            x: -100_000,
            y: y + 47,
            w: 200_000,
            h: 20,
        }]);
        let mut j = Jake::new(
            x,
            y,
            Rect {
                x: 2,
                y: 3,
                w: 18,
                h: 44,
            },
            std::collections::HashMap::new(),
        )
        .with_health(50, 0);
        j.tick(0, &w);
        (j, w)
    }

    pub fn ctx<'a>(j: &'a Jake, w: &'a World, on: &'a dyn Fn(i32) -> Option<bool>) -> Ctx<'a> {
        // um gerador por teste basta; o Ctx pede &mut, então vaza um Box
        let rng: &'a mut Rng = Box::leak(Box::new(Rng::new(1)));
        Ctx {
            tick: 0,
            jake: Some(j),
            jake_layer: 0,
            world: w,
            cells: w,
            layer_cells: &[],
            on,
            rng,
            hints: &[],
            level: (10_000, 10_000),
            difficulty: crate::Difficulty::Normal,
            any_taker: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::{ctx, ctx_with_jake};
    use super::*;

    #[test]
    fn cells_for_picks_the_layer_world() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(0, 0);
        let layers = [
            World::default(),
            World::new(vec![Rect {
                x: 0,
                y: 0,
                w: 1,
                h: 1,
            }]),
        ];
        let mut c = ctx(&j, &w, &on);
        c.layer_cells = &layers;
        assert!(std::ptr::eq(c.cells_for(0), &w), "a do Jake é `cells`");
        assert!(std::ptr::eq(c.cells_for(1), &layers[1]));
        assert!(std::ptr::eq(c.cells_for(7), &w), "camada sem mundo");
    }
}
