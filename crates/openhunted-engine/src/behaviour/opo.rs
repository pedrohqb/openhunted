// SPDX-License-Identifier: GPL-3.0-or-later
//! Obstáculos `OPO` (tipos 68–72, inicialização `0x0040f324`; `docs/re/objetos.md`): não são
//! plataformas. O 68 e o 69 (poça de veneno) só animam em laço (`0x0040f459`); o 70 (saliência
//! que goteja, `0x0040f46b`) anima e solta gotas (71, `0x0040f60a`); o 72 é o vapor de veneno
//! (`0x0040f801`), que liga e desliga pelos temporizadores e pelos gatilhos. O 68, o 69, o 70 e
//! o 72 têm a rotina `+0x1c8` `0x0040fa16`: a cada `+0x260` tiques (`values[0]`, padrão 10) de
//! `0x4879c4`, todo objeto presente na mesma camada que cruza o retângulo do quadro de
//! referência do rótulo atual (`0x0043585c`) leva o dano da linha do tipo
//! (`0x0043d629(obj, outro, 0)`: veneno, 1; o vapor, 5); um jogador (tipo 1/2) fica envenenado
//! (`+0x270 = 1`) e, se ainda não estava e é humano (`+0x19c ≠ 0`), fala "Oh no it's poison"
//! (`0x0040b9eb(…, 0xe85, 0xe99, 200, 200)`). O vapor desligado (estado 2) não testa.

use super::{Ctx, Event, Hazard, Out};
use crate::damage::{Geometry, Pending, Strike};
use crate::entity::{Behaviour, Entity};
use crate::physics::{Body, Rect, step};
use crate::sound::Cue;
use openhunted_formats::Animation;

/// Período padrão do teste (`+0x260 = 10` se a fase deu 0, `0x0040f3a7`).
pub const PERIOD: i32 = 10;
/// Fala do Jake ao se envenenar (`0xe85`; a do Garathe é 3737, `0xe99`).
pub const SPEECH: u16 = 3717;
/// Prioridade da fala (`0x0040b9eb(…, 200, 200)`).
pub const SPEECH_PRIORITY: u16 = 200;
/// A gota (`0x0040f4b1`).
pub const DROP: u32 = 71;
/// O vapor de veneno.
pub const STEAM: u32 = 72;
/// Largura de desenho da gota: escala `8 / largura` do quadro de referência (`0x0040f4ee`).
pub const DROP_WIDTH: i32 = 8;
/// A gota desce 17 px ao se soltar (`0x0040f661`, `+= 0xf`, `+= 2`).
pub const DROP_FALL: i32 = 17;
/// Estados da gota (`+0x278`): formando (rótulo 71), caindo (171), respingo (271).
pub const DROP_FORMING: i32 = 0;
pub const DROP_FALLING: i32 = 100;
pub const DROP_SPLASH: i32 = 200;
/// Estados do vapor (`+0x278`): 1 ligado (rótulo 72, fere), 2 desligado (272), 3 ligando
/// (172 de trás para a frente), 4 desligando (172).
pub const STEAM_ON: i32 = 1;
pub const STEAM_OFF: i32 = 2;
pub const STEAM_STARTING: i32 = 3;
pub const STEAM_ENDING: i32 = 4;
/// Rótulo por estado do vapor (tabela `0x00472070`).
pub const STEAM_LABELS: [u16; 5] = [0, 72, 272, 172, 172];
/// "poison steam starting" (`0xc60`) e "poison steam ending" (`0xc62`), posicionais.
pub const SOUND_STARTING: u16 = 3168;
pub const SOUND_ENDING: u16 = 3170;

/// A parte de cada tipo.
#[derive(Debug, Clone)]
pub enum Part {
    /// Poça (68, 69): só o teste.
    Puddle,
    /// Saliência (70): `+0x264` (`values[1]`, base do intervalo entre gotas; 0 = não
    /// goteja), `+0x27c` (`values[7]`, tiques até a próxima) e as caixas do `OPO.BRD` que a
    /// gota recebe.
    Ledge {
        drip: i32,
        timer: i32,
        drop_boxes: Vec<Rect>,
    },
    /// Gota (71).
    Drop(DropState),
    /// Vapor (72).
    Steam(SteamState),
}

#[derive(Debug, Clone, Default)]
pub struct DropState {
    /// `+0x278`.
    pub state: i32,
    /// Quadro no rótulo (`+0x184`).
    pub offset: i32,
    /// Velocidade vertical em 1/8 px.
    pub vy: i32,
}

#[derive(Debug, Clone, Default)]
pub struct SteamState {
    /// `+0x278` (0 até a primeira chamada, feita já na inicialização, `0x0040f450`).
    pub state: i32,
    /// `+0x27c`.
    pub timer: i32,
    /// Quadro no rótulo (`+0x184`; de trás para a frente no estado 3, até −1).
    pub offset: i32,
    /// `values[1]` (`+0x264`): começa ligado.
    pub starts_on: bool,
    /// `values[2]` (`+0x268`): tiques desligado; `values[3]` (`+0x26c`): ligado.
    pub off_ticks: i32,
    pub on_ticks: i32,
    /// `values[4]` (`+0x270`): volta a ligar; com 0 liga uma vez só (bit 0x40 de `+0x178`).
    pub repeat: bool,
    pub fired: bool,
}

