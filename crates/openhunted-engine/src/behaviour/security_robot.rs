// SPDX-License-Identifier: GPL-3.0-or-later
//! Robô de segurança voador (tipo 8, `OSR.ANM`, `0x0041c610`/`0x0041c7fc`;
//! `docs/re/robo-seguranca.md`): flutua, patrulha, persegue o Jake mantendo distância e
//! dispara raios laser (tipo 36) pelos dois canhões; ao morrer cai e explode dez vezes.

use super::enemy::{self, Label, Wound};
use super::{Blast, Ctx, Event, Out, angle_diff, angle16, cos_sin14};
use crate::entity::{Behaviour, Entity};
use crate::physics::{Body, Rect, SUB, step};
use crate::sound::{Cue, PRIORITY_FRAME};

/// Variante (tabela `0x00472a18`, 21 `i16` por registro).
#[derive(Debug, Clone, Copy)]
pub struct Variant {
    /// Passos de giro por tique (`+0x00`; volta = 32).
    pub turn: i32,
    /// Velocidade de patrulha em px/tique (`+0x02`).
    pub patrol: i32,
    /// Perda de atenção por tique com o alvo morto (`+0x04`).
    pub lose: i32,
    /// Multiplicador da atenção (`+0x06`).
    pub attention: i32,
    /// Chance de mudar de rumo, por 100 000 tiques (`+0x08`).
    pub change: i32,
    /// Período de tiro (`+0x0a`), velocidade do raio (`+0x0c`), % de tiro certeiro (`+0x0e`).
    pub period: u32,
    pub bolt_speed: i32,
    pub exact: u32,
    /// Tiros por rajada (`+0x10`) e pausa (`+0x12`).
    pub burst: i32,
    pub pause: i32,
    /// Distância em x ao alvo (`+0x14`, `+0x16`).
    pub min_dx: i32,
    pub max_dx: i32,
    /// Perseguição `v` (`+0x18`): `8·v + 4` em 1/8 px.
    pub chase: i32,
    /// Flutuação: altura-base, amplitude e período (`+0x1a..+0x1e`).
    pub hover_base: i32,
    pub hover_amp: i32,
    pub hover_period: i32,
    /// Valor posto em `+0x288` ao ser atingido (`+0x24`).
    pub hit_hold: i32,
    /// Período do pulso (`+0x26`).
    pub pulse: i32,
}

pub const VARIANTS: [Variant; 4] = [
    Variant {
        turn: 1,
        patrol: 2,
        lose: 1,
        attention: 1,
        change: 200,
        period: 8,
        bolt_speed: 9,
        exact: 10,
        burst: 6,
        pause: 40,
        min_dx: 80,
        max_dx: 200,
        chase: 2,
        hover_base: 74,
        hover_amp: 14,
        hover_period: 100,
        hit_hold: 35,
        pulse: 20,
    },
    Variant {
        turn: 1,
        patrol: 2,
        lose: 2,
        attention: 4,
        change: 155,
        period: 7,
        bolt_speed: 10,
        exact: 20,
        burst: 9,
        pause: 30,
        min_dx: 80,
        max_dx: 180,
        chase: 3,
        hover_base: 42,
        hover_amp: 32,
        hover_period: 90,
        hit_hold: 20,
        pulse: 15,
    },
    Variant {
        turn: 2,
        patrol: 2,
        lose: 3,
        attention: 6,
        change: 100,
        period: 6,
        bolt_speed: 14,
        exact: 40,
        burst: 12,
        pause: 20,
        min_dx: 80,
        max_dx: 180,
        chase: 5,
        hover_base: 54,
        hover_amp: 28,
        hover_period: 80,
        hit_hold: 15,
        pulse: 10,
    },
    Variant {
        turn: 4,
        patrol: 3,
        lose: 9999,
        attention: 9,
        change: 50,
        period: 4,
        bolt_speed: 17,
        exact: 50,
        burst: 15,
        pause: 10,
        min_dx: 80,
        max_dx: 180,
        chase: 8,
        hover_base: 74,
        hover_amp: 12,
        hover_period: 70,
        hit_hold: 5,
        pulse: 7,
    },
];

