// SPDX-License-Identifier: GPL-3.0-or-later
//! A varredura do original contra o cenário (`0x00432e5d`) com as formas do personagem
//! (`docs/re/personagem.md` §7.3, §8): caixas e arestas do BRD, contato em pixels inteiros
//! com as caixas em borda inclusiva (`x + w − 1`), o vetor encurtado de 1/8 px a cada
//! contato (`0x00433999`) e a resposta pela normal do contato (`0x00404842`, impulso
//! `0x0040550a`). Transcrição das rotinas, com a aritmética de 32 bits delas (produtos que
//! estouram dão a volta, divisões truncam para zero).

use super::{GRAVITY, Rect, SUB, World};
use std::sync::OnceLock;

/// Ponto ou vetor em pixels (ou 1/8 px, conforme o uso).
pub type Pt = (i32, i32);

/// Forma de colisão de um corpo, relativa à posição (canto do quadro de referência).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// Caixa (tipo 2).
    Box(Rect),
    /// Aresta (tipo 3) do vértice `a` (o registro de tipo 4 anterior, `+0xc`) ao ponto `b`,
    /// com a normal para fora `n` (largura e altura do registro).
    Edge { a: Pt, b: Pt, n: Pt },
}

const fn bx(x: i32, y: i32, w: i32, h: i32) -> Shape {
    Shape::Box(Rect { x, y, w, h })
}

const fn edge(a: Pt, b: Pt, n: Pt) -> Shape {
    Shape::Edge { a, b, n }
}

/// `JK_DEFLT.BRD` na ordem do arquivo (o original testa nessa ordem): os quatro chanfros do
/// octógono, depois corpo, topo e base. Simétrico no quadro de 22 px: o espelho (`+0x194`)
/// não muda nada.
pub const JK_DEFLT: [Shape; 7] = [
    edge((2, 6), (8, 4), (-2, -6)),
    edge((14, 4), (20, 6), (2, -6)),
    edge((20, 43), (14, 46), (3, 6)),
    edge((8, 46), (2, 43), (-3, 6)),
    bx(2, 6, 18, 37),
    bx(8, 3, 6, 3),
    bx(8, 43, 6, 4),
];

/// `MN_DEFLT.BRD` (Garathe), mesma estrutura; simétrico no quadro de 40 px.
pub const MN_DEFLT: [Shape; 7] = [
    edge((10, 8), (16, 5), (-3, -6)),
    edge((24, 5), (30, 8), (3, -6)),
    edge((30, 52), (24, 55), (3, 6)),
    edge((16, 55), (10, 52), (-3, 6)),
    bx(10, 8, 20, 44),
    bx(16, 5, 8, 3),
    bx(16, 52, 8, 4),
];

/// Caixa envolvente das caixas de `shapes` (a caixa única do porte para o resto do jogo).
pub fn bounds(shapes: &[Shape]) -> Option<Rect> {
    let boxes = shapes.iter().filter_map(|s| match s {
        Shape::Box(r) => Some(*r),
        Shape::Edge { .. } => None,
    });
    boxes.reduce(|u, r| {
        let (x0, y0) = (u.x.min(r.x), u.y.min(r.y));
        Rect {
            x: x0,
            y: y0,
            w: (u.x + u.w).max(r.x + r.w) - x0,
            h: (u.y + u.h).max(r.y + r.h) - y0,
        }
    })
}

/// Massa do cenário no impulso (`0x1f40`).
const SCENERY_MASS: i32 = 8000;
/// Abaixo deste módulo de velocidade a normal oblíqua não tira a componente normal
/// (`0x00471994`).
const OBLIQUE_MIN: i32 = 50;
/// Acima desta velocidade normal (4 px/tique) a resposta tira só metade (`0x00404aed`).
const HALF_ABOVE: i32 = 32;
/// Atrito por eixo no contato (`0x00404b11`).
const FRICTION: i32 = 7;

// ------------------------------------------------------------------ aritmética do original

fn mul(a: i32, b: i32) -> i32 {
    a.wrapping_mul(b)
}

fn div(a: i32, b: i32) -> i32 {
    a.wrapping_div(b)
}

fn shl(a: i32, n: u32) -> i32 {
    a.wrapping_shl(n)
}

/// `0x00405390`: `v` entre `a` e `b`, inclusive.
fn between(v: i32, a: i32, b: i32) -> bool {
    v >= a.min(b) && v <= a.max(b)
}

/// Segmento (x1, y1) → (x2, y2).
#[derive(Debug, Clone, Copy)]
struct Seg {
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
}

