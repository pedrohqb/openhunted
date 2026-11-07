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
/// Espera `+0x94 = 5·+0xdc` (`0x42d721`), em pensamentos: decrementada uma vez por
/// pensamento (`0x42f035`, `0x42f1fa`, `0x42f326`), dura 10 pensamentos = 20 tiques.
pub const WAIT: u32 = 5 * THINK_PERIOD;
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
    /// Folga horizontal para centrar na célula (`0x42d721`, `0x42db71`).
    pub center: i32,
    /// Folga vertical: `oy < −center_y` leva BAIXO no passo de lado (`0x42d721`); centrado
    /// exige `|oy| < center_y` (`0x42db71`).
    pub center_y: i32,
    /// Pousos da sonda de buraco (`0x478910…` Jake, `0x47d128…` Garathe).
    pub landings: [i32; 3],
}

pub const JAKE_TUNING: Tuning = Tuning {
    melee_min: 7,
    melee: 30,
    near: 30,
    center: 15,
    center_y: 50,
    landings: [110, 150, 255],
};
pub const GARATHE_TUNING: Tuning = Tuning {
    melee_min: 12,
    melee: 50,
    near: 50,
    center: 25,
    center_y: 40,
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

/// O alvo pode ser atacado no estado `s` (`0x42cdef`: o tipo `+0x18` e o estado `+0x180`
/// **do alvo**). Jake: não nos estados 9, 11, 12, 15, 25, 26, 76–78, 81–84, 87, 102, 162,
/// 163, 166, 168, 169, 171, 173, 177, 179, 181; Garathe: não nos 11, 12, 26–33, 35, 38, 45,
/// 55, 58, 59, 61, 63, 65, 67, 69, 80, 127. Os estados de morte negam todos (os pares do Jake
/// passariam, mas alvo morrendo já leva ao modo 3 antes do ataque); 102/45 (soltar o
/// empurrão) e 162/163 (beirada) não existem no port.
pub fn attackable(s: State, ch: Character) -> bool {
    use State::*;
    let common = matches!(
        s,
        LayerDown(_)
            | LayerUp(_)
            | Fall
            | Land(Landing::Fall | Landing::Hard)
            | WildFall
            | GetUp
            | KnockedDown(_)
            | StandUp
            | Unhide
            | Pushed
            | Jolted
            | Rise
            | Special(_)
            | Dying(_)
            | Dead
    );
    let own = match ch {
        // 15 choque
        Character::Jake => matches!(s, Shocked(_)),
        // 26 aterrissagem longa
        Character::Garathe => matches!(s, Land(Landing::Long)),
    };
    !(common || own)
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

/// Reação por tipo (tabela `0x42e328` para 11–41 e as comparações acima de 42).
fn reaction(kind: u32) -> Option<Reaction> {
    Some(match kind {
        19..=21 | 25 | 27 | 29 | 99 | 100 | 106 | 108 => Reaction::Duck,
        26 | 42 | 68..=72 => Reaction::JumpOver,
        39 => Reaction::Piston,
        40 | 41 => Reaction::Fence,
        11 | 89 | 105 | 109 => Reaction::Shoot,
        _ => return None,
    })
}

/// O que o cérebro consulta do mundo num pensamento.
pub struct Senses<'a> {
    /// Sólidos da camada do pensante (células, portas e objetos sólidos).
    pub world: &'a World,
    pub grid: &'a CellGrid,
    /// Perigos da camada dele (a cena já filtra a < `HAZARD_RANGE`; o cérebro ordena pela
    /// distância, como a lista de `0x42cad4`).
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

/// Passo de caminho em curso: célula-alvo (`+0x80…+0x88`) e direção (`+0x8c`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Step {
    pub cell: Cell,
    pub dir: u8,
}

/// A estrutura da IA (`+0x2a0`).
#[derive(Debug, Clone, Default)]
pub struct Brain {
    pub mode: Mode,
    /// Tem alvo (`+0x00`; o único candidato é o jogador humano): escolhido só quando não há
    /// ou ele morreu (`0x42d520` → `0x42c5cc`), e então fica.
    pub target: bool,
    /// Vê o alvo (`+0x54`).
    pub sees: bool,
    /// Última entrada (`+0x78`).
    pub last: u32,
    /// Tiques (o relógio `0x4879c4`) e o tique da última visão (`+0x7c`).
    calls: u32,
    last_seen: Option<u32>,
    /// Classe do alvo no pensamento anterior (`+0x04`, lida antes de ser atualizada).
    tclass: u8,
    pub step: Option<Step>,
    /// Espera `+0x94` (passo ou sem caminho), em pensamentos.
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

/// Vê o alvo: mesma camada, a < 400 px, e linha livre ou tocando (`0x42d520`).
pub fn sees(me: &View, t: &View, world: &World) -> bool {
    me.layer == t.layer
        && dist(me.center(), t.center()) < SIGHT
        && (clear_line(world, me.center(), t.center(), &[me.bounds, t.bounds])
            || me.bounds.overlaps(&crate::behaviour::grow(t.bounds, 1)))
}

/// Parede a `d` px para o lado `right` (`0x40718d(obj, ±d, 0)`).
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

/// Mira perto do alvo (`0x42cca4`, d < `p`): setores largos, sem "só BAIXO".
pub fn aim_near(a: i32) -> u32 {
    match a.rem_euclid(0x10000) {
        0xf800.. | ..0x800 => RIGHT,
        0x800..0x4000 => RIGHT | DOWN,
        0x4000..0x7800 => LEFT | DOWN,
        0x7800..0x9000 => LEFT,
        0x9000..0xb000 => LEFT | UP,
        0xb000..0xd000 => UP,
        _ => RIGHT | UP,
    }
}

/// Mira longe: só alinhado a uma das sete direções de tiro, em `[c − 0x200, c + 0x200)`.
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
    .find(|(c, _)| (-0x200..0x200).contains(&angle_diff(a, *c)))
    .map(|(_, bits)| bits)
}

/// Contexto de um pensamento: corpo, alvo, medidas, classes (`+0x04`/`+0x08`) e célula.
struct Look<'b> {
    me: &'b Me,
    t: Option<&'b View>,
    dx: i32,
    dy: i32,
    d: i32,
    angle: i32,
    tu: &'static Tuning,
    mc: u8,
    tc: u8,
    here: Cell,
}