/// Rótulos do `OSR.ANM`: 2 `osr00` (girando, 17), 1 `osr01` (5 × 5 posições dos canhões),
/// 3 `osr02` (destroçado).
pub const LABELS: [Label; 3] = [(2, 0, 17), (1, 17, 25), (3, 42, 8)];
/// `SOUND.TBL` tipo 8: quadro 42 (destroço) → 3100.
pub const SOUNDS: [(usize, u16, u16); 1] = [(42, 3100, 0)];
/// "security hovering", no pulso, no máximo a cada 250 ms (`0x488834`).
pub const SOUND_HOVER: u16 = 3157;
pub const SOUND_HOVER_TICKS: u32 = 5;
/// Queda (não posicional) e "Bullet hits metal" (linha `(19, 8)`).
pub const SOUND_CRASH: u16 = 3103;
pub const SOUND_BULLET: u16 = 5056;
/// Percepção (`0x41db0f`): até 256 px entre centros.
pub const SIGHT: i32 = 256;
/// Teto da vida ao ser atingido (`0x41c7fc` passo 6).
pub const HEALTH_CAP: i32 = 45;
/// Derrubada: 15 px/tique na direção do empurrão.
pub const KNOCK: i32 = 15;
/// Queda: dez explosões `(8, 0, 2)` a ±30 px do canto da caixa (raio 48, empurrão 15,
/// área 10).
pub const CRASH_BLASTS: usize = 10;
pub const CRASH_SPREAD: i32 = 30;
pub const BLAST_RADIUS: i32 = 48;
pub const BLAST_PUSH: i32 = 15;
pub const BLAST_DAMAGE: i32 = 10;
/// Canhões A (`0x4729a0`) e B (`0x4729dc`): (dx, dy) por posição 0–4 e o ângulo (`u16`).
pub const MUZZLE_A: [(i32, i32); 5] = [(2, -5), (-2, 3), (-3, 11), (-2, 20), (2, 27)];
pub const MUZZLE_B: [(i32, i32); 5] = [(6, -4), (3, 2), (1, 8), (3, 15), (6, 21)];
pub const ANGLES: [i32; 5] = [38912, 35840, 32768, 29696, 26624];
/// Desvio sorteado do ângulo (±0x600).
pub const SPREAD: i32 = 0x600;

/// Orientação `+0x278`: −1 de lado, −2 morrendo, 0–31 girando.
pub const SIDEWAYS: i32 = -1;
pub const DYING: i32 = -2;

#[derive(Debug, Clone)]
pub struct SecurityRobotState {
    pub variant: usize,
    /// Modo de patrulha `values[0]` (bit 1 = em x, bit 2 = em y; 0 → 3).
    pub mode: i32,
    pub orient: i32,
    /// Espelhado (`+0x194`): olhando à direita.
    pub mirrored: bool,
    pub target: bool,
    /// Atenção `+0x270`.
    pub attention: i32,
    /// Última posição vista `+0x280/+0x284`.
    pub last_seen: Option<(i32, i32)>,
    /// Rajada `+0x264` (< 0 = pausa).
    pub burst: i32,
    /// Posições dos canhões A e B (`+0x27c/+0x27e`).
    pub cannons: (usize, usize),
    /// Rumo da patrulha (`+0x26c/+0x26d`).
    pub heading: (i32, i32),
    /// Célula da última virada da patrulha (`+0x268`) e contador de viradas (`+0x26e`).
    pub last_cell: i32,
    pub turns: i8,
    /// Contato com o cenário no último passo (`+0x224`).
    pub contact: bool,
    /// Camada nova pedida pela perseguição (zonas PR, `0x41d2e6`).
    pub new_layer: Option<i32>,
    /// Já passou pela primeira física: nela a gravidade (2 px) age antes de a atualização
    /// pedir a compensação (`0x404bc7`) — oráculo: o robô da fase em y 317 aparece em 319.
    pub settled: bool,
    /// Velocidade da queda (1/8 px).
    pub fall: (i32, i32),
    /// Destroço no chão.
    pub dead: bool,
    pub wound: Wound,
    pub last_frame: Option<usize>,
    /// Fase própria da flutuação e do pulso (o original soma o endereço do objeto).
    pub phase: i32,
}

impl SecurityRobotState {
    /// `values[0]` = modo de patrulha; `variant` = `fields[5]` (0–3).
    pub fn new(values: &[i32], variant: i32, phase: i32) -> Self {
        let mode = values.first().copied().unwrap_or(0);
        Self {
            variant: variant.clamp(0, 3) as usize,
            mode: if mode == 0 { 3 } else { mode },
            orient: SIDEWAYS,
            mirrored: false,
            target: false,
            attention: 0,
            last_seen: None,
            burst: 0,
            cannons: (2, 2),
            heading: (0, 0),
            last_cell: 0,
            turns: 0,
            contact: false,
            new_layer: None,
            settled: false,
            fall: (0, 0),
            dead: false,
            wound: Wound::default(),
            last_frame: None,
            phase,
        }
    }

    pub fn variant(&self) -> &'static Variant {
        &VARIANTS[self.variant]
    }

    fn forget(&mut self) {
        self.target = false;
        self.last_seen = None;
    }

    /// Gira para `want` (`0x41d9c8`): devolve `true` se já está alinhado (sem mexer na
    /// orientação: de lado continua de lado, girado continua girado). De lado (−1) conta como
    /// 8 ou 24 conforme o espelho; o giro vai pelo lado mais curto (empate: `rand & 1`).
    fn turn_to(&mut self, want: i32, ctx: &mut Ctx) -> bool {
        if self.orient == DYING {
            return false;
        }
        let cur = match self.orient {
            SIDEWAYS if self.mirrored => 24,
            SIDEWAYS => 8,
            r => r,
        };
        if cur == want {
            return true;
        }
        let diff = (want - cur).rem_euclid(32);
        let up = diff < 16 || (diff == 16 && ctx.rng.next_u16() & 1 != 0);
        let step = self.variant().turn * if up { 1 } else { -1 };
        self.orient = (cur + step).rem_euclid(32);
        self.mirrored = self.orient > 16;
        false
    }
}