const fn seg(x1: i32, y1: i32, x2: i32, y2: i32) -> Seg {
    Seg { x1, y1, x2, y2 }
}

/// `0x00404dec`: interseção de dois segmentos em pixels, com 5 bits de fração no ponto de
/// corte; devolve se os segmentos se cortam e o ponto (calculado também quando não se
/// cortam, salvo nos casos paralelos, em que fica (0, 0)).
fn intersect(p: Seg, q: Seg) -> (bool, Pt) {
    let Seg { x1, y1, x2, y2 } = p;
    let Seg {
        x1: u1,
        y1: v1,
        x2: u2,
        y2: v2,
    } = q;
    let (dx1, dy1) = (x2.wrapping_sub(x1), y2.wrapping_sub(y1));
    let (dx2, dy2) = (u2.wrapping_sub(u1), v2.wrapping_sub(v1));
    if dx1 == 0 || dx2 == 0 {
        if dx1 != 0 {
            // q vertical
            let y = div(shl(mul(u1 - x1, dy1), 5), dx1).wrapping_add(shl(y1, 5));
            let ok = between(u1, x1, x2) && between(y, shl(v1, 5), shl(v2, 5));
            return (ok, (u1, y >> 5));
        }
        if dx2 != 0 {
            // p vertical
            let y = div(shl(mul(x1 - u1, dy2), 5), dx2).wrapping_add(shl(v1, 5));
            let ok = between(x1, u1, u2) && between(y, shl(y1, 5), shl(y2, 5));
            return (ok, (x1, y >> 5));
        }
        if u1 != x1 {
            return (false, (0, 0));
        }
        if between(y1, v1, v2) || between(y2, v1, v2) || between(v1, y1, y2) || between(v2, y1, y2)
        {
            return (true, (x1, y1.min(y2).max(v1.min(v2))));
        }
        return (false, (0, 0));
    }
    if dy1 == 0 || dy2 == 0 {
        if dy1 != 0 {
            // q horizontal
            let x = div(shl(mul(v1 - y1, dx1), 5), dy1).wrapping_add(shl(x1, 5));
            let ok = between(v1, y1, y2) && between(x, shl(u1, 5), shl(u2, 5));
            return (ok, (x >> 5, v1));
        }
        if dy2 != 0 {
            // p horizontal
            let x = div(shl(mul(y1 - v1, dx2), 5), dy2).wrapping_add(shl(u1, 5));
            let ok = between(y1, v1, v2) && between(x, shl(x1, 5), shl(x2, 5));
            return (ok, (x >> 5, y1));
        }
        if y1 != v1 {
            return (false, (0, 0));
        }
        if between(x1, u1, u2) || between(x2, u1, u2) || between(u1, x1, x2) || between(u2, x1, x2)
        {
            return (true, (x1.min(x2).max(u1.min(u2)), y1));
        }
        return (false, (0, 0));
    }
    let a = mul(dx1, y1).wrapping_sub(mul(dy1, x1));
    let b = mul(dx2, v1).wrapping_sub(mul(dy2, u1));
    let det = mul(dy2, dx1).wrapping_sub(mul(dy1, dx2));
    if det == 0 {
        // colineares (o teste do original é este, não o de colinearidade)
        if mul(dy2, x1).wrapping_add(mul(dx2, y1)) != 0 {
            return (false, (0, 0));
        }
        if between(y1, v1, v2) || between(y2, v1, v2) || between(v1, y1, y2) || between(v2, y1, y2)
        {
            // `0x0040527d` compara u1 com ele mesmo: o mínimo de q é sempre u1
            let out = if x1.min(x2) < u1 {
                if x1 < x2 { (x2, y2) } else { (x1, y1) }
            } else if u1 < u2 {
                (u2, v2)
            } else {
                (u1, v1)
            };
            return (true, out);
        }
        // o original cai nos testes finais com valores velhos da pilha; aqui, sem corte
        return (false, (0, 0));
    }
    let x = div(shl(mul(dx2, a).wrapping_sub(mul(dx1, b)), 5), det);
    let y = div(shl(mul(dy2, a).wrapping_sub(mul(dy1, b)), 5), det);
    let ok = between(x, shl(x1, 5), shl(x2, 5))
        && between(x, shl(u1, 5), shl(u2, 5))
        && between(y, shl(y1, 5), shl(y2, 5))
        && between(y, shl(v1, 5), shl(v2, 5));
    (ok, (x >> 5, y >> 5))
}

// ------------------------------------------------------------------ raiz e comprimento

