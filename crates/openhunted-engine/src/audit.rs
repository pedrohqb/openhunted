// SPDX-License-Identifier: GPL-3.0-or-later
//! Auditoria de vencibilidade das missões (M13; `docs/superpowers/specs/2026-11-07-m13-auditoria-design.md`).
//!
//! Ferramenta de diagnóstico, **não** jogabilidade (feature `audit`): lê uma `Scene` montada
//! e diz o que a missão exige (`inspect`), se a lógica de objetivos, gatilhos e veredito deixa
//! vencê-la com o jogador invulnerável e teletransportado (`god_mode`) e se a grade de células
//! da IA liga o início aos objetivos e à saída (`reach`, heurística). Só usa a API pública da
//! cena e os caminhos normais de dano.

use std::collections::VecDeque;

use crate::ai::path::Cell;
use crate::behaviour::{Outcome, enemy, interact};
use crate::damage::Pending;
use crate::entity::{Behaviour, Entity};
use crate::jake::zone;
use crate::physics::Rect;
use crate::scene::{CELL, Scene, TIMER_TICKS_PER_SECOND};
use crate::verdict::{OBJ_ACTIVATE, OBJ_DESTROY};

/// Como o objetivo se cumpre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum How {
    /// Item ou peça do carro: o jogador o apanha (fica ligado).
    Collect,
    /// Interruptor: CIMA diante dele.
    Switch,
    /// Zona de proximidade: o jogador dentro dela até ligar.
    Zone,
    /// Destruir: a vida chega a 0.
    Destroy,
    /// Barril "acionar": liga ao explodir.
    Explode,
    /// Tipo sem comportamento (`Static`): o veredito do porte o ignora (Ruling de
    /// `logica-fase.md`); no original travaria a fase.
    Unimplemented,
}

impl How {
    pub fn name(self) -> &'static str {
        match self {
            How::Collect => "coletar",
            How::Switch => "acionar",
            How::Zone => "zona",
            How::Destroy => "destruir",
            How::Explode => "explodir",
            How::Unimplemented => "NÃO IMPLEMENTADO",
        }
    }
}

/// Um objetivo da missão (fora a saída e o jogador).
#[derive(Debug, Clone)]
pub struct Objective {
    /// Índice em `Scene::entities`.
    pub index: usize,
    pub kind: u32,
    /// Bits `+0x1ac`.
    pub bits: i32,
    /// Variante do comportamento (`Pickup`, `Switch`…).
    pub behaviour: String,
    pub how: How,
    pub at: (i32, i32),
    pub layer: i32,
    /// Observação (quem pode apanhar, antídoto…), se houver.
    pub note: Option<String>,
}

/// Uma porta de célula.
#[derive(Debug, Clone)]
pub struct Door {
    pub cell: (i32, i32, i32),
    /// Começa fechada.
    pub closed: bool,
    /// Chaves pedidas (bit i = chave 62 + i).
    pub keys: i32,
    /// Alguma das chaves pedidas está na fase (ou não pede chave).
    pub key_present: bool,
    /// Gatilhos (entradas 3–4 contam para abrir).
    pub triggers: Vec<(i32, Option<i32>)>,
}

/// O que a missão exige.
#[derive(Debug, Clone)]
pub struct Report {
    /// "Jake", "Garathe" ou "nenhum".
    pub character: String,
    /// Limite de tempo em segundos (campo 1004; 0 = sem).
    pub time_limit: i32,
    pub objectives: Vec<Objective>,
    /// Saídas (tipo 5): posição, camada e se há zona PR 0x40 sobre ela.
    pub exits: Vec<((i32, i32), i32, bool)>,
    pub doors: Vec<Door>,
    /// Chaves (62–67) presentes na fase, bit i = chave 62 + i.
    pub keys: i32,
    /// Objetos com gatilhos para ids que não existem na fase (valem verdadeiro): tipo, id.
    pub dangling: Vec<(u32, i32)>,
    /// Objetos sem comportamento (`Static`) que barram o jogador: tipo, centro, camada.
    pub static_blockers: Vec<(u32, (i32, i32), i32)>,
}

/// Nome da variante do comportamento.
pub fn behaviour_name(b: &Behaviour) -> String {
    let s = format!("{b:?}");
    s.split(['(', ' ', '{']).next().unwrap_or("").to_string()
}

