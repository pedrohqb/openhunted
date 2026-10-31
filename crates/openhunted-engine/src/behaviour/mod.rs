// SPDX-License-Identifier: GPL-3.0-or-later
//! Comportamentos por tipo de objeto (`docs/re/logica-fase.md`, `dicas.md`, `itens.md`,
//! `minas.md`, `dano-vida.md`).

pub mod exit;
pub mod explosion;
pub mod hint;
pub mod mine;
pub mod pickup;
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
    /// Som do `SOUND.RES` (ignorado por enquanto).
    Sound(u16),
    Outcome(Outcome),
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
    /// Só as caixas das células (para quem se move sem ser o Jake).
    pub cells: &'a World,
    /// Estado "ligado" de um id de gatilho antes do tique (`None` = id inexistente).
    pub on: &'a dyn Fn(i32) -> Option<bool>,
    pub rng: &'a mut Rng,
    /// Linhas do `WIZARD.RES`.
    pub hints: &'a [String],
    /// Tamanho da fase em px.
    pub level: (i32, i32),
}

impl Ctx<'_> {
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
    /// Vida do Jake depois de apanhar itens (a última vale).
    pub heal: Vec<crate::damage::Health>,
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
            on,
            rng,
            hints: &[],
            level: (10_000, 10_000),
        }
    }
}
