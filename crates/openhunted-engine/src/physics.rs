// SPDX-License-Identifier: GPL-3.0-or-later
//! Física dos personagens (`docs/re/personagem.md`): velocidades em 1/8 px por tique,
//! gravidade, colisão com as caixas das células e sondagem de chão.

/// Subunidades por pixel nas velocidades.
pub const SUB: i32 = 8;
/// Gravidade: 2 px/tique² (`0x0047197c`).
pub const GRAVITY: i32 = 2 * SUB;
/// Acima desta velocidade (4 px/tique), o choque devolve metade da componente normal.
const BOUNCE_MIN: i32 = 4 * SUB;
/// Atrito no contato: 7/8 px/tique² por eixo (`0x00404842`).
const FRICTION: i32 = 7;
/// Limite de deslocamento por tique, para dados corrompidos não travarem o laço.
const MAX_STEP: i32 = 4096;

/// Retângulo em pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    fn overlaps(&self, o: &Rect) -> bool {
        let (ax1, ay1) = (
            i64::from(self.x) + i64::from(self.w),
            i64::from(self.y) + i64::from(self.h),
        );
        let (bx1, by1) = (
            i64::from(o.x) + i64::from(o.w),
            i64::from(o.y) + i64::from(o.h),
        );
        i64::from(self.x) < bx1
            && i64::from(o.x) < ax1
            && i64::from(self.y) < by1
            && i64::from(o.y) < ay1
    }

    fn moved(&self, dx: i32, dy: i32) -> Rect {
        Rect {
            x: self.x.saturating_add(dx),
            y: self.y.saturating_add(dy),
            ..*self
        }
    }
}

/// Geometria sólida da fase (caixas das células; sólidas dos quatro lados).
#[derive(Debug, Clone, Default)]
pub struct World {
    solids: Vec<Rect>,
}

impl World {
    pub fn new(solids: Vec<Rect>) -> Self {
        Self { solids }
    }

    pub fn solids(&self) -> &[Rect] {
        &self.solids
    }

    /// Algum sólido intersecta `r`?
    pub fn blocked(&self, r: &Rect) -> bool {
        self.solids.iter().any(|s| s.overlaps(r))
    }

    /// Há chão até `probe` pixels abaixo de `r` (e `r` está livre)?
    pub fn floor_at(&self, r: &Rect, probe: i32) -> bool {
        !self.blocked(r)
            && self.blocked(&Rect {
                y: r.y.saturating_add(r.h),
                h: probe.max(1),
                ..*r
            })
    }
}

/// Corpo de um personagem: posição (canto do quadro de referência, em pixels), velocidade em
/// 1/8 px por tique e caixa de colisão relativa à posição.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Body {
    pub x: i32,
    pub y: i32,
    pub vx: i32,
    pub vy: i32,
    pub boxr: Rect,
    pub on_ground: bool,
    pub hit_wall: bool,
    pub hit_ceiling: bool,
}

impl Body {
    pub fn new(x: i32, y: i32, boxr: Rect) -> Self {
        Self {
            x,
            y,
            vx: 0,
            vy: 0,
            boxr,
            on_ground: false,
            hit_wall: false,
            hit_ceiling: false,
        }
    }

    /// Caixa de colisão em coordenadas da fase.
    pub fn bounds(&self) -> Rect {
        Rect {
            x: self.x.saturating_add(self.boxr.x),
            y: self.y.saturating_add(self.boxr.y),
            ..self.boxr
        }
    }
}

/// Resposta ao choque na componente normal: meio rebote acima de 4 px/tique, senão zera.
fn bounce(v: i32) -> i32 {
    if v.saturating_abs() > BOUNCE_MIN {
        -(v / 2)
    } else {
        0
    }
}

/// Atrito na componente tangencial.
fn friction(v: i32) -> i32 {
    if v > 0 {
        (v - FRICTION).max(0)
    } else {
        (v + FRICTION).min(0)
    }
}

/// Move até `d` pixels num eixo, parando antes do primeiro sólido; devolve o quanto andou.
fn sweep(world: &World, start: Rect, d: i32, horizontal: bool) -> i32 {
    let d = d.clamp(-MAX_STEP, MAX_STEP);
    let unit = d.signum();
    let mut done = 0;
    while done != d {
        let next = done + unit;
        let r = if horizontal {
            start.moved(next, 0)
        } else {
            start.moved(0, next)
        };
        if world.blocked(&r) {
            break;
        }
        done = next;
    }
    done
}