fn isqrt_exact(n: u32) -> u32 {
    let mut r = f64::from(n).sqrt() as u32;
    while u64::from(r) * u64::from(r) > u64::from(n) {
        r -= 1;
    }
    while u64::from(r + 1) * u64::from(r + 1) <= u64::from(n) {
        r += 1;
    }
    r
}

/// `0x0046724c`: raiz por tabelas em faixas (cada entrada é a raiz inteira do último valor da
/// faixa: `0x00485d18` exata abaixo de 256, depois faixas de 16, 32, 64 e 128); de 65536 para
/// cima, busca binária.
fn isqrt(s: u32) -> i32 {
    match s {
        0 => 0,
        1..=3 => 1,
        4..=0xff => isqrt_exact(s) as i32,
        0x100..=0xffff => {
            let (lo, sh) = match s {
                0x4000.. => (0x4000, 7),
                0x1000.. => (0x1000, 6),
                0x400.. => (0x400, 5),
                _ => (0x100, 4),
            };
            let i = (s - lo) >> sh;
            isqrt_exact(lo + ((i + 1) << sh) - 1) as i32
        }
        _ => {
            let (mut lo, mut hi, mut mid) = (0u32, s.min(0xffff), 0u32);
            while lo < hi - 1 {
                mid = (lo + hi) >> 1;
                let sq = mid.wrapping_mul(mid);
                if s == sq {
                    break;
                }
                if s > sq {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            mid as i32
        }
    }
}

/// Tabela de cosseno (`0x00483f74`, 2049 × i16, 1/16384): `⌊16384·cos(iπ/2048)⌋`.
fn cos_table() -> &'static [i16; 2049] {
    static T: OnceLock<[i16; 2049]> = OnceLock::new();
    T.get_or_init(|| {
        let mut t = [0i16; 2049];
        for (i, v) in t.iter_mut().enumerate() {
            *v = (16384.0 * (i as f64 * std::f64::consts::PI / 2048.0).cos()).floor() as i16;
        }
        t
    })
}

/// Tabela de arco tangente (`0x00484f78`, 512 entradas): `⌊atan(i/512)·2048/π⌋`.
fn atan_table() -> &'static [i32; 512] {
    static T: OnceLock<[i32; 512]> = OnceLock::new();
    T.get_or_init(|| {
        let mut t = [0i32; 512];
        for (i, v) in t.iter_mut().enumerate() {
            *v = ((i as f64 / 512.0).atan() * 2048.0 / std::f64::consts::PI) as i32;
        }
        t
    })
}

/// `0x0046718e` para x, y ≥ 0: ângulo em 1/65536 de volta.
fn angle(x: i32, y: i32) -> i32 {
    let at = |r: i32| {
        if (0..0x200).contains(&r) {
            atan_table()[r as usize]
        } else {
            0
        }
    };
    let a = if x == 0 {
        0x400
    } else if y == 0 {
        0
    } else if y == x {
        0x200
    } else if y > x {
        0x400 - (at(div(shl(x, 9), y)) & 0xffff)
    } else {
        at(div(shl(y, 9), x)) & 0xffff
    };
    a << 4
}

/// `0x00467154`.
fn cos(a: i32) -> i32 {
    let mut i = ((a as u32) & 0xffff) >> 4;
    if i >= 0x800 {
        i = 0x1000 - i;
    }
    i32::from(cos_table()[i as usize])
}

/// `0x00467328`: comprimento do vetor (raiz por tabela até 65535; acima, pelo ângulo).
pub(crate) fn length(a: i32, b: i32) -> i32 {
    let (a, b) = (a.wrapping_abs(), b.wrapping_abs());
    if a == 0 {
        return b;
    }
    if b == 0 {
        return a;
    }
    let s = (mul(a, a).wrapping_add(mul(b, b))) as u32;
    if s < 0x10000 {
        return isqrt(s);
    }
    if b <= a && a < 0x20000 {
        let c = cos(angle(a, b));
        if c != 0 {
            return div(shl(a, 14), c);
        }
    } else if b > a && b < 0x20000 {
        let c = cos(angle(a, b).wrapping_add(0xc000));
        if c != 0 {
            return div(shl(b, 14), c);
        }
    }
    isqrt(s)
}

// ------------------------------------------------------------------ testes de forma

/// Contato: deslocamento em px até ele e a normal (do corpo para a superfície).
type Hit = (Pt, Pt);

