// SPDX-License-Identifier: GPL-3.0-or-later
//! Minas flutuantes (tipo 102, `0x0041e7f5`; `docs/re/minas.md`).

use super::{Blast, Ctx, Out};
use crate::entity::{Behaviour, Entity};
use crate::physics::{Body, Rect, SUB, step};

/// Variante de mina (tabela `0x00472bfc`).
#[derive(Debug, Clone, Copy)]
pub struct Variant {
    pub name: &'static str,
    /// Período da oscilação vertical (tiques).
    pub period: i32,
    /// Velocidade (1/8 px por tique).
    pub speed: i32,
    /// Alcance de detecção (px, centro a centro).
    pub range: i32,
    pub health: i32,
    /// Amplitude da oscilação (px).
    pub amplitude: i32,
    /// % por tique de começar / parar de pairar.
    pub hover_start: u32,
    pub hover_stop: u32,
}

pub const VARIANTS: [Variant; 4] = [
    Variant {
        name: "Alpha",
        period: 32,
        speed: 25,
        range: 200,
        health: 1,
        amplitude: 24,
        hover_start: 0,
        hover_stop: 100,
    },
    Variant {
        name: "Beta",
        period: 64,
        speed: 15,
        range: 250,
        health: 2,
        amplitude: 32,
        hover_start: 5,
        hover_stop: 20,
    },
    Variant {
        name: "Gamma",
        period: 32,
        speed: 45,
        range: 350,
        health: 3,
        amplitude: 28,
        hover_start: 10,
        hover_stop: 20,
    },
    Variant {
        name: "Delta",
        period: 32,
        speed: 20,
        range: 400,
        health: 4,
        amplitude: 12,
        hover_start: 15,
        hover_stop: 10,
    },
];

/// Linha do `DAMAGE.TBL` do tipo 102: raio 28, empurrão 15, dano de área 10.
pub const BLAST_RADIUS: i32 = 28;
pub const BLAST_PUSH: i32 = 15;
pub const BLAST_DAMAGE: i32 = 10;

/// Tipos que a mina toca sem explodir (`0x0041eb64`).
fn harmless(kind: u32) -> bool {
    matches!(kind, 6 | 22..=25 | 27 | 46 | 50..=53 | 65..=69 | 99 | 100)
}

#[derive(Debug, Clone)]
pub struct MineState {
    pub variant: usize,
    pub has_target: bool,
    pub hovering: bool,
    /// Velocidade pedida neste tique (1/8 px).
    pub velocity: (i32, i32),
    /// Gerador que a soltou (`mine_generator::sync`); 0 = mina da fase.
    pub owner: u32,
}

impl MineState {
    /// `values[0]` escolhe a variante (negativo → 0; ≥ 4 → 3).
    pub fn new(value: i32) -> Self {
        Self {
            variant: value.clamp(0, 3) as usize,
            has_target: false,
            hovering: false,
            velocity: (0, 0),
            owner: 0,
        }
    }

    pub fn variant(&self) -> &'static Variant {
        &VARIANTS[self.variant]
    }
}

/// Seno em escala 2^14 de uma fase em 1/65536 de volta (`0x0046717a`).
fn sin16(phase: i64) -> i64 {
    let rad = (phase as f64) * std::f64::consts::TAU / 65536.0;
    (rad.sin() * 16384.0).round() as i64
}

/// Distância entre centros (inteira).
pub fn distance((ax, ay): (i32, i32), (bx, by): (i32, i32)) -> i32 {
    let (dx, dy) = (i64::from(ax - bx), i64::from(ay - by));
    (((dx * dx + dy * dy) as f64).sqrt()) as i32
}

/// Linha de visão: 16 amostras do segmento entre os centros fora dos sólidos das células.
fn line_of_sight(ctx: &Ctx, a: (i32, i32), b: (i32, i32)) -> bool {
    (1..16).all(|i| {
        let p = Rect {
            x: a.0 + (b.0 - a.0) * i / 16,
            y: a.1 + (b.1 - a.1) * i / 16,
            w: 1,
            h: 1,
        };
        !ctx.cells.blocked(&p)
    })
}