#[derive(Debug, Clone)]
pub struct OpoState {
    /// Retângulo do quadro de referência, relativo à posição (`0x0043585c`).
    pub rect: Rect,
    /// `+0x260`: período em tiques.
    pub period: i32,
    pub part: Part,
}

impl OpoState {
    pub fn new(rect: Rect, value0: i32) -> Self {
        Self {
            rect,
            period: if value0 != 0 { value0 } else { PERIOD },
            part: Part::Puddle,
        }
    }

    /// A saliência que goteja (70), com os `values` da fase e as caixas do `OPO.BRD`.
    pub fn ledge(rect: Rect, values: &[i32], drop_boxes: Vec<Rect>) -> Self {
        let v = |i: usize| values.get(i).copied().unwrap_or(0);
        Self {
            part: Part::Ledge {
                drip: v(1),
                timer: v(7),
                drop_boxes,
            },
            ..Self::new(rect, v(0))
        }
    }

    /// O vapor (72): `+0x278 = 0` e a rotina chamada no primeiro tique (`0x0040f43d`).
    pub fn steam(values: &[i32]) -> Self {
        let v = |i: usize| values.get(i).copied().unwrap_or(0);
        Self {
            part: Part::Steam(SteamState {
                starts_on: v(1) != 0,
                off_ticks: v(2),
                on_ticks: v(3),
                repeat: v(4) != 0,
                ..SteamState::default()
            }),
            ..Self::new(
                Rect {
                    x: 0,
                    y: 0,
                    w: 0,
                    h: 0,
                },
                v(0),
            )
        }
    }
}

/// O dano da linha do tipo `kind` contra `victim` (`0x0043d629(opo, obj, 0)`), com o
/// envenenamento do jogador (`+0x270 = 1`, `0x0040fb2e`).
pub fn damage(kind: u32, victim: u32) -> Pending {
    let mut p = Strike {
        attacker: kind,
        variant: 0,
    }
    .row(victim)
    .map(|r| {
        r.pending(Geometry {
            vel: (0, 0),
            toward: (0, 0),
        })
    })
    .unwrap_or_default();
    p.poisons = matches!(victim, 1 | 2);
    p
}

/// Primeiro quadro e comprimento do rótulo `id` (`+0x18c`, `+0x188`: os quadros até o
/// início de outro rótulo ou o fim da animação, `0x446fda`).
pub fn label_span(anim: &Animation, id: u16) -> Option<(usize, i32)> {
    let start = usize::from(anim.labels.iter().find(|l| l.id == id)?.frame);
    let end = anim
        .labels
        .iter()
        .map(|l| usize::from(l.frame))
        .filter(|&f| f > start)
        .min()
        .unwrap_or(anim.frames.len())
        .min(anim.frames.len());
    Some((start, end.saturating_sub(start).max(1) as i32))
}

/// Mostra o quadro `offset` do rótulo `id` (quadro absoluto, como `+0x190`: fora do rótulo
/// mostra o vizinho, como o original no primeiro tique do estado 3).
fn show(e: &mut Entity, id: u16, offset: i32) {
    let Some(sp) = e.sprite.as_mut() else {
        return;
    };
    let Some((start, _)) = label_span(&sp.anim, id) else {
        return;
    };
    let f = (start as i32 + offset).clamp(0, sp.anim.frames.len() as i32 - 1);
    sp.player = crate::AnimPlayer::from_frame(&sp.anim, f as usize);
}

/// Retângulo do primeiro quadro do rótulo `id` na escala, relativo à posição (`0x0043585c`).
pub fn label_rect(e: &Entity, id: u16) -> Option<Rect> {
    let sp = e.sprite.as_ref()?;
    let (start, _) = label_span(&sp.anim, id)?;
    let f = sp.anim.frames.get(start)?;
    let (n, d) = sp.scale;
    let sc = |v: i16| (i64::from(v) * i64::from(n) / i64::from(d.max(1))) as i32;
    Some(Rect {
        x: sc(f.x),
        y: sc(f.y),
        w: sc(f.width),
        h: sc(f.height),
    })
}

pub fn tick(e: &mut Entity, ctx: &mut Ctx, out: &mut Out) {
    let Behaviour::Opo(st) = &e.behaviour else {
        return;
    };
    match &st.part {
        Part::Puddle => {}
        Part::Ledge { .. } => drip(e, ctx, out),
        Part::Drop(_) => return drop_tick(e, ctx, out),
        Part::Steam(_) => steam_tick(e, ctx, out),
    }
    let Behaviour::Opo(st) = &e.behaviour else {
        return;
    };
    let rect = match &st.part {
        // o vapor desligado não testa (`0x0040fa49`); o retângulo é o do rótulo atual
        Part::Steam(s) if s.state == STEAM_OFF => return,
        Part::Steam(s) => {
            let id = STEAM_LABELS[s.state.clamp(0, 4) as usize];
            match label_rect(e, id) {
                Some(r) => r,
                None => return,
            }
        }
        _ => st.rect,
    };
    poison_test(e, rect, st.period, ctx, out);
}

