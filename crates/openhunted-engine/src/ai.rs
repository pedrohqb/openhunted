// SPDX-License-Identifier: GPL-3.0-or-later
//! IA dos personagens sem jogador (`0x42ec1c`; `docs/re/minotauro-ia.md`): o cérebro que
//! devolve os bits de entrada (`crate::input`) de um Jake ou Garathe da IA. Puro: lê o
//! próprio corpo, o alvo e os sentidos (`Senses`); a cena aplica a entrada ao corpo.

pub mod path;

use crate::behaviour::{angle_diff, angle16};
use crate::character::Character;
use crate::input::{DOWN, JUMP, LEFT, PUNCH, RIGHT, SHOOT, STEALTH, UP};
use crate::jake::{FISTS, Jake, Landing, State, TAUNT_INPUT};
use crate::physics::{Rect, World};
use crate::rng::Rng;

pub use path::{CELL, Cell, CellGrid, CellInfo};

/// Período de pensamento (`+0xdc` = 2): nos tiques ímpares repete a última entrada.
pub const THINK_PERIOD: u32 = 2;
/// Alcance da visão, centro a centro (`0x42d520`).
pub const SIGHT: i32 = 400;
/// Pensamentos depois de perder de vista com 50 % de pose.
pub const LOST_THINKS: u32 = 2;
/// Espera sem caminho: `5 · +0xdc` tiques.
pub const NO_PATH_WAIT: u32 = 5 * THINK_PERIOD;
/// Espera de um passo de caminho (`+0x94 = 10`, `0x42d721`).
pub const STEP_WAIT: u32 = 10;
/// Alcance do ataque à distância (`0x42e659`).
pub const LONG_RANGE: i32 = 200;
/// Distância mínima do pulo longo de ataque do Garathe.
pub const LEAP_MIN: i32 = 90;
/// Raio da lista de perigos (`0x42cad4`).
pub const HAZARD_RANGE: i32 = 100;
/// Parede à frente: sonda de 20 px (`0x42df9f`, `0x42dd8f`).
pub const WALL_PROBE: i32 = 20;

/// Limiares por personagem (`0x4888b0` = tipo do pensante).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tuning {
    /// `m`: abaixo disso não ataca.
    pub melee_min: i32,
    /// `p`: alcance do golpe.
    pub melee: i32,
    /// Perto do alvo na aproximação (recua ou segue).
    pub near: i32,
    /// Folga para centrar na célula antes de subir/descer.
    pub center: i32,
    /// `oy` abaixo do qual o passo de lado leva BAIXO.
    pub high: i32,
    /// Pousos da sonda de buraco (`0x478910…` Jake, `0x47d128…` Garathe).
    pub landings: [i32; 3],
}

pub const JAKE_TUNING: Tuning = Tuning {
    melee_min: 7,
    melee: 30,
    near: 30,
    center: 15,
    high: -50,
    landings: [110, 150, 255],
};
pub const GARATHE_TUNING: Tuning = Tuning {
    melee_min: 12,
    melee: 50,
    near: 50,
    center: 25,
    high: -40,
    landings: [156, 350, 225],
};

pub fn tuning(ch: Character) -> &'static Tuning {
    match ch {
        Character::Jake => &JAKE_TUNING,
        Character::Garathe => &GARATHE_TUNING,
    }
}

/// Classes de estado (`0x42c7b6`).
pub mod class {
    pub const NONE: u8 = 0;
    pub const IDLE: u8 = 1;
    pub const POSE: u8 = 2;
    pub const WALK: u8 = 3;
    pub const LAYER: u8 = 4;
    pub const RISE: u8 = 5;
    pub const LAND: u8 = 6;
    pub const FALL: u8 = 7;
    pub const CROUCH: u8 = 8;
    pub const PUSH: u8 = 9;
    pub const HIDDEN: u8 = 10;
    pub const BAR: u8 = 11;
    pub const CLIMB: u8 = 12;
    pub const HURT: u8 = 13;
    pub const DYING: u8 = 14;
    pub const SPECIAL: u8 = 0xf;
    pub const ATTACK: u8 = 0x10;
}