/// `0x00438d55`: caixa `a` movida por `d` (px) contra a caixa `b`. Caixas que já se cruzam
/// (mais que a linha da borda) não contam; o canto de `a` que vai à face de `b` dá o
/// deslocamento (na linha da face: as duas passam a dividir a linha).
fn box_box(a: Rect, b: Rect, d: Pt) -> Option<Hit> {
    let (ax0, ay0, ax1, ay1) = (a.x, a.y, a.x + a.w - 1, a.y + a.h - 1);
    let (bx0, by0, bx1, by1) = (b.x, b.y, b.x + b.w - 1, b.y + b.h - 1);
    let (dx, dy) = d;
    let mut code = if bx0 >= ax1 {
        4
    } else if ax0 >= bx1 {
        8
    } else {
        0
    };
    if ay1 <= by0 {
        code += 1;
    } else if ay0 >= by1 {
        code += 2;
    }
    code = match code {
        5 => {
            if dx <= 0 || dy <= 0 {
                return None;
            }
            if mul(dy, bx0 - ax1) > mul(dx, by0 - ay1) {
                4
            } else {
                1
            }
        }
        6 => {
            if dx <= 0 || dy >= 0 {
                return None;
            }
            if mul(-dy, bx0 - ax1) > mul(dx, ay0 - by1) {
                4
            } else {
                2
            }
        }
        9 => {
            if dx >= 0 || dy <= 0 {
                return None;
            }
            if mul(dy, ax0 - bx1) > mul(-dx, by0 - ay1) {
                8
            } else {
                1
            }
        }
        10 => {
            if dx >= 0 || dy >= 0 {
                return None;
            }
            if mul(-dy, ax0 - bx1) > mul(-dx, ay0 - by1) {
                8
            } else {
                2
            }
        }
        c => c,
    };
    match code {
        1 | 2 => {
            // face de cima (1) ou de baixo (2) de b; cantos de baixo/de cima de a
            let (cy, fy) = if code == 1 {
                if dy <= 0 || ay1 + dy < by0 {
                    return None;
                }
                (ay1, by0)
            } else {
                if dy >= 0 || ay0 + dy > by1 {
                    return None;
                }
                (ay0, by1)
            };
            let face = seg(bx0, fy, bx1, fy);
            let (hit, p) = intersect(seg(ax0, cy, ax0 + dx, cy + dy), face);
            // o canto da esquerda; senão o da direita; senão a caixa passa por cima da
            // face inteira (os dois cantos caem dos dois lados dela)
            let (p, cx) = if hit {
                (p, ax0)
            } else if p.0 > bx1 {
                return None;
            } else {
                let (hit, p) = intersect(seg(ax1, cy, ax1 + dx, cy + dy), face);
                if !hit && p.0 <= bx1 {
                    return None;
                }
                (p, ax1)
            };
            Some(((p.0 - cx, p.1 - cy), (0, dy)))
        }
        4 | 8 => {
            let (cx, fx) = if code == 4 {
                if dx <= 0 || bx0 > ax1 + dx {
                    return None;
                }
                (ax1, bx0)
            } else {
                if dx >= 0 || ax0 + dx > bx1 {
                    return None;
                }
                (ax0, bx1)
            };
            let face = seg(fx, by0, fx, by1);
            let (hit, p) = intersect(seg(cx, ay0, cx + dx, ay0 + dy), face);
            let (p, cy) = if hit {
                (p, ay0)
            } else if p.1 > by1 {
                return None;
            } else {
                let (hit, p) = intersect(seg(cx, ay1, cx + dx, ay1 + dy), face);
                if !hit && p.1 <= by1 {
                    return None;
                }
                (p, ay1)
            };
            Some(((p.0 - cx, p.1 - cy), (dx, 0)))
        }
        _ => None,
    }
}

/// `0x00405703`: ponto dentro da caixa (bordas inclusivas).
fn inside(p: Pt, b: Rect) -> bool {
    p.0 >= b.x && p.0 < b.x + b.w && p.1 >= b.y && p.1 < b.y + b.h
}

