// SPDX-License-Identifier: GPL-3.0-or-later
//! Testes sintéticos do cérebro e da busca de caminho.

use super::*;
use crate::input::{DOWN, JUMP, LEFT, PUNCH, RIGHT, SHOOT, STEALTH, UP};
use crate::jake::{JumpKind, Zone, zone};

const FLOOR_Y: i32 = 100;

fn view(ch: Character, x: i32, state: State) -> View {
    View {
        bounds: Rect {
            x,
            y: FLOOR_Y - 44,
            w: 20,
            h: 44,
        },
        layer: 0,
        state,
        character: ch,
        vx: 0,
        vy: 0,
        facing_right: true,
        on_ground: true,
    }
}

fn me(v: View, weapon: usize) -> Me {
    Me {
        view: v,
        hp: 100,
        pending: 0,
        weapon,
    }
}

/// Chão de x0 a x1 (px) em `FLOOR_Y`.
fn floor(spans: &[(i32, i32)]) -> World {
    World::new(
        spans
            .iter()
            .map(|&(a, b)| Rect {
                x: a,
                y: FLOOR_Y,
                w: b - a,
                h: 20,
            })
            .collect(),
    )
}

fn look<'a>(m: &'a Me, t: Option<&'a View>) -> Look<'a> {
    let (dx, dy, d, angle) = match t {
        Some(t) => {
            let (a, b) = (m.view.center(), t.center());
            (
                b.0 - a.0,
                b.1 - a.1,
                dist(a, b),
                angle16(b.0 - a.0, b.1 - a.1),
            )
        }
        None => (0, 0, i32::MAX, 0),
    };
    Look {
        me: m,
        t,
        dx,
        dy,
        d,
        angle,
        tu: tuning(m.view.character),
    }
}

struct Fixture {
    world: World,
    grid: CellGrid,
    threats: Vec<Threat>,
    rng: Rng,
}

impl Fixture {
    fn new(world: World) -> Self {
        Self {
            world,
            grid: CellGrid::new((40, 4, 2)),
            threats: vec![],
            rng: Rng::new(3),
        }
    }
    fn senses(&mut self) -> Senses<'_> {
        Senses {
            world: &self.world,
            grid: &self.grid,
            threats: &self.threats,
            rng: &mut self.rng,
        }
    }
}

fn seeing() -> Brain {
    Brain {
        sees: true,
        ..Brain::default()
    }
}

#[test]
fn thinks_every_other_tick_and_repeats_on_odd_ticks() {
    let mut f = Fixture::new(floor(&[(-1000, 5000)]));
    let m = me(view(Character::Jake, 0, State::Idle), 2);
    let t = view(Character::Jake, 300, State::Idle);
    let mut b = Brain::new();
    let a = b.think(&m, Some(&t), &mut f.senses());
    assert_ne!(a, 0, "vê a 300 px e corre para ele");
    b.last = 0x1234;
    assert_eq!(
        b.think(&m, Some(&t), &mut f.senses()),
        0x1234,
        "tique ímpar repete"
    );
    assert_ne!(
        b.think(&m, Some(&t), &mut f.senses()),
        0x1234,
        "tique par pensa"
    );
}