/// O teste `0x0040fa16`.
fn poison_test(e: &Entity, rect: Rect, period: i32, ctx: &Ctx, out: &mut Out) {
    if period > 0 && ctx.tick % period as u32 != 0 {
        return;
    }
    let r = Rect {
        x: e.x + rect.x,
        y: e.y + rect.y,
        ..rect
    };
    for (k, j, layer) in ctx.players() {
        if layer != e.layer || !j.is_active() || !j.body.bounds().overlaps(&r) {
            continue;
        }
        let ch = j.character();
        if j.poison == 0 {
            out.events
                .push(Event::Sound(Cue::ui(ch.speech(SPEECH), SPEECH_PRIORITY)));
        }
        out.hurt_player(k, &damage(e.kind, ch.kind()));
    }
    // os demais objetos (barris, personagens da IA): a linha de vítima qualquer, e o
    // personagem da IA também se envenena (sem fala: `+0x19c = 0`)
    out.hazards.push(Hazard {
        rect: r,
        layer: e.layer,
        damage: Pending {
            poisons: true,
            ..damage(e.kind, 0)
        },
        source: None,
        from: 0,
    });
}

/// A saliência solta uma gota quando o temporizador vence (`0x0040f46b`).
fn drip(e: &mut Entity, ctx: &mut Ctx, out: &mut Out) {
    let rect = match &e.behaviour {
        Behaviour::Opo(st) => st.rect,
        _ => return,
    };
    let Behaviour::Opo(OpoState {
        part: Part::Ledge {
            drip,
            timer,
            drop_boxes,
        },
        ..
    }) = &mut e.behaviour
    else {
        return;
    };
    if *drip == 0 {
        return;
    }
    let t = *timer;
    *timer -= 1;
    if t > 0 {
        return;
    }
    let Some(sp) = e.sprite.as_ref() else {
        return;
    };
    let anim = sp.anim.clone();
    let fw = label_span(&anim, DROP as u16)
        .and_then(|(s, _)| anim.frames.get(s))
        .map_or(1, |f| i32::from(f.width).max(1));
    // `+0x194 = rand & 1 ? 0 : 1`; x sorteado na largura que sobra; y no meio da saliência
    let mirror = ctx.rng.next_u16() & 1 == 0;
    let room = (rect.w - DROP_WIDTH).max(1);
    let x = e.x + ctx.rng.below(room as u32) as i32;
    let y = e.y + rect.h / 2;
    let v = *drip;
    *timer = ctx.rng.below((v / 4 + 1).max(1) as u32) as i32 + v - v / 8;
    let boxes = drop_boxes.clone();
    e.flags |= 0x20;
    let player = crate::AnimPlayer::at_label(&anim, DROP as u16)
        .unwrap_or_else(|| crate::AnimPlayer::from_frame(&anim, 0));
    out.spawn.push(Entity {
        kind: DROP,
        x,
        y,
        // leitor 0x80000, `0x81` (`0x40b2c8`), 0x8 (`0x40f38c`), 0x4 (`0x40f40b`), 0x10
        flags: 0x8009d,
        layer: e.layer,
        id: 0,
        values: Vec::new(),
        triggers: Vec::new(),
        objective: 0,
        health: 0,
        armor: 0,
        boxes,
        solid_shapes: false,
        on: false,
        alive: true,
        sprite: Some(crate::Sprite {
            layer: crate::Layer::Object,
            anim,
            player,
            x,
            y,
            frozen: true,
            scale: (DROP_WIDTH, fw),
            mirror,
        }),
        behaviour: Behaviour::Opo(OpoState {
            rect: Rect {
                x: 0,
                y: 0,
                w: 0,
                h: 0,
            },
            period: PERIOD,
            part: Part::Drop(DropState::default()),
        }),
    });
}

/// A gota (`0x0040f60a`): forma-se no rótulo 71 (anima em laço; ao voltar ao quadro 0 cai),
/// cai com gravidade no 171 e, ao tocar algo que não seja outra gota nem só-de-contato, ou
/// ao sair da fase, respinga (271) envenenando o jogador tocado; some ao fim do respingo.
fn drop_tick(e: &mut Entity, ctx: &mut Ctx, out: &mut Out) {
    let len = |e: &Entity, id: u16| {
        e.sprite
            .as_ref()
            .and_then(|sp| label_span(&sp.anim, id))
            .map_or(1, |(_, n)| n)
    };
    let Behaviour::Opo(OpoState {
        part: Part::Drop(st),
        ..
    }) = &e.behaviour
    else {
        return;
    };
    let old = st.state;
    let n = len(e, (DROP as i32 + old) as u16);
    let mut d = st.clone();
    // `0x406c71(obj, 1)`
    d.offset += 1;
    if d.offset >= n {
        d.offset = 0;
    }
    match d.state {
        DROP_FORMING if d.offset == 0 => {
            d.state = DROP_FALLING;
            e.flags &= !0x28;
            e.y += DROP_FALL;
        }
        DROP_FALLING => {
            if let Some(hit) = fall(e, &mut d, ctx) {
                splash(e, hit, ctx, out);
                d.state = DROP_SPLASH;
                d.vy = 0;
                e.flags |= 0x28;
            }
        }
        DROP_SPLASH if d.offset == 0 => {
            e.alive = false;
            return;
        }
        _ => {}
    }
    if d.state != old {
        d.offset = 0;
    }
    let (state, offset) = (d.state, d.offset);
    if let Behaviour::Opo(OpoState {
        part: Part::Drop(st),
        ..
    }) = &mut e.behaviour
    {
        *st = d;
    }
    show(e, (DROP as i32 + state) as u16, offset);
}