fn hidden(ctx: &Ctx) -> bool {
    use crate::jake::State;
    ctx.jake
        .is_some_and(|j| matches!(j.state(), State::Hide | State::Hidden | State::Unhide))
}

/// Busca (`0x41db0f`): o Jake vivo na camada, não escondido, a até 256 px e à vista.
fn acquire(e: &Entity, me: Rect, ctx: &Ctx) -> bool {
    let Some(j) = enemy::jake_target(ctx, e.layer) else {
        return false;
    };
    let (c, jc) = (enemy::center(&me), enemy::center(&j));
    !hidden(ctx)
        && super::mine::distance(c, jc) <= SIGHT
        && enemy::line_of_sight(ctx, c, jc, &[me, j])
}

/// `célula+0x04` da célula em (`x`, `y`) px na camada (0 sem célula ou sem grade).
fn cell_bits(ctx: &Ctx, x: i32, y: i32, layer: i32) -> i32 {
    ctx.grid
        .and_then(|g| g.cell((x / 128, y / 128, layer)))
        .map_or(0, |c| c.bits)
}

/// OU dos tipos das zonas PR da camada sob a caixa `b` (`0x4075af`).
fn zones_under(ctx: &Ctx, b: &Rect, layer: i32) -> u16 {
    ctx.zones
        .iter()
        .filter(|z| z.layer == layer && z.rect.overlaps(b))
        .fold(0, |acc, z| acc | z.kind)
}

/// Perseguição (`0x41d08c`): velocidade pedida em 1/8 px.
fn chase(e: &Entity, me: Rect, st: &mut SecurityRobotState, ctx: &mut Ctx) -> (i32, i32) {
    let c = enemy::center(&me);
    let Some(j) = ctx.jake.filter(|j| j.is_active()) else {
        return (0, 0);
    };
    let jb = j.body.bounds();
    let p = st.last_seen.unwrap_or(enemy::center(&jb));
    let skip: Vec<Rect> = std::iter::once(me)
        .chain(enemy::jake_target(ctx, e.layer))
        .collect();
    if !enemy::line_of_sight_on(ctx, e.layer, c, p, &skip) {
        st.forget();
        return (0, 0);
    }
    let want = if c.0 < p.0 { 24 } else { 8 };
    if !st.turn_to(want, ctx) {
        return (0, 0);
    }
    st.orient = SIDEWAYS;
    if st.contact {
        return (0, 0);
    }
    let v = *st.variant();
    let s = 8 * v.chase + 4;
    let toward = if c.0 > p.0 { -s } else { s };
    let mut adx = (c.0 - p.0).abs();
    let mut vel = (0, 0);
    // outra linha de células (pelo `+0x150` dos dois): sobe ou desce se a célula deixa
    if e.y / 128 != j.body.y / 128 {
        let bits = cell_bits(ctx, c.0, e.y, e.layer);
        let present = ctx
            .grid
            .is_none_or(|g| g.cell((c.0 / 128, e.y / 128, e.layer)).is_some());
        if present && bits & crate::ai::path::bits::CLOSED_BELOW == 0 && c.1 < p.1 {
            vel.1 += s;
        }
        if present && bits & crate::ai::path::bits::CLOSED_ABOVE == 0 && c.1 > p.1 {
            vel.1 -= s;
        }
        adx += 64;
    }
    if st.last_seen.is_some() {
        if adx >= s / 2 {
            vel.0 += toward;
            return vel;
        }
        // chegou à última posição vista: troca de camada pelas zonas PR
        let z = zones_under(ctx, &me, e.layer);
        let layers = ctx.grid.map_or(i32::MAX, |g| g.size.2);
        let jl = j.layer();
        if z & crate::jake::zone::LAYER_UP != 0 && e.layer != 0 && jl < e.layer {
            st.new_layer = Some(e.layer - 1);
        } else if z & crate::jake::zone::LAYER_DOWN != 0 && e.layer < layers - 1 && jl > e.layer {
            st.new_layer = Some(e.layer + 1);
            return (0, 0);
        } else {
            st.forget();
        }
        return vel;
    }
    if adx > v.max_dx {
        vel.0 += toward;
    } else if adx < v.min_dx {
        vel.0 -= toward;
    }
    vel
}