fn take_bit(character: &str) -> i32 {
    if character == "Garathe" { 0x200 } else { 0x100 }
}

fn how(e: &Entity) -> How {
    match &e.behaviour {
        _ if e.objective & OBJ_DESTROY != 0 => How::Destroy,
        Behaviour::Pickup(_) | Behaviour::CarPiece => How::Collect,
        Behaviour::Switch(_) => How::Switch,
        Behaviour::Zone(_) => How::Zone,
        Behaviour::Barrel(_) => How::Explode,
        Behaviour::Static => How::Unimplemented,
        _ => How::Collect,
    }
}

/// O jogador da cena: "Jake", "Garathe" ou "nenhum".
pub fn character(s: &Scene) -> String {
    s.jake
        .as_ref()
        .map_or_else(|| "nenhum".into(), |j| format!("{:?}", j.character()))
}

/// Os objetivos da cena (as saídas não entram).
pub fn objectives(s: &Scene) -> Vec<Objective> {
    let ch = character(s);
    s.entities
        .iter()
        .enumerate()
        .filter(|(_, e)| {
            e.objective & (OBJ_ACTIVATE | OBJ_DESTROY) != 0
                && !matches!(e.behaviour, Behaviour::Exit(_))
        })
        .map(|(index, e)| {
            let how = how(e);
            let mut note = None;
            if how == How::Collect
                && matches!(e.behaviour, Behaviour::Pickup(_))
                && e.flags & take_bit(&ch) == 0
            {
                note = Some(format!("o {ch} não apanha (flags {:#x})", e.flags));
            } else if e.kind == 37 {
                note = Some("antídoto: só com veneno".into());
            }
            Objective {
                index,
                kind: e.kind,
                bits: e.objective,
                behaviour: behaviour_name(&e.behaviour),
                how,
                at: e.center(),
                layer: e.layer,
                note,
            }
        })
        .collect()
}

/// Zona PR 0x40 da camada que cruza `r`.
fn exit_zone(s: &Scene, r: &Rect, layer: i32) -> Option<Rect> {
    s.prop_zones
        .iter()
        .filter(|z| z.layer == layer && z.kind & zone::EXIT != 0)
        .map(|z| z.rect)
        .find(|z| touches(z, r))
}

/// Desigualdades não estritas (`0x0040e4dd`).
fn touches(a: &Rect, b: &Rect) -> bool {
    a.x <= b.x + b.w && a.x + a.w >= b.x && a.y <= b.y + b.h && a.y + a.h >= b.y
}

/// O relatório estático da cena.
pub fn inspect(s: &Scene) -> Report {
    let keys = s
        .entities
        .iter()
        .filter(|e| e.alive && (62..=67).contains(&e.kind))
        .fold(0, |m, e| m | 1 << (e.kind - 62));
    let exits = s
        .entities
        .iter()
        .filter(|e| matches!(e.behaviour, Behaviour::Exit(_)))
        .map(|e| {
            let zone = e
                .bounds()
                .is_some_and(|b| exit_zone(s, &b, e.layer).is_some());
            (e.center(), e.layer, zone)
        })
        .collect();
    let doors = s
        .cell_doors
        .iter()
        .map(|d| Door {
            cell: (d.origin.0 / CELL, d.origin.1 / CELL, d.layer),
            // estado `+0x54` 1 = fechada
            closed: d.state == 1,
            keys: d.keys,
            key_present: d.keys == 0 || d.keys & keys != 0,
            triggers: d.triggers.clone(),
        })
        .collect();
    let ids: Vec<i32> = s
        .entities
        .iter()
        .map(|e| e.id)
        .filter(|&i| i != 0)
        .collect();
    let mut dangling: Vec<(u32, i32)> = s
        .entities
        .iter()
        .flat_map(|e| {
            e.triggers
                .iter()
                .filter(|(id, c)| *id != 0 && c.unwrap_or(0) > 0 && !ids.contains(id))
                .map(move |(id, _)| (e.kind, *id))
        })
        .collect();
    dangling.dedup();
    let static_blockers = s
        .entities
        .iter()
        .filter(|e| {
            e.alive
                && e.solid_shapes
                && matches!(e.behaviour, Behaviour::Static)
                && crate::objects::blocks_flags(0, e.flags)
        })
        .map(|e| (e.kind, e.center(), e.layer))
        .collect();
    Report {
        character: character(s),
        time_limit: s.time_limit,
        objectives: objectives(s),
        exits,
        doors,
        keys,
        dangling,
        static_blockers,
    }
}