/// O que a gota tocou ao cair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Hit {
    /// Um jogador de `Ctx::players`.
    Player(Option<usize>),
    /// Um sólido das células (sem vítima).
    Scenery,
    /// Um objeto sólido (barril, porta…): a cena fere quem cruza a caixa.
    Object,
    /// Saiu da fase.
    Outside,
}

/// Um tique de queda contra as células (e, na camada do Jake, os objetos sólidos para ele);
/// `Some` = tocou algo.
fn fall(e: &mut Entity, d: &mut DropState, ctx: &Ctx) -> Option<Hit> {
    let boxr = e.boxes.first().copied()?;
    let world = if e.layer == ctx.jake_layer {
        ctx.world
    } else {
        ctx.cells_for(e.layer)
    };
    let prev = (e.x, e.y);
    let mut body = Body::new(e.x, e.y, boxr);
    body.vy = d.vy;
    step(&mut body, world, true);
    let want = (d.vy + crate::physics::GRAVITY) >> 3;
    let blocked = body.on_ground || body.y - e.y != want || body.x != e.x;
    e.x = body.x;
    e.y = body.y;
    d.vy = body.vy;
    if let Some(sw) = super::projectile::swept(e, Some(prev)) {
        for (k, j, layer) in ctx.players() {
            if layer == e.layer && j.is_active() && j.body.bounds().overlaps(&sw) {
                return Some(Hit::Player(k));
            }
        }
    }
    if e.x < 0 || e.y < 0 || e.x >= ctx.level.0 || e.y >= ctx.level.1 {
        return Some(Hit::Outside);
    }
    if !blocked {
        return None;
    }
    let near = e.bounds().map(|b| super::grow(b, 1));
    let scenery = near.is_some_and(|b| ctx.cells_for(e.layer).blocked(&b));
    Some(if scenery { Hit::Scenery } else { Hit::Object })
}

/// O respingo: o jogador tocado se envenena (com a fala, se ainda não estava) e leva a linha
/// `(71, tipo, 0)`; um objeto sólido tocado leva a linha de vítima qualquer.
fn splash(e: &Entity, hit: Hit, ctx: &Ctx, out: &mut Out) {
    match hit {
        Hit::Player(k) => {
            let Some((_, j, _)) = ctx.players().find(|(i, _, _)| *i == k) else {
                return;
            };
            let ch = j.character();
            if j.poison == 0 {
                out.events
                    .push(Event::Sound(Cue::ui(ch.speech(SPEECH), SPEECH_PRIORITY)));
            }
            out.hurt_player(k, &damage(DROP, ch.kind()));
        }
        Hit::Object => {
            if let Some(b) = e.bounds() {
                out.hazards.push(Hazard {
                    rect: super::grow(b, 1),
                    layer: e.layer,
                    damage: Pending {
                        poisons: true,
                        ..damage(DROP, 0)
                    },
                    source: None,
                    from: 0,
                });
            }
        }
        Hit::Scenery | Hit::Outside => {}
    }
}

/// O vapor (`0x0040f801`): anima (de trás para a frente ao ligar), lê os gatilhos (as 4
/// entradas, `0x435bd8(+0x11c, 0, 4)`; sem gatilho contado vale verdadeiro) e troca de
/// estado pelos temporizadores; a primeira chamada (estado 0) é a da inicialização.
fn steam_tick(e: &mut Entity, ctx: &Ctx, out: &mut Out) {
    let trig = crate::triggers::satisfied_range(&e.triggers, 0, 4, ctx.on);
    let Behaviour::Opo(OpoState {
        part: Part::Steam(st),
        ..
    }) = &e.behaviour
    else {
        return;
    };
    let mut s = st.clone();
    let len = |id: u16| {
        e.sprite
            .as_ref()
            .and_then(|sp| label_span(&sp.anim, id))
            .map_or(1, |(_, n)| n)
    };
    let mut sounds = Vec::new();
    if s.state == 0 {
        steam_step(&mut s, trig, &len, &mut sounds);
    }
    steam_step(&mut s, trig, &len, &mut sounds);
    for id in sounds {
        out.events.push(Event::Sound(Cue::at(
            id,
            crate::sound::PRIORITY_FRAME,
            (e.x, e.y),
            e.layer,
        )));
    }
    if s.fired {
        e.flags |= 0x40;
    }
    let (label, offset) = (STEAM_LABELS[s.state.clamp(0, 4) as usize], s.offset);
    if let Behaviour::Opo(OpoState {
        part: Part::Steam(st),
        ..
    }) = &mut e.behaviour
    {
        *st = s;
    }
    show(e, label, offset);
}