/// Um tique de física: gravidade (opcional), deslocamento de velocidade >> 3 eixo a eixo,
/// colisão e resposta (meio rebote, atrito no contato).
pub fn step(b: &mut Body, world: &World, gravity: bool) {
    if gravity {
        b.vy = b.vy.saturating_add(GRAVITY);
    }
    b.hit_wall = false;
    b.hit_ceiling = false;
    let (dx, dy) = (b.vx >> 3, b.vy >> 3);
    let moved_x = sweep(world, b.bounds(), dx, true);
    b.x = b.x.saturating_add(moved_x);
    let mut contact_x = false;
    if moved_x != dx.clamp(-MAX_STEP, MAX_STEP) {
        b.vx = bounce(b.vx);
        b.hit_wall = true;
        contact_x = true;
    }
    let moved_y = sweep(world, b.bounds(), dy, false);
    b.y = b.y.saturating_add(moved_y);
    let mut contact_y = false;
    if moved_y != dy.clamp(-MAX_STEP, MAX_STEP) {
        if dy < 0 {
            b.hit_ceiling = true;
        }
        b.vy = bounce(b.vy);
        contact_y = true;
    }
    b.on_ground = world.floor_at(&b.bounds(), 1);
    if contact_y || b.on_ground {
        b.vx = friction(b.vx);
    }
    if contact_x {
        b.vy = friction(b.vy);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(x: i32, y: i32) -> Body {
        Body::new(
            x,
            y,
            Rect {
                x: 2,
                y: 3,
                w: 18,
                h: 44,
            },
        )
    }

    fn floor() -> World {
        World::new(vec![
            Rect {
                x: 0,
                y: 100,
                w: 500,
                h: 20,
            },
            Rect {
                x: 300,
                y: 0,
                w: 20,
                h: 100,
            },
        ])
    }

    #[test]
    fn falls_with_gravity_and_lands_on_floor() {
        let w = floor();
        let mut b = body(50, 0);
        step(&mut b, &w, true);
        assert_eq!((b.vy, b.y), (2 * SUB, 2)); // vy = 2 px/tique; desce 2 px
        for _ in 0..30 {
            step(&mut b, &w, true);
        }
        assert!(b.on_ground);
        assert_eq!(b.y + 3 + 44, 100); // pés exatamente no topo do chão
        assert_eq!(b.vy, 0);
    }

    #[test]
    fn wall_stops_and_bounces_fast_bodies() {
        let w = floor();
        let mut b = body(250, 53);
        b.vx = 12 * SUB;
        let mut hit = false;
        for _ in 0..10 {
            step(&mut b, &w, true);
            if b.hit_wall {
                hit = true;
                break;
            }
        }
        assert!(hit);
        assert_eq!(b.x + 2 + 18, 300); // no tique do choque, encostado na parede
        let mut c = body(279, 53);
        c.vx = 12 * SUB;
        step(&mut c, &w, true);
        assert_eq!(c.vx, -6 * SUB + 7); // meio rebote (acima de 4 px/tique) e atrito do chão
        let mut d = body(279, 53);
        d.vx = 3 * SUB;
        step(&mut d, &w, true);
        assert_eq!(d.vx, 0);
    }

    #[test]
    fn floor_contact_applies_friction() {
        let w = floor();
        let mut b = body(50, 53);
        b.vx = 40;
        step(&mut b, &w, true);
        assert!(b.on_ground);
        assert_eq!(b.vx, 33); // 7/8 px/tique² de atrito
    }

    #[test]
    fn probes_floor_below_and_ahead() {
        let w = floor();
        assert!(w.floor_at(
            &Rect {
                x: 10,
                y: 50,
                w: 18,
                h: 48
            },
            4
        ));
        assert!(!w.floor_at(
            &Rect {
                x: 10,
                y: 40,
                w: 18,
                h: 48
            },
            4
        ));
        assert!(!w.floor_at(
            &Rect {
                x: 600,
                y: 52,
                w: 18,
                h: 48
            },
            4
        ));
    }

    #[test]
    fn extreme_velocities_do_not_overflow() {
        let w = floor();
        let mut b = body(i32::MAX - 10, i32::MIN + 10);
        b.vx = i32::MAX;
        b.vy = i32::MIN;
        for _ in 0..3 {
            step(&mut b, &w, true);
        }
    }
}