/// Patrulha sem alvo (`0x41d426`): anda num eixo por vez; ao passar o centro de uma célula
/// nova, vira se o lado à frente está fechado ou se a célula se abre para o outro eixo
/// (bits de `célula+0x04`), na beira da fase, por sorteio/contato ou pela coleira dos
/// gatilhos; só anda alinhado (girado, sem flutuar).
fn patrol(e: &Entity, st: &mut SecurityRobotState, ctx: &mut Ctx) -> (i32, i32) {
    use crate::ai::path::bits::{CLOSED_ABOVE, CLOSED_BELOW, WALL_LEFT, WALL_RIGHT};
    let v = *st.variant();
    let boxr = e.boxes.first().copied().unwrap_or(Rect {
        x: 0,
        y: 0,
        w: 0,
        h: 0,
    });
    let (mut hx, mut hy) = st.heading;
    let (old_hx, old_hy) = (hx, hy);
    let mut last = st.last_cell;
    let w = (128 - boxr.w) / 2;
    const H: i32 = 32;
    let (mut px, mut py) = (0, 0);
    if st.contact || ctx.rng.next_u16() < (v.change as u32 * 65536 / 100_000) {
        (px, py) = (hx, hy);
    }
    // gatilhos: código 0 com o objeto presente → parado; senão coleira de `código` px
    for &(id, code) in e.triggers.iter().take(4) {
        let Some(o) = (ctx.at)(id) else { continue };
        let code = code.unwrap_or(0);
        if code == 0 {
            st.orient = SIDEWAYS;
            return (0, 0);
        }
        let (dx, dy) = (o.0 - e.x - boxr.w / 2, o.1 - e.y - boxr.h / 2);
        if super::mine::distance((0, 0), (dx, dy)) > code {
            if px == 0 {
                px = -dx;
            }
            if py == 0 {
                py = -dy;
            }
        }
    }
    let (gx, gy) = (ctx.level.0 / 128, ctx.level.1 / 128);
    if !(e.x >= 0 && e.x < gx * 128 && e.y >= 0 && e.y < gy * 128) {
        (hx, hy) = (-hx, -hy);
    }
    let bits = cell_bits(ctx, e.x, e.y, e.layer);
    let cell = e.layer * gy * gx + (e.y / 128) * gx + e.x / 128;
    if hx == 0 && hy == 0 {
        hx = 1;
    }
    let (mx, my) = (e.x % 128, e.y % 128);
    let no_pref = px == 0 && py == 0;
    let mut n = 4;
    loop {
        let mut keep = false;
        if hx > 0 {
            let turn = px > 0
                || (st.mode & 1 == 0 && no_pref)
                || (gx - 1) * 128 + w <= e.x
                || (mx >= w && bits & WALL_RIGHT != 0)
                || !(mx < w || bits == 0 || bits & CLOSED_BELOW != 0 || cell == last);
            if turn {
                (last, hy, hx) = (cell, hx, 0);
                st.turns = st.turns.wrapping_add(1);
                if bits & CLOSED_BELOW != 0 && bits & CLOSED_ABOVE == 0 && st.turns < 4 {
                    hy = -hy;
                }
            } else {
                keep = true;
            }
        } else if hx < 0 {
            let turn = px < 0
                || (st.mode & 1 == 0 && no_pref)
                || e.x <= w
                || (mx <= w && bits & WALL_LEFT != 0)
                || !(mx > w || bits == 0 || bits & CLOSED_ABOVE != 0 || cell == last);
            if turn {
                (last, hy, hx) = (cell, hx, 0);
                st.turns = st.turns.wrapping_add(1);
                if bits & CLOSED_ABOVE != 0 && bits & CLOSED_BELOW == 0 && st.turns < 4 {
                    hy = -hy;
                }
            } else {
                keep = true;
            }
        }
        if hy > 0 {
            let turn = py > 0
                || (st.mode & 2 == 0 && no_pref)
                || (gy - 1) * 128 + H <= e.y
                || (my >= H && bits & CLOSED_BELOW != 0)
                || !(my < H || bits == 0 || bits & WALL_LEFT != 0 || cell == last);
            if turn {
                (last, hx, hy) = (cell, -hy, 0);
                st.turns = st.turns.wrapping_add(1);
                if bits & WALL_LEFT != 0 && bits & WALL_RIGHT == 0 && st.turns < 4 {
                    hx = -hx;
                }
            } else {
                keep = true;
            }
        } else if hy < 0 {
            let turn = py < 0
                || (st.mode & 2 == 0 && no_pref)
                || e.y <= H
                || (my <= H && bits & CLOSED_ABOVE != 0)
                || !(my > H || bits == 0 || bits & WALL_RIGHT != 0 || cell == last);
            if turn {
                (last, hx, hy) = (cell, -hy, 0);
                st.turns = st.turns.wrapping_add(1);
                if bits & WALL_RIGHT != 0 && bits & WALL_LEFT == 0 && st.turns < 4 {
                    hx = -hx;
                }
            } else {
                keep = true;
            }
        }
        n -= 1;
        if keep || n <= 0 {
            break;
        }
    }
    if st.turns != 0 && (hx, hy) == (old_hx, old_hy) {
        st.turns -= 1;
    }
    let want = match (hx, hy) {
        (x, _) if x < 0 => 8,
        (x, _) if x > 0 => 24,
        (_, y) if y < 0 => 0,
        _ => 16,
    };
    let mut vel = (0, 0);
    if st.turn_to(want, ctx) {
        vel = (v.patrol * hx * SUB, v.patrol * hy * SUB);
    }
    st.heading = (hx, hy);
    st.last_cell = last;
    vel
}

/// Posição do canhão (0–4) para a altura visada `h` (`0x41ce0d`).
fn cannon_pos(y: i32, table: &[(i32, i32); 5], h: i32) -> usize {
    if y + table[0].1 > h + 40 {
        0
    } else if y + table[1].1 > h + 10 {
        1
    } else if y + table[4].1 < h - 40 {
        4
    } else if y + table[3].1 < h - 10 {
        3
    } else {
        2
    }
}