impl Look<'_> {
    /// Centro próprio dentro da célula, de 0 a 127 (o `%` do C).
    fn in_cell(&self) -> (i32, i32) {
        let c = self.me.view.center();
        (c.0 % CELL, c.1 % CELL)
    }

    /// Centrado na célula (`0x42db71`): `|ox| < 15/25` e `|oy| < 50/40`.
    fn centred(&self) -> bool {
        let (x, y) = self.in_cell();
        let (cx, cy) = (self.tu.center, self.tu.center_y);
        x > 64 - cx && x < 64 + cx && y > 64 - cy && y < 64 + cy
    }

    fn crawling(&self) -> bool {
        self.me.view.state == State::Crawl
    }
}

/// Célula vizinha na direção `dir` (1 y−1, 2 y+1, 3 x−1, 4 x+1, 5 camada−1, 6 camada+1).
fn neighbour((x, y, z): Cell, dir: u8) -> Cell {
    match dir {
        1 => (x, y - 1, z),
        2 => (x, y + 1, z),
        3 => (x - 1, y, z),
        4 => (x + 1, y, z),
        5 => (x, y, z - 1),
        6 => (x, y, z + 1),
        _ => (x, y, z),
    }
}

fn manhattan(a: Cell, b: Cell) -> i32 {
    (a.0 - b.0).abs() + (a.1 - b.1).abs() + (a.2 - b.2).abs()
}

impl Brain {
    pub fn new() -> Self {
        Self::default()
    }