#[test]
fn modes_retreat_on_damage_wander_when_hidden_and_hunt_again() {
    let mut f = Fixture::new(floor(&[(-1000, 5000)]));
    let mut m = me(view(Character::Jake, 0, State::Idle), 2);
    m.pending = 15;
    let t = view(Character::Jake, 300, State::Idle);
    let mut b = Brain::new();
    assert_eq!(
        b.think(&m, Some(&t), &mut f.senses()),
        LEFT,
        "recua para longe"
    );
    assert_eq!(b.mode, Mode::Retreat);
    // dano com vida < 30 também
    let mut b = Brain::new();
    m.pending = 1;
    m.hp = 20;
    b.think(&m, Some(&t), &mut f.senses());
    assert_eq!(b.mode, Mode::Retreat);
    // escondido e visto → vaga; o alvo reaparece → caça (devolve a entrada antiga)
    m.pending = 0;
    m.hp = 100;
    let mut b = Brain::new();
    let hidden = view(Character::Jake, 300, State::Hidden);
    let w = b.think(&m, Some(&hidden), &mut f.senses());
    assert_eq!(b.mode, Mode::Wander);
    assert!(
        w & STEALTH != 0 || w == TAUNT_INPUT || w == 0,
        "vaga andando: {w:#x}"
    );
    b.think(&m, Some(&hidden), &mut f.senses());
    b.last = 0x77;
    assert_eq!(b.think(&m, Some(&t), &mut f.senses()), 0x77);
    assert_eq!(b.mode, Mode::Hunt);
    // sem alvo: vaga
    let mut b = Brain::new();
    b.think(&m, None, &mut f.senses());
    assert_eq!(b.mode, Mode::Wander);
    // retreat sem ver → caça
    let mut b = Brain {
        mode: Mode::Retreat,
        ..Brain::default()
    };
    let behind = view(Character::Jake, 900, State::Idle);
    b.think(&m, Some(&behind), &mut f.senses());
    assert_eq!(b.mode, Mode::Hunt);
}

#[test]
fn pose_landing_and_special_wait() {
    let mut f = Fixture::new(floor(&[(-1000, 5000)]));
    let t = view(Character::Jake, 100, State::Idle);
    for s in [State::Taunt, State::Land(Landing::Fall), State::Special(0)] {
        let m = me(view(Character::Jake, 0, s), 2);
        assert_eq!(
            Brain::new().think(&m, Some(&t), &mut f.senses()),
            0,
            "{s:?}"
        );
    }
}

/// Alvo à direita, à mesma altura, a `d` px de centro a centro.
fn at(ch: Character, d: i32, state: State) -> View {
    view(ch, d, state)
}

#[test]
fn jake_attack_by_distance() {
    let mut f = Fixture::new(floor(&[(-1000, 5000)]));
    let ch = Character::Jake;
    let m = me(view(ch, 0, State::Idle), 2);
    let b = seeing();
    let atk = |b: &Brain, f: &mut Fixture, t: &View| b.attack(&look(&m, Some(t)), &mut f.senses());
    assert_eq!(atk(&b, &mut f, &at(ch, 5, State::Idle)), 0, "< m");
    assert_eq!(
        atk(&b, &mut f, &at(ch, 15, State::Idle)),
        0,
        "sem tiro anterior"
    );
    let rep = Brain {
        last: PUNCH | RIGHT,
        ..seeing()
    };
    assert_eq!(
        atk(&rep, &mut f, &at(ch, 15, State::Idle)),
        PUNCH | RIGHT,
        "repete"
    );
    assert_eq!(
        atk(&b, &mut f, &at(ch, 25, State::Idle)),
        PUNCH | RIGHT,
        "soco"
    );
    let mut fleeing = at(ch, 25, State::Run);
    fleeing.vx = 96;
    assert_eq!(atk(&b, &mut f, &fleeing), 0, "não bate em quem foge");
    let mut pushing = at(ch, 25, State::Push);
    pushing.vx = 96;
    assert_eq!(atk(&b, &mut f, &pushing), PUNCH | RIGHT, "salvo empurrando");
    assert_eq!(
        atk(&b, &mut f, &at(ch, 100, State::Idle)),
        SHOOT | RIGHT,
        "pistola"
    );
    let fists = me(view(ch, 0, State::Idle), FISTS);
    assert_eq!(
        b.attack(
            &look(&fists, Some(&at(ch, 100, State::Idle))),
            &mut f.senses()
        ),
        0,
        "o Jake só atira com arma ≠ 1"
    );
    assert_eq!(atk(&b, &mut f, &at(ch, 250, State::Idle)), 0, "≥ 200");
    let unseen = Brain::new();
    assert_eq!(
        atk(&unseen, &mut f, &at(ch, 25, State::Idle)),
        0,
        "exige ver"
    );
    let falling = me(view(ch, 0, State::Fall), 2);
    assert_eq!(
        b.attack(
            &look(&falling, Some(&at(ch, 25, State::Idle))),
            &mut f.senses()
        ),
        0,
        "não ataca caindo"
    );
}