/// `0x00439177`: caixa `b` movida por `d` contra a aresta `e` (de `a` a `bb`, normal `n`).
/// Só arestas oblíquas: as horizontais e verticais vão ao teste aresta × aresta
/// (`0x00439709`), que nenhum dado do porte usa (as células só têm caixas e as arestas dos
/// personagens são os chanfros).
fn box_edge(b: Rect, a: Pt, bb: Pt, n: Pt, d: Pt) -> Option<Hit> {
    let (x1, y1, x2, y2) = (a.0, a.1, bb.0, bb.1);
    let (nx, ny) = n;
    if nx == 0 || ny == 0 {
        return None;
    }
    // canto da caixa voltado para a aresta, pelo quadrante da normal
    let code = match (nx > 0, ny > 0) {
        (true, true) => 1,
        (true, false) => 4,
        (false, true) => 2,
        (false, false) => 3,
    };
    let corner = match code {
        1 => (b.x, b.y),
        4 => (b.x, b.y + b.h - 1),
        2 => (b.x + b.w - 1, b.y),
        _ => (b.x + b.w - 1, b.y + b.h - 1),
    };
    let e = seg(x1, y1, x2, y2);
    if mul(d.0, nx).wrapping_add(mul(d.1, ny)) < 0 {
        let (hit, p) = intersect(seg(corner.0, corner.1, corner.0 + d.0, corner.1 + d.1), e);
        if hit && p != (x1, y1) && p != (x2, y2) {
            return Some(((p.0 - corner.0, p.1 - corner.1), (-nx, -ny)));
        }
    }
    // um vértice da aresta contra a face da caixa: (vértice, normal, início e fim da face)
    let lowest = if y1 > y2 { (x1, y1) } else { (x2, y2) };
    let highest = if y1 < y2 { (x1, y1) } else { (x2, y2) };
    let rightmost = if x1 > x2 { (x1, y1) } else { (x2, y2) };
    let leftmost = if x1 < x2 { (x1, y1) } else { (x2, y2) };
    let (bx0, by0, bxw, byh) = (b.x, b.y, b.x + b.w, b.y + b.h);
    let face = match code {
        1 => {
            if by0 > y1 && by0 > y2 {
                Some((lowest, (0, d.1), seg(bx0, by0, bxw, by0)))
            } else if bx0 > x1 && bx0 > x2 {
                Some((rightmost, (d.0, 0), seg(bx0, byh, bx0, by0)))
            } else {
                None
            }
        }
        2 => {
            if by0 > y1 && by0 > y2 {
                Some((lowest, (0, d.1), seg(bx0, by0, bxw, by0)))
            } else if bx0 < x1 && bx0 < x2 {
                Some((leftmost, (d.0, 0), seg(bxw, by0, bxw, byh)))
            } else {
                None
            }
        }
        3 => {
            if byh < y1 && byh < y2 {
                Some((highest, (0, d.1), seg(bxw, byh, bx0, byh)))
            } else if bxw < x1 && bxw < x2 {
                Some((leftmost, (d.0, 0), seg(bxw, by0, bxw, byh)))
            } else {
                None
            }
        }
        _ => {
            if byh < y1 && byh < y2 {
                Some((highest, (0, d.1), seg(bxw, byh, bx0, byh)))
            } else if bx0 > x1 && bx0 > x2 {
                Some((rightmost, (d.0, 0), seg(bx0, byh, bx0, by0)))
            } else {
                None
            }
        }
    };
    let Some((v, normal, f)) = face else {
        // `0x00439640`: vértice dentro da caixa: contato com o deslocamento inteiro
        if inside((x1, y1), b) || inside((x2, y2), b) {
            return Some((d, d));
        }
        return None;
    };
    // `0x004053c2`: perpendicular da face; a face tem de estar de frente para o movimento
    let fp = (f.y2 - f.y1, f.x1 - f.x2);
    if mul(d.0, fp.0).wrapping_add(mul(d.1, fp.1)) <= 0 {
        return None;
    }
    let (hit, p) = intersect(f, seg(v.0, v.1, v.0 - d.0, v.1 - d.1));
    if !hit {
        return None;
    }
    Some(((v.0 - p.0, v.1 - p.1), normal))
}

/// `0x0043339c` para uma forma do corpo (na posição `pos`) contra uma caixa do cenário.
fn shape_box(s: &Shape, pos: Pt, b: Rect, d: Pt) -> Option<Hit> {
    match *s {
        Shape::Box(r) => box_box(r.moved(pos.0, pos.1), b, d),
        Shape::Edge { a, b: e, n } => {
            // `0x00433198`: só a aresta de frente para o movimento
            if mul(n.0, d.0).wrapping_add(mul(n.1, d.1)) <= 0 {
                return None;
            }
            let a = (a.0 + pos.0, a.1 + pos.1);
            let e = (e.0 + pos.0, e.1 + pos.1);
            // a caixa anda ao contrário contra a aresta; o resultado volta de sinal (`0x00433896`)
            let ((cx, cy), (nx, ny)) = box_edge(b, a, e, n, (-d.0, -d.1))?;
            Some(((-cx, -cy), (-nx, -ny)))
        }
    }
}