/// Classe do estado semântico (as listas da RE por estado do Jake e do Garathe).
pub fn class_of(s: State, ch: Character) -> u8 {
    use class::*;
    match s {
        State::Idle | State::Grab | State::LetGo | State::Shake | State::Shocked(_) => IDLE,
        State::Taunt => POSE,
        State::Special(_) => SPECIAL,
        State::Stealth | State::Run => WALK,
        // o Garathe põe 11/12 na classe 3
        State::LayerUp(_) | State::LayerDown(_) if ch == Character::Garathe => WALK,
        State::LayerUp(_) | State::LayerDown(_) => LAYER,
        State::StandJump | State::RunUp | State::JumpUp(_) | State::Jump(_) => RISE,
        State::Land(_) | State::WildFall | State::GetUp | State::KnockedDown(_) | State::Rise => {
            LAND
        }
        State::Fall => FALL,
        State::Crouch | State::Crawl | State::StandUp => CROUCH,
        State::Push => PUSH,
        State::Hide | State::Hidden | State::Unhide => HIDDEN,
        State::BarHang | State::BarMove | State::BarPull | State::BarUp | State::BarDown => BAR,
        State::Climb(_) => CLIMB,
        State::Pushed | State::Jolted => HURT,
        State::Dying(_) | State::Dead => DYING,
        State::Fire { .. } | State::Recover(_) | State::Ready(..) | State::Holster(_) => ATTACK,
        State::EnteringExit(_) | State::Teleporting(_) => NONE,
    }
}

/// A IA não ataca nestes estados (listas de `0x42e659`): especial, camadas, choque, quedas,
/// pousos duros, derrubada, levantar, dano e morte.
pub fn can_attack(s: State, ch: Character) -> bool {
    !matches!(
        s,
        State::Special(_)
            | State::LayerUp(_)
            | State::LayerDown(_)
            | State::Shocked(_)
            | State::Fall
            | State::WildFall
            | State::Land(Landing::Fall | Landing::Hard)
            | State::GetUp
            | State::KnockedDown(_)
            | State::StandUp
            | State::Unhide
            | State::Pushed
            | State::Jolted
            | State::Rise
            | State::Dying(_)
            | State::Dead
            | State::EnteringExit(_)
            | State::Teleporting(_)
    ) && !(ch == Character::Garathe && s == State::Land(Landing::Long))
}

/// Um personagem da IA (`Behaviour::AiPlayer`): o corpo do jogador e o cérebro.
#[derive(Debug, Clone)]
pub struct AiPlayer {
    pub body: Jake,
    pub brain: Brain,
    /// Já investiu contra o jogador neste pulo longo.
    pub body_checked: bool,
    /// Sorteios do cérebro (próprios: não mexem na sequência dos demais objetos).
    pub rng: Rng,
}

/// O que o cérebro vê de um personagem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct View {
    pub bounds: Rect,
    pub layer: i32,
    pub state: State,
    pub character: Character,
    /// Velocidade em 1/8 px por tique.
    pub vx: i32,
    pub vy: i32,
    pub facing_right: bool,
    pub on_ground: bool,
}

impl View {
    pub fn of(j: &Jake) -> Self {
        Self {
            bounds: j.body.bounds(),
            layer: j.layer(),
            state: j.state(),
            character: j.character(),
            vx: j.body.vx,
            vy: j.body.vy,
            facing_right: j.facing_right(),
            on_ground: j.body.on_ground,
        }
    }

    pub fn center(&self) -> (i32, i32) {
        (
            self.bounds.x + self.bounds.w / 2,
            self.bounds.y + self.bounds.h / 2,
        )
    }

    pub fn class(&self) -> u8 {
        class_of(self.state, self.character)
    }
}

/// O próprio corpo: o que se vê dele e o que só ele sabe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Me {
    pub view: View,
    pub hp: i32,
    /// Dano pendente (`+0x24c`) ainda não aplicado.
    pub pending: i32,
    /// Arma atual (a melhor, `0x43a8cc(obj, 1)`).
    pub weapon: usize,
}