    /// Um tique: pensa nos tiques pares e repete a última entrada nos ímpares
    /// (`0x42ec1c`). `human` = o jogador humano, se presente e vivo.
    pub fn think(&mut self, me: &Me, human: Option<&View>, s: &mut Senses) -> u32 {
        let tick = self.calls;
        self.calls = self.calls.wrapping_add(1);
        if tick % THINK_PERIOD != 0 {
            return self.last;
        }
        let here = s.grid.cell_of(me.view.center(), me.view.layer);
        // alvo (`0x42c5cc`): o humano, se está na mesma célula ou a busca o alcança
        self.target = human.is_some_and(|h| {
            self.target || {
                let goal = s.grid.cell_of(h.center(), h.layer);
                goal == here || s.grid.search(here, goal, me.view.center().0).is_some()
            }
        });
        let t = if self.target { human } else { None };
        let (dx, dy, d, angle) = match t {
            Some(t) => {
                let (a, b) = (me.view.center(), t.center());
                let (dx, dy) = (b.0 - a.0, b.1 - a.1);
                (dx, dy, dist(a, b), angle16(dx, dy))
            }
            None => (0, 0, i32::MAX, 0),
        };
        match t {
            Some(t) => {
                self.sees = sees(&me.view, t, s.world)
                    && !(self.mode == Mode::Wander
                        && (self.tclass == class::HIDDEN || self.tclass == class::DYING));
                if self.sees {
                    self.last_seen = Some(tick);
                    self.reset();
                }
            }
            None => self.sees = false,
        }
        let tc = t.map_or(class::NONE, View::class);
        self.tclass = tc;
        let l = Look {
            me,
            t,
            dx,
            dy,
            d,
            angle,
            tu: tuning(me.view.character),
            mc: me.view.class(),
            tc,
            here,
        };
        let mut inp = 0;
        if t.is_none() {
            self.wander(&l, s, &mut inp);
        } else {
            match self.mode {
                Mode::Retreat => self.retreat_mode(&l, s, &mut inp),
                Mode::Wander => self.wander(&l, s, &mut inp),
                Mode::Hunt => self.hunt(&l, s, &mut inp),
            };
        }
        self.last = inp;
        inp
    }

    /// Apaga o passo e a espera (`0x42d6e3`).
    fn reset(&mut self) {
        self.step = None;
        self.wait = 0;
    }

    /// Modo 1 (`0x42ed64`). As funções devolvem o "conseguiu" do original e escrevem a
    /// entrada em `inp` (a global `0x487a2c`, que começa em 0).
    fn hunt(&mut self, l: &Look, s: &mut Senses, inp: &mut u32) -> bool {
        let me = l.me;
        if l.t.is_none() {
            self.mode = Mode::Wander;
            return true;
        }
        self.mode = Mode::Hunt;
        if me.pending > 10 || (me.pending != 0 && me.hp < 30) {
            return self.retreat_mode(l, s, inp);
        }
        if matches!(l.mc, class::POSE | class::SPECIAL | class::LAND) {
            return false;
        }
        if self.sees {
            if l.tc == class::HIDDEN || l.tc == class::DYING {
                self.mode = Mode::Wander;
                return true;
            }
            // ataque e os casos no ar/barra/escalada voltam sem os perigos
            match l.mc {
                class::RISE | class::FALL => {
                    if self.air(l, inp) {
                        return true;
                    }
                }
                class::BAR => {
                    if self.bar(l, s, inp) {
                        return true;
                    }
                }
                class::CLIMB => {
                    if self.climb(l, inp) {
                        return true;
                    }
                }
                class::ATTACK => return self.attack(l, inp),
                class::DYING => return false,
                _ => {
                    if self.attack(l, inp) {
                        return true;
                    }
                    if self.approach(l, s, inp) {
                        self.hazards(l, s, inp);
                        return true;
                    }
                }
            }
        }
        self.path(l, s, inp)
    }