/// Um raio de um canhão: boca (espelhada pela caixa), ângulo da tabela ± 0x600 ou certeiro.
fn shoot(
    e: &Entity,
    st: &SecurityRobotState,
    ctx: &mut Ctx,
    out: &mut Out,
    m: (i32, i32),
    pos: usize,
    aim: (i32, i32),
) {
    let v = *st.variant();
    let b = e.boxes.first().copied().unwrap_or(Rect {
        x: 0,
        y: 0,
        w: 0,
        h: 0,
    });
    let mx = if st.mirrored {
        2 * b.x + b.w - m.0
    } else {
        m.0
    };
    let mouth = (e.x + mx, e.y + m.1);
    let mut a = ANGLES[pos] + ctx.rng.below(2 * SPREAD as u32) as i32 - SPREAD;
    if ctx.rng.below(100) < v.exact {
        let dx = aim.0 - mouth.0;
        let exact = angle16(if st.mirrored { -dx } else { dx }, aim.1 - mouth.1);
        if angle_diff(exact, a).abs() < SPREAD {
            a = exact;
        }
    }
    let (c, s) = cos_sin14(a.rem_euclid(65536));
    let mut vel = ((c * v.bolt_speed) >> 14, (s * v.bolt_speed) >> 14);
    if st.mirrored {
        vel.0 = -vel.0;
    }
    out.events.push(Event::Sound(Cue::at(
        super::laser::sound(st.variant),
        PRIORITY_FRAME,
        mouth,
        e.layer,
    )));
    out.spawn
        .push(super::laser::bolt(mouth, vel, e.layer, st.variant));
}

/// Canhões (`0x41ce0d`): mira das bocas e disparo alternado a cada meio período.
fn cannons(e: &Entity, st: &mut SecurityRobotState, ctx: &mut Ctx, out: &mut Out) {
    let Some(j) = enemy::jake_target(ctx, e.layer) else {
        return;
    };
    let a = j.h / 8;
    st.cannons = (
        cannon_pos(e.y, &MUZZLE_A, j.y + a),
        cannon_pos(e.y, &MUZZLE_B, j.y + j.h - a),
    );
    if st.burst <= 0 {
        return;
    }
    let p = st.variant().period.max(1);
    let aim = enemy::center(&j);
    let (pa, pb) = st.cannons;
    for (due, m, pos) in [
        ((ctx.tick + p / 2) % p == 0, MUZZLE_A[pa], pa),
        (ctx.tick % p == 0, MUZZLE_B[pb], pb),
    ] {
        if due && st.burst > 0 {
            shoot(e, st, ctx, out, m, pos, aim);
            st.burst -= 1;
            if st.burst == 0 {
                st.burst = -st.variant().pause;
            }
        }
    }
}

/// Seno em escala 2^14 da fração `n/period` de volta.
fn sin_frac(n: i32, period: i32) -> i32 {
    let p = period.max(1);
    cos_sin14(n.rem_euclid(p) * (65536 / p)).1
}