impl Me {
    pub fn of(j: &Jake) -> Self {
        Self {
            view: View::of(j),
            hp: j.health.hp,
            pending: j.pending_damage(),
            weapon: j.inventory.weapon,
        }
    }
}

/// Um objeto da lista de perigos (`0x47d14c`): tipo, centro e velocidade em px/tique.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Threat {
    pub kind: u32,
    pub center: (i32, i32),
    pub vel: (i32, i32),
}

/// Tipos da lista de perigos que têm reação (`0x42e1a9`).
pub fn is_threat(kind: u32) -> bool {
    reaction(kind).is_some()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Reaction {
    Duck,
    JumpOver,
    Piston,
    Fence,
    Shoot,
}

fn reaction(kind: u32) -> Option<Reaction> {
    Some(match kind {
        19..=21 | 25 | 27 | 29 | 99 | 100 | 106 | 108 => Reaction::Duck,
        26 | 42 | 68..=72 => Reaction::JumpOver,
        39 => Reaction::Piston,
        40 | 41 => Reaction::Fence,
        11 | 89 | 109 => Reaction::Shoot,
        _ => return None,
    })
}

/// O que o cérebro consulta do mundo num pensamento.
pub struct Senses<'a> {
    /// Sólidos da camada do pensante (células, portas e objetos sólidos).
    pub world: &'a World,
    pub grid: &'a CellGrid,
    /// Perigos da camada dele (a cena já filtra a < `HAZARD_RANGE`).
    pub threats: &'a [Threat],
    pub rng: &'a mut Rng,
}

/// Modo (`+0x98`): 1 caçar (0 vale 1), 2 recuar, 3 vagar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Hunt,
    Retreat,
    Wander,
}

/// Passo de caminho em curso (`+0x80…+0x94`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Step {
    pub cell: Cell,
    pub dir: u8,
    pub wait: u32,
}

/// A estrutura da IA (`+0x2a0`).
#[derive(Debug, Clone, Default)]
pub struct Brain {
    pub mode: Mode,
    /// Vê o alvo (`+0x54`).
    pub sees: bool,
    /// Última entrada (`+0x78`).
    pub last: u32,
    /// Pensamentos feitos e o pensamento da última visão (`+0x7c`).
    thinks: u32,
    last_seen: Option<u32>,
    calls: u32,
    pub step: Option<Step>,
    /// Espera sem caminho, em tiques.
    pub wait: u32,
}

fn side(right: bool) -> u32 {
    if right { RIGHT } else { LEFT }
}

fn dist(a: (i32, i32), b: (i32, i32)) -> i32 {
    let (dx, dy) = (i64::from(b.0 - a.0), i64::from(b.1 - a.1));
    ((dx * dx + dy * dy) as f64).sqrt() as i32
}

/// Linha livre entre dois centros: 16 amostras fora dos sólidos, ignorando as caixas `skip`
/// (`0x42d475` → `0x433aa3`/`0x43735d`).
pub fn clear_line(world: &World, a: (i32, i32), b: (i32, i32), skip: &[Rect]) -> bool {
    (1..16).all(|i| {
        let p = (a.0 + (b.0 - a.0) * i / 16, a.1 + (b.1 - a.1) * i / 16);
        let r = Rect {
            x: p.0,
            y: p.1,
            w: 1,
            h: 1,
        };
        skip.iter().any(|s| s.overlaps(&r)) || !world.blocked(&r)
    })
}

/// Vê o alvo: tocando, ou na mesma camada a < 400 px com linha livre.
pub fn sees(me: &View, t: &View, world: &World) -> bool {
    if me.bounds.overlaps(&crate::behaviour::grow(t.bounds, 1)) {
        return true;
    }
    me.layer == t.layer
        && dist(me.center(), t.center()) < SIGHT
        && clear_line(world, me.center(), t.center(), &[me.bounds, t.bounds])
}

/// Parede a `d` px para o lado `right`.
fn wall(me: &View, world: &World, right: bool, d: i32) -> bool {
    let dir = if right { 1 } else { -1 };
    world.blocked(&me.bounds.moved(dir * d, 0))
}