    /// Caminho até o alvo (`0x42ef48`): pose logo após perder de vista, passo, espera.
    fn path(&mut self, l: &Look, s: &mut Senses, inp: &mut u32) -> bool {
        let Some(t) = l.t else {
            return self.hazards(l, s, inp);
        };
        let tick = self.calls.wrapping_sub(1);
        // `tique − 2·+0xdc < +0x7c < tique`: o primeiro pensamento sem ver
        let lost = self
            .last_seen
            .is_some_and(|n| n < tick && tick - n < 2 * THINK_PERIOD);
        if lost && taunt(l.mc, false, s, inp) {
            return true;
        }
        let here = l.here;
        let x = l.me.view.center().0;
        let search = |s: &Senses| {
            let goal = s.grid.cell_of(t.center(), t.layer);
            s.grid.search(here, goal, x).unwrap_or(0)
        };
        match self.step {
            None if self.wait == 0 => {
                let dir = search(s);
                *inp = self.step_out(l, s, dir, Some(here));
            }
            None => {
                self.wait -= 1;
                if l.mc == class::IDLE && s.rng.below(50) == 0 {
                    taunt(l.mc, true, s, inp);
                } else {
                    *inp = 0;
                    if s.rng.below(1000) == 0 {
                        self.reset();
                        self.mode = Mode::Wander;
                    }
                }
            }
            Some(st) if st.cell == here => {
                if l.centred() {
                    let dir = search(s);
                    let out = self.step_out(l, s, dir, Some(here));
                    if out != self.last && taunt(l.mc, false, s, inp) {
                        return true;
                    }
                    *inp = out;
                } else if self.wait == 0 {
                    let dir = search(s);
                    *inp = self.step_out(l, s, dir, Some(here));
                } else {
                    *inp = self.step_out(l, s, st.dir, None);
                    self.wait -= 1;
                }
            }
            Some(st) => {
                if manhattan(here, st.cell) > 1 {
                    self.reset();
                    *inp = 0;
                } else if self.wait == 0 {
                    let dir = search(s);
                    *inp = self.step_out(l, s, dir, Some(here));
                } else {
                    *inp = self.step_out(l, s, st.dir, None);
                    self.wait -= 1;
                }
            }
        }
        self.hazards(l, s, inp);
        true
    }

    /// Modo 2 (`0x42f35b`).
    fn retreat_mode(&mut self, l: &Look, s: &mut Senses, inp: &mut u32) -> bool {
        if l.t.is_none() {
            self.mode = Mode::Wander;
            return true;
        }
        self.mode = Mode::Retreat;
        if !self.sees {
            self.mode = Mode::Hunt;
            return false;
        }
        if l.mc == class::LAND {
            return false;
        }
        if self.retreat(l, s, inp) {
            self.hazards(l, s, inp);
            return true;
        }
        false
    }

    /// Corre para o lado oposto ao alvo (`0x42df9f`); parede ou buraco sem pouso → modo 1
    /// (a entrada de lado fica neste pensamento).
    fn retreat(&mut self, l: &Look, s: &mut Senses, inp: &mut u32) -> bool {
        let away = l.dx < 0;
        *inp = side(away);
        if wall(&l.me.view, s.world, away, WALL_PROBE) || !gap(l, s, inp) {
            self.mode = Mode::Hunt;
            return false;
        }
        // o segundo teste de ângulo (0x4c00 ≤ a < 0x4400) nunca vale
        if (0xbc00..0xc400).contains(&l.angle) && l.mc == class::CLIMB {
            *inp |= DOWN;
        }
        true
    }

    /// Modo 3 (`0x42f434`): vaga de célula em célula, de lado, sempre FURTIVO.
    fn wander(&mut self, l: &Look, s: &mut Senses, inp: &mut u32) -> bool {
        self.mode = Mode::Wander;
        if l.me.pending != 0 && l.t.is_some() {
            return self.retreat_mode(l, s, inp);
        }
        if l.t.is_some() && l.tc != class::HIDDEN {
            // o ramo põe o modo 0 (= caçar) e devolve, com a entrada ainda 0
            self.mode = Mode::Hunt;
            return true;
        }
        match l.mc {
            2 | 4..=7 | 15 | 16 => return false,
            8 | 9 | 13 => {
                self.mode = Mode::Hunt;
                return false;
            }
            11 | 12 => {
                *inp = JUMP;
                return true;
            }
            _ => {}
        }
        if l.t.is_none() {
            return self.hazards(l, s, inp);
        }
        let here = l.here;
        // passo de lado; com PULAR (buraco) o passo é desfeito e fica só FURTIVO
        let walk = |b: &mut Self, s: &mut Senses, dir: u8, start: Option<Cell>| {
            let out = b.step_out(l, s, dir, start);
            if out & JUMP != 0 {
                b.reset();
                STEALTH
            } else {
                out | STEALTH
            }
        };
        match self.step {
            None if self.wait == 0 => {
                let dir = if s.rng.below(2) == 1 { 3 } else { 4 };
                *inp = walk(self, s, dir, Some(here));
            }
            None => {
                self.wait -= 1;
                if l.mc == class::IDLE && s.rng.below(20) == 0 {
                    taunt(l.mc, true, s, inp);
                    *inp |= STEALTH;
                } else {
                    *inp = 0;
                }
            }
            Some(st) if st.cell == here => {
                if l.centred() {
                    let bits = s.grid.cell(here).map_or(0, |c| c.bits);
                    let dir = match st.dir {
                        3 if bits & path::bits::WALL_LEFT != 0 => 4,
                        4 if bits & path::bits::WALL_RIGHT != 0 => 3,
                        d => d,
                    };
                    let out = walk(self, s, dir, Some(here));
                    if s.rng.below(20) == 0 {
                        taunt(l.mc, true, s, inp);
                        *inp |= STEALTH;
                        return true;
                    }
                    *inp = out;
                } else if self.wait == 0 {
                    *inp = walk(self, s, st.dir, Some(here));
                } else {
                    self.wait -= 1;
                    *inp = walk(self, s, st.dir, None);
                }
            }
            Some(st) => {
                if manhattan(here, st.cell) > 1 {
                    self.reset();
                    *inp = 0;
                } else {
                    // com a espera no fim, segue o passo sem renová-la
                    self.wait = self.wait.saturating_sub(1);
                    *inp = walk(self, s, st.dir, None);
                }
            }
        }
        self.hazards(l, s, inp);
        true
    }

