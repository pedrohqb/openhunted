// SPDX-License-Identifier: GPL-3.0-or-later
//! Máquina de estados do Jake, fatia M3a (`docs/re/personagem.md`): parado, correr, pulos
//! balísticos, queda e aterrissagem.

use std::collections::HashMap;

use crate::input::{JUMP, LEFT, RIGHT};
use crate::physics::{Body, Rect, SUB, World, step};

/// Correr: 12 px/tique (`0x00478928`).
const RUN: i32 = 12 * SUB;
/// Velocidade de lançamento dos pulos (vx, vy) em px/tique (`0x0047890c`…).
const MEDIUM: (i32, i32) = (10, -11);
const LONG: (i32, i32) = (10, -15);
const EXTENDED: (i32, i32) = (17, -15);
/// Distâncias de pouso sondadas para escolher o pulo (`0x00425144`).
const PROBE_MEDIUM: i32 = 110;
const PROBE_LONG: i32 = 150;
const PROBE_EXTENDED: i32 = 255;
/// Queda: descendo a 5 px/tique ou mais sem chão (`0x0042416f`).
const FALL_SPEED: i32 = 5 * SUB;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JumpKind {
    Medium,
    Long,
    Extended,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Idle,
    Run,
    Push,
    StandJump,
    RunUp,
    Jump(JumpKind),
    Fall,
    /// Aterrissagem; `true` se veio de uma queda.
    Land(bool),
}

/// Rótulo do `JAKE.ANM` e campo c (avança um quadro a cada c+1 tiques) de cada estado.
fn anim_of(s: State) -> (u16, u32) {
    match s {
        State::Idle => (1, 1),
        State::Run => (10, 0),
        State::Push => (100, 0),
        State::StandJump => (27, 0),
        State::RunUp => (31, 0),
        State::Jump(JumpKind::Medium) => (29, 0),
        State::Jump(JumpKind::Long) => (28, 0),
        State::Jump(JumpKind::Extended) => (80, 0),
        State::Fall => (25, 0),
        State::Land(true) => (26, 0),
        State::Land(false) => (35, 0),
    }
}

/// O Jake controlável.
#[derive(Debug, Clone)]
pub struct Jake {
    pub body: Body,
    state: State,
    facing_right: bool,
    ticks: u32,
    prev_input: u32,
    lengths: HashMap<u16, usize>,
    label_changed: bool,
    advance: bool,
}

impl Jake {
    /// `lengths`: número de quadros de cada rótulo do `JAKE.ANM`.
    pub fn new(x: i32, y: i32, boxr: Rect, lengths: HashMap<u16, usize>) -> Self {
        Self {
            body: Body::new(x, y, boxr),
            state: State::Idle,
            facing_right: false, // começa olhando para a esquerda (+0x288 = 4)
            ticks: 0,
            prev_input: 0,
            lengths,
            label_changed: true,
            advance: false,
        }
    }

    pub fn state(&self) -> State {
        self.state
    }
    pub fn label(&self) -> u16 {
        anim_of(self.state).0
    }
    pub fn facing_right(&self) -> bool {
        self.facing_right
    }
    /// O rótulo mudou neste tique (a animação recomeça).
    pub fn label_changed(&self) -> bool {
        self.label_changed
    }
    /// A animação avança um quadro neste tique.
    pub fn advance(&self) -> bool {
        self.advance
    }

    fn set(&mut self, s: State) {
        if s != self.state {
            self.state = s;
            self.ticks = 0;
            self.label_changed = true;
        }
    }

    /// A animação do estado já tocou inteira.
    fn anim_done(&self) -> bool {
        let (label, c) = anim_of(self.state);
        let n = self.lengths.get(&label).copied().unwrap_or(1).max(1) as u32;
        self.ticks >= n.saturating_mul(c + 1)
    }

    fn dir(&self) -> i32 {
        if self.facing_right { 1 } else { -1 }
    }