#[test]
fn garathe_attack_by_distance_and_leap() {
    let mut f = Fixture::new(floor(&[(-1000, 5000)]));
    let ch = Character::Garathe;
    let b = seeing();
    let m = me(view(ch, 0, State::Idle), 2);
    let atk = |m: &Me, f: &mut Fixture, t: &View| b.attack(&look(m, Some(t)), &mut f.senses());
    let j = Character::Jake;
    assert_eq!(atk(&m, &mut f, &at(j, 10, State::Idle)), 0, "< 12");
    assert_eq!(
        atk(&m, &mut f, &at(j, 40, State::Idle)),
        PUNCH | RIGHT,
        "p = 50"
    );
    assert_eq!(
        atk(&m, &mut f, &at(j, 100, State::Idle)),
        0,
        "clava não é de longe"
    );
    let knives = me(view(ch, 0, State::Idle), 4);
    assert_eq!(
        atk(&knives, &mut f, &at(j, 100, State::Idle)),
        SHOOT | RIGHT
    );
    let mut runner = me(view(ch, 0, State::Run), 2);
    runner.view.facing_right = true;
    let mut t = at(j, 120, State::Run);
    t.vx = 96;
    assert_eq!(atk(&runner, &mut f, &t), RIGHT | JUMP | UP, "pulo longo");
    let near = at(j, 80, State::Run);
    assert_eq!(atk(&runner, &mut f, &near), 0, "d ≤ 90");
}

#[test]
fn aim_sectors_never_give_the_special() {
    assert_eq!(aim_near(0), RIGHT);
    assert_eq!(aim_near(0x2000), RIGHT | DOWN);
    assert_eq!(aim_near(0x4000), RIGHT | DOWN);
    assert_eq!(aim_near(0x6000), LEFT | DOWN);
    assert_eq!(aim_near(0x8000), LEFT);
    assert_eq!(aim_near(0xa000), LEFT | UP);
    assert_eq!(aim_near(0xc000), UP);
    assert_eq!(aim_near(0xe000), RIGHT | UP);
    assert_eq!(aim_far(0x100), Some(RIGHT));
    assert_eq!(aim_far(0x400), None, "longe só alinhado");
    assert_eq!(aim_far(0x4000), None, "nunca só BAIXO");
    assert_eq!(aim_far(0xc1ff), Some(UP));
    for a in (0..0x10000).step_by(0x80) {
        assert_ne!(aim_near(a) & (LEFT | RIGHT | UP), 0, "{a:#x}");
        assert!(aim_far(a).is_none_or(|b| b & (LEFT | RIGHT | UP) != 0));
    }
}

#[test]
fn approach_runs_backs_off_and_jumps_up() {
    let mut f = Fixture::new(floor(&[(-1000, 5000)]));
    let ch = Character::Jake;
    let m = me(view(ch, 0, State::Idle), 2);
    let b = seeing();
    let ap = |f: &mut Fixture, t: &View| b.approach(&look(&m, Some(t)), &mut f.senses());
    assert_eq!(ap(&mut f, &at(ch, 200, State::Idle)), Some(RIGHT), "corre");
    assert_eq!(
        ap(&mut f, &at(ch, 20, State::Idle)),
        Some(STEALTH | LEFT),
        "recua"
    );
    let mut walking = at(ch, 20, State::Run);
    walking.vx = -96;
    assert_eq!(ap(&mut f, &walking), Some(LEFT), "segue o vx");
    let mut above = at(ch, 0, State::Idle);
    above.bounds.y -= 200;
    assert_eq!(ap(&mut f, &above), Some(JUMP | UP));
    assert_eq!(
        ap(&mut f, &at(ch, 200, State::Dying(173))),
        None,
        "classe 14"
    );
    // parede atrás: recua para o outro lado
    let mut fw = Fixture::new(World::new(vec![
        Rect {
            x: -1000,
            y: FLOOR_Y,
            w: 6000,
            h: 20,
        },
        Rect {
            x: -15,
            y: 0,
            w: 5,
            h: FLOOR_Y,
        },
    ]));
    assert_eq!(ap(&mut fw, &at(ch, 20, State::Idle)), Some(STEALTH | RIGHT));
}