/// Resultado da sonda de buraco (`0x42cfc8`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gap {
    /// Chão à frente (ou no ar): segue.
    Ground,
    /// Buraco com pouso a uma das distâncias: pula.
    Landing,
    /// Buraco sem pouso.
    NoLanding,
}

/// Sonda de buraco: faixa de 3 px sob os pés a `2·largura` à frente; senão faixas de meia
/// largura a cada distância de pouso.
pub fn probe_gap(me: &View, world: &World, right: bool, tu: &Tuning) -> Gap {
    let b = me.bounds;
    let below = |r: Rect| world.blocked(&r);
    if !me.on_ground {
        return Gap::Ground;
    }
    let dir = if right { 1 } else { -1 };
    let feet = b.y + b.h + 1;
    if below(Rect {
        x: b.x + dir * 2 * b.w,
        y: feet,
        w: b.w,
        h: 3,
    }) {
        return Gap::Ground;
    }
    let land = tu.landings.iter().any(|&l| {
        below(Rect {
            x: b.x + b.w / 4 + dir * l,
            y: feet,
            w: (b.w / 2).max(1),
            h: 3,
        })
    });
    if land { Gap::Landing } else { Gap::NoLanding }
}

/// Mira perto do alvo (`0x42cca4`, < `p`): setores largos, sem "só BAIXO".
pub fn aim_near(a: i32) -> u32 {
    match a.rem_euclid(0x10000) {
        a if !(0x800..0xf800).contains(&a) => RIGHT,
        0x800..=0x4000 => RIGHT | DOWN,
        0x4001..0x7800 => LEFT | DOWN,
        0x7800..0x8800 => LEFT,
        0x8800..0xb800 => LEFT | UP,
        0xb800..0xc800 => UP,
        _ => RIGHT | UP,
    }
}

/// Mira longe: só alinhado a ±0x200 de uma das sete direções de tiro.
pub fn aim_far(a: i32) -> Option<u32> {
    [
        (0x0000, RIGHT),
        (0x2000, RIGHT | DOWN),
        (0x6000, LEFT | DOWN),
        (0x8000, LEFT),
        (0xa000, LEFT | UP),
        (0xc000, UP),
        (0xe000, RIGHT | UP),
    ]
    .into_iter()
    .find(|(c, _)| angle_diff(a, *c).abs() <= 0x200)
    .map(|(_, bits)| bits)
}

/// Contexto de um pensamento: corpo, alvo e medidas.
struct Look<'b> {
    me: &'b Me,
    t: Option<&'b View>,
    dx: i32,
    dy: i32,
    d: i32,
    angle: i32,
    tu: &'static Tuning,
}

impl Brain {
    pub fn new() -> Self {
        Self::default()
    }

    /// Um tique: pensa nos tiques pares e repete a última entrada nos ímpares.
    pub fn think(&mut self, me: &Me, target: Option<&View>, s: &mut Senses) -> u32 {
        let call = self.calls;
        self.calls = self.calls.wrapping_add(1);
        if call % THINK_PERIOD != 0 {
            return self.last;
        }
        self.thinks = self.thinks.wrapping_add(1);
        let (dx, dy, d, angle) = match target {
            Some(t) => {
                let (a, b) = (me.view.center(), t.center());
                let (dx, dy) = (b.0 - a.0, b.1 - a.1);
                (dx, dy, dist(a, b), angle16(dx, dy))
            }
            None => (0, 0, i32::MAX, 0),
        };
        let l = Look {
            me,
            t: target,
            dx,
            dy,
            d,
            angle,
            tu: tuning(me.view.character),
        };
        self.sees = target.is_some_and(|t| {
            let c = t.class();
            sees(&me.view, t, s.world)
                && !(self.mode == Mode::Wander && (c == class::HIDDEN || c == class::DYING))
        });
        if self.sees {
            self.last_seen = Some(self.thinks);
            self.step = None;
        }
        let input = match self.mode {
            _ if target.is_none() => {
                self.mode = Mode::Wander;
                self.wander(&l, s)
            }
            Mode::Hunt => self.hunt(&l, s),
            Mode::Retreat => self.retreat(&l, s),
            Mode::Wander => self.wander(&l, s),
        };
        self.last = input;
        input
    }

