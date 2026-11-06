// SPDX-License-Identifier: GPL-3.0-or-later
//! Olho-laser (tipo 104, `0x0041f288`/`0x0041f58f`; `docs/re/olho-laser.md`): varre o
//! semicírculo de baixo, vê o Jake só dentro do cone do feixe e dispara rajadas de raios.

use super::{Blast, Ctx, Event, Out, angle_diff, angle16, cos_sin14, mine::distance};
use crate::damage::Pending;
use crate::entity::{Behaviour, Entity};
use crate::physics::Rect;
use crate::sound::{Cue, PRIORITY_FRAME};

/// Variante (tabela `0x00472d30`, registros de 0x44 bytes).
#[derive(Debug, Clone, Copy)]
pub struct Variant {
    pub name: &'static str,
    /// Alcance padrão (`+0x14`).
    pub range: i32,
    /// Desvio angular máximo do tiro (`+0x1c`, 1/65536 de volta).
    pub deviation: i32,
    /// Velocidade do raio em px/tique (`+0x20`; ±3 sorteado).
    pub speed: i32,
    /// Intervalo entre tiros, tiros por rajada e pausa (`+0x24`, `+0x28`, `+0x2c`).
    pub interval: u8,
    pub shots: u8,
    pub pause: u8,
    /// Tiques por passo da varredura (`+0x30`).
    pub sweep: u32,
    /// Variante do raio (`+0x3c`); `None` = sorteio 0–3 por raio.
    pub bolt: Option<usize>,
}

pub const VARIANTS: [Variant; 5] = [
    Variant {
        name: "Alpha",
        range: 150,
        deviation: 1024,
        speed: 8,
        interval: 3,
        shots: 5,
        pause: 10,
        sweep: 5,
        bolt: Some(0),
    },
    Variant {
        name: "Beta",
        range: 150,
        deviation: 1280,
        speed: 10,
        interval: 2,
        shots: 5,
        pause: 15,
        sweep: 4,
        bolt: Some(1),
    },
    Variant {
        name: "Gamma",
        range: 350,
        deviation: 1536,
        speed: 20,
        interval: 1,
        shots: 6,
        pause: 20,
        sweep: 3,
        bolt: Some(2),
    },
    Variant {
        name: "Delta",
        range: 300,
        deviation: 1792,
        speed: 30,
        interval: 0,
        shots: 8,
        pause: 25,
        sweep: 2,
        bolt: Some(3),
    },
    Variant {
        name: "All",
        range: 300,
        deviation: 768,
        speed: 45,
        interval: 10,
        shots: 10,
        pause: 30,
        sweep: 1,
        bolt: None,
    },
];

/// Bocas (dx, dy) por posição da varredura, em px do sprite em escala 1 (tabelas
/// `0x00472e84` e `0x00472f14`, registros de 12 bytes; o 3º campo é sempre 0).
pub const MUZZLE1: [(i32, i32); 12] = [
    (1, 4),
    (2, 8),
    (6, 12),
    (10, 17),
    (14, 22),
    (18, 22),
    (22, 22),
    (27, 22),
    (33, 17),
    (43, 13),
    (45, 8),
    (45, 4),
];
pub const MUZZLE2: [(i32, i32); 12] = [
    (2, 4),
    (3, 8),
    (7, 14),
    (18, 19),
    (27, 22),
    (31, 22),
    (35, 22),
    (38, 20),
    (43, 17),
    (45, 12),
    (45, 8),
    (45, 4),
];