    /// Lado pedido pelas setas (`true` = direita): só uma delas, ou, com as duas, a apertada
    /// por último (segurar as duas mantém o lado atual). `None` sem nenhuma.
    fn wanted_side(&self, input: u32) -> Option<bool> {
        let (l, r) = (input & LEFT != 0, input & RIGHT != 0);
        let fresh = |bit: u32| input & bit != 0 && self.prev_input & bit == 0;
        match (l, r) {
            (true, false) => Some(false),
            (false, true) => Some(true),
            (true, true) if fresh(RIGHT) && !fresh(LEFT) => Some(true),
            (true, true) if fresh(LEFT) && !fresh(RIGHT) => Some(false),
            (true, true) => Some(self.facing_right),
            (false, false) => None,
        }
    }

    fn choose_jump(&self, world: &World) -> JumpKind {
        let b = self.body.bounds();
        let ahead = |d: i32| {
            world.floor_at(
                &Rect {
                    x: b.x.saturating_add(d * self.dir()),
                    ..b
                },
                8,
            )
        };
        if ahead(PROBE_MEDIUM) {
            JumpKind::Medium
        } else if ahead(PROBE_LONG) {
            JumpKind::Long
        } else if ahead(PROBE_EXTENDED) {
            JumpKind::Extended
        } else {
            JumpKind::Medium
        }
    }

    fn launch(&mut self, kind: JumpKind) {
        let (vx, vy) = match kind {
            JumpKind::Medium => MEDIUM,
            JumpKind::Long => LONG,
            JumpKind::Extended => EXTENDED,
        };
        self.body.vx = vx * SUB * self.dir();
        self.body.vy = vy * SUB;
        self.set(State::Jump(kind));
    }