/// Resultado do modo deus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Win { ticks: u32 },
    Fail(String),
}

/// Tiques máximos para cada passo.
const STEP_TICKS: u32 = 400;
/// Vida mantida no jogador a cada tique.
const GOD_HP: i32 = 10_000;

/// Um tique invulnerável: vida alta (o veneno fica: o antídoto o exige) e cronômetro cheio.
fn god_tick(s: &mut Scene, input: u32) {
    if let Some(j) = s.jake.as_mut() {
        j.health.hp = GOD_HP;
    }
    if s.time_limit > 0 {
        s.timer = s.time_limit * TIMER_TICKS_PER_SECOND;
    }
    s.tick(input);
}

/// Leva o centro da caixa do jogador a `c` na camada `layer`.
pub fn teleport(s: &mut Scene, c: (i32, i32), layer: i32) {
    let Some(j) = s.jake.as_mut() else { return };
    let b = j.body.bounds();
    let (ox, oy) = (b.x - j.body.x, b.y - j.body.y);
    j.set_layer(layer);
    j.body.x = c.0 - b.w / 2 - ox;
    j.body.y = c.1 - b.h / 2 - oy;
    j.body.vx = 0;
    j.body.vy = 0;
    s.set_jake_layer(layer);
}

/// Espera o jogador parar em pé (pode usar a saída), até `n` tiques.
/// O `on_ground` do corpo é o do último tique: depois de um teletransporte, espera ao menos
/// alguns tiques e a velocidade vertical zerar.
fn settle(s: &mut Scene, n: u32) -> bool {
    let ready = |s: &Scene| {
        s.jake
            .as_ref()
            .is_some_and(|j| j.can_use_exit() && j.body.vy == 0)
    };
    for i in 0..n {
        if i >= 3 && ready(s) {
            return true;
        }
        god_tick(s, 0);
    }
    ready(s)
}

/// Põe o jogador em pé diante de `target` (caixa do objeto), sobre a zona PR 0x40, e aperta
/// CIMA. Devolve a falha, se houver.
fn press_up_at(s: &mut Scene, target: Rect, layer: i32) -> Result<(), String> {
    let Some(z) = exit_zone(s, &target, layer) else {
        return Err("sem zona PR 0x40 sobre o objeto".into());
    };
    // centro na interseção da zona com o objeto, no meio da altura da zona; se der, a caixa
    // também cruza as zonas-objetivo vizinhas (que desligam quando ele sai delas)
    let w = s.jake.as_ref().map_or(20, |j| j.body.bounds().w);
    let (mut x0, mut x1) = (z.x.max(target.x), (z.x + z.w).min(target.x + target.w));
    for zr in s
        .entities
        .iter()
        .filter(|e| {
            e.layer == layer
                && e.objective & OBJ_ACTIVATE != 0
                && matches!(e.behaviour, Behaviour::Zone(_))
        })
        .filter_map(Entity::bounds)
    {
        // centros c com a caixa [c − w/2, c + w/2] cruzando zr e z (estrito)
        let lo = (zr.x - w / 2 + 1).max(z.x - w / 2 + 1);
        let hi = (zr.x + zr.w + w / 2 - 1).min(z.x + z.w + w / 2 - 1);
        if lo <= hi {
            (x0, x1) = (lo, hi);
        }
    }
    let cx = (x0 + x1) / 2;
    let cy = z.y + z.h / 2;
    // um teletransporte sob a saída (H_J_001) é por onde o jogador chega: o passageiro
    // recém-chegado parado nele não é levado de volta (`last_on`); aqui, sem a viagem, o
    // teletransporte só fica em espera
    let tick = s.tick_count;
    for e in s.entities.iter_mut().filter(|e| e.layer == layer) {
        let feet = Rect { h: z.h + 64, ..z };
        let near = e.bounds().is_some_and(|b| touches(&b, &feet));
        if let Behaviour::Teleporter(t) = &mut e.behaviour
            && near
        {
            t.until = tick + 10_000;
        }
    }
    // personagens da IA por perto o empurram: tenta, e depois os derruba (dano normal)
    let mut last = None;
    for attempt in 0..5 {
        if attempt > 0 {
            calm_ai(s, (cx, cy), layer);
        }
        teleport(s, (cx, cy), layer);
        if !settle(s, 80) {
            last = Some(format!(
                "não ficou em pé diante do objeto ({:?})",
                s.jake.as_ref().map(|j| j.state())
            ));
            continue;
        }
        let Some(jb) = s.jake.as_ref().map(|j| j.body.bounds()) else {
            return Err("sem jogador".into());
        };
        // a zona vale pela sobreposição estrita (`zones_under`), o objeto pela não estrita
        if touches(&jb, &target) && jb.overlaps(&z) {
            last = None;
            break;
        }
        last = Some(format!(
            "em pé fora do alcance do objeto (jogador {jb:?}, objeto {target:?}, zona {z:?})"
        ));
    }
    if let Some(e) = last {
        return Err(e);
    }
    s.events.clear();
    god_tick(s, crate::input::UP);
    let refused = s
        .events
        .iter()
        .any(|e| e.sound_id() == Some(crate::behaviour::exit::SOUND_OBJECTIVES));
    god_tick(s, 0);
    if refused {
        return Err(OBJECTIVES.into());
    }
    Ok(())
}