/// Mira da posição `p`: `0x7a24 − 3000·p` (16 bits).
pub const AIM_ZERO: i32 = 0x7a24;
pub const AIM_STEP: i32 = 3000;
/// Posições da varredura (0…11).
pub const POSITIONS: i32 = 12;
/// Meia-abertura do cone em que o olho vê o alvo (`0x41f974`).
pub const CONE: i32 = 1500;
/// Desvio a partir do qual o olho segue o alvo que anda.
pub const FOLLOW: i32 = 375;
/// Linha do `DAMAGE.TBL` do atacante 104: raio 48, empurrão 15, dano de área 10.
pub const BLAST_RADIUS: i32 = 48;
pub const BLAST_PUSH: i32 = 15;
pub const BLAST_DAMAGE: i32 = 10;
/// "lasereye move", nos quadros 2, 5 e 8 do rótulo 1, no máximo a cada 250 ms (5 tiques).
pub const SOUND_MOVE: u16 = 3112;
const SOUND_MOVE_FRAMES: [i32; 3] = [2, 5, 8];
const SOUND_MOVE_TICKS: u32 = 5;
/// Rótulos do `OLE.ANM`: parado (12 quadros) e atirando (24).
pub const LABEL_IDLE: u16 = 1;
pub const LABEL_FIRING: u16 = 2;

#[derive(Debug, Clone)]
pub struct LaserEyeState {
    pub variant: usize,
    /// Alcance (`+0x264`).
    pub range: i32,
    /// Interruptor "ativo" (`+0x270`, `values[4]`).
    pub active: i32,
    /// Posição da varredura (`+0x26c`) e sentido (`+0x279`, `true` = crescente).
    pub pos: i32,
    pub ascending: bool,
    /// Atirando (`+0x278`) e "acabou de ver o alvo" (`+0x27a`).
    pub firing: bool,
    pub lost: bool,
    /// x do alvo quando visto pela última vez (`+0x28c`).
    pub last_x: i32,
    /// Contadores da rajada (`+0x268/+0x269/+0x26a`).
    pub interval: u8,
    pub shots: u8,
    pub pause: u8,
    /// Tiros disparados (`+0x27c`): alterna o clarão.
    pub fired: u32,
    pub sweep_ticks: u32,
    /// Dano pendente (`+0x24c`): qualquer um mata.
    pub hurt: Pending,
    /// Último tique em que tocou 3112.
    pub last_sound: Option<u32>,
}

impl LaserEyeState {
    /// `values[0]` = variante (negativa → 0; ≥ 5 → 4); `values[1]` = alcance quando
    /// `values[15] = 1`, senão o da variante; `values[4]` = ativo.
    pub fn new(values: &[i32]) -> Self {
        let value = |i: usize| values.get(i).copied().unwrap_or(0);
        let variant = value(0).clamp(0, 4) as usize;
        let v = &VARIANTS[variant];
        let range = if value(15) == 1 { value(1) } else { v.range };
        Self {
            variant,
            range,
            active: value(4),
            pos: 0,
            ascending: true,
            firing: false,
            lost: false,
            last_x: 0,
            interval: v.interval,
            shots: v.shots,
            pause: v.pause,
            fired: 0,
            sweep_ticks: 0,
            hurt: Pending::default(),
            last_sound: None,
        }
    }

    pub fn variant(&self) -> &'static Variant {
        &VARIANTS[self.variant]
    }

    /// Mira atual em 16 bits.
    pub fn aim(&self) -> i32 {
        (AIM_ZERO - AIM_STEP * self.pos).rem_euclid(65536)
    }

    /// Um passo da varredura (`0x41fdf0`): vaivém 0…11.
    pub fn step(&mut self) {
        self.pos += if self.ascending { 1 } else { -1 };
        if self.pos >= POSITIONS {
            self.ascending = false;
            self.pos = POSITIONS - 2;
        } else if self.pos < 0 {
            self.ascending = true;
            self.pos = 1;
        }
    }

    fn reload(&mut self) {
        let v = self.variant();
        self.interval = v.interval;
        self.shots = v.shots;
        self.pause = v.pause;
    }

    /// Um tique da rajada; devolve `true` se dispara agora.
    fn burst(&mut self) -> bool {
        if self.interval > 0 {
            self.interval -= 1;
            return false;
        }
        self.interval = self.variant().interval;
        if self.shots > 0 {
            self.fired += 1;
            self.shots -= 1;
            true
        } else {
            if self.pause > 0 {
                self.pause -= 1;
            } else {
                let v = self.variant();
                self.shots = v.shots;
                self.pause = v.pause;
            }
            false
        }
    }
}

