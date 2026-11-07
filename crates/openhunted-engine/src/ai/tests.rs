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

fn look<'a>(m: &'a Me, t: Option<&'a View>, grid: &CellGrid) -> Look<'a> {
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
        mc: m.view.class(),
        tc: t.map_or(class::NONE, View::class),
        here: grid.cell_of(m.view.center(), m.view.layer),
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
    fn open() -> Self {
        Self::new(floor(&[(-1000, 5000)]))
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
        target: true,
        ..Brain::default()
    }
}

/// Alvo à direita, à mesma altura, com o canto esquerdo em `x`.
fn at(ch: Character, x: i32, state: State) -> View {
    view(ch, x, state)
}

/// Uma chamada de `f` sobre o cérebro `b`: (conseguiu, entrada).
fn run(
    f: &mut Fixture,
    m: &Me,
    t: Option<&View>,
    call: impl FnOnce(&mut Brain, &Look, &mut Senses, &mut u32) -> bool,
    b: &mut Brain,
) -> (bool, u32) {
    let grid = f.grid.clone();
    let l = look(m, t, &grid);
    let mut inp = 0;
    let ok = call(b, &l, &mut f.senses(), &mut inp);
    (ok, inp)
}

#[test]
fn thinks_every_other_tick_and_repeats_on_odd_ticks() {
    let mut f = Fixture::open();
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
fn target_is_acquired_by_cell_or_path_and_then_kept() {
    // humano na outra camada, sem passagem: fora do alcance da busca
    let mut f = Fixture::open();
    let m = me(view(Character::Jake, 44, State::Idle), 2);
    let mut t = view(Character::Jake, 300, State::Idle);
    t.layer = 1;
    let mut b = Brain::new();
    for _ in 0..40 {
        assert_eq!(b.think(&m, Some(&t), &mut f.senses()), 0, "parado");
    }
    assert!(!b.target);
    assert_eq!(b.mode, Mode::Wander);
    // na mesma célula: vira alvo (mesmo sem a busca)
    let mut same = view(Character::Jake, 60, State::Idle);
    same.layer = 0;
    f.grid.get_mut((0, 0, 0)).unwrap().bits = path::bits::WALL_RIGHT | path::bits::WALL_LEFT;
    b.think(&m, Some(&same), &mut f.senses());
    assert!(b.target, "mesma célula");
    // e fica, ainda que agora a busca não o alcance
    b.think(&m, Some(&t), &mut f.senses());
    b.think(&m, Some(&t), &mut f.senses());
    assert!(b.target, "o alvo fica");
    // morto (ou ausente): sem alvo
    b.think(&m, None, &mut f.senses());
    b.think(&m, None, &mut f.senses());
    assert!(!b.target);
    // alcançável pela busca
    let mut f = Fixture::open();
    let far = view(Character::Jake, 900, State::Idle);
    let mut b = Brain::new();
    b.think(&m, Some(&far), &mut f.senses());
    assert!(b.target, "a busca chega");
}

#[test]
fn without_a_target_only_hazards_act() {
    let mut f = Fixture::open();
    let m = me(view(Character::Jake, 44, State::Idle), 2);
    let mut b = Brain::new();
    for _ in 0..60 {
        assert_eq!(b.think(&m, None, &mut f.senses()), 0, "sem pose nem passo");
    }
    let c = m.view.center();
    f.threats = vec![threat(11, c.0 + 30, c.1, (0, 0))];
    b.think(&m, None, &mut f.senses());
    assert_eq!(b.think(&m, None, &mut f.senses()), b.last);
    assert_eq!(b.last, SHOOT | RIGHT, "o perigo ainda reage");
}

#[test]
fn modes_retreat_on_damage_wander_when_hidden_and_hunt_again() {
    let mut f = Fixture::open();
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
    // escondido e visto → vaga; o alvo reaparece → caça, com a entrada 0
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
    assert_eq!(b.think(&m, Some(&t), &mut f.senses()), 0);
    assert_eq!(b.mode, Mode::Hunt);
    // recuar sem ver → caça
    let mut b = Brain {
        mode: Mode::Retreat,
        target: true,
        ..Brain::default()
    };
    let mut behind = view(Character::Jake, 600, State::Idle);
    behind.layer = 1;
    b.think(&m, Some(&behind), &mut f.senses());
    assert_eq!(b.mode, Mode::Hunt);
}

#[test]
fn pose_landing_and_special_wait() {
    let mut f = Fixture::open();
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

fn attack(f: &mut Fixture, b: &mut Brain, m: &Me, t: &View) -> (bool, u32) {
    run(f, m, Some(t), |b, l, _, i| b.attack(l, i), b)
}

#[test]
fn jake_attack_by_distance() {
    let mut f = Fixture::open();
    let ch = Character::Jake;
    let m = me(view(ch, 0, State::Idle), 2);
    let mut b = seeing();
    assert_eq!(
        attack(&mut f, &mut b, &m, &at(ch, 5, State::Idle)),
        (false, 0),
        "< m"
    );
    assert_eq!(
        attack(&mut f, &mut b, &m, &at(ch, 15, State::Idle)),
        (false, 0),
        "sem tiro anterior"
    );
    let mut rep = Brain {
        last: PUNCH | RIGHT,
        ..seeing()
    };
    assert_eq!(
        attack(&mut f, &mut rep, &m, &at(ch, 15, State::Idle)),
        (true, PUNCH | RIGHT),
        "repete"
    );
    assert_eq!(
        attack(&mut f, &mut b, &m, &at(ch, 25, State::Idle)),
        (true, PUNCH | RIGHT),
        "soco"
    );
    let mut fleeing = at(ch, 25, State::Run);
    fleeing.vx = 96;
    assert_eq!(
        attack(&mut f, &mut b, &m, &fleeing),
        (false, 0),
        "não bate em quem foge"
    );
    let mut pushing = at(ch, 25, State::Push);
    pushing.vx = 96;
    assert_eq!(
        attack(&mut f, &mut b, &m, &pushing),
        (true, PUNCH | RIGHT),
        "salvo empurrando"
    );
    assert_eq!(
        attack(&mut f, &mut b, &m, &at(ch, 100, State::Idle)),
        (true, SHOOT | RIGHT),
        "pistola"
    );
    let fists = me(view(ch, 0, State::Idle), FISTS);
    assert_eq!(
        attack(&mut f, &mut b, &fists, &at(ch, 100, State::Idle)),
        (false, 0),
        "o Jake só atira com arma ≠ 1"
    );
    assert_eq!(
        attack(&mut f, &mut b, &m, &at(ch, 250, State::Idle)),
        (false, 0),
        "≥ 200"
    );
    let mut unseen = Brain::new();
    assert!(
        !attack(&mut f, &mut unseen, &m, &at(ch, 25, State::Idle)).0,
        "exige ver"
    );
    // o estado próprio não barra o ataque (só a classe, na escolha do caso)
    let falling = me(view(ch, 0, State::Fall), 2);
    assert_eq!(
        attack(&mut f, &mut b, &falling, &at(ch, 25, State::Idle)),
        (true, PUNCH | RIGHT)
    );
}

#[test]
fn a_shoved_or_downed_target_is_not_attacked() {
    let mut f = Fixture::open();
    let j = Character::Jake;
    let g = Character::Garathe;
    let mut b = seeing();
    for (ch, s) in [
        (j, State::Pushed),
        (j, State::KnockedDown(true)),
        (j, State::GetUp),
        (j, State::Land(Landing::Hard)),
        (j, State::Shocked(3)),
        (g, State::Pushed),
        (g, State::Jolted),
        (g, State::Rise),
        (g, State::Land(Landing::Long)),
    ] {
        let m = me(view(j, 0, State::Idle), 2);
        // perto: conseguiu, com a entrada 0 (o pensamento para aí)
        assert_eq!(
            attack(&mut f, &mut b, &m, &at(ch, 25, s)),
            (true, 0),
            "{s:?}"
        );
        // longe: não conseguiu
        assert_eq!(
            attack(&mut f, &mut b, &m, &at(ch, 100, s)),
            (false, 0),
            "{s:?}"
        );
    }
    // o Jake aterrissando longo (34) e o Garathe em choque (121) podem levar
    let m = me(view(j, 0, State::Idle), 2);
    assert_eq!(
        attack(&mut f, &mut b, &m, &at(j, 25, State::Land(Landing::Long))).1,
        PUNCH | RIGHT
    );
    assert_eq!(
        attack(&mut f, &mut b, &m, &at(g, 25, State::Shocked(3))).1,
        PUNCH | RIGHT
    );
    // no pensamento: humano empurrado a 25 px → nenhuma entrada de ataque
    let t = at(j, 25, State::Pushed);
    let mut b = Brain::new();
    for _ in 0..10 {
        let i = b.think(&m, Some(&t), &mut f.senses());
        assert_eq!(i & (SHOOT | PUNCH), 0);
    }
}

#[test]
fn garathe_attack_by_distance_and_leap() {
    let mut f = Fixture::open();
    let ch = Character::Garathe;
    let mut b = seeing();
    let m = me(view(ch, 0, State::Idle), 2);
    let j = Character::Jake;
    assert!(
        !attack(&mut f, &mut b, &m, &at(j, 10, State::Idle)).0,
        "< 12"
    );
    assert_eq!(
        attack(&mut f, &mut b, &m, &at(j, 40, State::Idle)),
        (true, PUNCH | RIGHT),
        "p = 50"
    );
    assert_eq!(
        attack(&mut f, &mut b, &m, &at(j, 100, State::Idle)),
        (false, 0),
        "clava não é de longe"
    );
    let knives = me(view(ch, 0, State::Idle), 4);
    assert_eq!(
        attack(&mut f, &mut b, &knives, &at(j, 100, State::Idle)),
        (true, SHOOT | RIGHT)
    );
    // pulo longo: correndo, mesmo sinal de vx dos dois (0 conta como +), alvo correndo
    let mut runner = me(view(ch, 0, State::Run), 2);
    runner.view.facing_right = false;
    runner.view.vx = 120;
    let mut t = at(j, 120, State::Run);
    t.vx = 96;
    assert_eq!(
        attack(&mut f, &mut b, &runner, &t),
        (true, RIGHT | JUMP | UP),
        "pulo longo (o lado em que olha não conta)"
    );
    t.vx = 0;
    assert_eq!(
        attack(&mut f, &mut b, &runner, &t).1,
        RIGHT | JUMP | UP,
        "vx 0 = +"
    );
    t.vx = -96;
    assert_eq!(
        attack(&mut f, &mut b, &runner, &t),
        (false, 0),
        "sentidos opostos"
    );
    let walking = at(j, 120, State::Stealth);
    assert_eq!(
        attack(&mut f, &mut b, &runner, &walking),
        (false, 0),
        "o alvo precisa correr (estado 10)"
    );
    let near = at(j, 80, State::Run);
    assert!(!attack(&mut f, &mut b, &runner, &near).0, "d ≤ 90");
}

#[test]
fn aim_sectors_never_give_the_special() {
    let near = [
        (0xf800, RIGHT),
        (0x7ff, RIGHT),
        (0x800, RIGHT | DOWN),
        (0x3fff, RIGHT | DOWN),
        (0x4000, LEFT | DOWN),
        (0x77ff, LEFT | DOWN),
        (0x7800, LEFT),
        (0x8fff, LEFT),
        (0x9000, LEFT | UP),
        (0xafff, LEFT | UP),
        (0xb000, UP),
        (0xcfff, UP),
        (0xd000, RIGHT | UP),
        (0xf7ff, RIGHT | UP),
    ];
    for (a, bits) in near {
        assert_eq!(aim_near(a), bits, "{a:#x}");
    }
    assert_eq!(aim_far(0x100), Some(RIGHT));
    assert_eq!(aim_far(0xfe00), Some(RIGHT));
    assert_eq!(aim_far(0x1ff), Some(RIGHT));
    assert_eq!(aim_far(0x200), None, "[c − 0x200, c + 0x200)");
    assert_eq!(aim_far(0x400), None, "longe só alinhado");
    assert_eq!(aim_far(0x4000), None, "nunca só BAIXO");
    assert_eq!(aim_far(0xc1ff), Some(UP));
    assert_eq!(aim_far(0xc200), None);
    for a in (0..0x10000).step_by(0x80) {
        assert_ne!(aim_near(a) & (LEFT | RIGHT | UP), 0, "{a:#x}");
        assert!(aim_far(a).is_none_or(|b| b & (LEFT | RIGHT | UP) != 0));
    }
}

fn approach(f: &mut Fixture, m: &Me, t: &View) -> (bool, u32) {
    run(
        f,
        m,
        Some(t),
        |b, l, s, i| b.approach(l, s, i),
        &mut seeing(),
    )
}

#[test]
fn approach_runs_backs_off_follows_and_jumps_up() {
    let mut f = Fixture::open();
    let ch = Character::Jake;
    let m = me(view(ch, 0, State::Idle), 2);
    assert_eq!(
        approach(&mut f, &m, &at(ch, 200, State::Idle)),
        (true, RIGHT),
        "corre"
    );
    assert_eq!(
        approach(&mut f, &m, &at(ch, 20, State::Idle)),
        (true, STEALTH | LEFT),
        "recua"
    );
    assert_eq!(
        approach(&mut f, &m, &at(ch, 5, State::Idle)),
        (true, STEALTH | LEFT),
        "|dx| ≤ 10 perto: recua"
    );
    // segue o vx de qualquer classe, salvo empurrando
    let mut walking = at(ch, 20, State::Run);
    walking.vx = -96;
    assert_eq!(approach(&mut f, &m, &walking), (true, LEFT), "segue o vx");
    let mut sliding = at(ch, 20, State::Idle);
    sliding.vx = 40;
    assert_eq!(
        approach(&mut f, &m, &sliding),
        (true, RIGHT),
        "classe 1 também"
    );
    let mut pushing = at(ch, 20, State::Push);
    pushing.vx = 40;
    assert_eq!(
        approach(&mut f, &m, &pushing),
        (true, STEALTH | LEFT),
        "empurrando: recua"
    );
    // perto pela distância, não pelo dx: 25 de dx e 15 de dy → d = 29 < 30
    let mut diag = at(ch, 25, State::Idle);
    diag.bounds.y -= 15;
    assert_eq!(approach(&mut f, &m, &diag), (true, STEALTH | LEFT));
    // na barra/escalada parado: nenhuma entrada, mas conseguiu
    let mut bar = at(ch, 20, State::BarHang);
    bar.bounds.y = m.view.bounds.y;
    assert_eq!(approach(&mut f, &m, &bar), (true, 0));
    let mut above = at(ch, 0, State::Idle);
    above.bounds.y -= 200;
    assert_eq!(approach(&mut f, &m, &above), (true, JUMP | UP));
    assert_eq!(
        approach(&mut f, &m, &at(ch, 200, State::Dying(173))),
        (false, 0),
        "classe 14"
    );
    // escalando: lado e CIMA/BAIXO pelo ângulo; fora de 0x9000–0xf000, sempre BAIXO
    let climbing = me(view(ch, 0, State::Climb(crate::jake::Climb::Vertical)), 2);
    let mut up_right = at(ch, 100, State::Idle);
    up_right.bounds.y -= 60;
    assert_eq!(approach(&mut f, &climbing, &up_right), (true, RIGHT | UP));
    let mut up_left = at(ch, -100, State::Idle);
    up_left.bounds.y += 60;
    assert_eq!(approach(&mut f, &climbing, &up_left), (true, LEFT | DOWN));
    assert_eq!(
        approach(&mut f, &m, &up_right),
        (false, 0),
        "sem escalar, fora de ±20 de altura e sem estar bem acima: falha"
    );
    // parede: inverte tanto o recuo quanto o seguir
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
    assert_eq!(
        approach(&mut fw, &m, &at(ch, 20, State::Idle)),
        (true, STEALTH | RIGHT)
    );
    assert_eq!(approach(&mut fw, &m, &walking), (true, RIGHT));
}

fn probe(f: &mut Fixture, m: &Me, input: u32) -> (bool, u32) {
    let grid = f.grid.clone();
    let l = look(m, None, &grid);
    let mut i = input;
    let ok = gap(&l, &f.senses(), &mut i);
    (ok, i)
}

#[test]
fn gap_probe_jumps_with_landing_or_fails() {
    let ch = Character::Jake;
    let m = me(view(ch, 170, State::Run), 2);
    let mut ground = Fixture::open();
    assert_eq!(probe(&mut ground, &m, RIGHT), (true, RIGHT));
    // borda em 200; pouso a 110 px (faixa em 285–295)
    let mut land = Fixture::new(floor(&[(-1000, 200), (280, 400)]));
    assert_eq!(probe(&mut land, &m, RIGHT), (true, RIGHT | JUMP));
    let crouched = me(view(ch, 170, State::Crouch), 2);
    assert_eq!(
        probe(&mut land, &crouched, RIGHT),
        (true, UP),
        "agachado: CIMA"
    );
    let mut hole = Fixture::new(floor(&[(-1000, 200)]));
    assert_eq!(probe(&mut hole, &m, RIGHT), (false, RIGHT), "sem pouso");
    assert_eq!(
        probe(&mut hole, &m, RIGHT | STEALTH),
        (true, RIGHT | STEALTH),
        "só com um lado"
    );
    // o Garathe tem outros pousos (156 → 331–341)
    let g = me(view(Character::Garathe, 170, State::Run), 2);
    let mut gl = Fixture::new(floor(&[(-1000, 200), (340, 400)]));
    assert_eq!(probe(&mut gl, &g, RIGHT), (true, RIGHT | JUMP));
    assert!(!probe(&mut gl, &m, RIGHT).0);
}

fn threat(kind: u32, x: i32, y: i32, vel: (i32, i32)) -> Threat {
    Threat {
        kind,
        center: (x, y),
        vel,
    }
}

fn react(ts: Vec<Threat>, m: &Me, input: u32) -> u32 {
    let mut f = Fixture::open();
    f.threats = ts;
    let grid = f.grid.clone();
    let l = look(m, None, &grid);
    let mut i = input;
    seeing().hazards(&l, &mut f.senses(), &mut i);
    i
}

#[test]
fn hazard_reactions_from_the_list() {
    let ch = Character::Jake;
    let m = me(view(ch, 300, State::Idle), 2);
    let c = m.view.center();
    // projétil: só o sinal do vx em relação à IA
    assert_eq!(
        react(vec![threat(19, c.0 + 50, c.1, (-20, 0))], &m, RIGHT),
        DOWN
    );
    assert_eq!(
        react(vec![threat(19, c.0 + 50, c.1 - 40, (-1, 30))], &m, RIGHT),
        DOWN,
        "só o x conta"
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
    // rastejando não se abaixa; agachado (classe 8) sem outra reação, CIMA
    let crawling = me(view(ch, 300, State::Crawl), 2);
    assert_eq!(
        react(vec![threat(19, c.0 + 50, c.1, (-20, 0))], &crawling, LEFT),
        UP,
        "rastejando"
    );
    // pular por cima: a entrada aponta para o objeto → +PULAR+CIMA; senão lado ao acaso
    assert_eq!(
        react(vec![threat(26, c.0 + 40, c.1, (0, 0))], &m, RIGHT | STEALTH),
        RIGHT | STEALTH | JUMP | UP
    );
    let r = react(vec![threat(42, c.0 + 40, c.1, (0, 0))], &m, LEFT);
    assert!(r == LEFT | JUMP | UP || r == RIGHT | JUMP | UP, "{r:#x}");
    // pistão: mantém PULAR/ATIRAR/SOCO; abaixo a < 75 → BAIXO; senão a entrada inteira
    assert_eq!(
        react(vec![threat(39, c.0, c.1 - 60, (0, 0))], &m, RIGHT | SHOOT),
        RIGHT | SHOOT
    );
    assert_eq!(
        react(vec![threat(39, c.0, c.1 - 60, (0, 0))], &m, RIGHT | JUMP),
        RIGHT | JUMP
    );
    assert_eq!(
        react(vec![threat(39, c.0, c.1 - 60, (0, 0))], &m, RIGHT),
        DOWN
    );
    assert_eq!(
        react(vec![threat(39, c.0, c.1 - 85, (0, 0))], &m, RIGHT),
        RIGHT,
        "75–95: não mexe no movimento"
    );
    let crouched = me(view(ch, 300, State::Crouch), 2);
    assert_eq!(
        react(vec![threat(39, c.0, c.1 - 85, (0, 0))], &crouched, LEFT),
        LEFT | DOWN,
        "agachado: +BAIXO"
    );
    // cerca: pela entrada, não pelo vx
    assert_eq!(
        react(vec![threat(40, c.0 + 40, c.1, (0, 0))], &m, RIGHT),
        RIGHT | JUMP | UP
    );
    assert_eq!(
        react(vec![threat(41, c.0 + 40, c.1, (0, 0))], &m, LEFT),
        LEFT
    );
    // atirar: 11, 89, 105, 109
    assert_eq!(
        react(vec![threat(11, c.0 - 40, c.1 + 10, (0, 0))], &m, RIGHT),
        LEFT | SHOOT | DOWN,
        "mina saltadora: atira virado"
    );
    assert_eq!(
        react(vec![threat(105, c.0 + 40, c.1, (0, 0))], &m, 0),
        RIGHT | SHOOT
    );
    // percorre a lista pela distância: o primeiro cuja condição vale
    assert_eq!(
        react(
            vec![
                threat(11, c.0 - 40, c.1, (0, 0)),
                threat(19, c.0 + 30, c.1, (20, 0)),
            ],
            &m,
            0
        ),
        LEFT | SHOOT,
        "o projétil mais perto vai embora: passa ao próximo"
    );
    assert_eq!(
        react(
            vec![
                threat(11, c.0 - 40, c.1, (0, 0)),
                threat(19, c.0 + 30, c.1, (-20, 0)),
            ],
            &m,
            0
        ),
        DOWN
    );
    // agachado sem perigo: CIMA
    assert_eq!(react(vec![], &crouched, 0), UP);
    assert_eq!(react(vec![], &m, RIGHT), RIGHT);
}

#[test]
fn hazard_cell_codes() {
    let ch = Character::Jake;
    let cell = |code: i32, mirrored: bool, x: i32, input: u32, s: State| {
        let mut f = Fixture::open();
        let ci = f.grid.get_mut((2, 0, 0)).unwrap();
        ci.code = code * 10;
        ci.flag40 = i32::from(mirrored);
        let m = me(view(ch, 256 + x - 10, s), 2);
        let grid = f.grid.clone();
        let l = look(&m, None, &grid);
        let mut i = input;
        seeing().hazards(&l, &mut f.senses(), &mut i);
        i
    };
    let idle = State::Idle;
    assert_eq!(cell(30, false, 64, RIGHT | SHOOT, idle), DOWN, "30: sempre");
    assert_eq!(cell(30, false, 64, LEFT, State::Crouch), LEFT | DOWN);
    for code in [29, 36, 37, 39] {
        // bloco 1: põe BAIXO
        assert_eq!(cell(code, true, 80, RIGHT, idle), DOWN, "{code}");
        assert_eq!(cell(code, false, 40, LEFT, idle), DOWN, "{code}");
        assert_eq!(cell(code, true, 80, RIGHT, State::Crouch), RIGHT | DOWN);
        // bloco 2: soma BAIXO
        assert_eq!(cell(code, false, 40, RIGHT, idle), RIGHT | DOWN, "{code}");
        assert_eq!(cell(code, true, 80, LEFT, idle), LEFT | DOWN, "{code}");
        // nada: outro lado da célula, ou entrada que não é só um lado
        assert_eq!(cell(code, false, 80, RIGHT, idle), RIGHT, "{code}");
        assert_eq!(cell(code, true, 40, LEFT, idle), LEFT, "{code}");
        assert_eq!(cell(code, true, 80, RIGHT | STEALTH, idle), RIGHT | STEALTH);
    }
    assert_eq!(cell(31, true, 80, RIGHT, idle), RIGHT);
    // célula vazia: nem os objetos
    let mut f = Fixture::open();
    f.grid.get_mut((2, 0, 0)).unwrap().present = false;
    let m = me(view(ch, 300, State::Idle), 2);
    let c = m.view.center();
    f.threats = vec![threat(11, c.0 + 30, c.1, (0, 0))];
    let grid = f.grid.clone();
    let l = look(&m, None, &grid);
    let mut i = RIGHT;
    assert!(!seeing().hazards(&l, &mut f.senses(), &mut i));
    assert_eq!(i, RIGHT);
}

#[test]
fn attack_and_air_bar_climb_skip_hazards_but_approach_and_retreat_do_not() {
    let ch = Character::Jake;
    let mut f = Fixture::open();
    let m = me(view(ch, 300, State::Idle), FISTS);
    let c = m.view.center();
    // a mina a 30 px faria ATIRAR+lado; o soco não passa pelos perigos
    f.threats = vec![threat(11, c.0 - 30, c.1, (0, 0))];
    let t = at(ch, 325, State::Idle);
    let mut b = seeing();
    assert_eq!(
        run(&mut f, &m, Some(&t), |b, l, s, i| b.hunt(l, s, i), &mut b),
        (true, PUNCH | RIGHT)
    );
    // aproximação: os perigos agem
    let far = at(ch, 450, State::Idle);
    let mut b = seeing();
    assert_eq!(
        run(&mut f, &m, Some(&far), |b, l, s, i| b.hunt(l, s, i), &mut b),
        (true, LEFT | SHOOT)
    );
    // no ar
    let air = me(view(ch, 300, State::Fall), FISTS);
    let mut above = at(ch, 300, State::Idle);
    above.bounds.y -= 100;
    let mut b = seeing();
    assert_eq!(
        run(
            &mut f,
            &air,
            Some(&above),
            |b, l, s, i| b.hunt(l, s, i),
            &mut b
        ),
        (true, UP)
    );
    // escalando: alvo à direita e à altura → CIMA+DIREITA, sem atirar na mina
    let cl = me(
        view(ch, 300, State::Climb(crate::jake::Climb::Vertical)),
        FISTS,
    );
    let mut b = seeing();
    assert_eq!(
        run(
            &mut f,
            &cl,
            Some(&far),
            |b, l, s, i| b.hunt(l, s, i),
            &mut b
        ),
        (true, UP | RIGHT)
    );
    // recuo bem-sucedido passa pelos perigos
    let mut hurt = m;
    hurt.pending = 15;
    let mut b = seeing();
    assert_eq!(
        run(
            &mut f,
            &hurt,
            Some(&far),
            |b, l, s, i| b.hunt(l, s, i),
            &mut b
        ),
        (true, LEFT | SHOOT)
    );
    assert_eq!(b.mode, Mode::Retreat);
    // recuo pousando (classe 6): 0
    let mut landing = me(view(ch, 300, State::Land(Landing::Medium)), FISTS);
    landing.pending = 15;
    let mut b = seeing();
    assert_eq!(
        run(
            &mut f,
            &landing,
            Some(&far),
            |b, l, s, i| b.hunt(l, s, i),
            &mut b
        ),
        (false, 0)
    );
    // recuo contra a parede: modo 1, e a entrada de lado fica
    let mut fw = Fixture::new(World::new(vec![
        Rect {
            x: -1000,
            y: FLOOR_Y,
            w: 6000,
            h: 20,
        },
        Rect {
            x: 290,
            y: 0,
            w: 5,
            h: FLOOR_Y,
        },
    ]));
    let mut b = seeing();
    assert_eq!(
        run(
            &mut fw,
            &hurt,
            Some(&far),
            |b, l, s, i| b.hunt(l, s, i),
            &mut b
        ),
        (false, LEFT)
    );
    assert_eq!(b.mode, Mode::Hunt);
}

#[test]
fn bar_and_climb_handlers() {
    let ch = Character::Jake;
    let mut f = Fixture::open();
    let bar = |f: &mut Fixture, m: &Me, t: &View| {
        run(f, m, Some(t), |b, l, s, i| b.bar(l, s, i), &mut seeing())
    };
    let hang = me(view(ch, 300, State::BarHang), 2);
    let mut below = at(ch, 300, State::Idle);
    below.bounds.y += 100;
    assert_eq!(
        bar(&mut f, &hang, &below),
        (true, JUMP | DOWN),
        "logo abaixo"
    );
    let mut low_right = at(ch, 500, State::Idle);
    low_right.bounds.y += 50;
    assert_eq!(
        bar(&mut f, &hang, &low_right),
        (true, RIGHT),
        "aberta embaixo: lado"
    );
    f.grid.get_mut((2, 0, 0)).unwrap().bits = path::bits::CLOSED_BELOW;
    assert_eq!(
        bar(&mut f, &hang, &low_right),
        (true, DOWN),
        "fechada: BAIXO"
    );
    let level = at(ch, 100, State::Idle);
    assert_eq!(bar(&mut f, &hang, &level), (true, LEFT), "à altura: lado");
    let pulling = me(view(ch, 300, State::BarPull), 2);
    assert_eq!(bar(&mut f, &pulling, &level), (true, DOWN), "105: BAIXO");
    let climb = |m: &Me, t: &View| {
        run(
            &mut Fixture::open(),
            m,
            Some(t),
            |b, l, _, i| b.climb(l, i),
            &mut seeing(),
        )
        .1
    };
    let cl = me(view(ch, 300, State::Climb(crate::jake::Climb::Vertical)), 2);
    let mut down = at(ch, 300, State::Idle);
    down.bounds.y += 100;
    assert_eq!(climb(&cl, &down), SHOOT, "logo abaixo: solta");
    let mut down_right = at(ch, 400, State::Idle);
    down_right.bounds.y += 100;
    assert_eq!(climb(&cl, &down_right), SHOOT | RIGHT);
    assert_eq!(climb(&cl, &at(ch, 400, State::Idle)), UP | RIGHT);
    let mut mid = at(ch, 305, State::Idle);
    mid.bounds.y += 10;
    assert_eq!(climb(&cl, &mid), SHOOT, "nada: ATIRAR");
}

fn grid(size: (i32, i32, i32)) -> CellGrid {
    CellGrid::new(size)
}

#[test]
fn path_search_obeys_each_bit() {
    use path::bits::*;
    let mut g = grid((5, 3, 2));
    assert_eq!(g.search((0, 1, 0), (3, 1, 0), 64), Some(4));
    assert_eq!(g.search((3, 1, 0), (0, 1, 0), 64), Some(3));
    assert_eq!(g.search((1, 0, 0), (1, 2, 0), 64), Some(2));
    assert_eq!(g.search((1, 1, 0), (1, 1, 0), 64), None, "já está");
    // subir exige corda e não 0x40
    assert_eq!(g.search((1, 1, 0), (1, 0, 0), 64), None, "sem corda");
    g.get_mut((1, 1, 0)).unwrap().zones = zone::ROPE;
    assert_eq!(g.search((1, 1, 0), (1, 0, 0), 64), Some(1));
    g.get_mut((1, 1, 0)).unwrap().bits = CLOSED_ABOVE;
    assert_eq!(g.search((1, 1, 0), (1, 0, 0), 64), None);
    // parede à direita: contorna por baixo e sobe pela corda da outra coluna
    let mut g = grid((5, 3, 1));
    g.get_mut((0, 1, 0)).unwrap().bits = WALL_RIGHT;
    assert_eq!(
        g.search((0, 1, 0), (2, 1, 0), 64),
        None,
        "sem corda não volta a subir"
    );
    g.get_mut((2, 2, 0)).unwrap().zones = zone::ROPE;
    assert_eq!(g.search((0, 1, 0), (2, 1, 0), 64), Some(2));
    g.get_mut((0, 1, 0)).unwrap().bits = WALL_RIGHT | CLOSED_BELOW;
    assert_eq!(g.search((0, 1, 0), (2, 1, 0), 64), None);
    // parede à esquerda
    let mut g = grid((5, 1, 1));
    g.get_mut((2, 0, 0)).unwrap().bits = WALL_LEFT;
    assert_eq!(g.search((2, 0, 0), (0, 0, 0), 64), None);
    // porta fechada no destino
    let mut g = grid((5, 1, 1));
    g.get_mut((1, 0, 0)).unwrap().door_closed = true;
    assert_eq!(g.search((0, 0, 0), (3, 0, 0), 64), None);
    g.get_mut((1, 0, 0)).unwrap().door_closed = false;
    assert_eq!(g.search((0, 0, 0), (3, 0, 0), 64), Some(4));
    // camadas pelas zonas 1 e 2
    let mut g = grid((2, 1, 2));
    assert_eq!(g.search((0, 0, 0), (0, 0, 1), 64), None);
    g.get_mut((0, 0, 0)).unwrap().zones = zone::LAYER_DOWN;
    assert_eq!(g.search((0, 0, 0), (0, 0, 1), 64), Some(6));
    assert_eq!(g.search((1, 0, 0), (0, 0, 1), 192), Some(3));
    g.get_mut((0, 0, 1)).unwrap().zones = zone::LAYER_UP;
    assert_eq!(g.search((0, 0, 1), (1, 0, 0), 64), Some(5));
    // Manhattan > 10: nem tenta
    let g = grid((30, 1, 1));
    assert_eq!(g.search((0, 0, 0), (10, 0, 0), 64), Some(4));
    assert_eq!(g.search((0, 0, 0), (11, 0, 0), 64), None);
    // empate na ordem das direções; fora da grade
    let g = grid((3, 3, 1));
    assert_eq!(
        g.search((0, 0, 0), (1, 1, 0), 64),
        Some(2),
        "descer vem antes de direita"
    );
    assert_eq!(g.allowed((0, 0, 0), 3), None);
}

#[test]
fn empty_cells_are_closed_and_own_door_blocks_from_behind() {
    // célula vazia no meio: entra (é o destino), mas não sai dela
    let mut g = grid((5, 1, 1));
    g.get_mut((2, 0, 0)).unwrap().present = false;
    assert_eq!(
        g.search((0, 0, 0), (2, 0, 0), 64),
        Some(4),
        "vazia como destino"
    );
    assert_eq!(
        g.search((0, 0, 0), (3, 0, 0), 64),
        None,
        "não passa por ela"
    );
    assert_eq!(g.allowed((2, 0, 0), 4), None);
    assert_eq!(g.cell((2, 0, 0)), None);
    // porta fechada na própria célula: barra o primeiro passo só do lado de trás
    let mut g = grid((3, 1, 1));
    g.get_mut((1, 0, 0)).unwrap().door_closed = true;
    let own = 128;
    assert_eq!(
        g.search((1, 0, 0), (0, 0, 0), own + 80),
        None,
        "à direita indo à esq."
    );
    assert_eq!(g.search((1, 0, 0), (0, 0, 0), own + 40), Some(3));
    assert_eq!(
        g.search((1, 0, 0), (2, 0, 0), own + 40),
        None,
        "à esq. indo à dir."
    );
    assert_eq!(g.search((1, 0, 0), (2, 0, 0), own + 80), Some(4));
    assert_eq!(
        g.search((1, 0, 0), (2, 0, 0), own + 64),
        Some(4),
        "64 passa"
    );
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

/// Parede entre a IA (célula 0) e o alvo (célula 3): não o vê, segue o caminho.
fn walled() -> Fixture {
    Fixture::new(World::new(vec![
        Rect {
            x: -1000,
            y: FLOOR_Y,
            w: 6000,
            h: 20,
        },
        Rect {
            x: 300,
            y: 0,
            w: 40,
            h: FLOOR_Y,
        },
    ]))
}

#[test]
fn unseen_target_is_followed_by_path_steps_then_waits_20_ticks() {
    let mut f = walled();
    let m = me(view(Character::Jake, 10, State::Idle), 2);
    let t = view(Character::Jake, 500, State::Idle);
    let mut b = Brain::new();
    assert_eq!(b.think(&m, Some(&t), &mut f.senses()), RIGHT, "passo 4");
    assert!(!b.sees);
    let st = b.step.expect("passo");
    assert_eq!((st.cell, st.dir, b.wait), ((1, 0, 0), 4, WAIT));
    // sem caminho (parede na célula): espera 10 pensamentos = 20 tiques
    f.grid.get_mut((0, 0, 0)).unwrap().bits = path::bits::WALL_RIGHT;
    let mut b = Brain {
        target: true,
        ..Brain::new()
    };
    assert_eq!(b.think(&m, Some(&t), &mut f.senses()), 0);
    assert_eq!(b.wait, WAIT);
    let mut ticks = 0;
    while b.wait > 0 {
        b.think(&m, Some(&t), &mut f.senses());
        ticks += 1;
        assert!(ticks < 100);
    }
    assert_eq!(ticks, 20, "uma unidade por pensamento");
}

#[test]
fn no_path_taunts_only_when_idle() {
    let mut f = walled();
    f.grid.get_mut((0, 0, 0)).unwrap().bits = path::bits::WALL_RIGHT;
    let t = view(Character::Jake, 500, State::Idle);
    let count = |f: &mut Fixture, s: State| {
        let m = me(view(Character::Jake, 10, s), 2);
        let mut b = Brain {
            target: true,
            ..Brain::new()
        };
        (0..4000)
            .filter(|_| b.think(&m, Some(&t), &mut f.senses()) == TAUNT_INPUT)
            .count()
    };
    assert!(count(&mut f, State::Idle) > 0);
    assert_eq!(
        count(&mut f, State::Stealth),
        0,
        "andando: só com a classe 1"
    );
}

#[test]
fn step_arrival_needs_centring_and_leaving_resets_with_no_input() {
    let mut f = walled();
    let t = view(Character::Jake, 500, State::Idle);
    // o passo é para a célula 1; na célula 1, fora do centro: repete a direção
    let m = me(view(Character::Jake, 140, State::Run), 2);
    let mut b = Brain {
        target: true,
        step: Some(Step {
            cell: (1, 0, 0),
            dir: 4,
        }),
        wait: 5,
        ..Brain::new()
    };
    assert_eq!(b.think(&m, Some(&t), &mut f.senses()), RIGHT);
    assert_eq!(b.step.unwrap().cell, (1, 0, 0), "o passo fica");
    assert_eq!(b.wait, 4);
    // centrado: novo passo para a célula 2
    let c = me(view(Character::Jake, 182, State::Run), 2);
    b.think(&c, Some(&t), &mut f.senses());
    b.think(&c, Some(&t), &mut f.senses());
    assert_eq!(b.step.unwrap().cell, (2, 0, 0));
    assert_eq!(b.wait, WAIT);
    // a mais de uma célula do passo: apaga e entrada 0 neste pensamento
    let away = me(view(Character::Jake, 400, State::Run), 2);
    let mut b = Brain {
        target: true,
        step: Some(Step {
            cell: (1, 0, 0),
            dir: 4,
        }),
        wait: 5,
        ..Brain::new()
    };
    let mut fw = walled();
    fw.world = floor(&[(-1000, 5000)]);
    let hidden = view(Character::Jake, 1200, State::Idle);
    let mut far = hidden;
    far.layer = 1;
    assert_eq!(b.think(&away, Some(&far), &mut fw.senses()), 0);
    assert_eq!((b.step, b.wait), (None, 0));
}

#[test]
fn path_steps_centre_and_use_layers() {
    let mut f = Fixture::open();
    let mut b = Brain::new();
    let out = |b: &mut Brain, f: &mut Fixture, m: &Me, dir: u8| {
        let grid = f.grid.clone();
        let l = look(m, None, &grid);
        b.step_out(&l, &mut f.senses(), dir, None)
    };
    // centro x = 10 + 10 = 20 → ox = −44: vai para a direita até centrar
    let m = me(view(Character::Jake, 10, State::Idle), 2);
    assert_eq!(out(&mut b, &mut f, &m, 1), RIGHT);
    assert_eq!(out(&mut b, &mut f, &m, 5), RIGHT, "camada também centra");
    let c = me(view(Character::Jake, 54, State::Idle), 2);
    assert_eq!(out(&mut b, &mut f, &c, 1), UP);
    assert_eq!(out(&mut b, &mut f, &c, 2), JUMP | DOWN);
    assert_eq!(out(&mut b, &mut f, &c, 5), UP);
    assert_eq!(out(&mut b, &mut f, &c, 6), DOWN);
    let climbing = me(
        view(
            Character::Jake,
            54,
            State::Climb(crate::jake::Climb::Vertical),
        ),
        2,
    );
    assert_eq!(out(&mut b, &mut f, &climbing, 6), JUMP);
    // escalando de lado: CIMA só com oy > 0 (centro y = 78 → oy = 14)
    assert_eq!(out(&mut b, &mut f, &climbing, 4), RIGHT | UP);
    let mut high = climbing;
    high.view.bounds.y -= 30;
    high.view.on_ground = false;
    assert_eq!(out(&mut b, &mut f, &high, 4), RIGHT);
    // alto na célula (oy < −50): de lado leva BAIXO, camada nada
    let mut top = c;
    top.view.bounds.y -= 120;
    assert_eq!(out(&mut b, &mut f, &top, 3), LEFT | DOWN);
    assert_eq!(out(&mut b, &mut f, &top, 5), 0);
    // sem caminho: apaga e espera
    let grid = f.grid.clone();
    let l = look(&c, None, &grid);
    assert_eq!(b.step_out(&l, &mut f.senses(), 0, Some((0, 0, 0))), 0);
    assert_eq!((b.step, b.wait), (None, WAIT));
}

#[test]
fn wander_walks_sideways_stealthily() {
    let mut f = Fixture::open();
    let m = me(view(Character::Jake, 300, State::Idle), 2);
    let hidden = view(Character::Jake, 600, State::Hidden);
    let mut b = Brain {
        mode: Mode::Wander,
        target: true,
        ..Brain::new()
    };
    let i = b.think(&m, Some(&hidden), &mut f.senses());
    assert!(i == STEALTH | LEFT || i == STEALTH | RIGHT, "{i:#x}");
    let st = b.step.unwrap();
    assert!(matches!(st.dir, 3 | 4));
    assert_eq!(b.wait, WAIT);
    // na barra/escada: solta
    let bar = me(view(Character::Jake, 300, State::BarHang), 2);
    let mut b = Brain {
        mode: Mode::Wander,
        target: true,
        ..Brain::new()
    };
    assert_eq!(b.think(&bar, Some(&hidden), &mut f.senses()), JUMP);
    // agachado: volta ao modo 1, entrada 0
    let cr = me(view(Character::Jake, 300, State::Crouch), 2);
    let mut b = Brain {
        mode: Mode::Wander,
        target: true,
        ..Brain::new()
    };
    assert_eq!(b.think(&cr, Some(&hidden), &mut f.senses()), 0);
    assert_eq!(b.mode, Mode::Hunt);
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
    assert!(attackable(State::Land(Landing::Long), j));
    assert!(!attackable(State::Land(Landing::Long), g));
    assert!(attackable(State::Land(Landing::Medium), g));
    assert!(!attackable(State::Shocked(1), j));
    assert!(attackable(State::Shocked(1), g));
    for s in [
        State::Idle,
        State::Run,
        State::Crouch,
        State::Crawl,
        State::Hidden,
    ] {
        assert!(attackable(s, j) && attackable(s, g), "{s:?}");
    }
}