#[test]
fn gap_probe_jumps_with_landing_or_in_place() {
    let ch = Character::Jake;
    let m = me(view(ch, 170, State::Run), 2);
    let b = seeing();
    let mut ground = Fixture::new(floor(&[(-1000, 5000)]));
    assert_eq!(
        b.with_gap(&look(&m, None), &mut ground.senses(), RIGHT),
        RIGHT
    );
    // borda em 200; pouso a 110 px (faixa em 285–295)
    let mut land = Fixture::new(floor(&[(-1000, 200), (280, 400)]));
    assert_eq!(
        b.with_gap(&look(&m, None), &mut land.senses(), RIGHT),
        RIGHT | JUMP
    );
    let mut hole = Fixture::new(floor(&[(-1000, 200)]));
    assert_eq!(
        b.with_gap(&look(&m, None), &mut hole.senses(), RIGHT),
        JUMP | UP,
        "sem pouso: pula no lugar"
    );
    assert_eq!(
        b.with_gap(&look(&m, None), &mut hole.senses(), RIGHT | STEALTH),
        RIGHT | STEALTH,
        "só com um lado"
    );
    // o Garathe tem outros pousos (156 → 331–341)
    let g = me(view(Character::Garathe, 170, State::Run), 2);
    let mut gl = Fixture::new(floor(&[(-1000, 200), (340, 400)]));
    assert_eq!(
        b.with_gap(&look(&g, None), &mut gl.senses(), RIGHT),
        RIGHT | JUMP
    );
    assert_eq!(
        b.with_gap(&look(&m, None), &mut gl.senses(), RIGHT),
        JUMP | UP
    );
}

fn threat(kind: u32, x: i32, y: i32, vel: (i32, i32)) -> Threat {
    Threat {
        kind,
        center: (x, y),
        vel,
    }
}

#[test]
fn hazard_reactions() {
    let ch = Character::Jake;
    let m = me(view(ch, 0, State::Idle), 2);
    let c = m.view.center();
    let b = seeing();
    let react = |ts: Vec<Threat>, m: &Me, input: u32| {
        let mut f = Fixture::new(floor(&[(-1000, 5000)]));
        f.threats = ts;
        b.hazards(&look(m, None), &mut f.senses(), input)
    };
    assert_eq!(
        react(vec![threat(19, c.0 + 50, c.1, (-20, 0))], &m, RIGHT),
        DOWN
    );
    assert_eq!(
        react(vec![threat(19, c.0 + 50, c.1, (20, 0))], &m, RIGHT),
        RIGHT,
        "indo embora"
    );
    assert_eq!(
        react(vec![threat(19, c.0 + 150, c.1, (-20, 0))], &m, RIGHT),
        RIGHT,
        "longe"
    );
    let crawling = me(view(ch, 0, State::Crawl), 2);
    assert_eq!(
        react(vec![threat(19, c.0 + 50, c.1, (-20, 0))], &crawling, LEFT),
        LEFT,
        "rastejando"
    );
    assert_eq!(
        react(vec![threat(26, c.0 + 40, c.1, (-5, 0))], &m, 0),
        RIGHT | JUMP,
        "granada vindo: pula por cima"
    );
    assert_eq!(
        react(vec![threat(42, c.0 + 40, c.1, (0, 0))], &m, 0) & JUMP,
        JUMP
    );
    assert_eq!(
        react(vec![threat(39, c.0, c.1 - 85, (0, 0))], &m, RIGHT | SHOOT),
        SHOOT,
        "pistão: só PULAR/ATIRAR/SOCO"
    );
    assert_eq!(
        react(vec![threat(39, c.0, c.1 - 60, (0, 0))], &m, RIGHT),
        DOWN
    );
    let mut running = me(view(ch, 0, State::Run), 2);
    running.view.vx = 96;
    assert_eq!(
        react(vec![threat(40, c.0 + 40, c.1, (0, 0))], &running, RIGHT),
        RIGHT | JUMP | UP
    );
    assert_eq!(
        react(vec![threat(11, c.0 - 40, c.1 + 10, (0, 0))], &m, RIGHT),
        LEFT | SHOOT | DOWN,
        "mina saltadora: atira virado"
    );
    // o mais próximo decide
    assert_eq!(
        react(
            vec![
                threat(19, c.0 + 80, c.1, (-20, 0)),
                threat(11, c.0 - 40, c.1, (0, 0))
            ],
            &m,
            0
        ),
        LEFT | SHOOT
    );
}