    /// Modo 1 (`0x42ed64`).
    fn hunt(&mut self, l: &Look, s: &mut Senses) -> u32 {
        let me = l.me;
        let Some(t) = l.t else {
            self.mode = Mode::Wander;
            return self.wander(l, s);
        };
        if me.pending > 10 || (me.pending > 0 && me.hp < 30) {
            self.mode = Mode::Retreat;
            return self.retreat(l, s);
        }
        let mc = me.view.class();
        if matches!(mc, class::POSE | class::LAND | class::SPECIAL) {
            return 0;
        }
        let mut input = None;
        if self.sees {
            let tc = t.class();
            if tc == class::HIDDEN || tc == class::DYING {
                self.mode = Mode::Wander;
                return self.wander(l, s);
            }
            input = match mc {
                // no ar: CIMA (corda) se o alvo está acima e ainda não cai rápido
                class::RISE | class::FALL => {
                    Some(if l.dy < 0 && me.view.vy > -16 { UP } else { 0 })
                }
                class::BAR => Some(if l.dy > 0 && l.dx.abs() < 32 {
                    JUMP | DOWN
                } else if l.dy.abs() < 20 {
                    side(l.dx > 0)
                } else {
                    DOWN
                }),
                class::CLIMB => Some(side(l.dx > 0) | if l.dy < 0 { UP } else { DOWN }),
                class::ATTACK => Some(self.attack(l, s)),
                _ => match self.attack(l, s) {
                    0 => self.approach(l, s),
                    a => Some(a),
                },
            };
        }
        let input = match input {
            Some(i) => i,
            None => self.follow_path(l, s),
        };
        self.hazards(l, s, input)
    }

    /// Modo 2 (`0x42df9f`): corre para o lado oposto ao alvo.
    fn retreat(&mut self, l: &Look, s: &mut Senses) -> u32 {
        if !self.sees {
            self.mode = Mode::Hunt;
            return 0;
        }
        let away = l.dx < 0;
        if wall(&l.me.view, s.world, away, WALL_PROBE)
            || probe_gap(&l.me.view, s.world, away, l.tu) == Gap::NoLanding
        {
            self.mode = Mode::Hunt;
            return 0;
        }
        self.with_gap(l, s, side(away))
    }

    /// Modo 3 (`0x42f434`): vaga andando de célula em célula.
    fn wander(&mut self, l: &Look, s: &mut Senses) -> u32 {
        if let Some(t) = l.t {
            if l.me.pending > 0 {
                self.mode = Mode::Retreat;
                return self.retreat(l, s);
            }
            if t.class() != class::HIDDEN {
                // Em aberto na RE: o ramo põe o modo 0 (= caçar) e devolve a entrada antiga
                self.mode = Mode::Hunt;
                return self.last;
            }
        }
        let mc = l.me.view.class();
        if mc == class::BAR || mc == class::CLIMB {
            return JUMP;
        }
        if s.rng.below(20) == 0 {
            return TAUNT_INPUT;
        }
        let here = s.grid.cell_of(l.me.view.center(), l.me.view.layer);
        self.expire_step(here);
        if self.step.is_none() {
            let first = 3 + s.rng.below(2) as u8;
            let dir = [first, 7 - first]
                .into_iter()
                .find(|&d| s.grid.allowed(here, d).is_some());
            if let Some(dir) = dir {
                self.start_step(s.grid, here, dir);
            }
        }
        match self.step {
            Some(st) => self.step_input(l, s, st.dir) | STEALTH,
            None => 0,
        }
    }