/// Linha de visão entre dois pontos pelas células.
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

/// Escala do sprite aplicada a um deslocamento.
fn scaled(e: &Entity, v: i32) -> i32 {
    let (num, den) = e.sprite.as_ref().map_or((1, 1), |s| s.scale);
    v * num / den.max(1)
}

pub fn tick(e: &mut Entity, ctx: &mut Ctx, out: &mut Out) {
    let center = e.center();
    let layer = e.layer;
    let Behaviour::LaserEye(st) = &mut e.behaviour else {
        return;
    };
    let pos_before = st.pos;
    // 1. qualquer dano mata: explosão (linha 104) e o olho some
    if !st.hurt.is_empty() {
        st.active = 0;
        e.alive = false;
        e.health = 0;
        e.on = true;
        out.blasts.push(Blast {
            center,
            layer,
            radius: BLAST_RADIUS,
            push: BLAST_PUSH,
            damage: BLAST_DAMAGE,
        });
        return;
    }
    if st.active <= 0 {
        return;
    }
    // 3. alvo: o Jake vivo, na camada, ao alcance, à vista e dentro do cone do feixe
    let aim = st.aim();
    let target = ctx.jake_bounds().filter(|j| {
        let jc = (j.x + j.w / 2, j.y + j.h / 2);
        ctx.jake_layer == layer
            && distance(center, jc) <= st.range
            && line_of_sight(ctx, center, jc)
            && angle_diff(angle16(jc.0 - center.0, jc.1 - center.1), aim).abs() <= CONE
    });
    // 4. varredura
    if !st.firing {
        st.sweep_ticks += 1;
        if st.sweep_ticks >= st.variant().sweep {
            st.sweep_ticks = 0;
            st.step();
        }
    }
    // 5. com alvo, atira; perdido agora, varre para o lado em que ele foi
    let mut shoot = false;
    if let Some(j) = target {
        st.firing = true;
        st.lost = true;
        st.last_x = j.x;
        shoot = st.burst();
    } else if st.lost {
        let now = ctx.jake_bounds().map_or(st.last_x, |j| j.x);
        st.ascending = now >= st.last_x;
        st.step();
        st.reload();
        st.firing = false;
        st.lost = false;
    }
    // 6. o tiro sai da posição de antes de seguir (o disparo vem antes do passo 7)
    let shot_pos = st.pos;
    // 7. seguir o alvo que anda
    if let (Some(j), Some(jake)) = (target, ctx.jake) {
        let jc = (j.x + j.w / 2, j.y + j.h / 2);
        let d = angle_diff(angle16(jc.0 - center.0, jc.1 - center.1), st.aim());
        let vx = jake.body.vx;
        if d.abs() >= FOLLOW {
            if d > 0 && vx < 0 {
                st.pos -= 1;
                st.ascending = false;
            } else if d <= 0 && vx > 0 {
                st.pos += 1;
                st.ascending = true;
            }
            st.pos = st.pos.clamp(0, POSITIONS - 1);
        }
    }
    let (pos, firing, fired, variant) = (st.pos, st.firing, st.fired, *st.variant());
    let sound_due = !firing
        && pos != pos_before
        && SOUND_MOVE_FRAMES.contains(&pos)
        && st
            .last_sound
            .is_none_or(|t| ctx.tick.saturating_sub(t) >= SOUND_MOVE_TICKS);
    if sound_due {
        st.last_sound = Some(ctx.tick);
    }
    // 8. quadro
    if firing {
        super::show(e, LABEL_FIRING, (2 * pos + (fired & 1) as i32) as usize);
    } else {
        super::show(e, LABEL_IDLE, pos as usize);
    }
    if sound_due {
        out.events.push(Event::Sound(Cue::at(
            SOUND_MOVE,
            PRIORITY_FRAME,
            center,
            layer,
        )));
    }
    if let (true, Some(j)) = (shoot, target) {
        fire(
            e,
            ctx,
            out,
            &variant,
            shot_pos,
            (j.x + j.w / 2, j.y + j.h / 2),
        );
    }
}