fn grid(size: (i32, i32, i32)) -> CellGrid {
    CellGrid::new(size)
}

#[test]
fn path_search_obeys_each_bit() {
    use path::bits::*;
    let mut g = grid((5, 3, 2));
    // fechar as bordas inferiores da linha 2 não importa para andar de lado
    assert_eq!(g.search((0, 1, 0), (3, 1, 0)), Some(4));
    assert_eq!(g.search((3, 1, 0), (0, 1, 0)), Some(3));
    assert_eq!(g.search((1, 0, 0), (1, 2, 0)), Some(2));
    assert_eq!(g.search((1, 1, 0), (1, 1, 0)), None, "já está");
    // subir exige corda e não 0x40
    assert_eq!(g.search((1, 1, 0), (1, 0, 0)), None, "sem corda");
    g.get_mut((1, 1, 0)).unwrap().zones = zone::ROPE;
    assert_eq!(g.search((1, 1, 0), (1, 0, 0)), Some(1));
    g.get_mut((1, 1, 0)).unwrap().bits = CLOSED_ABOVE;
    assert_eq!(g.search((1, 1, 0), (1, 0, 0)), None);
    // parede à direita: contorna por baixo e sobe pela corda da outra coluna
    let mut g = grid((5, 3, 1));
    g.get_mut((0, 1, 0)).unwrap().bits = WALL_RIGHT;
    assert_eq!(
        g.search((0, 1, 0), (2, 1, 0)),
        None,
        "sem corda não volta a subir"
    );
    g.get_mut((2, 2, 0)).unwrap().zones = zone::ROPE;
    assert_eq!(g.search((0, 1, 0), (2, 1, 0)), Some(2));
    g.get_mut((0, 1, 0)).unwrap().bits = WALL_RIGHT | CLOSED_BELOW;
    assert_eq!(g.search((0, 1, 0), (2, 1, 0)), None);
    // parede à esquerda
    let mut g = grid((5, 1, 1));
    g.get_mut((2, 0, 0)).unwrap().bits = WALL_LEFT;
    assert_eq!(g.search((2, 0, 0), (0, 0, 0)), None);
    // porta fechada no destino
    let mut g = grid((5, 1, 1));
    g.get_mut((1, 0, 0)).unwrap().door_closed = true;
    assert_eq!(g.search((0, 0, 0), (3, 0, 0)), None);
    g.get_mut((1, 0, 0)).unwrap().door_closed = false;
    assert_eq!(g.search((0, 0, 0), (3, 0, 0)), Some(4));
    // camadas pelas zonas 1 e 2
    let mut g = grid((2, 1, 2));
    assert_eq!(g.search((0, 0, 0), (0, 0, 1)), None);
    g.get_mut((0, 0, 0)).unwrap().zones = zone::LAYER_DOWN;
    assert_eq!(g.search((0, 0, 0), (0, 0, 1)), Some(6));
    assert_eq!(g.search((1, 0, 0), (0, 0, 1)), Some(3));
    g.get_mut((0, 0, 1)).unwrap().zones = zone::LAYER_UP;
    assert_eq!(g.search((0, 0, 1), (1, 0, 0)), Some(5));
    // Manhattan > 10: nem tenta
    let g = grid((30, 1, 1));
    assert_eq!(g.search((0, 0, 0), (10, 0, 0)), Some(4));
    assert_eq!(g.search((0, 0, 0), (11, 0, 0)), None);
    // empate na ordem das direções; fora da grade
    let g = grid((3, 3, 1));
    assert_eq!(
        g.search((0, 0, 0), (1, 1, 0)),
        Some(2),
        "descer vem antes de direita"
    );
    assert_eq!(g.allowed((0, 0, 0), 3), None);
}