/// `0x00433999`: o contato vira o novo deslocamento em 1/8 px, encurtado em 1/8 px ao longo
/// do vetor (`v·(L − 1)/L`).
fn shorten(c: Pt) -> Pt {
    let (mut x, mut y) = (shl(c.0, 3), shl(c.1, 3));
    if x != 0 || y != 0 {
        let l = length(x, y);
        if l != 0 {
            x = div(mul(l - 1, x), l);
            y = div(mul(l - 1, y), l);
        }
    }
    (x, y)
}

/// Contato da varredura: normal do corpo para a superfície e se foi um objeto sólido.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Contact {
    pub normal: Pt,
    pub object: bool,
}

impl Contact {
    /// `0x00407031`: chão (inclinação menor que ~26,6°).
    pub fn floor(&self) -> bool {
        self.normal.1 > 0 && 2 * self.normal.0.abs() < self.normal.1
    }
    /// `0x0040712f`: teto.
    pub fn ceiling(&self) -> bool {
        self.normal.1 < 0 && self.normal.1.abs() > 2 * self.normal.0.abs()
    }
    /// `0x0040707c`: parede.
    pub fn wall(&self) -> bool {
        self.normal.1.abs() < 2 * self.normal.0.abs()
    }
}

/// Célula de 128 px de uma caixa do cenário (pelo canto).
fn cell(r: &Rect) -> (i32, i32) {
    (r.x.div_euclid(128), r.y.div_euclid(128))
}

/// A varredura (`0x00432e5d`) do deslocamento `disp` (1/8 px) das formas na posição `pos`:
/// devolve o deslocamento final (1/8 px) e o último contato. Os objetos sólidos vêm antes
/// (`0x00437614`, aqui como caixas paradas do cenário), depois as caixas do cenário célula a célula (linhas
/// de cima para baixo; colunas no sentido de x), cada forma do corpo contra cada caixa da
/// célula, na ordem do BRD. Cada contato encurta o vetor e a busca segue com o que sobrou.
pub fn sweep(world: &World, pos: Pt, shapes: &[Shape], disp: Pt) -> (Pt, Option<Contact>) {
    let mut disp = disp;
    let mut d = (disp.0 >> 3, disp.1 >> 3);
    if d == (0, 0) {
        return (disp, None);
    }
    // caixa envolvente varrida, para só testar as caixas por perto
    let bounds = shapes.iter().fold(None::<Rect>, |acc, s| {
        let r = match *s {
            Shape::Box(r) => r,
            Shape::Edge { a, b, .. } => {
                let (x0, y0) = (a.0.min(b.0), a.1.min(b.1));
                Rect {
                    x: x0,
                    y: y0,
                    w: a.0.max(b.0) - x0 + 1,
                    h: a.1.max(b.1) - y0 + 1,
                }
            }
        };
        Some(match acc {
            None => r,
            Some(u) => {
                let (x0, y0) = (u.x.min(r.x), u.y.min(r.y));
                Rect {
                    x: x0,
                    y: y0,
                    w: (u.x + u.w).max(r.x + r.w) - x0,
                    h: (u.y + u.h).max(r.y + r.h) - y0,
                }
            }
        })
    });
    let Some(bb) = bounds else {
        return (disp, None);
    };
    let bb = bb.moved(pos.0, pos.1);
    let reach = Rect {
        x: bb.x + d.0.min(0) - 2,
        y: bb.y + d.1.min(0) - 2,
        w: bb.w + d.0.abs() + 4,
        h: bb.h + d.1.abs() + 4,
    };
    let (cells, objects) = world.split();
    // objetos: a colisão do original é outra rotina (`0x00437614`, não transcrita); como o
    // resto do porte, os que o corpo já cruza (nasceu ou ficou sobreposto) não prendem
    let mut near: Vec<(bool, usize, &Rect)> = objects
        .iter()
        .filter(|r| r.overlaps(&reach) && !r.overlaps(&bb))
        .map(|r| (true, 0, r))
        .collect();
    let mut scenery: Vec<(bool, usize, &Rect)> = cells
        .iter()
        .enumerate()
        .filter(|(_, r)| r.overlaps(&reach))
        .map(|(i, r)| (false, i, r))
        .collect();
    let right = d.0 >= 0;
    scenery.sort_by_key(|(_, i, r)| {
        let (cx, cy) = cell(r);
        (cy, if right { cx } else { -cx }, *i)
    });
    near.extend(scenery);
    let mut contact = None;
    for (object, _, r) in near {
        for s in shapes {
            if let Some((c, n)) = shape_box(s, pos, *r, d) {
                disp = shorten(c);
                d = (disp.0 >> 3, disp.1 >> 3);
                contact = Some(Contact { normal: n, object });
                if d == (0, 0) {
                    return (disp, contact);
                }
            }
        }
    }
    (disp, contact)
}