    /// No ar (`0x42e0f9`): CIMA (corda) com o alvo na metade de cima e ainda sem cair rápido.
    fn air(&self, l: &Look, inp: &mut u32) -> bool {
        if l.angle >= 0x8000 && l.me.view.vy > -16 {
            *inp = UP;
        }
        true
    }

    /// Na barra (`0x42e05d`).
    fn bar(&self, l: &Look, s: &Senses, inp: &mut u32) -> bool {
        if (0x3800..0x4800).contains(&l.angle) {
            *inp = JUMP | DOWN;
            return true;
        }
        let open_below = s
            .grid
            .cell(l.here)
            .is_some_and(|c| c.bits & path::bits::CLOSED_BELOW == 0);
        if !open_below && (l.dy <= -10 || l.dy >= 10) {
            *inp = DOWN;
            return true;
        }
        let v = &l.me.view;
        if v.character == Character::Jake
            && matches!(v.state, State::BarUp | State::BarPull | State::BarDown)
        {
            *inp = DOWN;
            return true;
        }
        *inp = side(l.dx > 0);
        true
    }

    /// Na escada/corda (`0x42e129`): ATIRAR solta.
    fn climb(&self, l: &Look, inp: &mut u32) -> bool {
        let (dx, dy) = (l.dx, l.dy);
        *inp = 0;
        if dy > 20 && dx > -10 && dx < 10 {
            *inp |= SHOOT;
        }
        if dy < 5 {
            *inp |= UP;
        } else if dy > 20 {
            *inp |= SHOOT;
        }
        if dx > 20 {
            *inp |= RIGHT;
        } else if dx < -20 {
            *inp |= LEFT;
        }
        if *inp == 0 {
            *inp = SHOOT;
        }
        true
    }

    /// Ataque (`0x42e659`). Perto, conseguir não exige entrada: alvo que não pode ser
    /// atacado (`0x42cdef`) dá entrada 0 e o pensamento acaba aí.
    fn attack(&self, l: &Look, inp: &mut u32) -> bool {
        let (me, tu) = (l.me, l.tu);
        let Some(t) = l.t else { return false };
        if !self.sees {
            return false;
        }
        let d = l.d;
        if d < tu.melee_min {
            return false;
        }
        if d < tu.melee {
            if d < (tu.melee - tu.melee_min) / 2 + tu.melee_min {
                if self.last & (SHOOT | PUNCH) != 0 {
                    *inp = self.last;
                    return true;
                }
                return false;
            }
            if l.tc != class::PUSH && ((l.dx > 0 && t.vx > 0) || (l.dx < 0 && t.vx < 0)) {
                return false;
            }
            *inp = if attackable(t.state, t.character) {
                PUNCH | aim_near(l.angle)
            } else {
                0
            };
            return true;
        }
        if d >= LONG_RANGE {
            return false;
        }
        match me.view.character {
            Character::Jake if me.weapon == FISTS => return false,
            Character::Garathe => {
                let sign = |v: i32| if v < 0 { -1 } else { 1 };
                let a = l.angle;
                let horizontal = !(0x1000..0xf000).contains(&a) || (0x7000..0x9000).contains(&a);
                if me.view.state == State::Run
                    && d > LEAP_MIN
                    && sign(me.view.vx) == sign(t.vx)
                    && horizontal
                    && t.state == State::Run
                {
                    *inp = side(l.dx > 0) | JUMP | UP;
                    return true;
                }
                if matches!(me.weapon, 1 | 2) {
                    return false;
                }
            }
            Character::Jake => {}
        }
        *inp = match aim_far(l.angle) {
            Some(a) if attackable(t.state, t.character) => a | SHOOT,
            _ => 0,
        };
        *inp != 0
    }