    /// Ataque (`0x42e659`): 0 = nenhum.
    fn attack(&self, l: &Look, _s: &mut Senses) -> u32 {
        let (me, tu) = (l.me, l.tu);
        let Some(t) = l.t else { return 0 };
        if !self.sees || !can_attack(me.view.state, me.view.character) {
            return 0;
        }
        let d = l.d;
        if d < tu.melee_min {
            0
        } else if d < (tu.melee - tu.melee_min) / 2 + tu.melee_min {
            if self.last & (SHOOT | PUNCH) != 0 {
                self.last
            } else {
                0
            }
        } else if d < tu.melee {
            let fleeing = t.vx != 0 && l.dx.signum() == t.vx.signum();
            if fleeing && t.class() != class::PUSH {
                0
            } else {
                PUNCH | aim_near(l.angle)
            }
        } else if d < LONG_RANGE {
            let g = me.view.character == Character::Garathe;
            let horizontal = angle_diff(l.angle, 0).abs() <= 0x1000
                || angle_diff(l.angle, 0x8000).abs() <= 0x1000;
            let same_way = (t.vx > 0) == (l.dx > 0) && t.vx != 0;
            if g && me.view.state == State::Run
                && d > LEAP_MIN
                && me.view.facing_right == (l.dx > 0)
                && same_way
                && horizontal
                && t.class() == class::WALK
            {
                return side(l.dx > 0) | JUMP | UP;
            }
            let ranged = if g {
                me.weapon >= 3
            } else {
                me.weapon != FISTS
            };
            match aim_far(l.angle) {
                Some(a) if ranged => a | SHOOT,
                _ => 0,
            }
        } else {
            0
        }
    }

    /// Aproximação (`0x42dd8f`); `None` = falhou (segue o caminho).
    fn approach(&self, l: &Look, s: &mut Senses) -> Option<u32> {
        let t = l.t?;
        let tc = t.class();
        if !matches!(tc, 1 | 3 | 4 | 5 | 7 | 8 | 9 | 11 | 12 | 13 | 15 | 16) {
            return None;
        }
        let me = &l.me.view;
        let dy = if l.dy.abs() < 64 && tc == class::RISE {
            0
        } else {
            l.dy
        };
        if dy.abs() < 20 {
            let right = l.dx > 0;
            if l.dx.abs() < l.tu.near {
                if tc == class::WALK && t.vx != 0 {
                    return Some(side(t.vx > 0));
                }
                // alvo parado: recua andando para ganhar espaço
                let away = if wall(me, s.world, !right, WALL_PROBE) {
                    right
                } else {
                    !right
                };
                return Some(STEALTH | side(away));
            }
            if l.dx.abs() > 10 {
                return Some(self.with_gap(l, s, side(right)));
            }
            return None;
        }
        if (0xbc00..=0xc400).contains(&l.angle) {
            return Some(JUMP | UP);
        }
        if me.class() == class::CLIMB {
            return Some(side(l.dx > 0) | if dy < 0 { UP } else { DOWN });
        }
        None
    }

    /// A sonda de buraco sobre uma entrada de um lado só (`0x42cf70`).
    fn with_gap(&self, l: &Look, s: &mut Senses, input: u32) -> u32 {
        if input != LEFT && input != RIGHT {
            return input;
        }
        match probe_gap(&l.me.view, s.world, input == RIGHT, l.tu) {
            Gap::Ground => input,
            Gap::Landing if l.me.view.class() == class::CROUCH => input | UP,
            Gap::Landing => input | JUMP,
            Gap::NoLanding => JUMP | UP,
        }
    }

    /// Sem ver o alvo (ou sem aproximação): pose, caminho ou espera.
    fn follow_path(&mut self, l: &Look, s: &mut Senses) -> u32 {
        let Some(t) = l.t else { return 0 };
        let lost = self.last_seen.map(|n| self.thinks.wrapping_sub(n));
        if !self.sees && lost.is_some_and(|n| (1..=LOST_THINKS).contains(&n)) && s.rng.below(2) == 0
        {
            return TAUNT_INPUT;
        }
        if self.wait > 0 {
            self.wait = self.wait.saturating_sub(THINK_PERIOD);
            if s.rng.below(1000) == 0 {
                self.mode = Mode::Wander;
            }
            return if s.rng.below(50) == 0 { TAUNT_INPUT } else { 0 };
        }
        let here = s.grid.cell_of(l.me.view.center(), l.me.view.layer);
        self.expire_step(here);
        if self.step.is_none() {
            let goal = s.grid.cell_of(t.center(), t.layer);
            match s.grid.search(here, goal) {
                Some(dir) => self.start_step(s.grid, here, dir),
                None => {
                    self.wait = NO_PATH_WAIT;
                    return 0;
                }
            }
        }
        match self.step {
            Some(st) => self.step_input(l, s, st.dir),
            None => 0,
        }
    }