/// `0x0040550a` com o outro corpo parado (cenário): impulso ao longo da normal, se o corpo se
/// aproxima. As velocidades acima de 75 em módulo são divididas por 8 antes e multiplicadas
/// depois (perdem os bits baixos), como no original.
fn impulse(n: Pt, v: &mut Pt, mass: i32) {
    let k = mul(
        mul(n.0, n.0).wrapping_add(mul(n.1, n.1)),
        mass + SCENERY_MASS,
    );
    if k == 0 {
        return;
    }
    let mut rel = *v;
    let big = rel.0.abs().max(rel.1.abs()) > 75;
    if big {
        rel = (rel.0 / 8, rel.1 / 8);
        *v = (v.0 / 8, v.1 / 8);
    }
    let j = mul(rel.0, n.0)
        .wrapping_add(mul(rel.1, n.1))
        .wrapping_mul(2);
    if j > 0 {
        v.0 = v.0.wrapping_add(div(mul(mul(-SCENERY_MASS, j), n.0), k));
        v.1 = v.1.wrapping_add(div(mul(mul(-SCENERY_MASS, j), n.1), k));
    }
    if big {
        *v = (shl(v.0, 3), shl(v.1, 3));
    }
}

/// `0x00404842`: resposta ao contato com o cenário (corpo com gravidade): impulso, retira a
/// componente normal (metade acima de 4 px/tique; com normal oblíqua só se |v| ≥ 50) e
/// atrito de 7/8 px em cada eixo.
pub fn respond(v: &mut Pt, n: Pt, mass: i32) {
    impulse(n, v, mass);
    let nn = mul(n.0, n.0).wrapping_add(mul(n.1, n.1));
    if nn == 0 {
        return;
    }
    if !(n.0 != 0 && n.1 != 0 && length(v.0, v.1) < OBLIQUE_MIN) {
        let mut vn = mul(v.1, n.1).wrapping_add(mul(v.0, n.0));
        let r = isqrt(nn as u32 & 0xffff);
        if r != 0 && div(vn, r).abs() > HALF_ABOVE {
            vn >>= 1;
        }
        v.0 = v.0.wrapping_sub(div(mul(vn, n.0), nn));
        v.1 = v.1.wrapping_sub(div(mul(vn, n.1), nn));
    }
    let fr = |c: i32| {
        if c.abs() > FRICTION {
            c - FRICTION * c.signum()
        } else {
            0
        }
    };
    *v = (fr(v.0), fr(v.1));
}

/// Resultado de um tique de física com as formas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Outcome {
    pub contact: Option<Contact>,
    /// Deslocamento do tique (`+0x15c/+0x160`), 1/8 px.
    pub disp: Pt,
}

/// Um tique de física do original (`0x004045a5`) para um corpo com formas: gravidade, a
/// varredura com a velocidade como deslocamento, posição += deslocamento >> 3 e a resposta
/// (só com gravidade, `+0x178 & 8`).
pub fn step(b: &mut super::Body, world: &World, gravity: bool, shapes: &[Shape]) -> Outcome {
    if gravity {
        b.vy = b.vy.saturating_add(GRAVITY);
    }
    let (disp, contact) = sweep(world, (b.x, b.y), shapes, (b.vx, b.vy));
    b.x = b.x.saturating_add(disp.0 >> 3);
    b.y = b.y.saturating_add(disp.1 >> 3);
    if let (Some(c), true) = (contact, gravity) {
        let mut v = (b.vx, b.vy);
        respond(&mut v, c.normal, b.mass);
        (b.vx, b.vy) = v;
    }
    b.hit_wall = contact.is_some_and(|c| c.wall());
    b.hit_ceiling = contact.is_some_and(|c| c.ceiling());
    b.on_ground = contact.is_some_and(|c| c.floor()) || on_floor(world, (b.x, b.y), shapes);
    Outcome { contact, disp }
}

/// Sonda de chão (`0x0040718d(obj, 0, 1)` com `0x00407031`): 1 px para baixo dá contato de
/// chão?
pub fn on_floor(world: &World, pos: Pt, shapes: &[Shape]) -> bool {
    sweep(world, pos, shapes, (0, SUB))
        .1
        .is_some_and(|c| c.floor())
}