    /// Um tique (50 ms) com a máscara de entrada `input`.
    pub fn tick(&mut self, input: u32, world: &World) {
        self.label_changed = false;
        let pressed = |bit: u32| input & bit != 0;
        let jump_edge = pressed(JUMP) && self.prev_input & JUMP == 0;
        let side = self.wanted_side(input);
        let airborne = matches!(self.state, State::Jump(_) | State::Fall);
        if let (false, Some(right)) = (airborne, side) {
            self.facing_right = right;
        }
        let moving = side.is_some();
        match self.state {
            State::Idle | State::Run | State::Push => {
                if jump_edge && self.body.on_ground {
                    if moving {
                        match self.choose_jump(world) {
                            JumpKind::Extended => self.set(State::RunUp),
                            k => self.launch(k),
                        }
                    } else {
                        self.body.vx = 0;
                        self.set(State::StandJump);
                    }
                } else if moving {
                    self.body.vx = RUN * self.dir();
                    let next = if self.body.hit_wall {
                        State::Push
                    } else {
                        State::Run
                    };
                    self.set(next);
                } else {
                    self.body.vx = 0;
                    self.set(State::Idle);
                }
            }
            State::StandJump => {
                self.body.vx = 0;
                if self.anim_done() {
                    self.set(State::Idle);
                }
            }
            State::RunUp => {
                // Impulso no lugar: o pulo sai do ponto onde o chão adiante foi sondado (o
                // deslocamento do esqueleto, SKE, ainda não é aplicado).
                self.body.vx = 0;
                if self.anim_done() {
                    self.launch(JumpKind::Extended);
                }
            }
            State::Jump(_) | State::Fall => {
                // No ar, o lado oposto inverte a velocidade horizontal.
                let opposite = match side {
                    Some(right) => (right && self.body.vx < 0) || (!right && self.body.vx > 0),
                    None => false,
                };
                if opposite {
                    self.body.vx = -self.body.vx;
                    self.facing_right = self.body.vx > 0;
                }
            }
            State::Land(_) => {
                self.body.vx = 0;
                if self.anim_done() {
                    self.set(State::Idle);
                }
            }
        }
        if matches!(self.state, State::Run | State::Push) {
            // −1 px por tique enquanto corre (sobe degraus pequenos), se houver espaço.
            let up = Rect {
                y: self.body.bounds().y.saturating_sub(1),
                ..self.body.bounds()
            };
            if !world.blocked(&up) {
                self.body.y = self.body.y.saturating_sub(1);
            }
        }
        let was_rising = self.body.vy < 0;
        step(&mut self.body, world, true);
        match self.state {
            State::Jump(_) | State::Fall if self.body.on_ground && !was_rising => {
                self.body.vx = 0;
                self.body.vy = 0;
                let from_fall = self.state == State::Fall;
                self.set(State::Land(from_fall));
            }
            State::Idle | State::Run | State::Push | State::RunUp
                if !self.body.on_ground && self.body.vy >= FALL_SPEED =>
            {
                self.set(State::Fall)
            }
            _ => {}
        }
        let c = anim_of(self.state).1;
        self.advance = !self.label_changed && self.ticks % (c + 1) == 0;
        self.ticks = self.ticks.saturating_add(1);
        self.prev_input = input;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{JUMP, LEFT, RIGHT};
    use crate::physics::Rect;

    fn lengths() -> HashMap<u16, usize> {
        [
            (1, 34),
            (10, 12),
            (27, 26),
            (29, 9),
            (28, 8),
            (31, 9),
            (80, 15),
            (35, 7),
            (34, 7),
            (25, 1),
            (26, 11),
            (100, 6),
        ]
        .into_iter()
        .collect()
    }

    fn jake_on(world: &World, x: i32) -> Jake {
        let mut j = Jake::new(
            x,
            53,
            Rect {
                x: 2,
                y: 3,
                w: 18,
                h: 44,
            },
            lengths(),
        );
        j.tick(0, world); // assenta no chão
        j
    }

    fn long_floor() -> World {
        World::new(vec![Rect {
            x: -10_000,
            y: 100,
            w: 20_000,
            h: 20,
        }])
    }

    #[test]
    fn idle_runs_and_stops() {
        let w = long_floor();
        let mut j = jake_on(&w, 0);
        assert_eq!((j.state(), j.label()), (State::Idle, 1));
        let x0 = j.body.x;
        j.tick(RIGHT, &w);
        j.tick(RIGHT, &w);
        assert_eq!(j.state(), State::Run);
        assert_eq!(j.label(), 10);
        assert_eq!(j.body.x - x0, 24); // 12 px/tique
        assert!(j.facing_right());
        j.tick(0, &w);
        assert_eq!(j.state(), State::Idle);
        assert_eq!(j.body.vx, 0);
    }

    #[test]
    fn running_jump_with_floor_ahead_is_medium() {
        let w = long_floor();
        let mut j = jake_on(&w, 0);
        j.tick(RIGHT, &w);
        j.tick(RIGHT | JUMP, &w);
        assert_eq!(j.state(), State::Jump(JumpKind::Medium));
        assert_eq!(j.label(), 29);
        assert!(!j.body.on_ground);
        let mut landed = false;
        for _ in 0..30 {
            j.tick(RIGHT, &w);
            if matches!(j.state(), State::Land(_)) {
                landed = true;
                break;
            }
        }
        assert!(landed);
        assert_eq!(j.label(), 35);
    }

    #[test]
    fn jump_kind_follows_floor_probe() {
        // buraco de 20 a 140 px adiante: o pulo médio (110) cairia no buraco, o longo (150) não
        let w = World::new(vec![
            Rect {
                x: -1000,
                y: 100,
                w: 1040,
                h: 20,
            },
            Rect {
                x: 160,
                y: 100,
                w: 2000,
                h: 20,
            },
        ]);
        let mut j = jake_on(&w, 0);
        j.tick(RIGHT, &w);
        j.tick(RIGHT | JUMP, &w);
        assert_eq!(j.state(), State::Jump(JumpKind::Long));
    }

    #[test]
    fn opposite_direction_in_air_reverses() {
        let w = long_floor();
        let mut j = jake_on(&w, 0);
        j.tick(RIGHT, &w);
        j.tick(RIGHT | JUMP, &w);
        let vx = j.body.vx;
        j.tick(LEFT, &w);
        assert_eq!(j.body.vx, -vx);
        assert!(!j.facing_right());
    }

    #[test]
    fn walking_off_a_ledge_falls_then_lands() {
        let w = World::new(vec![
            Rect {
                x: -1000,
                y: 100,
                w: 1030,
                h: 20,
            },
            Rect {
                x: -1000,
                y: 400,
                w: 3000,
                h: 20,
            },
        ]);
        let mut j = jake_on(&w, 0);
        let mut fell = false;
        for _ in 0..40 {
            j.tick(RIGHT, &w);
            if j.state() == State::Fall {
                fell = true;
                assert_eq!(j.label(), 25);
            }
            if fell && j.body.on_ground {
                break;
            }
        }
        assert!(fell);
        j.tick(0, &w);
        assert_eq!(j.label(), 26);
    }

    #[test]
    fn extended_run_up_stays_on_the_ledge_and_clears_the_gap() {
        // chão até 50 px, buraco até 200 px (fundo em y = 400), chão adiante: só o pulo
        // estendido (255 px) acha onde pousar
        let w = World::new(vec![
            Rect {
                x: -1000,
                y: 100,
                w: 1050,
                h: 20,
            },
            Rect {
                x: 200,
                y: 100,
                w: 2000,
                h: 20,
            },
            Rect {
                x: -1000,
                y: 400,
                w: 4000,
                h: 20,
            },
        ]);
        let mut j = jake_on(&w, 0);
        let x0 = j.body.x;
        j.tick(RIGHT | JUMP, &w);
        assert_eq!(j.state(), State::RunUp);
        while j.state() == State::RunUp {
            assert!(
                j.body.on_ground,
                "saiu da beirada no impulso em x={}",
                j.body.x
            );
            j.tick(RIGHT, &w);
        }
        assert_eq!(j.state(), State::Jump(JumpKind::Extended));
        // lança de onde sondou: só o primeiro tique de voo (17 px) desde o início
        assert_eq!(j.body.x - x0, 17);
        for _ in 0..40 {
            j.tick(RIGHT, &w);
            if matches!(j.state(), State::Land(_)) {
                break;
            }
        }
        assert!(matches!(j.state(), State::Land(_)));
        assert!(j.body.y < 100, "pousou no buraco: y={}", j.body.y);
        assert!(j.body.x >= 200);
    }

    #[test]
    fn both_arrows_held_in_air_do_not_jitter() {
        let w = long_floor();
        let mut j = jake_on(&w, 0);
        j.tick(RIGHT, &w);
        j.tick(RIGHT | JUMP, &w);
        let vx = j.body.vx;
        for _ in 0..4 {
            j.tick(LEFT | RIGHT, &w);
        }
        // a seta esquerda apertada por último inverte uma vez só
        assert_eq!(j.body.vx, -vx);
        assert!(!j.facing_right());
    }

    #[test]
    fn newest_arrow_wins_on_the_ground() {
        let w = long_floor();
        let mut j = jake_on(&w, 0);
        j.tick(LEFT, &w);
        assert!(!j.facing_right());
        j.tick(LEFT | RIGHT, &w);
        assert!(j.facing_right());
        assert!(j.body.vx > 0);
        j.tick(LEFT | RIGHT, &w);
        assert!(j.facing_right(), "segurar as duas mantém o lado");
    }

    #[test]
    fn standing_jump_plays_in_place_then_idles() {
        let w = long_floor();
        let mut j = jake_on(&w, 0);
        j.tick(JUMP, &w);
        assert_eq!((j.state(), j.label()), (State::StandJump, 27));
        for _ in 0..30 {
            j.tick(0, &w);
        }
        assert_eq!(j.state(), State::Idle);
    }
}