/// Mata os inimigos sem objetivo a menos de 300 px de `at` na camada, pelo dano normal (os
/// personagens da IA e a Morte empurram o jogador invulnerável para fora da zona).
fn calm_ai(s: &mut Scene, at: (i32, i32), layer: i32) {
    let lethal = Pending {
        amount: 1000,
        ..Default::default()
    };
    for e in s.entities.iter_mut().filter(|e| {
        let c = e.center();
        e.alive
            && e.layer == layer
            && e.objective == 0
            && (c.0 - at.0).abs() < 300
            && (c.1 - at.1).abs() < 300
    }) {
        enemy::hurt(e, &lethal, true);
    }
    for _ in 0..3 {
        god_tick(s, 0);
    }
}

/// Falha da saída: choque de "objetivos incompletos" (3772).
const OBJECTIVES: &str = "a saída recusou (choque: objetivos incompletos)";

/// Estado "ligado" do objeto de id `id` (`None` se não existe), como as listas de gatilhos o
/// leem.
fn on_of(s: &Scene, id: i32) -> Option<bool> {
    s.entities.iter().find(|e| e.id == id).map(|e| e.on)
}

/// Liga os alvos dos gatilhos do objeto `index` que precisam estar ligados (dezena 1 do
/// código), até ficarem satisfeitos: acordar o Bruiser, armar o pistão… `depth` limita a
/// cadeia.
fn wake(s: &mut Scene, index: usize, depth: u32) -> Result<(), String> {
    let triggers = s.entities[index].triggers.clone();
    let ok = |s: &Scene| crate::triggers::satisfied(&triggers, &|id| on_of(s, id));
    if ok(s) || depth == 0 {
        return Ok(());
    }
    // a primeira entrada contada cujos grupos "ligado" podem ser cumpridos
    for &(_, code) in &triggers {
        let mut c = code.unwrap_or(0);
        while c > 0 {
            let (entry, want_on) = ((c % 100) % 10, (c % 100) / 10 == 1);
            c /= 100;
            let Some(&(id, _)) = usize::try_from(entry)
                .ok()
                .and_then(|n| n.checked_sub(1))
                .and_then(|n| triggers.get(n))
            else {
                continue;
            };
            let Some(t) = s.entities.iter().position(|e| e.id == id && e.id != 0) else {
                continue;
            };
            if want_on && !s.entities[t].on {
                wake(s, t, depth - 1)?;
                let e = &s.entities[t];
                let target = Objective {
                    index: t,
                    kind: e.kind,
                    bits: e.objective,
                    behaviour: behaviour_name(&e.behaviour),
                    how: match &e.behaviour {
                        Behaviour::Barrel(_) => How::Explode,
                        b => how(&Entity {
                            objective: 0,
                            behaviour: b.clone(),
                            ..e.clone()
                        }),
                    },
                    at: e.center(),
                    layer: e.layer,
                    note: None,
                };
                complete(s, &target)
                    .map_err(|why| format!("gatilho do tipo {}: {why}", s.entities[index].kind))?;
            }
        }
        if ok(s) {
            return Ok(());
        }
    }
    if ok(s) {
        Ok(())
    } else {
        Err(format!(
            "gatilhos do tipo {} em {:?} não se cumprem: {:?}",
            s.entities[index].kind,
            s.entities[index].center(),
            triggers
        ))
    }
}