/// Uma chamada de `0x0040f801` sobre o estado.
fn steam_step(s: &mut SteamState, trig: bool, len: &dyn Fn(u16) -> i32, sounds: &mut Vec<u16>) {
    let label = |st: i32| STEAM_LABELS[st.clamp(0, 4) as usize];
    // `0x406c71`: modo 4 (só para trás, sem laço) no estado 3, senão modo 1 (laço)
    if s.state == STEAM_STARTING {
        s.offset -= 1;
    } else {
        s.offset += 1;
        if s.offset >= len(label(s.state)) {
            s.offset = 0;
        }
    }
    let mut new = s.state;
    if new == 0 {
        if s.starts_on {
            new = STEAM_ON;
            s.timer = s.on_ticks;
        } else {
            new = STEAM_OFF;
            s.timer = s.off_ticks;
        }
    }
    let expired = |t: &mut i32| {
        let old = *t;
        *t -= 1;
        old <= 0
    };
    if new == STEAM_OFF && trig {
        let start = if s.off_ticks == 0 && s.on_ticks == 0 {
            true
        } else if !s.repeat && s.fired {
            false
        } else {
            expired(&mut s.timer)
        };
        if start {
            s.fired = true;
            s.timer = s.on_ticks;
            sounds.push(SOUND_STARTING);
            return set(s, STEAM_STARTING, len);
        }
    }
    if new == STEAM_ON {
        let end = !trig || (s.on_ticks != 0 && expired(&mut s.timer));
        if end {
            s.timer = s.off_ticks;
            sounds.push(SOUND_ENDING);
            return set(s, STEAM_ENDING, len);
        }
    } else if new == STEAM_STARTING && s.offset < 0 {
        new = STEAM_ON;
    } else if new == STEAM_ENDING && s.offset == 0 {
        new = STEAM_OFF;
    }
    set(s, new, len);
}

