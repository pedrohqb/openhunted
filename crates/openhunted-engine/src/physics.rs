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
    pub fn overlaps(&self, o: &Rect) -> bool {
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

    pub fn moved(&self, dx: i32, dy: i32) -> Rect {
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
    /// Os primeiros `scenery` sólidos são do cenário (células); os demais, de objetos.
    scenery: usize,
}

/// Bits de contato (`+0x220`, `docs/re/personagem.md`): 2 = cenário, 1 = objeto.
pub const CONTACT_SCENERY: u32 = 2;
pub const CONTACT_OBJECT: u32 = 1;

impl World {
    /// Mundo só de cenário.
    pub fn new(solids: Vec<Rect>) -> Self {
        let scenery = solids.len();
        Self { solids, scenery }
    }

    /// Cenário (`cells`) mais as caixas dos objetos sólidos (`objects`).
    pub fn with_objects(cells: Vec<Rect>, objects: Vec<Rect>) -> Self {
        let scenery = cells.len();
        let mut solids = cells;
        solids.extend(objects);
        Self { solids, scenery }
    }

    /// Máscara de contato de `r`: `CONTACT_SCENERY` se cruza o cenário, `CONTACT_OBJECT`
    /// se cruza um objeto sólido (0 = livre).
    pub fn contact(&self, r: &Rect) -> u32 {
        let (cells, objects) = self.solids.split_at(self.scenery.min(self.solids.len()));
        let mut bits = 0;
        if cells.iter().any(|s| s.overlaps(r)) {
            bits |= CONTACT_SCENERY;
        }
        if objects.iter().any(|s| s.overlaps(r)) {
            bits |= CONTACT_OBJECT;
        }
        bits
    }

    pub fn solids(&self) -> &[Rect] {
        &self.solids
    }

    /// Algum sólido intersecta `r`?
    pub fn blocked(&self, r: &Rect) -> bool {
        self.solids.iter().any(|s| s.overlaps(r))
    }

    /// Algum sólido que `start` ainda não intersecta intersecta `r`? (Um corpo que nasceu ou
    /// ficou sobreposto a um sólido pode sair dele em vez de ficar preso.)
    pub fn blocked_from(&self, start: &Rect, r: &Rect) -> bool {
        self.solids
            .iter()
            .any(|s| s.overlaps(r) && !s.overlaps(start))
    }

    /// Há chão até `probe` pixels abaixo de `r`, ignorando os sólidos que `r` já intersecta?
    pub fn standing_on(&self, r: &Rect, probe: i32) -> bool {
        let below = Rect {
            y: r.y.saturating_add(r.h),
            h: probe.max(1),
            ..*r
        };
        self.blocked_from(r, &below)
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

/// Move até `d` pixels num eixo, parando antes do primeiro sólido que `start` ainda não
/// intersecta; devolve o quanto andou.
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
        if world.blocked_from(&start, &r) {
            break;
        }
        done = next;
    }
    done
}

/// Um tique de física: gravidade (opcional), deslocamento de velocidade >> 3 eixo a eixo,
/// colisão e resposta (meio rebote, atrito no contato).
///
/// Chão (medido no oráculo; a varredura do original, `0x00432e5d`, encurta o vetor inteiro até
/// o contato): descendo, quem chega ao chão neste tique anda só a fração `g/vy` do vetor, com
/// `g = 8·livres + 8` (1 px de folga além dos pixels livres, em 1/8 px); na horizontal, para a
/// direita perde mais 1/8 px e para a esquerda arredonda para baixo (a matemática fina da
/// varredura segue em aberto: a regra reproduz todos os casos medidos). Correndo (vx 96, vy 8
/// depois da gravidade) anda 11 px para a direita e 12 para a esquerda; o Garathe (120) 14;
/// os estados do esqueleto e as minas saltadoras, com a vy guardada em 0 (16 depois da
/// gravidade), andam metade: furtivo −72 → −5 px, −17 → −2. Caindo de 4 px a 48, desce 4 e a
/// vy gravada é −24 + 7 = −17: o atrito (`0x00404842`) pega as duas componentes depois do
/// rebote.
pub fn step(b: &mut Body, world: &World, gravity: bool) {
    if gravity {
        b.vy = b.vy.saturating_add(GRAVITY);
    }
    b.hit_wall = false;
    b.hit_ceiling = false;
    let r0 = b.bounds();
    let mut ux = b.vx;
    let mut dy = b.vy >> 3;
    let mut floor = false;
    if b.vy > 0 {
        // folga até o chão onde o passo horizontal chega
        let rx = r0.moved(sweep(world, r0, b.vx >> 3, true), 0);
        let reach = (b.vy >> 3).saturating_add(1);
        let free = sweep(world, rx, reach, false);
        let g = free.saturating_mul(SUB).saturating_add(SUB);
        if free < reach && b.vy >= g {
            // a fração do vetor até o chão; o arredondamento medido difere pelo sentido
            // (para a direita perde 1/8 px, para a esquerda arredonda para baixo)
            let part = i64::from(ux) * i64::from(g);
            let vy = i64::from(b.vy);
            ux = if ux > 0 {
                (part / vy) as i32 - 1
            } else {
                part.div_euclid(vy) as i32
            };
            dy = free;
            floor = true;
        }
    }
    let dx = ux >> 3;
    let moved_x = sweep(world, r0, dx, true);
    b.x = b.x.saturating_add(moved_x);
    let mut contact_x = false;
    if moved_x != dx.clamp(-MAX_STEP, MAX_STEP) {
        b.vx = bounce(b.vx);
        b.hit_wall = true;
        contact_x = true;
    }
    let moved_y = sweep(world, b.bounds(), dy, false);
    b.y = b.y.saturating_add(moved_y);
    let mut contact_y = floor;
    if moved_y != dy.clamp(-MAX_STEP, MAX_STEP) {
        if dy < 0 {
            b.hit_ceiling = true;
        }
        contact_y = true;
    }
    if contact_y {
        b.vy = bounce(b.vy);
    }
    b.on_ground = world.standing_on(&b.bounds(), 1);
    // atrito uma vez em cada eixo, também na componente normal depois do rebote
    // (`0x00404842`; a cápsula quica na parede a −4 px/tique no oráculo)
    if contact_x || contact_y || b.on_ground {
        b.vx = friction(b.vx);
    }
    if contact_x || contact_y {
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
    fn body_overlapping_a_solid_stands_and_walks_out() {
        // nasce com a caixa (x 302..320) dentro da parede (x 300..320), de pé no chão
        let w = floor();
        let mut b = body(300, 53);
        step(&mut b, &w, true);
        assert!(b.on_ground, "o chão sustenta mesmo sobreposto à parede");
        // correndo: vy −1 px/tique, que a gravidade leva a +1 (o vetor inteiro chega ao chão)
        let run = |b: &mut Body, vx: i32| {
            b.vx = vx;
            b.vy = -SUB;
            step(b, &w, true);
        };
        run(&mut b, -12 * SUB);
        assert_eq!(b.x, 288, "anda para a esquerda dentro da parede");
        run(&mut b, -12 * SUB);
        assert_eq!(b.x, 276, "sai da parede");
        // já fora, a parede volta a barrar
        run(&mut b, 12 * SUB);
        assert_eq!(b.bounds().x + b.bounds().w, 300);
        assert!(b.hit_wall);
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
        let mut hit = false;
        for _ in 0..10 {
            b.vx = 12 * SUB;
            b.vy = -SUB;
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
        c.vy = -SUB;
        step(&mut c, &w, true);
        assert_eq!(c.vx, -6 * SUB + 7); // meio rebote (acima de 4 px/tique) e atrito do chão
        let mut d = body(279, 53);
        d.vx = 3 * SUB;
        d.vy = -SUB;
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

    /// Números medidos no oráculo (o `Hunter.exe` no Wine): o vetor que chega ao chão anda
    /// só até ele (com 1 px de folga) e recua 1/8 px.
    #[test]
    fn floor_contact_shortens_the_vector_like_the_original() {
        let w = floor();
        // correndo (vx ±96, vy −8): +11 px para a direita, −12 para a esquerda (`T_J_003`,
        // `T_J_001`); depois do choque o atrito leva vx a ±89
        for (vx, dx) in [(96, 11), (-96, -12)] {
            let mut b = body(50, 53);
            b.on_ground = true;
            (b.vx, b.vy) = (vx, -SUB);
            step(&mut b, &w, true);
            assert_eq!(
                (b.x - 50, b.y, b.vx, b.vy),
                (dx, 53, vx - 7 * vx.signum(), 0)
            );
        }
        // pelo esqueleto (vy 0): metade do vetor; furtivo −72 → −5, engatinhar −80 → −5,
        // −128 → −8, aterrissagem −40 → −3 (`T_J_001`)
        // e a mina saltadora `H_J_006` (−25 → −2, −17 → −2, −11 → −1, −3 → −1, 33 → 1)
        for (vx, dx) in [
            (-72, -5),
            (-80, -5),
            (-128, -8),
            (-40, -3),
            (-24, -2),
            (-120, -8),
            (-96, -6),
            (-25, -2),
            (-17, -2),
            (-11, -1),
            (-3, -1),
            (33, 1),
        ] {
            let mut b = body(50, 53);
            b.vx = vx;
            step(&mut b, &w, true);
            assert_eq!(b.x - 50, dx, "vx {vx}");
        }
        // caindo a 48 com o chão a 4 px: desce 4 e quica a −24, com atrito −17 (`I_J_002`)
        let mut b = body(50, 49);
        b.vy = 32;
        step(&mut b, &w, true);
        assert_eq!((b.y, b.vy), (53, -17));
        // −1 + 16 = 15 com o chão a 1 px: desce 1 sem choque e guarda 15
        let mut b = body(50, 52);
        b.vy = -1;
        step(&mut b, &w, true);
        assert_eq!((b.y, b.vy), (53, 15));
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