/// Índices das caixas de `solids` em que as formas na posição entram mais que a linha da
/// borda (o "dentro" do original: caixas que dividem 2 linhas ou mais, `0x00438d55` só aceita
/// como separadas as que dividem uma; ou um vértice de aresta estritamente dentro). Dividir a
/// linha da borda é normal: subindo ou indo para a esquerda contra uma face, o encurtamento de
/// 1/8 px arredonda para ela (a caixa de topo entra 1 px no teto).
pub fn embedded(solids: &[Rect], pos: Pt, shapes: &[Shape]) -> Vec<usize> {
    (0..solids.len())
        .filter(|&i| {
            let c = solids[i];
            shapes.iter().any(|s| match *s {
                Shape::Box(r) => {
                    let r = r.moved(pos.0, pos.1);
                    r.x < c.x + c.w - 1
                        && c.x < r.x + r.w - 1
                        && r.y < c.y + c.h - 1
                        && c.y < r.y + r.h - 1
                }
                Shape::Edge { a, b, .. } => [a, b].iter().any(|p| {
                    let (x, y) = (p.0 + pos.0, p.1 + pos.1);
                    x > c.x && x < c.x + c.w - 1 && y > c.y && y < c.y + c.h - 1
                }),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::Body;

    #[test]
    fn tables_match_the_executable() {
        // somas das tabelas como estão no `Hunter.exe` (`0x00483f74`, `0x00484f78`)
        let c = cos_table();
        assert_eq!(c.iter().map(|&v| i64::from(v)).sum::<i64>(), -1023);
        assert_eq!(
            c.iter()
                .enumerate()
                .map(|(i, &v)| i as i64 * i64::from(v))
                .sum::<i64>(),
            -13_943_288_546
        );
        assert_eq!((c[1], c[1025], c[1500]), (16383, -26, -10929));
        let a = atan_table();
        assert_eq!(a.iter().map(|&v| i64::from(v)).sum::<i64>(), 145_963);
        assert_eq!(a[300], 345);
        // raiz por faixas: 9280 cai na faixa de 64 (4096 + 81·64 … 9343) → 96
        assert_eq!(isqrt(9280), 96);
        // (a tabela `0x00485cb8` começa 16, 16, 17, 17, 18)
        assert_eq!(isqrt(272), 16);
        assert_eq!(isqrt(288), 17);
        assert_eq!(isqrt(335), 18);
    }

    fn floor_world() -> World {
        World::new(vec![
            Rect {
                x: 0,
                y: 100,
                w: 400,
                h: 50,
            },
            Rect {
                x: 300,
                y: 0,
                w: 50,
                h: 100,
            },
        ])
    }

    fn jake(x: i32, y: i32) -> Body {
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

    /// Correndo no chão: (12, 1) px até o contato, encurtado para (95, 7) → 11 px para a
    /// direita e −12 para a esquerda (`hud-som-colisao.md`).
    #[test]
    fn running_on_the_floor_moves_11_right_and_12_left() {
        let w = floor_world();
        let mut b = jake(100, 53);
        (b.vx, b.vy) = (96, -8);
        let o = step(&mut b, &w, true, &JK_DEFLT);
        assert_eq!(o.disp, (95, 7));
        assert_eq!((b.x, b.y), (111, 53));
        assert!(b.on_ground);
        let mut b = jake(100, 53);
        (b.vx, b.vy) = (-96, -8);
        step(&mut b, &w, true, &JK_DEFLT);
        assert_eq!((b.x, b.y), (88, 53));
    }

    /// Queda de vy 32 a 4 px do chão: desce 4 e a vy fica −17 (oráculo; `nucleo.md`).
    #[test]
    fn landing_from_four_pixels_keeps_minus_17() {
        let w = floor_world();
        let mut b = jake(100, 49);
        b.vy = 32;
        step(&mut b, &w, true, &JK_DEFLT);
        assert_eq!((b.y, b.vy), (53, -17));
    }

    /// Parede à direita: o vértice do chanfro em x + 20 para 1 px antes dela (1749 = 1770 −
    /// 21 na `T_J_001`).
    #[test]
    fn right_wall_stops_one_pixel_early() {
        let w = floor_world();
        let mut b = jake(270, 53);
        for _ in 0..10 {
            (b.vx, b.vy) = (96, -8);
            step(&mut b, &w, true, &JK_DEFLT);
        }
        assert_eq!(b.x, 300 - 21);
    }
}