    /// Aproximação (`0x42dd8f`).
    fn approach(&self, l: &Look, s: &mut Senses, inp: &mut u32) -> bool {
        let Some(t) = l.t else { return false };
        let tc = l.tc;
        if !matches!(tc, 1 | 3 | 4 | 5 | 7 | 8 | 9 | 11 | 12 | 13 | 15 | 16) {
            return false;
        }
        let me = &l.me.view;
        let dy = if tc == class::RISE && l.dy.abs() < 64 {
            0
        } else {
            l.dy
        };
        if dy > -20 && dy < 20 {
            if l.d < l.tu.near {
                if tc != class::PUSH && t.vx != 0 {
                    // segue o alvo que anda
                    *inp = side(t.vx > 0);
                } else if tc != class::CLIMB && tc != class::BAR {
                    // alvo parado: recua andando para ganhar espaço
                    *inp = STEALTH | side(l.dx < 0);
                }
                if *inp & (LEFT | RIGHT) != 0 && wall(me, s.world, *inp & RIGHT != 0, WALL_PROBE) {
                    *inp ^= LEFT | RIGHT;
                }
            } else if l.dx > 10 {
                *inp = RIGHT;
            } else if l.dx < -10 {
                *inp = LEFT;
            }
            if !gap(l, s, inp) {
                *inp = JUMP | UP;
            }
            return true;
        }
        let a = l.angle;
        if (0xbc00..0xc400).contains(&a) {
            *inp = JUMP | UP;
            return true;
        }
        if l.mc == class::CLIMB {
            if (0x4800..0xbc00).contains(&a) {
                *inp = LEFT;
            } else if !(0x3800..0xc400).contains(&a) {
                *inp = RIGHT;
            }
            // o teste de BAIXO do original vale para todo ângulo fora de 0x9000–0xf000
            *inp |= if (0x9000..0xf000).contains(&a) {
                UP
            } else {
                DOWN
            };
            return true;
        }
        false
    }

    /// Passo de caminho (`0x42d721`): `start` = célula de onde parte (fixa a célula-alvo e
    /// a espera), `None` = continua o passo. Direção 0 = sem caminho: apaga e espera.
    fn step_out(&mut self, l: &Look, s: &mut Senses, dir: u8, start: Option<Cell>) -> u32 {
        if dir == 0 {
            self.reset();
            self.wait = WAIT;
            return 0;
        }
        if let Some(here) = start.filter(|c| c.0 >= 0 && c.1 >= 0 && c.2 >= 0) {
            self.wait = WAIT;
            self.step = Some(Step {
                cell: neighbour(here, dir),
                dir,
            });
        }
        let (x, y) = l.in_cell();
        let (ox, oy) = (x - 64, y - 64);
        let (c, h, mc) = (l.tu.center, l.tu.center_y, l.mc);
        let centre = |i: u32| {
            if ox < -c {
                RIGHT
            } else if ox > c {
                LEFT
            } else {
                i
            }
        };
        match dir {
            1 => centre(UP),
            2 => centre(JUMP | DOWN),
            3 | 4 => {
                let mut i = side(dir == 4);
                if oy < -h && mc != class::BAR {
                    i |= DOWN;
                } else if oy > 0 && (mc == class::CLIMB || mc == class::RISE) {
                    i |= UP;
                }
                if gap(l, s, &mut i) { i } else { JUMP | UP }
            }
            _ => {
                let mut i = if dir == 5 {
                    UP
                } else if mc == class::CLIMB {
                    JUMP
                } else {
                    DOWN
                };
                if oy < -h {
                    i = 0;
                }
                centre(i)
            }
        }
    }