/// Cumpre um objetivo; `Err` com o motivo se não deu.
fn complete(s: &mut Scene, o: &Objective) -> Result<(), String> {
    let done = |s: &Scene| {
        let e = &s.entities[o.index];
        match o.how {
            How::Destroy => !e.alive || e.health <= 0,
            _ => e.on,
        }
    };
    if done(s) {
        return Ok(());
    }
    match o.how {
        How::Unimplemented => Ok(()),
        How::Collect => {
            // o antídoto é recusado sem veneno (`0x40b7b6`): antes, apanha um veneno da fase
            if o.kind == 37 && s.jake.as_ref().is_some_and(|j| j.poison == 0) {
                let Some(p) = s
                    .entities
                    .iter()
                    .position(|e| e.alive && matches!(e.kind, 33 | 35))
                else {
                    return Err(format!(
                        "antídoto em {:?} recusado: o jogador não está envenenado e a fase não \
                         tem veneno (33/35)",
                        o.at
                    ));
                };
                let e = &s.entities[p];
                let poison = Objective {
                    index: p,
                    kind: e.kind,
                    bits: 0,
                    behaviour: behaviour_name(&e.behaviour),
                    how: How::Collect,
                    at: e.center(),
                    layer: e.layer,
                    note: None,
                };
                complete(s, &poison)?;
            }
            for _ in 0..STEP_TICKS / 4 {
                let e = &s.entities[o.index];
                let (c, layer) = (e.center(), e.layer);
                teleport(s, c, layer);
                god_tick(s, 0);
                if done(s) {
                    return Ok(());
                }
            }
            Err(format!(
                "não apanhou o tipo {} em {:?}{}",
                o.kind,
                o.at,
                o.note.as_ref().map_or(String::new(), |n| format!(" ({n})"))
            ))
        }
        How::Switch => {
            for _ in 0..3 {
                let e = &s.entities[o.index];
                let b = e.bounds().ok_or("interruptor sem caixa")?;
                press_up_at(s, b, e.layer)?;
                if done(s) {
                    return Ok(());
                }
                for _ in 0..3 {
                    god_tick(s, 0);
                }
            }
            Err(format!("o interruptor em {:?} não ligou", o.at))
        }
        How::Zone => {
            let e = &s.entities[o.index];
            let b = e.bounds().ok_or("zona sem caixa")?;
            let layer = e.layer;
            for _ in 0..STEP_TICKS {
                teleport(s, (b.x + b.w / 2, b.y + b.h / 2), layer);
                god_tick(s, 0);
                if done(s) {
                    return Ok(());
                }
            }
            Err(format!("a zona em {:?} não ligou", o.at))
        }
        How::Destroy | How::Explode => {
            // a bala do jogador tem natureza "explosão" contra o barril (`(19, 0)` do
            // `DAMAGE.TBL`, `barril-fogo-cerca.md`)
            let lethal = Pending {
                amount: 1000,
                explosion: o.how == How::Explode,
                ..Default::default()
            };
            // o Bruiser adormecido não sente dano até os gatilhos o acordarem
            if matches!(s.entities[o.index].behaviour, Behaviour::Bruiser(_)) {
                wake(s, o.index, 3)?;
            }
            for _ in 0..STEP_TICKS {
                let e = &mut s.entities[o.index];
                let (c, layer) = (e.center(), e.layer);
                if e.alive && !enemy::hurt(e, &lethal, true) && !interact::hurt(e, &lethal) {
                    return Err(format!("o tipo {} não aceita dano", o.kind));
                }
                // perto do alvo, mas acima dele (fora do caminho)
                teleport(s, (c.0, c.1 - 64), layer);
                god_tick(s, 0);
                if done(s) {
                    return Ok(());
                }
            }
            Err(format!("o tipo {} em {:?} não morreu", o.kind, o.at))
        }
    }
}