pub fn tick(e: &mut Entity, ctx: &mut Ctx, out: &mut Out) {
    let Some(me) = e.bounds() else {
        return;
    };
    let mut st = match &e.behaviour {
        Behaviour::SecurityRobot(s) if !s.dead => s.clone(),
        _ => return,
    };
    let c = enemy::center(&me);
    let v = *st.variant();
    // 2–4: atenção e alvo
    if st.target {
        match ctx.jake {
            Some(j) if !j.is_active() => st.attention -= v.lose,
            Some(j) if j.layer() != e.layer => {
                if (j.layer() - e.layer).abs() > 1 {
                    st.forget();
                } else if st.last_seen.is_none() {
                    st.last_seen = Some(enemy::center(&j.body.bounds()));
                }
            }
            _ => {}
        }
        // compara o valor de antes do decremento (`0x41c90d`)
        let before = st.attention;
        st.attention -= 1;
        if before <= 0 {
            st.forget();
        }
    }
    if !st.target && st.orient != DYING && acquire(e, me, ctx) {
        st.target = true;
        st.last_seen = None;
        // um sorteio escolhe entre os candidatos (`rand % n`), outro a atenção
        let _ = ctx.rng.next_u16();
        st.attention = (20 + (ctx.rng.next_u16() % 20) as i32) * v.attention;
    }
    // 5: rajada
    if st.burst < 0 {
        st.burst += 1;
    }
    if st.burst == 0 {
        st.burst = v.burst;
    }
    // 6: dano
    let mut vel = (0, 0);
    let w = st.wound.take();
    if w.pending.amount != 0 {
        if w.pending.knockdown {
            let (px, py) = w.pending.push;
            let len = super::mine::distance((0, 0), (px, py)).max(1);
            vel = (KNOCK * SUB * px / len, KNOCK * SUB * py / len);
        }
        // dificuldade (`0x41c712`); dano negativo → 1 (`robo-seguranca.md`, passo 6)
        let amount = ctx.difficulty.hit(w.pending.amount).unwrap_or(0);
        e.health = e.health.min(HEALTH_CAP) - amount;
        if e.health < 0 && st.orient != DYING {
            st.orient = DYING;
            st.fall = (0, 0);
            st.forget();
        }
    }
    if st.orient == DYING {
        fall(e, &mut st, ctx, out);
        return;
    }
    // 7–8: perseguição ou patrulha; canhões
    let req = if st.target {
        chase(e, me, &mut st, ctx)
    } else {
        patrol(e, &mut st, ctx)
    };
    vel = (vel.0 + req.0, vel.1 + req.1);
    if st.orient == SIDEWAYS && st.target {
        cannons(e, &mut st, ctx, out);
    }
    // 9: flutuação, só de lado, sem contato e sem velocidade pedida (`0x41cb94`)
    if st.orient == SIDEWAYS && !st.contact && vel == (0, 0) {
        let n = ctx.tick as i32 + st.phase;
        let target_y = v.hover_base + ((v.hover_amp * sin_frac(n, v.hover_period)) >> 14);
        let local = e.y % 128;
        let mut vy = (target_y - local).clamp(-64, 64);
        // `local` em px, `vy` em 1/8 px
        if local * SUB + vy < 0 {
            vy = 0;
        }
        vel.1 = vy;
    }
    let mut body = Body::new(e.x, e.y, e.boxes.first().copied().unwrap_or(me));
    body.vx = vel.0;
    body.vy = vel.1;
    if !st.settled {
        body.vy += crate::physics::GRAVITY;
        st.settled = true;
    }
    step(&mut body, ctx.cells_for(e.layer), false);
    e.x = body.x;
    e.y = body.y;
    st.contact = body.hit_wall || body.hit_ceiling || (vel.1 > 0 && body.on_ground);
    if let Some(l) = st.new_layer.take() {
        // descendo: centrado na célula (y = 64) e parado (`0x41d346`)
        if l > e.layer {
            e.y += 64 - e.y % 128;
        }
        e.layer = l;
    }
    // pulso (`0x41da5a`): com p > 30, o zumbido
    if 32 * sin_frac(ctx.tick as i32 + st.phase, v.pulse) / 16384 > 30 {
        out.events.push(Event::Sound(Cue::at(
            SOUND_HOVER,
            PRIORITY_FRAME,
            c,
            e.layer,
        )));
    }
    display(e, &mut st, out);
    e.behaviour = Behaviour::SecurityRobot(st);
}

/// Queda (estado −2): cai com gravidade e, parado no cenário, explode dez vezes.
fn fall(e: &mut Entity, st: &mut SecurityRobotState, ctx: &mut Ctx, out: &mut Out) {
    let Some(boxr) = e.boxes.first().copied() else {
        return;
    };
    let mut body = Body::new(e.x, e.y, boxr);
    body.vx = st.fall.0;
    body.vy = st.fall.1;
    step(&mut body, ctx.cells_for(e.layer), true);
    e.x = body.x;
    e.y = body.y;
    st.fall = (body.vx, body.vy);
    let settled = (body.on_ground || body.hit_wall) && body.vx.abs() < 8 && body.vy.abs() < 8;
    let out_of_level = e.y >= ctx.level.1;
    if settled || out_of_level {
        st.dead = true;
        e.on = true;
        e.solid_shapes = false;
        e.flags |= 0x48;
        out.events
            .push(Event::Sound(Cue::ui(SOUND_CRASH, PRIORITY_FRAME)));
        let corner = e.bounds().map_or((e.x, e.y), |b| (b.x, b.y));
        for _ in 0..CRASH_BLASTS {
            out.blasts.push(Blast {
                center: enemy::scatter(ctx, corner, CRASH_SPREAD),
                layer: e.layer,
                radius: BLAST_RADIUS,
                push: BLAST_PUSH,
                damage: BLAST_DAMAGE,
            });
        }
    }
    display(e, st, out);
    e.behaviour = Behaviour::SecurityRobot(Box::new(st.clone()));
}