/// Disparo (`0x0041fbe6`): dois raios, um por boca, na direção boca 1 → alvo com desvio
/// sorteado e velocidade ±3.
fn fire(e: &Entity, ctx: &mut Ctx, out: &mut Out, v: &Variant, pos: i32, target: (i32, i32)) {
    let p = pos.clamp(0, POSITIONS - 1) as usize;
    let m1 = (e.x + scaled(e, MUZZLE1[p].0), e.y + scaled(e, MUZZLE1[p].1));
    let m2 = (e.x + scaled(e, MUZZLE2[p].0), e.y + scaled(e, MUZZLE2[p].1));
    let dev = v.deviation.max(1);
    let a = angle16(target.0 - m1.0, target.1 - m1.1) + ctx.rng.below(2 * dev as u32) as i32 - dev;
    let s = v.speed + ctx.rng.below(6) as i32 - 3;
    let (c, sn) = cos_sin14(a);
    let vel = ((c * s) >> 14, (sn * s) >> 14);
    for m in [m1, m2] {
        let variant = v.bolt.unwrap_or_else(|| ctx.rng.below(4) as usize);
        let mut b = super::laser::bolt(m, vel, e.layer, variant);
        if let Behaviour::Laser(l) = &mut b.behaviour {
            l.owner = Some((e.x, e.y));
        }
        out.events.push(Event::Sound(Cue::at(
            super::laser::sound(variant),
            PRIORITY_FRAME,
            m,
            e.layer,
        )));
        out.spawn.push(b);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviour::test_support::{ctx, ctx_with_jake, logic};
    use crate::entity::LASER_EYE;

    fn eye(x: i32, y: i32, range: i32) -> Entity {
        let mut e = logic(
            LASER_EYE,
            x,
            y,
            &[0, range, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
        );
        e.flags = 0x80809;
        e.boxes = vec![Rect {
            x: 20,
            y: 0,
            w: 10,
            h: 10,
        }];
        e.solid_shapes = true;
        e.behaviour = Behaviour::LaserEye(LaserEyeState::new(&e.values));
        e
    }

    fn state(e: &Entity) -> &LaserEyeState {
        match &e.behaviour {
            Behaviour::LaserEye(s) => s,
            _ => unreachable!(),
        }
    }

    #[test]
    fn variants_and_range_follow_the_values() {
        let s = LaserEyeState::new(&[0, 400, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
        assert_eq!((s.variant().name, s.range, s.active), ("Alpha", 400, 5));
        let s = LaserEyeState::new(&[9, 400, 0, 0, 5]);
        assert_eq!((s.variant().name, s.range), ("All", 300), "values[15] ≠ 1");
        let s = LaserEyeState::new(&[-3]);
        assert_eq!(s.variant().name, "Alpha");
    }

    #[test]
    fn sweep_goes_back_and_forth_over_twelve_positions() {
        let mut s = LaserEyeState::new(&[0]);
        let mut seen = vec![];
        for _ in 0..24 {
            s.step();
            seen.push(s.pos);
        }
        assert_eq!(
            seen,
            vec![
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0, 1, 2
            ]
        );
        // a mira: posição 0 ≈ 172° (esquerda), 5 ≈ 89° (baixo), 11 ≈ −9,5°
        s.pos = 0;
        assert_eq!(s.aim(), 0x7a24);
        s.pos = 11;
        assert_eq!(angle_diff(s.aim(), 0), -1732);
    }

    #[test]
    fn burst_is_five_shots_every_four_ticks_then_a_pause() {
        let mut s = LaserEyeState::new(&[0]);
        let shots: Vec<u32> = (0..140).filter(|_| s.burst()).collect();
        // tiros nos tiques 3, 7, 11, 15, 19; pausa 10 × 4; recarga; próxima rajada
        assert_eq!(&shots[..5], &[3, 7, 11, 15, 19]);
        assert_eq!(&shots[5..10], &[67, 71, 75, 79, 83]);
    }

    #[test]
    fn eye_sees_jake_only_in_its_cone_and_fires_two_bolts() {
        let on = |_: i32| None;
        // Jake bem abaixo do olho: a mira vertical (posição 5–6) o vê
        let (j, w) = ctx_with_jake(110, 150);
        let mut c = ctx(&j, &w, &on);
        let mut e = eye(100, 0, 400);
        let mut out = Out::default();
        let mut fired_at = None;
        for t in 0..200 {
            c.tick = t;
            tick(&mut e, &mut c, &mut out);
            if out.spawn.len() >= 2 && fired_at.is_none() {
                fired_at = Some((t, state(&e).pos));
            }
        }
        let (_, pos) = fired_at.expect("o olho não atirou");
        assert!((4..=7).contains(&pos), "posição {pos}");
        assert_eq!(out.spawn.len() % 2, 0, "dois raios por tiro");
        let bolt = &out.spawn[0];
        assert_eq!(bolt.kind, crate::entity::LASER);
        let Behaviour::Laser(b) = &bolt.behaviour else {
            panic!()
        };
        assert!(b.vy > 0, "o raio desce: {:?}", (b.vx, b.vy));
        assert!(
            out.events
                .iter()
                .any(|e| matches!(e, Event::Sound(c) if c.id == 3037))
        );
        // fora do alcance: nunca atira
        let (j2, w2) = ctx_with_jake(110, 600);
        let mut c2 = ctx(&j2, &w2, &on);
        let mut e = eye(100, 0, 150);
        let mut out = Out::default();
        for t in 0..200 {
            c2.tick = t;
            tick(&mut e, &mut c2, &mut out);
        }
        assert!(out.spawn.is_empty());
        assert!(
            out.events
                .iter()
                .any(|e| matches!(e, Event::Sound(c) if c.id == SOUND_MOVE)),
            "o olho que varre faz barulho"
        );
    }

    #[test]
    fn any_damage_blows_the_eye_up() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(110, 600);
        let mut c = ctx(&j, &w, &on);
        let mut e = eye(100, 0, 150);
        if let Behaviour::LaserEye(s) = &mut e.behaviour {
            s.hurt.add(&Pending {
                amount: 2,
                ..Default::default()
            });
        }
        let mut out = Out::default();
        tick(&mut e, &mut c, &mut out);
        assert!(!e.alive);
        assert_eq!(out.blasts.len(), 1);
        assert_eq!(
            (
                out.blasts[0].radius,
                out.blasts[0].damage,
                out.blasts[0].push
            ),
            (48, 10, 15)
        );
    }

    /// `olho-laser.md`: atira (passo 6) antes de seguir o alvo (passo 7): o raio sai da boca
    /// da posição de antes do passo de seguir.
    #[test]
    fn shot_leaves_from_the_position_before_following() {
        let on = |_: i32| None;
        // Jake um pouco à esquerda do eixo vertical da posição 5, andando para a esquerda
        let (mut j, w) = ctx_with_jake(104, 150);
        j.body.vx = -10;
        let mut c = ctx(&j, &w, &on);
        let mut e = eye(100, 0, 400);
        if let Behaviour::LaserEye(s) = &mut e.behaviour {
            s.pos = 5;
            s.firing = true;
            s.interval = 0;
            s.shots = 3;
        }
        let mut out = Out::default();
        tick(&mut e, &mut c, &mut out);
        assert_eq!(state(&e).pos, 4, "seguiu o alvo");
        assert_eq!(out.spawn.len(), 2, "atirou");
        assert_eq!(
            (out.spawn[0].x, out.spawn[0].y),
            (100 + MUZZLE1[5].0, MUZZLE1[5].1),
            "boca da posição 5"
        );
    }
}