/// Modo deus: cumpre cada objetivo por ação direta e usa a saída. Ordem: itens,
/// interruptores, destruir, barris e, por último, as zonas (podem desligar).
pub fn god_mode(s: &mut Scene) -> Verdict {
    if s.jake.is_none() {
        return Verdict::Fail("sem personagem controlável".into());
    }
    for _ in 0..5 {
        god_tick(s, 0);
    }
    let mut objs = objectives(s);
    objs.sort_by_key(|o| match o.how {
        How::Collect => 0,
        How::Switch => 1,
        How::Destroy => 2,
        How::Explode => 3,
        How::Zone => 4,
        How::Unimplemented => 5,
    });
    for o in &objs {
        if let Err(e) = complete(s, o) {
            return Verdict::Fail(e);
        }
        if let Some(out) = s.outcome {
            return Verdict::Fail(format!("partida decidida no meio: {out:?}"));
        }
    }
    // os já cumpridos continuam? (zonas que desligam, interruptores que voltam)
    let undone: Vec<String> = objs
        .iter()
        .filter(|o| {
            let e = &s.entities[o.index];
            match o.how {
                How::Destroy => e.alive && e.health > 0,
                How::Unimplemented => false,
                _ => !e.on,
            }
        })
        .map(|o| format!("{} {}", o.how.name(), o.kind))
        .collect();
    let Some(exit) = s
        .entities
        .iter()
        .position(|e| matches!(e.behaviour, Behaviour::Exit(_)) && e.alive)
    else {
        return Verdict::Fail("sem saída".into());
    };
    let Some(b) = s.entities[exit].bounds() else {
        return Verdict::Fail("saída sem caixa".into());
    };
    let layer = s.entities[exit].layer;
    match press_up_at(s, b, layer) {
        Err(e) if e == OBJECTIVES && !undone.is_empty() => {
            return Verdict::Fail(format!("{e}; desfeitos antes dela: {}", undone.join(", ")));
        }
        Err(e) => return Verdict::Fail(format!("saída: {e}")),
        Ok(()) => {}
    }
    if !s.entities[exit].on {
        return Verdict::Fail(format!(
            "CIMA diante da saída não a ligou (estado {:?})",
            s.jake.as_ref().map(|j| j.state())
        ));
    }
    for _ in 0..80 {
        god_tick(s, 0);
        if s.outcome.is_some() {
            break;
        }
    }
    match s.outcome {
        Some(Outcome::Victory) => Verdict::Win {
            ticks: s.tick_count,
        },
        Some(o) => Verdict::Fail(format!("veredito {o:?}")),
        None => Verdict::Fail("sem veredito 80 tiques depois da saída".into()),
    }
}