    fn start_step(&mut self, grid: &CellGrid, here: Cell, dir: u8) {
        self.step = grid.allowed(here, dir).map(|cell| Step {
            cell,
            dir,
            wait: STEP_WAIT,
        });
    }

    /// Fim do passo: chegou, passou da célula-alvo por mais de 1 ou esgotou a espera.
    fn expire_step(&mut self, here: Cell) {
        if let Some(st) = self.step.as_mut() {
            let off = (st.cell.0 - here.0).abs()
                + (st.cell.1 - here.1).abs()
                + (st.cell.2 - here.2).abs();
            st.wait = st.wait.saturating_sub(THINK_PERIOD);
            if off == 0 || off > 1 || st.wait == 0 {
                self.step = None;
            }
        }
    }

    /// Entrada de um passo de caminho (`0x42d721`).
    fn step_input(&self, l: &Look, s: &mut Senses, dir: u8) -> u32 {
        let me = &l.me.view;
        let c = me.center();
        let (ox, oy) = (
            c.0.rem_euclid(CELL) - CELL / 2,
            c.1.rem_euclid(CELL) - CELL / 2,
        );
        let mc = me.class();
        let centered = ox.abs() <= l.tu.center;
        match dir {
            1 if !centered => side(ox < 0),
            1 => UP,
            2 if !centered => side(ox < 0),
            2 => JUMP | DOWN,
            3 | 4 => {
                let mut i = side(dir == 4);
                if oy < l.tu.high && mc != class::BAR {
                    i |= DOWN;
                }
                if mc == class::CLIMB {
                    i |= UP;
                }
                self.with_gap(l, s, i)
            }
            5 => UP,
            _ if mc == class::CLIMB => JUMP,
            _ => DOWN,
        }
    }

    /// Perigos (`0x42e1a9`): a célula e o objeto mais próximo da lista.
    fn hazards(&self, l: &Look, s: &mut Senses, input: u32) -> u32 {
        let me = &l.me.view;
        let c = me.center();
        let here = s.grid.cell_of(c, me.layer);
        if let Some(ci) = s.grid.get(here) {
            let code = (ci.code / 10) % 100;
            let ox = c.0.rem_euclid(CELL) - CELL / 2;
            // [inferido] andando para a borda da célula de buraco/escada: BAIXO
            if [29, 30, 36, 37, 39].contains(&code)
                && ci.flag40 & 1 != 0
                && me.class() == class::WALK
                && me.vx != 0
                && ox.signum() == me.vx.signum()
                && ox.abs() > CELL / 4
            {
                return DOWN;
            }
        }
        let near = s
            .threats
            .iter()
            .filter_map(|t| reaction(t.kind).map(|r| (r, t, dist(c, t.center))))
            .filter(|(_, _, d)| *d < HAZARD_RANGE)
            .min_by_key(|(_, _, d)| *d);
        let Some((r, t, d)) = near else {
            return input;
        };
        let toward = t.center.0 > c.0;
        let coming = {
            let (rx, ry) = (c.0 - t.center.0, c.1 - t.center.1);
            i64::from(t.vel.0) * i64::from(rx) + i64::from(t.vel.1) * i64::from(ry) > 0
        };
        match r {
            Reaction::Duck if coming && me.state != State::Crawl => DOWN,
            Reaction::JumpOver if coming => side(toward) | JUMP,
            Reaction::JumpOver => side(s.rng.below(2) == 0) | JUMP,
            Reaction::Piston if d < 95 => {
                if d < 75 && c.1 > t.center.1 {
                    DOWN
                } else {
                    input & (JUMP | SHOOT | PUNCH)
                }
            }
            Reaction::Fence if d < 50 && me.vx != 0 && (me.vx > 0) == toward => input | JUMP | UP,
            Reaction::Shoot if d < 50 => {
                side(toward) | SHOOT | if c.1 < t.center.1 { DOWN } else { 0 }
            }
            _ => input,
        }
    }
}

#[cfg(test)]
mod tests;