/// Troca de estado (`0x0040f972`): rótulo novo do quadro 0, ou do fim no estado 3.
fn set(s: &mut SteamState, new: i32, len: &dyn Fn(u16) -> i32) {
    if new == s.state {
        return;
    }
    s.state = new;
    s.offset = if new == STEAM_STARTING {
        len(STEAM_LABELS[3])
    } else {
        0
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviour::Ctx;
    use crate::behaviour::test_support::{ctx, ctx_with_jake, logic};

    fn puddle(x: i32, y: i32) -> Entity {
        let mut e = logic(68, x, y, &[]);
        e.boxes.clear();
        e.behaviour = Behaviour::Opo(OpoState::new(
            Rect {
                x: 0,
                y: 0,
                w: 66,
                h: 8,
            },
            0,
        ));
        e
    }

    #[test]
    fn rows_68_to_72_poison_with_one_and_the_gate_with_five() {
        for k in 68..=71 {
            let p = damage(k, 1);
            assert_eq!((p.amount, p.poison, p.poisons), (1, true, true), "{k}");
        }
        assert_eq!(damage(72, 2).amount, 5);
        assert!(!damage(68, 28).poisons, "só o jogador se envenena");
    }

    #[test]
    fn poisons_whoever_crosses_it_every_ten_ticks() {
        let on = |_: i32| None;
        // Jake de pé com os pés em y + 47: a poça (66 × 8) no chão, sob ele
        let (j, w) = ctx_with_jake(100, 0);
        let b = j.body.bounds();
        let mut e = puddle(b.x - 20, b.y + b.h - 6);
        let mut c = ctx(&j, &w, &on);
        let mut hits = vec![];
        for t in 0..30 {
            c.tick = t;
            let mut out = Out::default();
            tick(&mut e, &mut c, &mut out);
            if !out.jake_damage.is_empty() {
                hits.push(t);
                assert_eq!(out.jake_damage.amount, 1);
                assert!(out.jake_damage.poison && out.jake_damage.poisons);
                assert_eq!(
                    out.events
                        .iter()
                        .filter_map(Event::sound_id)
                        .collect::<Vec<_>>(),
                    vec![SPEECH],
                    "fala com o jogador ainda sem veneno"
                );
                assert_eq!(out.hazards.len(), 1);
            }
        }
        assert_eq!(hits, vec![0, 10, 20]);
        // fora da caixa, ou em outra camada: nada
        let far = puddle(b.x + 200, b.y);
        let mut e2 = far.clone();
        c.tick = 0;
        let mut out = Out::default();
        tick(&mut e2, &mut c, &mut out);
        assert!(out.jake_damage.is_empty());
        let mut e3 = puddle(b.x - 20, b.y + b.h - 6);
        e3.layer = 1;
        let mut out = Out::default();
        tick(&mut e3, &mut c, &mut out);
        assert!(out.jake_damage.is_empty());
    }

    #[test]
    fn already_poisoned_player_is_hurt_without_speech() {
        let on = |_: i32| None;
        let (mut j, w) = ctx_with_jake(100, 0);
        j.poison = 2;
        let b = j.body.bounds();
        let mut e = puddle(b.x, b.y);
        let mut c = ctx(&j, &w, &on);
        let mut out = Out::default();
        tick(&mut e, &mut c, &mut out);
        assert_eq!(out.jake_damage.amount, 1);
        assert!(out.events.is_empty());
    }

    #[test]
    fn hurt_with_poisons_sets_the_poison_to_one() {
        let (mut j, w) = ctx_with_jake(100, 0);
        j.poison = 3;
        j.hurt(damage(68, 1));
        j.tick(0, &w);
        assert_eq!(j.poison, 1, "`+0x270 = 1` (atribui, não soma)");
        let (mut j, w) = ctx_with_jake(100, 0);
        j.hurt(damage(68, 1));
        j.tick(0, &w);
        assert_eq!(j.poison, 1);
    }

    /// Animação com a forma do `OPO.ANM` (rótulos 68, 70, 71, 171, 271, 72, 172, 272 e os
    /// tamanhos dos quadros).
    fn opo_anim() -> std::rc::Rc<Animation> {
        let f = |w: i16, h: i16| openhunted_formats::AnimFrame {
            x: 0,
            y: 0,
            width: w,
            height: h,
            script_offset: 0,
            extra: 0,
        };
        let mut frames = vec![f(78, 10); 12];
        frames.extend(vec![f(60, 7); 12]);
        frames.extend([f(13, 4), f(10, 12), f(9, 18), f(10, 19)]);
        frames.extend(vec![f(9, 9); 3]);
        frames.extend(vec![f(15, 14); 5]);
        frames.extend(vec![f(18, 64); 13]);
        frames.extend(vec![f(20, 64); 8]);
        let l = |frame: u16, id: u16| openhunted_formats::AnimLabel {
            frame,
            id,
            name: None,
        };
        std::rc::Rc::new(Animation {
            version: 1004,
            width: 78,
            height: 64,
            extra: [0; 3],
            table_a: vec![],
            frames,
            labels: vec![
                l(0, 68),
                l(12, 70),
                l(24, 71),
                l(28, 171),
                l(31, 271),
                l(36, 72),
                l(49, 172),
                l(56, 272),
            ],
            script: vec![],
            bitmaps: vec![],
            palette: None,
        })
    }

    fn with_sprite(mut e: Entity, label: u16) -> Entity {
        let anim = opo_anim();
        e.sprite = Some(crate::Sprite {
            layer: crate::Layer::Object,
            player: crate::AnimPlayer::at_label(&anim, label).unwrap(),
            anim,
            x: e.x,
            y: e.y,
            frozen: label == STEAM_LABELS[2] || label == 72,
            scale: (1, 1),
            mirror: false,
        });
        e
    }

    fn ledge(x: i32, y: i32, values: &[i32]) -> Entity {
        let mut e = with_sprite(logic(70, x, y, values), 70);
        e.boxes.clear();
        e.behaviour = Behaviour::Opo(OpoState::ledge(
            Rect {
                x: 0,
                y: 0,
                w: 60,
                h: 7,
            },
            values,
            vec![Rect {
                x: 2,
                y: 2,
                w: 3,
                h: 2,
            }],
        ));
        e
    }

    fn frame(e: &Entity) -> usize {
        e.sprite.as_ref().unwrap().player.frame()
    }

    fn drop_state(e: &Entity) -> DropState {
        match &e.behaviour {
            Behaviour::Opo(OpoState {
                part: Part::Drop(d),
                ..
            }) => d.clone(),
            _ => panic!("não é gota"),
        }
    }

    #[test]
    fn the_ledge_drips_on_its_timer() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(5000, 0);
        let mut c = ctx(&j, &w, &on);
        // values[1] = 25: intervalo rand % 7 + 25 − 3 (22…28); values[7] = 2: a primeira no 3.º
        let mut e = ledge(100, 200, &[45, 25, 0, 0, 0, 0, 0, 2]);
        let mut drops = vec![];
        for t in 0..200 {
            c.tick = t;
            let mut out = Out::default();
            tick(&mut e, &mut c, &mut out);
            for d in out.spawn {
                assert_eq!(d.kind, DROP);
                assert!(
                    (100..100 + 52).contains(&d.x),
                    "x na largura que sobra: {}",
                    d.x
                );
                assert_eq!(d.y, 203, "y no meio da saliência (7 / 2)");
                assert_eq!(d.layer, e.layer);
                assert_eq!(d.boxes.len(), 1, "OPO.BRD");
                let sp = d.sprite.as_ref().unwrap();
                assert_eq!(sp.scale, (8, 13), "8 / largura do quadro 24");
                assert_eq!(sp.player.frame(), 24, "rótulo 71");
                assert!(d.flags & 0x10 != 0 && d.flags & 0x28 == 0x8);
                drops.push(t);
            }
        }
        assert_eq!(drops[0], 2, "temporizador 2: vence no terceiro tique");
        for p in drops.windows(2) {
            assert!((23..=29).contains(&(p[1] - p[0])), "intervalo {:?}", p);
        }
        assert!(e.flags & 0x20 != 0, "a saliência fica só de contato");
        // values[1] = 0: não goteja
        let mut e = ledge(100, 200, &[45, 0]);
        for t in 0..100 {
            c.tick = t;
            let mut out = Out::default();
            tick(&mut e, &mut c, &mut out);
            assert!(out.spawn.is_empty());
        }
    }

    /// Gota formada em (x, y), já fora da saliência.
    fn spawn_drop(c: &mut Ctx, x: i32, y: i32) -> Entity {
        let mut l = ledge(x, y, &[45, 25]);
        let mut out = Out::default();
        tick(&mut l, c, &mut out);
        out.spawn.pop().expect("gota")
    }

    #[test]
    fn the_drop_forms_falls_splashes_and_vanishes() {
        let on = |_: i32| None;
        // Jake longe; chão em y = 247 (o do contexto)
        let (j, w) = ctx_with_jake(5000, 200);
        let mut c = ctx(&j, &w, &on);
        let mut d = spawn_drop(&mut c, 100, 50);
        let y0 = d.y;
        let mut frames = vec![];
        for _ in 0..4 {
            let mut out = Out::default();
            tick(&mut d, &mut c, &mut out);
            frames.push(frame(&d));
        }
        assert_eq!(
            frames,
            vec![25, 26, 27, 28],
            "quatro quadros do 71, depois o 171"
        );
        assert_eq!(drop_state(&d).state, DROP_FALLING);
        assert_eq!(d.y, y0 + DROP_FALL);
        assert_eq!(d.flags & 0x28, 0, "ganha gravidade e passa a sólida");
        let mut ys = vec![];
        let mut n = 0;
        while drop_state(&d).state == DROP_FALLING {
            let mut out = Out::default();
            tick(&mut d, &mut c, &mut out);
            assert!(out.jake_damage.is_empty() && out.events.is_empty());
            ys.push(d.y);
            n += 1;
            assert!(n < 100, "nunca chegou ao chão");
        }
        assert!(ys.windows(2).all(|w| w[1] >= w[0]), "cai");
        let b = d.bounds().unwrap();
        assert_eq!(b.y + b.h, 247, "respinga no chão: {b:?}");
        assert_eq!(frame(&d), 31, "rótulo 271");
        assert_eq!(d.flags & 0x28, 0x28);
        let mut alive_ticks = 0;
        while d.alive {
            let mut out = Out::default();
            tick(&mut d, &mut c, &mut out);
            alive_ticks += 1;
            assert!(alive_ticks < 10);
        }
        assert_eq!(
            alive_ticks, 5,
            "o respingo tem 5 quadros e some ao voltar ao 0"
        );
    }

    #[test]
    fn the_drop_poisons_the_player_it_lands_on() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(100, 300);
        let mut c = ctx(&j, &w, &on);
        let b = j.body.bounds();
        let mut d = spawn_drop(&mut c, b.x, b.y - 120);
        d.x = b.x + 4;
        let mut hurt = None;
        for t in 0..60 {
            let mut out = Out::default();
            tick(&mut d, &mut c, &mut out);
            if !out.jake_damage.is_empty() {
                hurt = Some((t, out.jake_damage, out.events.clone()));
                break;
            }
        }
        let (_, p, ev) = hurt.expect("a gota acerta o Jake");
        assert_eq!((p.amount, p.poison, p.poisons), (1, true, true));
        assert_eq!(
            ev.iter().filter_map(Event::sound_id).collect::<Vec<_>>(),
            vec![SPEECH]
        );
        assert_eq!(drop_state(&d).state, DROP_SPLASH);
    }

    #[test]
    fn a_drop_leaving_the_level_splashes() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(5000, 0);
        let mut c = ctx(&j, &w, &on);
        c.level = (10_000, 30);
        let mut d = spawn_drop(&mut c, 100, 0);
        // a fase tem 30 px de altura e o chão está em 47: respinga ao passar de y = 30
        for _ in 0..10 {
            let mut out = Out::default();
            tick(&mut d, &mut c, &mut out);
            if drop_state(&d).state == DROP_SPLASH {
                break;
            }
        }
        assert_eq!(drop_state(&d).state, DROP_SPLASH);
        assert!(d.y >= 30);
    }

    fn steam(values: &[i32], label: u16) -> Entity {
        let mut e = with_sprite(logic(STEAM, 0, 0, values), label);
        e.boxes.clear();
        e.behaviour = Behaviour::Opo(OpoState::steam(values));
        e
    }

    fn steam_state(e: &Entity) -> SteamState {
        match &e.behaviour {
            Behaviour::Opo(OpoState {
                part: Part::Steam(s),
                ..
            }) => s.clone(),
            _ => panic!(),
        }
    }

    /// Estado e sons de cada tique, com o Jake dentro do vapor.
    fn run_steam(
        e: &mut Entity,
        ticks: u32,
        on: &dyn Fn(i32) -> Option<bool>,
    ) -> Vec<(i32, Vec<u16>, i32)> {
        let (j, w) = ctx_with_jake(0, 0);
        let mut c = ctx(&j, &w, on);
        (0..ticks)
            .map(|t| {
                c.tick = t;
                let mut out = Out::default();
                tick(e, &mut c, &mut out);
                let snd = out
                    .events
                    .iter()
                    .filter_map(|e| match e {
                        Event::Sound(c) if c.at.is_some() => Some(c.id),
                        _ => None,
                    })
                    .collect();
                (steam_state(e).state, snd, out.jake_damage.amount)
            })
            .collect()
    }

    #[test]
    fn the_steam_cycles_off_starting_on_ending() {
        let on = |_: i32| None;
        // a maioria dos 37 da fase: (10, 0, 10, 20, 1), sem gatilhos
        let mut e = steam(&[10, 0, 10, 20, 1], 272);
        let log = run_steam(&mut e, 120, &on);
        let states: Vec<i32> = log.iter().map(|l| l.0).collect();
        // desligado: o temporizador 10 vence com 0; a chamada da inicialização já conta um
        assert!(states[..9].iter().all(|&s| s == STEAM_OFF), "{states:?}");
        assert_eq!(states[9], STEAM_STARTING);
        assert_eq!(log[9].1, vec![SOUND_STARTING]);
        // ligando: 7 quadros de trás para a frente e −1 no 8.º tique
        assert!(states[10..17].iter().all(|&s| s == STEAM_STARTING));
        assert_eq!(states[17], STEAM_ON);
        // ligado: o temporizador (20) conta a partir do estado 3
        let on_ticks = states.iter().filter(|&&s| s == STEAM_ON).count();
        let ending = states.iter().position(|&s| s == STEAM_ENDING).unwrap();
        assert_eq!(log[ending].1, vec![SOUND_ENDING]);
        assert_eq!(ending, 17 + 21, "ligado por 21 tiques");
        assert!(on_ticks >= 21);
        // desligando: 7 quadros e volta ao 2
        assert_eq!(states[ending + 7], STEAM_OFF);
        // fere só fora do estado 2, a cada 10 tiques: 5 de veneno
        for (t, (s, _, dmg)) in log.iter().enumerate() {
            if *dmg > 0 {
                assert_ne!(*s, STEAM_OFF);
                assert_eq!(t % 10, 0);
                assert_eq!(*dmg, 5);
            }
            if *s == STEAM_OFF {
                assert_eq!(*dmg, 0);
            }
        }
        assert!(log.iter().any(|l| l.2 == 5));
        // e liga de novo (values[4] = 1)
        assert!(states[ending + 8..].contains(&STEAM_STARTING));
    }

    #[test]
    fn the_steam_shows_the_label_of_its_state() {
        let on = |_: i32| None;
        let mut e = steam(&[10, 0, 10, 20, 1], 272);
        let (j, w) = ctx_with_jake(0, 0);
        let mut c = ctx(&j, &w, &on);
        let mut seen = vec![];
        for t in 0..40 {
            c.tick = t;
            let mut out = Out::default();
            tick(&mut e, &mut c, &mut out);
            seen.push(frame(&e));
        }
        assert_eq!(seen[0], 56, "272");
        // no tique da troca para o 3 o quadro é o seguinte ao rótulo (o 56), depois 55…49
        assert_eq!(&seen[9..17], &[56, 55, 54, 53, 52, 51, 50, 49]);
        assert_eq!(seen[17], 36, "72");
        assert_eq!(seen[18], 37, "o laço anda já no tique seguinte");
        assert_eq!(seen[19], 38);
    }

    #[test]
    fn steam_starting_on_and_one_shot() {
        let on = |_: i32| None;
        // values[1] = 1: começa ligado (rótulo 72 na fase)
        let mut e = steam(&[10, 1, 10, 20, 1], 72);
        let log = run_steam(&mut e, 30, &on);
        assert_eq!(log[0].0, STEAM_ON);
        assert_eq!(log[0].2, 5, "fere já no tique 0");
        // a chamada da inicialização já conta um tique do temporizador
        assert_eq!(log.iter().position(|l| l.0 == STEAM_ENDING), Some(19));
        // values[4] = 0: liga uma vez só
        let mut e = steam(&[10, 0, 10, 20, 0], 272);
        let log = run_steam(&mut e, 200, &on);
        let starts = log.iter().filter(|l| l.1.contains(&SOUND_STARTING)).count();
        assert_eq!(starts, 1);
        assert_eq!(log.last().unwrap().0, STEAM_OFF);
    }

    #[test]
    fn steam_follows_its_triggers() {
        // gatilho falso: desligado fica; ligado desliga já
        let off = |_: i32| Some(false);
        let mut e = steam(&[10, 0, 10, 20, 1], 272);
        e.triggers = vec![(5, Some(11))];
        let log = run_steam(&mut e, 60, &off);
        assert!(log.iter().all(|l| l.0 == STEAM_OFF));
        let mut e = steam(&[10, 1, 10, 20, 1], 72);
        e.triggers = vec![(5, Some(11))];
        let log = run_steam(&mut e, 3, &off);
        assert_eq!(
            log[0].0, STEAM_ENDING,
            "liga na inicialização e desliga no mesmo tique"
        );
        // sem temporizadores: liga assim que o gatilho dá verdadeiro
        let yes = |_: i32| Some(true);
        let mut e = steam(&[10, 0, 0, 0, 1], 272);
        e.triggers = vec![(5, Some(11))];
        let log = run_steam(&mut e, 2, &yes);
        assert_eq!(log[0].0, STEAM_STARTING);
    }
}