/// Alcance pela grade da IA: avisos para objetivos e saídas fora do alcance a partir do
/// início do jogador. Movimentos: cair, andar, trocar de camada nas zonas PR 1/2, os
/// teletransportes (46) até os ids da lista deles (em qualquer camada) e — otimista, no lugar dos pulos que a
/// grade não tem — subir a qualquer célula sem teto (`CLOSED_ABOVE`). As portas de célula com
/// a chave na fase (ou sem chave) contam como abertas. Aviso não é falha: a grade é grossa
/// (uma célula de 128 px) e não vê caixas internas.
pub fn reach(s: &Scene) -> Vec<String> {
    let Some(j) = s.jake.as_ref() else {
        return vec!["sem personagem".into()];
    };
    let mut grid = s.ai_grid().clone();
    let keys = inspect(s).keys;
    for d in &s.cell_doors {
        if d.keys == 0 || d.keys & keys != 0 {
            let c = grid.cell_of(d.origin, d.layer);
            if let Some(ci) = grid.get_mut(c) {
                ci.door_closed = false;
            }
        }
    }
    // teletransporte: célula dele → células dos destinos (mesma camada)
    let g = &grid;
    let jumps: Vec<(Cell, Cell)> = s
        .entities
        .iter()
        .filter(|e| matches!(e.behaviour, Behaviour::Teleporter(_)))
        .flat_map(|e| {
            let from = g.cell_of(e.center(), e.layer);
            e.triggers
                .iter()
                .filter(|(id, _)| *id != 0)
                .filter_map(|(id, _)| s.entities.iter().find(|d| d.id == *id))
                .map(move |d| (from, g.cell_of(d.center(), d.layer)))
                .collect::<Vec<_>>()
        })
        .collect();
    let b = j.body.bounds();
    let start = grid.cell_of((b.x + b.w / 2, b.y + b.h / 2), j.layer());
    let (gx, gy, gz) = grid.size;
    let idx = |c: Cell| (c.0 + c.1 * gx + c.2 * gx * gy) as usize;
    let mut seen = vec![false; (gx * gy * gz) as usize];
    let mut q = VecDeque::new();
    if grid.get(start).is_some() {
        seen[idx(start)] = true;
        q.push_back(start);
    }
    while let Some(c) = q.pop_front() {
        let climb = grid
            .get(c)
            .filter(|ci| ci.bits & crate::ai::path::bits::CLOSED_ABOVE == 0)
            .map(|_| (c.0, c.1 - 1, c.2))
            .filter(|&n| grid.get(n).is_some());
        let next = (1..=6)
            .filter_map(|d| grid.allowed(c, d))
            .chain(climb)
            .chain(jumps.iter().filter(|(f, _)| *f == c).map(|(_, t)| *t));
        for n in next.collect::<Vec<_>>() {
            if !seen[idx(n)] {
                seen[idx(n)] = true;
                q.push_back(n);
            }
        }
    }
    let ok = |p: (i32, i32), layer: i32| {
        let c = grid.cell_of(p, layer);
        grid.get(c).is_some() && seen[idx(c)]
    };
    let mut out = Vec::new();
    for o in objectives(s) {
        if o.how != How::Unimplemented && !ok(o.at, o.layer) {
            out.push(format!(
                "{} {} em {:?} camada {} fora do alcance",
                o.how.name(),
                o.kind,
                o.at,
                o.layer
            ));
        }
    }
    for e in s
        .entities
        .iter()
        .filter(|e| matches!(e.behaviour, Behaviour::Exit(_)))
    {
        if !ok(e.center(), e.layer) {
            out.push(format!(
                "saída em {:?} camada {} fora do alcance",
                e.center(),
                e.layer
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviour::test_support::logic;

    fn with(kind: u32, objective: i32, behaviour: Behaviour) -> Entity {
        let mut e = logic(kind, 10, 10, &[]);
        e.objective = objective;
        e.behaviour = behaviour;
        e
    }

    /// Cada objetivo pelo modo de cumprir; a saída e quem não tem objetivo ficam de fora; o
    /// tipo sem comportamento aparece como não implementado.
    #[test]
    fn objectives_are_classified_by_behaviour() {
        let mut s = Scene::empty(256, 256);
        s.entities = vec![
            with(5, OBJ_ACTIVATE, Behaviour::Exit(Default::default())),
            with(80, OBJ_ACTIVATE, Behaviour::Pickup(Default::default())),
            with(9, OBJ_ACTIVATE, Behaviour::CarPiece),
            with(74, OBJ_ACTIVATE, Behaviour::Switch(Default::default())),
            with(73, OBJ_ACTIVATE, Behaviour::Zone(Default::default())),
            with(
                28,
                OBJ_ACTIVATE,
                Behaviour::Barrel(crate::behaviour::barrel::BarrelState::new(1, vec![])),
            ),
            with(7, OBJ_DESTROY, Behaviour::Static),
            with(13, OBJ_ACTIVATE, Behaviour::Static),
            with(32, 1, Behaviour::Pickup(Default::default())),
        ];
        let got: Vec<(u32, How)> = objectives(&s).iter().map(|o| (o.kind, o.how)).collect();
        assert_eq!(
            got,
            vec![
                (80, How::Collect),
                (9, How::Collect),
                (74, How::Switch),
                (73, How::Zone),
                (28, How::Explode),
                (7, How::Destroy),
                (13, How::Unimplemented),
            ]
        );
        assert_eq!(behaviour_name(&s.entities[3].behaviour), "Switch");
    }

    /// Sem jogador, o modo deus falha logo; o relatório diz "nenhum".
    #[test]
    fn no_player_fails() {
        let mut s = Scene::empty(256, 256);
        assert_eq!(inspect(&s).character, "nenhum");
        assert!(matches!(god_mode(&mut s), Verdict::Fail(_)));
        assert_eq!(reach(&s), vec!["sem personagem".to_string()]);
    }
}