/// Quadro (`0x0041c712`).
fn display(e: &mut Entity, st: &mut SecurityRobotState, out: &mut Out) {
    let (label, off, mirror) = match st.orient {
        SIDEWAYS => (1, 5 * st.cannons.0 + st.cannons.1, st.mirrored),
        DYING => (3, 0, st.mirrored),
        r if r <= 16 => (2, r as usize, false),
        r => (2, (32 - r) as usize, true),
    };
    super::show(e, label, off);
    if let Some(sp) = e.sprite.as_mut() {
        sp.mirror = mirror;
    }
    let (_, start, _) = enemy::label(&LABELS, label);
    let c = e.center();
    enemy::sound_frame(&SOUNDS, start + off, &mut st.last_frame, c, e.layer, out);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviour::test_support::{ctx, ctx_with_jake, logic};
    use crate::entity::SECURITY_ROBOT;

    fn robot(x: i32, y: i32, variant: i32) -> Entity {
        let mut e = logic(SECURITY_ROBOT, x, y, &[1, 0, 0, 0, 0, 0, -1, 131_074]);
        e.flags = 0x84801;
        e.health = 100;
        e.solid_shapes = true;
        e.boxes = vec![Rect {
            x: 9,
            y: 5,
            w: 36,
            h: 28,
        }];
        let mut st = SecurityRobotState::new(&e.values, variant, 0);
        st.settled = true;
        e.behaviour = Behaviour::SecurityRobot(Box::new(st));
        e
    }

    /// Oráculo `H_J_001`: na primeira física a gravidade age antes da compensação; o robô da
    /// fase em y 317 fica em 319 (e não flutua girando).
    #[test]
    fn first_physics_drops_two_px() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(5000, 0);
        let empty = crate::World::new(vec![]);
        let mut c = ctx(&j, &w, &on);
        c.cells = &empty;
        c.world = &empty;
        let mut r = robot(533, 317, 2);
        if let Behaviour::SecurityRobot(s) = &mut r.behaviour {
            s.settled = false;
        }
        tick(&mut r, &mut c, &mut Out::default());
        tick(&mut r, &mut c, &mut Out::default());
        assert_eq!(r.y, 319);
    }

    fn st(e: &Entity) -> &SecurityRobotState {
        match &e.behaviour {
            Behaviour::SecurityRobot(s) => s,
            _ => panic!(),
        }
    }

    fn bolts(out: &Out) -> usize {
        out.spawn
            .iter()
            .filter(|e| matches!(e.behaviour, Behaviour::Laser(_)))
            .count()
    }

    #[test]
    fn robot_ignores_hidden_jake() {
        let on = |_: i32| None;
        let (mut j, w) = ctx_with_jake(250, 0);
        let mut r = robot(100, 0, 0);
        assert!(acquire(&r, r.bounds().unwrap(), &ctx(&j, &w, &on)));
        // CIMA sem zona: esconde-se (164 → 165)
        for _ in 0..10 {
            j.tick(crate::input::UP, &w);
        }
        assert!(matches!(
            j.state(),
            crate::jake::State::Hide | crate::jake::State::Hidden
        ));
        let mut c = ctx(&j, &w, &on);
        assert!(!acquire(&r, r.bounds().unwrap(), &c));
        tick(&mut r, &mut c, &mut Out::default());
        assert!(!st(&r).target);
    }

    #[test]
    fn robot_cannons_alternate_and_pause() {
        let on = |_: i32| None;
        // variante 0: período 8, 6 por rajada, pausa 40; Jake a 130 px (entre 80 e 200)
        let (j, w) = ctx_with_jake(230, -10);
        let mut c = ctx(&j, &w, &on);
        let mut r = robot(100, 0, 0);
        let mut out = Out::default();
        let mut fired = vec![];
        for t in 0..120 {
            c.tick = t;
            let n = bolts(&out);
            tick(&mut r, &mut c, &mut out);
            if bolts(&out) > n {
                fired.push(t);
            }
        }
        assert!(st(&r).target);
        assert!(st(&r).mirrored, "vira para a direita");
        let gaps: Vec<u32> = fired.windows(2).map(|w| w[1] - w[0]).collect();
        assert!(fired.len() >= 6, "{fired:?}");
        assert!(gaps[..5].iter().all(|&g| g == 4), "{fired:?}");
        assert!(gaps[5] >= 40, "pausa depois de 6: {fired:?}");
        let Some(Behaviour::Laser(l)) = out.spawn.first().map(|e| &e.behaviour) else {
            panic!()
        };
        assert!(l.vx > 0, "atira para a direita");
    }

    #[test]
    fn robot_health_caps_at_45_and_falls_and_explodes() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(5000, 0);
        let mut c = ctx(&j, &w, &on);
        let mut r = robot(100, 0, 0);
        let p = crate::damage::Pending {
            amount: 2,
            ..Default::default()
        };
        let mut out = Out::default();
        enemy::hurt(&mut r, &p, true);
        tick(&mut r, &mut c, &mut out);
        assert_eq!(r.health, 43);
        // dano negativo vira 1 (`0x41c7fc` passo 6)
        let neg = crate::damage::Pending {
            amount: -3,
            ..Default::default()
        };
        enemy::hurt(&mut r, &neg, false);
        tick(&mut r, &mut c, &mut out);
        assert_eq!(r.health, 42);
        r.health = 43;
        for _ in 0..22 {
            enemy::hurt(&mut r, &p, true);
            tick(&mut r, &mut c, &mut out);
        }
        assert_eq!(r.health, -1, "23 tiros: vida abaixo de 0");
        assert_eq!(st(&r).orient, DYING);
        assert!(enemy::killed(&r));
        // cai até o chão (y + 47 do Jake → chão em y 47) e explode dez vezes
        let mut n = 0;
        while !st(&r).dead && n < 200 {
            tick(&mut r, &mut c, &mut out);
            n += 1;
        }
        assert!(st(&r).dead && r.alive && !r.solid_shapes);
        assert_eq!(out.blasts.len(), CRASH_BLASTS);
        assert!(out.events.iter().any(|e| e.sound_id() == Some(SOUND_CRASH)));
        assert!(out.events.iter().any(|e| e.sound_id() == Some(3100)));
    }

    /// Oráculo `H_J_001` (robô 42, variante 2, sem alvo): parte de −1 (8), gira 2 por tique até
    /// 24 e anda 2 px/tique **girado** (rótulo 2, quadro 8 espelhado) e sem flutuar: y fixo.
    #[test]
    fn patrol_moves_turned_and_level() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(5000, 0);
        let empty = crate::World::new(vec![]);
        let mut c = ctx(&j, &w, &on);
        c.cells = &empty;
        c.world = &empty;
        let mut r = robot(100, 300, 2);
        let mut ys = vec![];
        for t in 0..30 {
            c.tick = t;
            tick(&mut r, &mut c, &mut Out::default());
            ys.push(r.y);
        }
        assert!(
            st(&r).orient == 24 || st(&r).orient == 8,
            "{}",
            st(&r).orient
        );
        assert!(ys.iter().all(|&y| y == 300), "sem flutuar: {ys:?}");
        assert_ne!(r.x, 100, "anda");
    }

    /// Na beira da fase (`(gx − 1)·128 + (128 − largura)/2`) vira para o outro eixo: com o modo
    /// 3, desce (`0x41d651`); com o modo 1 (só em x), volta.
    #[test]
    fn patrol_turns_at_the_level_edge() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(5000, 0);
        let empty = crate::World::new(vec![]);
        let mut c = ctx(&j, &w, &on);
        c.cells = &empty;
        c.world = &empty;
        c.level = (512, 1024);
        let mut r = robot(400, 300, 3);
        let start = |r: &mut Entity, mode: i32| {
            if let Behaviour::SecurityRobot(s) = &mut r.behaviour {
                s.heading = (1, 0);
                s.orient = 24;
                s.mirrored = true;
                s.mode = mode;
            }
        };
        start(&mut r, 3);
        for t in 0..40 {
            c.tick = t;
            tick(&mut r, &mut c, &mut Out::default());
        }
        assert_eq!(st(&r).heading, (0, 1), "desce");
        assert!((430..434).contains(&r.x), "parou em x {}", r.x);
        assert!(r.y > 300);
        let mut r = robot(400, 300, 3);
        start(&mut r, 1);
        for t in 0..40 {
            c.tick = t;
            tick(&mut r, &mut c, &mut Out::default());
        }
        assert_eq!(st(&r).heading, (-1, 0), "só em x: volta");
        assert!(r.x < 430);
    }

    /// A atenção compara o valor de antes do decremento: com 1, o alvo ainda vale um tique.
    #[test]
    fn attention_runs_out_one_tick_late() {
        let on = |_: i32| None;
        let (mut j, w) = ctx_with_jake(250, 0);
        for _ in 0..10 {
            j.tick(crate::input::UP, &w);
        }
        let mut c = ctx(&j, &w, &on);
        let mut r = robot(100, 0, 0);
        if let Behaviour::SecurityRobot(s) = &mut r.behaviour {
            s.target = true;
            s.attention = 1;
        }
        tick(&mut r, &mut c, &mut Out::default());
        assert!(st(&r).target, "atenção 1 → 0: ainda com alvo");
        tick(&mut r, &mut c, &mut Out::default());
        assert!(!st(&r).target, "0: esquece");
    }

    /// Perseguição em outra linha de células: desce só se a célula do robô deixa passar
    /// embaixo (`célula+0x04 & 2`), não pelo teste de passo.
    #[test]
    fn chase_climbs_by_the_cell_bits() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(260, 300);
        let grid = crate::ai::CellGrid::new((10, 10, 1));
        let mut closed = grid.clone();
        closed.get_mut((0, 1, 0)).unwrap().bits = crate::ai::path::bits::CLOSED_BELOW;
        let mut c = ctx(&j, &w, &on);
        let run = |c: &mut Ctx| {
            let mut r = robot(100, 150, 3);
            if let Behaviour::SecurityRobot(s) = &mut r.behaviour {
                s.target = true;
                s.attention = 1000;
                s.orient = 24;
            }
            let y0 = r.y;
            tick(&mut r, c, &mut Out::default());
            r.y - y0
        };
        c.grid = Some(&grid);
        assert_eq!(run(&mut c), 68 >> 3, "aberta embaixo: desce s = 68");
        c.grid = Some(&closed);
        assert_eq!(run(&mut c), 0, "fechada embaixo: fica");
    }

    #[test]
    fn robot_keeps_its_distance() {
        let on = |_: i32| None;
        // Jake a 24 px: afasta-se; a 244 px (além dos 180 da variante, mas visto): aproxima-se
        let (j, w) = ctx_with_jake(140, -10);
        let mut c = ctx(&j, &w, &on);
        let mut r = robot(100, 0, 3);
        for t in 0..30 {
            c.tick = t;
            tick(&mut r, &mut c, &mut Out::default());
        }
        assert!(r.x < 100, "afastou-se: x {}", r.x);
        let (j2, w2) = ctx_with_jake(260, -10);
        let mut c2 = ctx(&j2, &w2, &on);
        let mut r2 = robot(0, 0, 3);
        for t in 0..10 {
            c2.tick = t;
            tick(&mut r2, &mut c2, &mut Out::default());
        }
        assert!(r2.x > 0, "aproximou-se: x {}", r2.x);
    }
}