pub fn tick(e: &mut Entity, ctx: &mut Ctx, out: &mut Out) {
    let Some(bounds) = e.bounds() else { return };
    let Behaviour::Mine(st) = &mut e.behaviour else {
        return;
    };
    let v = *st.variant();
    let center = (bounds.x + bounds.w / 2, bounds.y + bounds.h / 2);
    // contato com o Jake (tipo 1: não está na lista dos inofensivos) → explode
    if let Some(j) = ctx.jake_bounds() {
        if ctx.jake_layer == e.layer && bounds.overlaps(&j) && !harmless(1) {
            e.health = 0;
        }
    }
    let out_of_level =
        center.0 < 0 || center.1 < 0 || center.0 >= ctx.level.0 || center.1 >= ctx.level.1;
    if e.health <= 0 || out_of_level {
        e.alive = false;
        if e.health <= 0 {
            out.blasts.push(Blast {
                center,
                layer: e.layer,
                radius: BLAST_RADIUS,
                push: BLAST_PUSH,
                damage: BLAST_DAMAGE,
            });
        }
        return;
    }
    // alvo: o Jake vivo, na mesma camada, ao alcance e à vista
    let target = ctx.jake_bounds().filter(|j| {
        let jc = (j.x + j.w / 2, j.y + j.h / 2);
        ctx.jake_layer == e.layer
            && distance(center, jc) <= v.range
            && line_of_sight(ctx, center, jc)
    });
    st.has_target = target.is_some();
    let boxr = e.boxes.first().copied().unwrap_or(Rect {
        x: 0,
        y: 0,
        w: 1,
        h: 1,
    });
    let mut vel = (0, 0);
    if let Some(j) = target {
        if !st.hovering && ctx.rng.below(100) < v.hover_start {
            st.hovering = true;
        }
        if st.hovering {
            vel = st.velocity;
            if ctx.rng.below(100) < v.hover_stop {
                st.hovering = false;
            }
        } else {
            let above = bounds.y + bounds.h < j.y;
            let below = bounds.y > j.y + j.h;
            // mergulha/sobe só na vertical, e só se o passo é possível (`0x41f057`/`0x41f140`);
            // senão oscila e avança na horizontal
            let can_move = |dy: i32| {
                let mut probe = Body::new(e.x, e.y, boxr);
                probe.vy = dy;
                step(&mut probe, ctx.cells, false);
                probe.y != e.y
            };
            if above && can_move(v.speed) {
                vel.1 = v.speed;
            } else if below && can_move(-v.speed) {
                vel.1 = -v.speed;
            } else {
                let phase = i64::from((ctx.tick as i32).wrapping_add(e.x).rem_euclid(v.period))
                    * 65536
                    / i64::from(v.period);
                let target_y = 64 + i64::from(v.amplitude) * sin16(phase) / 16384;
                let local = i64::from(e.y.rem_euclid(128));
                let mut vy = (target_y - local).clamp(-64, 64);
                if local + vy < 0 {
                    vy = 0;
                }
                vel.1 = vy as i32;
                let jc = j.x + j.w / 2;
                vel.0 = if jc < center.0 { -v.speed } else { v.speed };
            }
        }
    }
    st.velocity = vel;
    // física contra o cenário, sem gravidade
    let mut body = Body::new(e.x, e.y, boxr);
    body.vx = vel.0;
    body.vy = vel.1;
    step(&mut body, ctx.cells, false);
    e.x = body.x;
    e.y = body.y;
    let _ = SUB;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviour::test_support::{ctx, ctx_with_jake, logic};
    use crate::entity::MINE;

    fn mine(x: i32, y: i32) -> Entity {
        let mut m = logic(MINE, x, y, &[0]);
        m.flags = 0x80801;
        m.boxes = vec![Rect {
            x: 0,
            y: 0,
            w: 17,
            h: 17,
        }];
        m.health = 1;
        m.behaviour = Behaviour::Mine(MineState::new(0));
        m
    }

    #[test]
    fn mine_dives_climbs_oscillates_and_chases() {
        let on = |_: i32| None;
        // Jake em (250, 200) (caixa y 203..247), mina toda acima: desce e vai para a direita
        let (j, w) = ctx_with_jake(250, 200);
        let mut m = mine(100, 100);
        let mut out = Out::default();
        let mut c = ctx(&j, &w, &on);
        tick(&mut m, &mut c, &mut out);
        assert_eq!((m.x, m.y), (100, 103)); // só desce, 25/8 px por tique
        // toda abaixo (sem chão no caminho da visão): sobe
        let empty = crate::World::new(vec![]);
        let mut ce = ctx(&j, &empty, &on);
        let mut m = mine(100, 300);
        tick(&mut m, &mut ce, &mut out);
        assert_eq!((m.x, m.y), (100, 296)); // só sobe; −25 >> 3 = −4 (como o sar do original)
        // na altura do Jake: oscila em torno de y = 64 da célula, corrigindo 1/8 da diferença
        let mut m = mine(100, 210);
        tick(&mut m, &mut c, &mut out);
        assert_eq!(m.x, 103);
        assert!(m.y < 210, "y local 82 acima do alvo ~64 (+ 24·sen): sobe");
        // Jake à esquerda: vai para a esquerda
        let (j2, w2) = ctx_with_jake(0, 200);
        let mut c2 = ctx(&j2, &w2, &on);
        let mut m = mine(100, 210); // na altura do Jake: oscila e avança
        tick(&mut m, &mut c2, &mut out);
        assert_eq!(m.x, 96); // −25 >> 3 = −4
        // fora do alcance (200 px): parada
        let (j3, w3) = ctx_with_jake(400, 200);
        let mut c3 = ctx(&j3, &w3, &on);
        let mut m = mine(100, 100);
        tick(&mut m, &mut c3, &mut out);
        assert_eq!((m.x, m.y), (100, 100));
        assert!(out.blasts.is_empty());
    }

    #[test]
    fn mine_blocked_from_diving_falls_back_to_chasing() {
        let on = |_: i32| None;
        // teto logo abaixo da mina: não consegue mergulhar → oscila e avança
        let (j, w) = ctx_with_jake(250, 200);
        let w = crate::World::new(
            w.solids()
                .iter()
                .copied()
                .chain(std::iter::once(Rect {
                    x: 0,
                    y: 117,
                    w: 400,
                    h: 2,
                }))
                .collect(),
        );
        let mut c = ctx(&j, &w, &on);
        let mut m = mine(100, 100);
        tick(&mut m, &mut c, &mut Out::default());
        assert_eq!(m.x, 103);
    }

    #[test]
    fn mine_explodes_on_contact_and_when_hurt() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(100, 200);
        let mut c = ctx(&j, &w, &on);
        let mut m = mine(105, 210);
        let mut out = Out::default();
        tick(&mut m, &mut c, &mut out);
        assert!(!m.alive);
        assert_eq!(out.blasts.len(), 1);
        assert_eq!(out.blasts[0].center, (113, 218));
        assert_eq!(out.blasts[0].damage, BLAST_DAMAGE);
        let mut hurt = mine(500, 500);
        hurt.health = 0;
        tick(&mut hurt, &mut c, &mut out);
        assert!(!hurt.alive && out.blasts.len() == 2);
    }

    #[test]
    fn mine_stops_at_walls() {
        let on = |_: i32| None;
        // parede baixa entre a mina e o Jake, abaixo da linha de visão entre os centros
        let (j, mut w) = ctx_with_jake(250, 200);
        w = crate::World::new(
            w.solids()
                .iter()
                .copied()
                .chain(std::iter::once(Rect {
                    x: 120,
                    y: 222,
                    w: 4,
                    h: 200,
                }))
                .collect(),
        );
        let mut c = ctx(&j, &w, &on);
        let mut m = mine(100, 210);
        let mut xs = vec![];
        for _ in 0..20 {
            tick(&mut m, &mut c, &mut out_of());
            xs.push((m.x, m.y));
        }
        assert_eq!(m.x + 17, 120, "{xs:?}");
    }

    fn out_of() -> Out {
        Out::default()
    }
}