    /// Perigos (`0x42e1a9`): o código da célula própria, depois o primeiro objeto da lista
    /// (ordenada pela distância) cuja condição vale; agachado sem perigo → CIMA.
    fn hazards(&self, l: &Look, s: &mut Senses, inp: &mut u32) -> bool {
        let me = &l.me.view;
        let c = me.center();
        let Some(ci) = s.grid.cell(l.here) else {
            return false;
        };
        let code = (ci.code / 10) % 100;
        let x = c.0 % CELL;
        let m = ci.flag40 & 1 != 0;
        let edge = [29, 36, 37, 39].contains(&code);
        if code == 30 || edge && ((m && *inp == RIGHT && x > 64) || (!m && *inp == LEFT && x < 64))
        {
            if l.mc == class::CROUCH {
                *inp |= DOWN;
            } else {
                *inp = DOWN;
            }
            return true;
        }
        if edge && ((!m && *inp == RIGHT && x < 64) || (m && *inp == LEFT && x > 64)) {
            *inp |= DOWN;
            return true;
        }
        let mut list: Vec<(i32, &Threat)> =
            s.threats.iter().map(|t| (dist(c, t.center), t)).collect();
        list.sort_by_key(|(d, _)| *d);
        for (d, t) in list {
            let Some(r) = reaction(t.kind) else { continue };
            let toward =
                (c.0 < t.center.0 && *inp & RIGHT != 0) || (c.0 > t.center.0 && *inp & LEFT != 0);
            match r {
                Reaction::Duck if d < 100 => {
                    let coming =
                        (c.0 < t.center.0 && t.vel.0 < 0) || (c.0 > t.center.0 && t.vel.0 > 0);
                    if coming && !l.crawling() {
                        *inp = DOWN;
                        return true;
                    }
                }
                Reaction::Piston if d < 95 => {
                    if *inp & (JUMP | SHOOT | PUNCH) != 0 {
                        return true;
                    }
                    if d < 75 && c.1 > t.center.1 && !l.crawling() {
                        *inp = DOWN;
                        return true;
                    }
                    if l.mc == class::CROUCH {
                        *inp |= DOWN;
                    }
                    return true;
                }
                Reaction::Fence if d < 50 && toward => {
                    *inp |= JUMP | UP;
                    return true;
                }
                Reaction::JumpOver if d < 100 => {
                    if toward {
                        *inp |= JUMP | UP;
                    } else {
                        *inp = side(s.rng.below(2) == 1) | JUMP | UP;
                    }
                    return true;
                }
                Reaction::Shoot if d < 50 => {
                    *inp = SHOOT | if c.1 < t.center.1 { DOWN } else { 0 } | side(c.0 < t.center.0);
                    return true;
                }
                _ => {}
            }
        }
        if l.mc == class::CROUCH {
            *inp = UP;
            return true;
        }
        false
    }
}

/// Sonda de buraco sobre uma entrada de um lado só (`0x42cf70`): pouso → +PULAR (agachado:
/// a entrada vira CIMA, `0x42e8d0`); sem pouso → `false`.
fn gap(l: &Look, s: &Senses, inp: &mut u32) -> bool {
    if *inp != LEFT && *inp != RIGHT {
        return true;
    }
    match probe_gap(&l.me.view, s.world, *inp == RIGHT, l.tu) {
        Gap::Ground => true,
        Gap::Landing if l.mc == class::CROUCH => {
            *inp = UP;
            true
        }
        Gap::Landing => {
            *inp |= JUMP;
            true
        }
        Gap::NoLanding => false,
    }
}

/// Pose (`0x42e8f3`): sem `force`, parado ou andando tem 501/1000 de chance; as demais
/// classes sempre.
fn taunt(mc: u8, force: bool, s: &mut Senses, inp: &mut u32) -> bool {
    if !force && (mc == class::IDLE || mc == class::WALK) && s.rng.below(1000) > 500 {
        return false;
    }
    *inp = TAUNT_INPUT;
    true
}

#[cfg(test)]
mod tests;