#[test]
fn zones_go_to_the_cell_of_their_centre() {
    let mut g = grid((3, 2, 2));
    g.add_zones(&[Zone {
        layer: 1,
        kind: zone::ROPE,
        rect: Rect {
            x: 300,
            y: 10,
            w: 20,
            h: 200,
        },
    }]);
    assert_eq!(g.get((2, 0, 1)).unwrap().zones, zone::ROPE);
    assert_eq!(g.get((2, 0, 0)).unwrap().zones, 0);
}

#[test]
fn unseen_target_is_followed_by_path_steps_then_waits() {
    // parede entre os dois na mesma linha de células; caminho livre por baixo? não: só lado
    let mut f = Fixture::new(World::new(vec![
        Rect {
            x: -1000,
            y: FLOOR_Y,
            w: 6000,
            h: 20,
        },
        Rect {
            x: 300,
            y: 0,
            w: 10,
            h: FLOOR_Y,
        },
    ]));
    let m = me(view(Character::Jake, 10, State::Idle), 2);
    let t = view(Character::Jake, 500, State::Idle);
    let mut b = Brain::new();
    assert_eq!(b.think(&m, Some(&t), &mut f.senses()), RIGHT, "passo 4");
    assert!(!b.sees);
    let st = b.step.expect("passo");
    assert_eq!((st.cell, st.dir, st.wait), ((1, 0, 0), 4, STEP_WAIT));
    // sem caminho (parede na célula): espera 10 tiques
    f.grid.get_mut((0, 0, 0)).unwrap().bits = path::bits::WALL_RIGHT;
    let mut b = Brain::new();
    assert_eq!(b.think(&m, Some(&t), &mut f.senses()), 0);
    assert_eq!(b.wait, NO_PATH_WAIT);
}

#[test]
fn path_steps_centre_before_climbing_and_use_layers() {
    let mut f = Fixture::new(floor(&[(-1000, 5000)]));
    let b = Brain::new();
    // centro x = 10 + 10 = 20 → ox = −44: vai para a direita até centrar
    let m = me(view(Character::Jake, 10, State::Idle), 2);
    assert_eq!(b.step_input(&look(&m, None), &mut f.senses(), 1), RIGHT);
    let c = me(view(Character::Jake, 54, State::Idle), 2);
    assert_eq!(b.step_input(&look(&c, None), &mut f.senses(), 1), UP);
    assert_eq!(
        b.step_input(&look(&c, None), &mut f.senses(), 2),
        JUMP | DOWN
    );
    assert_eq!(b.step_input(&look(&c, None), &mut f.senses(), 5), UP);
    assert_eq!(b.step_input(&look(&c, None), &mut f.senses(), 6), DOWN);
    let climbing = me(
        view(
            Character::Jake,
            54,
            State::Climb(crate::jake::Climb::Vertical),
        ),
        2,
    );
    assert_eq!(
        b.step_input(&look(&climbing, None), &mut f.senses(), 6),
        JUMP
    );
    assert_eq!(
        b.step_input(&look(&climbing, None), &mut f.senses(), 4),
        RIGHT | UP
    );
}

#[test]
fn classes_follow_the_state_lists() {
    let j = Character::Jake;
    let g = Character::Garathe;
    assert_eq!(class_of(State::Idle, j), class::IDLE);
    assert_eq!(class_of(State::Taunt, g), class::POSE);
    assert_eq!(class_of(State::Run, j), class::WALK);
    assert_eq!(class_of(State::LayerUp(3), j), class::LAYER);
    assert_eq!(class_of(State::LayerUp(3), g), class::WALK);
    assert_eq!(class_of(State::Jump(JumpKind::Long), j), class::RISE);
    assert_eq!(class_of(State::Fall, g), class::FALL);
    assert_eq!(class_of(State::Hidden, j), class::HIDDEN);
    assert_eq!(class_of(State::Dead, j), class::DYING);
    assert!(can_attack(State::Land(Landing::Long), j));
    assert!(!can_attack(State::Land(Landing::Long), g));
}
