// SPDX-License-Identifier: GPL-3.0-or-later
//! Máquina de estados do Jake: parado, correr, pulos e quedas (M3a, `docs/re/personagem.md`)
//! e, na M6b, o deslocamento pelo esqueleto (`docs/re/ske.md`), cordas, camadas, esconder,
//! agachar, barra e o especial (`docs/re/jake-movimentos.md`, `jake-escalada.md`,
//! `escada-camadas.md`, `furtividade.md`). Os estados novos estão em `jake/moves.rs`.

pub mod garathe;
mod moves;
pub mod weapons;

use std::collections::HashMap;

pub use moves::{BAR_BOX, CRAWL_BOX, Env, Skel, SpecialBlast, SpecialEffect, Zone, zone};

pub use crate::character::Character;
use crate::character::Profile;
use crate::damage::{Health, Pending, apply};
use crate::input::{DOWN, JUMP, LEFT, NEXT_WEAPON, PREV_WEAPON, PUNCH, RIGHT, SHOOT, UP};
use crate::physics::{Body, Rect, SUB, World, step};
pub use weapons::{FISTS, GRENADES, PISTOL, ROCKETS, SHOTGUN, STARS, ShotKind, weapon};

// Corrida, pulos e sondas: no perfil do personagem (`crate::character`).
/// Queda: descendo a 5 px/tique ou mais sem chão (`0x0042416f`).
const FALL_SPEED: i32 = 5 * SUB;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JumpKind {
    Medium,
    Long,
    Extended,
}

/// Aterrissagens (`docs/re/jake-movimentos.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Landing {
    /// Depois da queda controlada (25 → 26) ou de ser derrubado.
    Fall,
    /// Depois do pulo médio (29 → 35).
    Medium,
    /// Depois do pulo longo (28 → 34).
    Long,
    /// Depois do pulo estendido ou da queda descontrolada (80/76 → 77).
    Hard,
}

/// Escalada: vertical/diagonal (30, `MNCLIMB1`) ou de lado (32, `MNCLIMB2`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Climb {
    Vertical,
    Side,
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
    /// Queda descontrolada (76, fim do pulo estendido no ar).
    WildFall,
    /// Aterrissagem (26, 35, 34 ou 77).
    Land(Landing),
    /// Levantar após a queda descontrolada (78).
    GetUp,
    /// Sacudir a poeira (79).
    Shake,
    /// Andar furtivo (estado 2, rótulo 149).
    Stealth,
    /// Pulo para cima (16); `true` = saindo da corda (17).
    JumpUp(bool),
    /// Especial BAIXO+ATIRAR (estado 9, rótulo 9); tiques nele.
    Special(u32),
    /// Diante da corda (3) e soltar/descer dela (8: o 3 ao contrário).
    Grab,
    LetGo,
    /// Escalando: 30 (vertical e diagonais) ou 32 (para o lado).
    Climb(Climb),
    /// Troca de camada: 11 (BAIXO, camada + 1) e 12 (CIMA, camada − 1), com o contador
    /// `+0x268`.
    LayerDown(i32),
    LayerUp(i32),
    /// Agachar (85), engatinhar (86), levantar (87: o 85 ao contrário).
    Crouch,
    Crawl,
    StandUp,
    /// Esconder-se (164), escondido (165), sair (166: o 164 ao contrário).
    Hide,
    Hidden,
    Unhide,
    /// Barra: pendurado (103), mão ante mão (107), puxar (105), em cima (104), descer (106:
    /// o 105 ao contrário).
    BarHang,
    BarMove,
    BarPull,
    BarUp,
    BarDown,
    /// Empurrado (estado 168, `0x00423366`).
    Pushed,
    /// Derrubado em voo (estados 81/82, `0x004235fc`); `true` = jogado para a direita.
    KnockedDown(bool),
    /// Choque elétrico (estado 15, `0x00423577`); tiques restantes.
    Shocked(u32),
    /// Morrendo (`0x004236b9`): rótulo da animação da causa.
    Dying(u16),
    /// Cadáver (`0x004268ac`): último quadro, sem colisão com objetos.
    Dead,
    /// Entrando na porta de saída (estado 13, `0x00426820`); tiques desde a entrada.
    EnteringExit(u32),
    /// Atirando ou socando (`0x0043b50b`): mira, contador `t` (`+0x260`), arma e qual dos
    /// dois estados do par (sorteio `rand & 1`).
    Fire {
        aim: Aim,
        t: u32,
        w: u8,
        alt: bool,
    },
    /// Recuperação após o tiro (pistola 22, `J022`…) da arma.
    Recover(u8),
    /// Arma pronta (pistola 18, `J018`…); tiques nela e a arma.
    Ready(u32, u8),
    /// Guardando a arma (pistola 19, `J019`…).
    Holster(u8),
    /// Levado por um teletransporte (`0x4136ef`): invisível, sem entrada nem dano; tiques
    /// desde a partida.
    Teleporting(u32),
    /// Só o Garathe: levantar depois de derrubado para trás ou do tranco (35 `MNBLGTUP`).
    Rise,
    /// Só o Garathe: tranco (59, rótulo 63; desliza pelo esqueleto com k = 2).
    Jolted,
    /// Pose de provocação com a entrada exatamente 0xf (Jake: estado 4, rótulo 18,
    /// `0x426f05`; Garathe: 76, `0x42bcbc`); no fim do rótulo, parado.
    Taunt,
}

/// Entrada da pose: as quatro direções e nada mais (`0x423940`, passo 1).
pub const TAUNT_INPUT: u32 = UP | DOWN | LEFT | RIGHT;

/// O jogador (o Jake ou o Garathe, pelo perfil).
pub type Player = Jake;

/// Viagem do teletransporte: some, muda de lugar na metade e reaparece no fim.
pub const TELEPORT_TICKS: u32 = 42;
/// Tique da viagem em que o passageiro muda de lugar (metade).
pub const TELEPORT_MOVE: u32 = TELEPORT_TICKS / 2;

/// Mira da pistola (`0x0043ae4c`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Aim {
    Forward,
    Up,
    DiagUp,
    DiagDown,
}

impl Aim {
    /// Pela direção apertada: nada, CIMA+BAIXO ou ESQ+DIR → frente; CIMA → cima; CIMA+lado →
    /// diagonal cima; BAIXO(+lado) → diagonal baixo (o especial de BAIXO fica de fora).
    pub fn from_input(bits: u32) -> Aim {
        let (up, down) = (bits & UP != 0, bits & DOWN != 0);
        let side = (bits & LEFT != 0) != (bits & RIGHT != 0);
        match (up && !down, down && !up, side) {
            (true, _, false) => Aim::Up,
            (true, _, true) => Aim::DiagUp,
            (_, true, _) => Aim::DiagDown,
            _ => Aim::Forward,
        }
    }
}

/// Um projétil (ou cápsula) saindo do cano: posição e velocidade em px/tique.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shot {
    pub x: i32,
    pub y: i32,
    pub vx: i32,
    pub vy: i32,
    pub kind: ShotKind,
}

impl Shot {
    /// Cápsula ou cartucho (tipos 22/23).
    pub fn is_casing(&self) -> bool {
        self.kind.is_casing()
    }
}

/// Inventário: 30 pares (tem, munição), índice = número da arma (1 punhos, 2 pistola…), e a
/// arma corrente (`+0x254`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inventory {
    pub slots: [(i32, i32); 30],
    pub weapon: usize,
    /// Dono: escolhe a lista das armas e a coleta.
    pub character: Character,
}
/// Som "troca de arma / sem munição".
pub const SOUND_NO_AMMO: u16 = 3090;
/// Ticks em "pronto" antes de guardar a arma (descritor da pistola, `+0x14`).
const READY_TICKS: u32 = 10;

impl Inventory {
    /// De `points` do objeto do Jake; a inicialização força o par 1 a (1, 20) (`0x0042277e`).
    pub fn from_points(p: &[(i32, i32)]) -> Self {
        let mut slots = [(0, 0); 30];
        for (s, &v) in slots.iter_mut().zip(p) {
            *s = v;
        }
        slots[1] = (1, 20);
        let mut inv = Self {
            slots,
            weapon: FISTS,
            character: Character::Jake,
        };
        inv.weapon = inv.best();
        inv
    }

    /// De `points` do objeto do personagem `ch` (o Garathe também força o par 1 a (1, 20),
    /// `0x004279b3`).
    pub fn for_character(p: &[(i32, i32)], ch: Character) -> Self {
        let mut inv = Self::from_points(p);
        inv.character = ch;
        inv.weapon = inv.best();
        inv
    }

    fn order(&self) -> &'static [usize; 6] {
        &self.character.profile().order
    }

    pub fn pistol_ammo(&self) -> i32 {
        self.ammo(PISTOL)
    }

    /// Munição de uma arma que o Jake tem (0 sem o "tem").
    pub fn ammo(&self, w: usize) -> i32 {
        match self.slots.get(w) {
            Some(&(has, n)) if has != 0 => n,
            _ => 0,
        }
    }

    /// Disponível: tem ≠ 0 e munição > 0 (`0x43a8cc`).
    pub fn available(&self, w: usize) -> bool {
        self.ammo(w) > 0
    }

    /// A melhor disponível (modo 1 de `0x43a8cc`); nada disponível → punhos.
    pub fn best(&self) -> usize {
        self.order()
            .iter()
            .rev()
            .copied()
            .find(|&w| self.available(w))
            .unwrap_or(FISTS)
    }

    /// A disponível seguinte (`next`) ou anterior à corrente, dando a volta (modos 8 e 4).
    pub fn cycle(&self, next: bool) -> usize {
        let order = self.order();
        let n = order.len();
        let i = order.iter().position(|&w| w == self.weapon).unwrap_or(0);
        (1..=n)
            .map(|k| {
                let j = if next { i + k } else { i + n * 2 - k };
                order[j % n]
            })
            .find(|&w| self.available(w))
            .unwrap_or(self.weapon)
    }

    /// Ganhou a arma `w` (`0x43af6c`): se ela fica depois da melhor disponível na lista,
    /// vira a corrente.
    pub fn gained(&mut self, w: usize) {
        let pos = |x: usize| self.order().iter().position(|&o| o == x);
        if pos(w) > pos(self.best()) {
            self.weapon = w;
        }
    }
}

/// Rótulos das animações de morte (`docs/re/dano-vida.md`). O estado 177 do original toca o
/// rótulo `J077`: o rótulo 177 não existe no `JAKE.ANM`.
pub const DIE_DEFAULT: u16 = 173;
pub const DIE_KNOCKED_RIGHT: u16 = 77;
pub const DIE_KNOCKED_LEFT: u16 = 179;
pub const DIE_POISON: u16 = 171;
/// Todos os rótulos usados pelos estados (para conferir contra o `JAKE.ANM` real).
pub const ALL_LABELS: [u16; 48] = [
    1,
    10,
    100,
    27,
    31,
    29,
    28,
    80,
    25,
    26,
    35,
    34,
    76,
    77,
    78,
    79,
    168,
    81,
    82,
    15,
    12,
    DIE_DEFAULT,
    DIE_KNOCKED_RIGHT,
    DIE_KNOCKED_LEFT,
    23,
    56,
    54,
    55,
    22,
    18,
    19,
    149,
    16,
    17,
    9,
    3,
    30,
    32,
    11,
    85,
    86,
    164,
    165,
    103,
    104,
    105,
    107,
    DIE_POISON,
];
/// Alcance do soco além da caixa do Jake (`0x0043accb`, segmento até x + l + 5).
pub const PUNCH_REACH: i32 = 5;
/// Período do veneno em tiques.
pub const POISON_PERIOD: u32 = 20;
/// Tiques do choque elétrico.
const SHOCK_TICKS: u32 = 10;
/// Escala mínima ao entrar na porta (numerador sobre a altura do quadro de referência).
const EXIT_MIN_SCALE: i32 = 30;

/// Rótulo do `JAKE.ANM` e campo c (avança um quadro a cada c+1 tiques) de cada estado
/// (tabela `0x473300`, colunas `+0x10` e `+0x18`).
fn jake_anim(s: State) -> (u16, u32) {
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
        State::WildFall => (76, 0),
        State::Land(Landing::Fall) => (26, 0),
        State::Land(Landing::Medium) => (35, 0),
        State::Land(Landing::Long) => (34, 0),
        State::Land(Landing::Hard) => (77, 0),
        State::GetUp => (78, 0),
        State::Shake => (79, 1),
        State::Pushed => (168, 0),
        State::KnockedDown(true) => (81, 0),
        State::KnockedDown(false) => (82, 0),
        State::Shocked(_) => (15, 0),
        State::Dying(label) => (label, 0),
        State::Dead => (0, 0),
        State::EnteringExit(_) => (12, 0),
        State::Fire { aim, w, alt, .. } => (
            weapon(w as usize).states[weapons::aim_slot(aim)][usize::from(alt)].0,
            0,
        ),
        State::Recover(w) => {
            let wp = weapon(w as usize);
            (wp.states[2][0].1.recover, 0)
        }
        State::Ready(_, w) => (weapon(w as usize).ready, 0),
        State::Holster(w) => (weapon(w as usize).holster, 0),
        State::Stealth => (149, 0),
        State::JumpUp(false) => (16, 0),
        State::JumpUp(true) => (17, 0),
        State::Special(_) => (9, 0),
        State::Grab | State::LetGo => (3, 0),
        State::Climb(Climb::Vertical) => (30, 0),
        State::Climb(Climb::Side) => (32, 0),
        State::LayerDown(_) => (11, 0),
        State::LayerUp(_) => (12, 0),
        State::Crouch | State::StandUp => (85, 0),
        State::Crawl => (86, 0),
        State::Hide | State::Unhide => (164, 0),
        State::Hidden => (165, 0),
        State::BarHang => (103, 0),
        State::BarMove => (107, 1),
        State::BarPull | State::BarDown => (105, 0),
        State::BarUp => (104, 0),
        State::Teleporting(_) => (1, 1),
        State::Rise => (78, 0),
        State::Jolted => (168, 0),
        State::Taunt => (18, 0),
    }
}

/// Estados que tocam o rótulo de trás para a frente (`0x00423894`).
fn jake_reversed(s: State) -> bool {
    matches!(
        s,
        State::LetGo | State::StandUp | State::Unhide | State::BarDown
    )
}

/// Como a animação avança neste tique.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AnimMode {
    /// Pelo estado: para a frente em laço, ou para trás nos estados de `reversed`.
    Auto,
    /// Parada no quadro.
    Hold,
    /// Para trás em laço.
    Reverse,
    /// Dois quadros por tique (sem passar do último).
    Double,
    /// Direto ao quadro dado (limitado ao último).
    Target(usize),
}

/// Pedido de velocidade pelo esqueleto neste tique (`0x00406fc0`): fator k e eixos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SkeReq {
    k: i32,
    x: bool,
    y: bool,
    /// Diagonal da corda: vx = diag · vy.
    diag: i32,
}

/// O Jake controlável.
#[derive(Debug, Clone)]
pub struct Jake {
    pub body: Body,
    /// Posição inicial (`+0x29c`), para onde volta ao sair da fase.
    start: (i32, i32),
    state: State,
    facing_right: bool,
    ticks: u32,
    prev_input: u32,
    lengths: HashMap<u16, usize>,
    label_changed: bool,
    advance: bool,
    pub health: Health,
    pending: Pending,
    /// Rótulo em que morreu (o cadáver fica no último quadro dele).
    death_label: u16,
    /// Deslocamento pendente da viagem de teletransporte.
    teleport: Option<(i32, i32)>,
    /// Numerador da escala ao entrar na porta (encolhe 1 por tique após 10 tiques).
    exit_scale: Option<i32>,
    pub inventory: Inventory,
    /// Balas e cápsulas disparadas neste tique (a cena drena).
    pub shots: Vec<Shot>,
    /// Apertou ATIRAR sem munição neste tique (a cena emite o som).
    pub no_ammo: bool,
    /// Quadro corrente dentro do rótulo do estado (`+0x184`).
    frame: usize,
    anim_mode: AnimMode,
    ske_req: Option<SkeReq>,
    /// Esqueleto (`JAKE.SKE`) com a escala do Jake; sem ele, nada se move pelo esqueleto.
    skel: Option<Skel>,
    /// Camada (`+0x17c`).
    layer: i32,
    /// Caixas de colisão: padrão (`JK_DEFLT`), engatinhando (`JK_CRAWL`) e na barra
    /// (`JK_BARUP`).
    box_default: Rect,
    box_crawl: Rect,
    box_bar: Rect,
    /// Gerador para o sorteio do 79 (`rand & 3`).
    rng: u32,
    /// Especial disparado neste tique (a cena aplica nos objetos).
    pub special: Option<SpecialBlast>,
    /// Caixa do soco no primeiro tique da janela do golpe (a cena fere quem cruza).
    pub punch: Option<Rect>,
    /// Dano desse golpe (linha do `DAMAGE.TBL` da arma e da mira).
    pub blow: Option<Pending>,
    /// Trocou de arma neste tique (a cena toca 3090).
    pub switched: bool,
    /// Tiques restantes da pose "pronto" depois da troca (`+0x26c`).
    switch_pose: u32,
    /// Veneno (`+0x270`): quanto perde a cada 20 tiques.
    pub poison: i32,
    /// Chegou ao fim do rótulo 100 empurrando neste tique (`0x00425d04`: a cena tenta a
    /// porta da célula numa zona PR 0x20).
    pub door_push: bool,
    /// Tiques desde a criação (o veneno age em `tique % 20 = 0`).
    clock: u32,
    /// Quem é (perfil de movimentos, rótulos e armas).
    character: Character,
    /// Som posicional pedido neste tique (Garathe: pouso do pulo longo, empurrar, pulo
    /// contra a parede; a cena toca).
    pub sound: Option<u16>,
}

impl Jake {
    /// `lengths`: número de quadros de cada rótulo do `JAKE.ANM`.
    pub fn new(x: i32, y: i32, boxr: Rect, lengths: HashMap<u16, usize>) -> Self {
        Self {
            body: Body::new(x, y, boxr),
            start: (x, y),
            state: State::Idle,
            facing_right: false, // começa olhando para a esquerda (+0x288 = 4)
            ticks: 0,
            prev_input: 0,
            lengths,
            label_changed: true,
            advance: false,
            health: Health { hp: 100, armor: 0 },
            pending: Pending::default(),
            death_label: DIE_DEFAULT,
            teleport: None,
            exit_scale: None,
            inventory: Inventory::from_points(&[]),
            shots: Vec::new(),
            no_ammo: false,
            frame: 0,
            anim_mode: AnimMode::Auto,
            ske_req: None,
            skel: None,
            layer: 0,
            box_default: boxr,
            box_crawl: moves::CRAWL_BOX,
            box_bar: moves::BAR_BOX,
            rng: 0x4a41_4b45,
            special: None,
            punch: None,
            blow: None,
            switched: false,
            switch_pose: 0,
            poison: 0,
            door_push: false,
            clock: 0,
            character: Character::Jake,
            sound: None,
        }
    }

    fn is_garathe(&self) -> bool {
        self.character == Character::Garathe
    }

    /// Passa a ser o personagem `ch` (perfil, rótulos e armas; o inventário segue).
    pub fn with_character(mut self, ch: Character) -> Self {
        self.character = ch;
        self.inventory.character = ch;
        if let Some(sk) = self.skel.as_mut() {
            sk.draw = ch.profile().draw;
        }
        self.inventory.weapon = self.inventory.best();
        self
    }

    pub fn character(&self) -> Character {
        self.character
    }

    pub(crate) fn profile(&self) -> &'static Profile {
        self.character.profile()
    }

    /// Rótulo e atraso do estado `s` para este personagem.
    fn anim_of(&self, s: State) -> (u16, u32) {
        match (self.character, s) {
            (Character::Jake, _) => jake_anim(s),
            (Character::Garathe, State::KnockedDown(right)) => {
                (garathe::knocked_label(right, self.facing_right), 1)
            }
            (Character::Garathe, _) => garathe::anim(s),
        }
    }

    /// O estado `s` toca o rótulo de trás para a frente.
    fn reversed_state(&self, s: State) -> bool {
        match self.character {
            Character::Jake => jake_reversed(s),
            Character::Garathe => garathe::reversed(s),
        }
    }

    /// Estado de arte espelhada.
    fn mirrored(&self, s: State) -> bool {
        match self.character {
            Character::Jake => moves::mirrored_art(s),
            Character::Garathe => garathe::mirrored(s),
        }
    }

    /// Descritor da arma `w` deste personagem.
    pub fn weapon(&self, w: usize) -> &'static weapons::Weapon {
        weapons::weapon_of(self.character, w)
    }

    /// Usa o esqueleto (deslocamento e velocidades dos estados que o seguem).
    pub fn with_skeleton(mut self, mut skel: Skel) -> Self {
        skel.draw = self.profile().draw;
        self.skel = Some(skel);
        self
    }

    /// Caixas de engatinhar e de pendurar na barra (dos BRDs do jogo).
    pub fn with_boxes(mut self, crawl: Rect, bar: Rect) -> Self {
        self.box_crawl = crawl;
        self.box_bar = bar;
        self
    }

    /// Camada inicial.
    pub fn with_layer(mut self, layer: i32) -> Self {
        self.layer = layer;
        self
    }

    /// Põe o Jake na camada `layer` (sem a animação das portas).
    pub fn set_layer(&mut self, layer: i32) {
        self.layer = layer;
    }

    /// Camada corrente (`+0x17c`).
    pub fn layer(&self) -> i32 {
        self.layer
    }

    /// Quadro corrente dentro do rótulo (o cadáver fica no último).
    pub fn frame(&self) -> usize {
        match self.state {
            State::Dead => self.len_of(self.death_label) - 1,
            _ => self.frame,
        }
    }

    pub fn with_inventory(mut self, inv: Inventory) -> Self {
        self.inventory = inv;
        self.inventory.character = self.character;
        self
    }

    /// Começa (ou recomeça) um tiro ou soco com a mira da entrada (`0x0043b43e`). Sem
    /// munição (`0x0043b1d0`): aviso (som 3090), a melhor arma e a pose "pronto" dela. A boca
    /// dentro da parede dá um soco (`0x47d7b8 = 2`).
    fn start_fire(&mut self, input: u32, world: &World) -> bool {
        if moves::wants_special(input) && self.health.hp > moves::SPECIAL_MIN_HP {
            self.body.vx = 0;
            self.set(State::Special(0));
            self.tick_special(input, world);
            return true;
        }
        let melee = self.melee_weapon();
        let mut w = if input & PUNCH != 0 {
            melee
        } else {
            self.inventory.weapon
        };
        if self.weapon(w).shot.is_some() && !self.inventory.available(w) {
            self.no_ammo = true;
            self.inventory.weapon = self.inventory.best();
            self.body.vx = 0;
            let nw = self.inventory.weapon as u8;
            self.set(State::Ready(0, nw));
            return true;
        }
        let aim = Aim::from_input(input);
        if self.weapon(w).shot.is_some() {
            let r = self.weapon(w).states[weapons::aim_slot(aim)][0].1;
            let dx = if self.facing_right { r.dx.1 } else { r.dx.0 };
            if !self.muzzle_ok(world, w, (self.body.x + dx, self.body.y + r.dy)) {
                w = melee;
            }
        }
        self.body.vx = 0;
        self.rng = self.rng.wrapping_mul(1_103_515_245).wrapping_add(12345);
        let alt = w == FISTS && (self.rng >> 16) & 1 != 0;
        self.state = State::Fire {
            aim,
            t: 0,
            w: w as u8,
            alt,
        };
        self.ticks = 0;
        self.frame = 0;
        self.label_changed = true;
        true
    }

    /// Arma do SOCO e da boca na parede: os punhos/garras ou, no Garathe com o par 2, a
    /// clava (`0x43ab05`, ramo `0x43ab82`).
    fn melee_weapon(&self) -> usize {
        if self.is_garathe() && self.inventory.slots[weapons::CLUB].0 != 0 {
            weapons::CLUB
        } else {
            FISTS
        }
    }

    /// A boca da arma `w` está livre; o chicote só testa a metade da linha (`0x43bbe3`).
    fn muzzle_ok(&self, world: &World, w: usize, muzzle: (i32, i32)) -> bool {
        if self.weapon(w).half_line {
            let b = self.body.bounds();
            let c = (b.x + b.w / 2, b.y + b.h / 2);
            self.muzzle_clear(world, ((c.0 + muzzle.0) / 2, (c.1 + muzzle.1) / 2))
        } else {
            self.muzzle_clear(world, muzzle)
        }
    }

    /// Dano do golpe corpo a corpo do estado de tiro corrente (linha do `DAMAGE.TBL` da
    /// variante), com o empurrão pelo lado.
    fn blow_damage(&self, w: u8, aim: Aim) -> Option<Pending> {
        let b = self.weapon(w as usize).blow?[weapons::aim_slot(aim)];
        let dir = self.dir();
        Some(Pending {
            amount: b.damage,
            knockdown: b.knockdown,
            shove: !b.knockdown,
            push: (b.push.0 * dir, b.push.1),
            ..Default::default()
        })
    }

    /// Caixa do golpe (`0x0043accb`): metade da frente da caixa do Jake e um terço da altura
    /// (cima, meio ou baixo conforme a mira). Ruling: o alcance vai até a ponta do segmento
    /// do recorte (5 px além da caixa), sem o recorte pela parede.
    pub fn punch_box(&self, aim: Aim) -> Rect {
        let b = self.body.bounds();
        let mut r = Rect {
            x: b.x - PUNCH_REACH,
            y: b.y,
            w: b.w / 2 + PUNCH_REACH,
            h: b.h / 3,
        };
        if self.facing_right {
            r.x = b.x + b.w / 2;
        }
        match aim {
            Aim::Up | Aim::DiagUp => {}
            Aim::Forward => r.y += b.h / 3,
            Aim::DiagDown => r.y += 2 * b.h / 3,
        }
        r
    }

    /// Troca de arma (`0x0043b0cb`): PRÓXIMA/ANTERIOR na borda, ou a escolha direta dos bits
    /// `SELECT` (extensão do port).
    fn switch_weapon(&mut self, input: u32) {
        let edge = |bit: u32| input & bit != 0 && self.prev_input & bit == 0;
        let sel = ((input & crate::input::SELECT_MASK) >> crate::input::SELECT_SHIFT) as usize;
        let prev_sel =
            ((self.prev_input & crate::input::SELECT_MASK) >> crate::input::SELECT_SHIFT) as usize;
        let new = if edge(PREV_WEAPON) {
            self.inventory.cycle(false)
        } else if edge(NEXT_WEAPON) {
            self.inventory.cycle(true)
        } else if (1..=weapons::ORDER.len()).contains(&sel)
            && sel != prev_sel
            && self.inventory.available(sel)
        {
            sel
        } else {
            return;
        };
        if new != self.inventory.weapon {
            self.inventory.weapon = new;
            self.switched = true;
            self.switch_pose = weapons::SWITCH_POSE_TICKS;
        }
    }

    /// Som do tiro: no primeiro tique de cada estado de tiro da arma que tem som.
    pub fn fire_sound(&self) -> Option<u16> {
        match self.state {
            State::Fire { w, .. } if self.label_changed => self.weapon(w as usize).sound,
            _ => None,
        }
    }

    /// A linha do centro da caixa até a boca da arma está livre de sólidos
    /// (`docs/re/tiro-jake.md`, passo 4: senão o tiro é pulado).
    fn muzzle_clear(&self, world: &World, muzzle: (i32, i32)) -> bool {
        let b = self.body.bounds();
        let c = (b.x + b.w / 2, b.y + b.h / 2);
        (1..=8).all(|i| {
            let p = Rect {
                x: c.0 + (muzzle.0 - c.0) * i / 8,
                y: c.1 + (muzzle.1 - c.1) * i / 8,
                w: 1,
                h: 1,
            };
            !world.blocked(&p)
        })
    }

    /// Tique de um estado de tiro ou soco (`0x0043b50b`/`0x0043b1d0`).
    fn tick_fire(&mut self, input: u32, world: &World, st: (Aim, u32, u8, bool)) {
        let (aim, t, w, alt) = st;
        let pressed = |bit: u32| input & bit != 0;
        let wp = self.weapon(w as usize);
        let r = wp.states[weapons::aim_slot(aim)][usize::from(alt)].1;
        let t = t + wp.step;
        self.state = State::Fire { aim, t, w, alt };
        let right = self.facing_right;
        if r.hit != (0, 0) && t == r.hit.0 {
            self.punch = Some(self.punch_box(aim));
            self.blow = self.blow_damage(w, aim);
        }
        let muzzle = (
            self.body.x + if right { r.dx.1 } else { r.dx.0 },
            self.body.y + r.dy,
        );
        let in_window = r.fire != (0, 0) && r.fire.0 <= t && t < r.fire.1 + wp.step;
        if let Some(kind) = wp.shot.filter(|_| in_window) {
            if self.inventory.available(w as usize) && self.muzzle_ok(world, w as usize, muzzle) {
                let slot = &mut self.inventory.slots[w as usize].1;
                *slot = (*slot - 1).max(0);
                let sign = if right { 1 } else { -1 };
                let (vx, vy) = (r.v.0 * sign, r.v.1);
                // 1ª e 3ª balas da pistola desviam 1 px/tique perpendicular (`0x0043bab7`):
                // (−vy, vx)/|v|, negado olhando para a esquerda
                let (px, py) = (-vy / 20 * sign, vx / 20 * sign);
                let n = t + 1 - r.fire.0;
                let (dx, dy) = match n {
                    1 if wp.spread => (px, py),
                    3 if wp.spread => (-px, -py),
                    _ => (0, 0),
                };
                self.shots.push(Shot {
                    x: muzzle.0,
                    y: muzzle.1,
                    vx: vx + dx,
                    vy: vy + dy,
                    kind,
                });
                if let (Some(ck), Some(c), 1) = (wp.casing, r.casing, n) {
                    self.shots.push(Shot {
                        x: self.body.x + if right { c.dx.1 } else { c.dx.0 },
                        y: self.body.y + c.dy,
                        vx: c.v.0 * sign,
                        vy: c.v.1,
                        kind: ck,
                    });
                }
            }
        }
        let moving = pressed(LEFT) || pressed(RIGHT) || pressed(JUMP);
        if t > r.again && pressed(SHOOT | PUNCH) {
            self.start_fire(input, world);
        } else if t > r.again && moving {
            self.set(State::Idle);
        } else if t > r.reeval {
            if r.recover != 0 {
                self.set(State::Recover(w));
            } else {
                self.set(State::Ready(0, w));
            }
        }
    }

    /// Vida e armadura iniciais (campos `fields[14]`/`[15]` do objeto).
    pub fn with_health(mut self, hp: i32, armor: i32) -> Self {
        self.health = Health { hp, armor };
        self
    }

    pub fn is_dead(&self) -> bool {
        matches!(self.state, State::Dying(_) | State::Dead) || self.health.is_dead()
    }

    /// Vivo e no jogo (não morrendo, saindo pela porta nem em viagem).
    pub fn is_active(&self) -> bool {
        !self.is_dead() && !matches!(self.state, State::EnteringExit(_) | State::Teleporting(_))
    }

    /// Em viagem de teletransporte.
    pub fn is_teleporting(&self) -> bool {
        matches!(self.state, State::Teleporting(_))
    }

    /// Desenhado (fica invisível durante o teletransporte).
    pub fn is_visible(&self) -> bool {
        !matches!(self.state, State::Teleporting(_))
    }

    /// Entra no teletransporte: some, e na metade da viagem anda `delta` px.
    pub fn start_teleport(&mut self, delta: (i32, i32)) {
        if self.is_dead() {
            return;
        }
        self.body.vx = 0;
        self.body.vy = 0;
        self.pending = Pending::default();
        self.teleport = Some(delta);
        self.set(State::Teleporting(0));
    }

    /// Parado em pé, podendo usar a saída.
    pub fn can_use_exit(&self) -> bool {
        self.state == State::Idle && self.body.on_ground
    }

    /// Soma dano pendente, aplicado no próximo tique.
    pub fn hurt(&mut self, p: Pending) {
        self.pending.add(&p);
    }

    /// Entra na porta de saída: estado 13, sem colisão; encolhe depois de 10 tiques.
    /// `scale_num` é o numerador atual da escala do sprite.
    pub fn enter_exit(&mut self, scale_num: i32) {
        self.body.vx = 0;
        self.body.vy = 0;
        self.exit_scale = Some(scale_num);
        self.set(State::EnteringExit(0));
    }

    /// Numerador da escala do sprite enquanto entra na porta.
    pub fn scale_override(&self) -> Option<i32> {
        match self.state {
            State::EnteringExit(_) => self.exit_scale,
            _ => None,
        }
    }

    /// Está num estado de tiro (a cena emite o som 3040 quando o rótulo muda).
    pub fn is_firing(&self) -> bool {
        matches!(self.state, State::Fire { .. })
    }

    /// O cadáver não avança a animação.
    fn controllable(&self) -> bool {
        !matches!(
            self.state,
            State::Pushed
                | State::Jolted
                | State::KnockedDown(_)
                | State::Shocked(_)
                | State::Dying(_)
                | State::Dead
                | State::EnteringExit(_)
                | State::Teleporting(_)
        )
    }

    /// Reações ao dano pendente (`0x004231f9`) e aplicação; devolve `true` se morreu agora.
    fn take_damage(&mut self, tick: u32) -> bool {
        let p = std::mem::take(&mut self.pending);
        if p.is_empty()
            || self.is_dead()
            || matches!(self.state, State::EnteringExit(_) | State::Teleporting(_))
        {
            return false;
        }
        let g = self.is_garathe();
        let layering = matches!(self.state, State::LayerUp(_) | State::LayerDown(_));
        // empurrão: Jake em 1/10/101; Garathe em 1, 54–56 e 58 (`0x428531`)
        let shovable = if g {
            matches!(
                self.state,
                State::Idle | State::Hide | State::Hidden | State::Unhide | State::Pushed
            )
        } else {
            matches!(self.state, State::Idle | State::Run | State::Push)
        };
        if p.knockdown && !(g && layering) {
            let right = p.push.0 >= 0;
            self.body.vx = p.push.0 * SUB;
            self.body.vy = p.push.1 * SUB;
            self.set(State::KnockedDown(right));
        } else if p.shove && shovable {
            self.body.vx = p.push.0 * SUB;
            self.set(State::Pushed);
        } else if g && p.jolt && moves::joltable(self.state) {
            // tranco (`0x42859e`): 59, ou queda na corda
            self.body.vx = 0;
            if matches!(self.state, State::Climb(_)) {
                self.set(State::Fall);
            } else {
                self.set(State::Jolted);
            }
        } else if p.electric && !matches!(self.state, State::Shocked(_)) && !(g && layering) {
            self.body.vx = 0;
            self.set(State::Shocked(SHOCK_TICKS));
            if g {
                self.frame = garathe::SHOCK_FRAME.min(self.len_of(121) - 1);
            }
        }
        if apply(&mut self.health, &p, tick) == 0 {
            self.death_label = match self.state {
                State::KnockedDown(right) if g => {
                    if garathe::knocked_label(right, self.facing_right) == 32 {
                        garathe::DIE_FORWARD
                    } else {
                        garathe::DIE_BACK
                    }
                }
                State::Land(Landing::Hard) if g => garathe::DIE_FORWARD,
                State::Rise if g => garathe::DIE_BACK,
                _ if g && p.poison => garathe::DIE_POISON,
                _ if g => garathe::DIE_DEFAULT,
                State::KnockedDown(true) => DIE_KNOCKED_RIGHT,
                State::KnockedDown(false) => DIE_KNOCKED_LEFT,
                _ if p.poison => DIE_POISON,
                _ => DIE_DEFAULT,
            };
            self.set(State::Dying(self.death_label));
            return true;
        }
        false
    }

    pub fn state(&self) -> State {
        self.state
    }
    pub fn label(&self) -> u16 {
        match self.state {
            State::Dead => self.death_label,
            State::Idle if self.switch_pose > 0 => self.weapon(self.inventory.weapon).ready,
            _ => self.anim_of(self.state).0,
        }
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
            self.frame = if self.reversed_state(s) {
                self.len_of(self.anim_of(s).0) - 1
            } else {
                0
            };
        }
    }

    /// Troca de estado mantendo o quadro (o 27 vira 16 "no mesmo quadro").
    fn set_keep_frame(&mut self, s: State) {
        let f = self.frame;
        self.set(s);
        self.frame = f.min(self.len_of(self.anim_of(s).0) - 1);
    }

    /// Número de quadros de um rótulo (ao menos 1).
    fn len_of(&self, label: u16) -> usize {
        self.lengths.get(&label).copied().unwrap_or(1).max(1)
    }

    /// A animação do estado chegou ao fim e avançaria neste tique (o modo "parar no fim" de
    /// `0x00406c71` devolve 1).
    fn anim_done(&self) -> bool {
        let (label, c) = self.anim_of(self.state);
        if self.label_changed || self.ticks % (c + 1) != 0 {
            return false;
        }
        if self.reversed_state(self.state) {
            self.frame == 0
        } else {
            self.frame + 1 >= self.len_of(label)
        }
    }

    /// Avança o quadro conforme o modo pedido neste tique (antes da física, como o original,
    /// que anima e depois pede a velocidade do quadro).
    fn animate(&mut self) {
        let (label, c) = self.anim_of(self.state);
        let n = self.len_of(label);
        self.advance = false;
        self.frame = self.frame.min(n - 1);
        if self.label_changed || self.ticks % (c + 1) != 0 {
            return;
        }
        let before = self.frame;
        let mode = match self.anim_mode {
            AnimMode::Auto if self.reversed_state(self.state) => AnimMode::Reverse,
            m => m,
        };
        self.frame = match mode {
            AnimMode::Hold => before,
            AnimMode::Auto => (before + 1) % n,
            AnimMode::Reverse => (before + n - 1) % n,
            AnimMode::Double => (before + 2).min(n - 1),
            AnimMode::Target(t) => t.min(n - 1),
        };
        self.advance = self.frame != before;
    }

    /// Velocidade pedida pelo esqueleto (`0x00406fc0`): k · Δ da raiz no quadro corrente.
    fn apply_ske(&mut self) {
        let (Some(req), Some(sk)) = (self.ske_req, self.skel.as_ref()) else {
            return;
        };
        let back = self.reversed_state(self.state) || self.anim_mode == AnimMode::Reverse;
        let (mut dx, dy) = sk.delta(
            self.anim_of(self.state).0,
            self.frame,
            back,
            self.facing_right,
        );
        if self.mirrored(self.state) {
            dx = -dx;
        }
        if req.x {
            self.body.vx = req.k * dx * SUB;
        }
        if req.y {
            self.body.vy = req.k * dy * SUB;
        }
        if req.diag != 0 {
            self.body.vx = req.diag * self.body.vy;
        }
    }

    /// Deslocamento do desenho em px (`+0x16c/+0x170`, `0x00406ea2` sem o bit 0x10): o
    /// sprite vai a `(x − 0x16c, y − 0x170)`. Somar ao corpo para desenhar.
    pub fn draw_offset(&self) -> (i32, i32) {
        let Some(sk) = self.skel.as_ref() else {
            return (0, 0);
        };
        let (label, frame) = match self.state {
            State::Dead => return (0, 0),
            _ => (self.label(), self.frame),
        };
        let (mut ox, oy) = sk.offset(label, frame, self.facing_right);
        if self.mirrored(self.state) {
            ox = -ox;
        }
        (-ox, -oy)
    }

    /// O sprite olha para a direita (a arte olha para a esquerda; os estados de arte
    /// espelhada, como o 149, olham para o outro lado: `0x00422fce`).
    pub fn sprite_mirrored(&self) -> bool {
        self.facing_right != self.mirrored(self.state)
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
        let p = self.profile();
        if ahead(p.probe_medium) {
            JumpKind::Medium
        } else if ahead(p.probe_long) {
            JumpKind::Long
        } else if p.probe_extended.is_some_and(ahead) {
            JumpKind::Extended
        } else {
            JumpKind::Medium
        }
    }

    fn launch(&mut self, kind: JumpKind) {
        let (vx, vy) = match kind {
            JumpKind::Medium => self.profile().medium,
            JumpKind::Long => self.profile().long,
            JumpKind::Extended => self.profile().extended.unwrap_or(self.profile().long),
        };
        self.body.vx = vx * SUB * self.dir();
        self.body.vy = vy * SUB;
        self.set(State::Jump(kind));
    }

    /// O centro da caixa saiu da fase (`0x00404716`): o original repõe o jogador na posição
    /// inicial, com velocidade zero (`0x00404760`).
    pub fn respawn_if_outside(&mut self, level: (i32, i32)) -> bool {
        if self.is_dead() {
            return false;
        }
        let b = self.body.bounds();
        let (cx, cy) = (b.x.saturating_add(b.w / 2), b.y.saturating_add(b.h / 2));
        if cx < 0 || cy < 0 || cx > level.0 || cy > level.1 {
            self.body = Body::new(self.start.0, self.start.1, self.body.boxr);
            self.set(State::Idle);
            return true;
        }
        false
    }

    /// Um tique (50 ms) com a máscara de entrada `input`, sem zonas PR (só cenário).
    pub fn tick(&mut self, input: u32, world: &World) {
        self.tick_in(input, &Env::new(world));
    }

    /// Um tique com o ambiente completo: sólidos, zonas PR e número de camadas.
    pub fn tick_in(&mut self, input: u32, env: &Env) {
        let world = env.world;
        self.label_changed = false;
        self.no_ammo = false;
        self.special = None;
        self.punch = None;
        self.blow = None;
        self.switched = false;
        self.door_push = false;
        self.sound = None;
        self.anim_mode = AnimMode::Auto;
        self.ske_req = None;
        if self.state == State::Dead {
            self.advance = false;
            return;
        }
        self.clock = self.clock.wrapping_add(1);
        // veneno (`0x004231f9`, passo 3): a cada 20 tiques, `+0x270` de dano de veneno
        if self.poison > 0 && self.clock % POISON_PERIOD == 0 && self.is_active() {
            self.pending.add(&Pending {
                amount: self.poison,
                poison: true,
                ..Default::default()
            });
        }
        self.take_damage(self.ticks);
        if !self.controllable() {
            self.tick_passive(input, env);
            return;
        }
        self.switch_pose = self.switch_pose.saturating_sub(1);
        self.switch_weapon(input);
        let pressed = |bit: u32| input & bit != 0;
        let jump_edge = pressed(JUMP) && self.prev_input & JUMP == 0;
        let side = self.wanted_side(input);
        let airborne = matches!(self.state, State::Jump(_) | State::Fall | State::WildFall);
        if let (false, Some(right)) = (airborne, side) {
            self.facing_right = right;
        }
        let moving = side.is_some();
        let shooting = pressed(SHOOT | PUNCH) && self.body.on_ground;
        let can_fire = matches!(
            self.state,
            State::Idle
                | State::Run
                | State::Push
                | State::Land(_)
                | State::Stealth
                | State::GetUp
                | State::Shake
                | State::Recover(_)
                | State::Ready(..)
                | State::Holster(_)
                | State::Crouch
                | State::StandUp
                | State::Hide
                | State::Hidden
                | State::Unhide
                | State::Taunt
        ) || (self.state == State::Crawl
            && self.free_above(world, self.profile().headroom));
        let fired = !matches!(self.state, State::Fire { .. })
            && can_fire
            && shooting
            && self.start_fire(input, world);
        match self.state {
            _ if fired => {}
            State::Fire { aim, t, w, alt } => self.tick_fire(input, world, (aim, t, w, alt)),
            State::Recover(_) | State::Ready(..) | State::Holster(_) => {
                if moving || jump_edge || pressed(UP) || pressed(DOWN) {
                    self.set(State::Idle);
                } else {
                    match self.state {
                        State::Recover(w) if self.anim_done() => self.set(State::Ready(0, w)),
                        State::Ready(n, w) if n + 1 >= READY_TICKS => self.set(State::Holster(w)),
                        State::Ready(n, w) => self.state = State::Ready(n + 1, w),
                        State::Holster(_) if self.anim_done() => self.set(State::Idle),
                        _ => {}
                    }
                }
                self.body.vx = 0;
            }
            State::Idle | State::Run | State::Push => {
                self.decide_standing(input, env, jump_edge, side);
            }
            State::Taunt => {
                // outra entrada passa pela decisão em pé; sem decisão, a pose toca até o fim
                self.body.vx = 0;
                if input != TAUNT_INPUT && self.decide(input, env, jump_edge, side) {
                } else if self.anim_done() {
                    self.set(State::Idle);
                }
            }
            State::RunUp if self.is_garathe() => {
                // preparo 24 (uma vez) → pulo longo 25 (`0x429a0b`)
                self.body.vx = 0;
                if self.anim_done() {
                    self.launch(JumpKind::Long);
                }
            }
            State::RunUp => {
                // Impulso (`0x00425382`): vx pelo esqueleto com k = 2, mantendo a vertical;
                // no fim, o pulo estendido sai de onde o impulso levou o Jake.
                if self.anim_done() {
                    self.launch(JumpKind::Extended);
                } else {
                    self.ske_req = Some(SkeReq {
                        k: 2,
                        x: true,
                        y: false,
                        diag: 0,
                    });
                }
            }
            State::Jump(kind) => {
                if self.try_grab_in_air(input, env) {
                } else if kind == JumpKind::Extended && self.anim_done() {
                    // fim do 80 ainda no ar → queda descontrolada (76)
                    self.set(State::WildFall);
                } else {
                    if self.is_garathe() && self.anim_done() {
                        // 22/25: a animação toca uma vez e para no último quadro
                        self.anim_mode = AnimMode::Hold;
                    }
                    self.air_reverse(side);
                }
            }
            State::Fall | State::WildFall => {
                if !self.try_grab_in_air(input, env) {
                    self.air_reverse(side);
                }
            }
            _ => self.tick_move(input, env, jump_edge, side),
        }
        let (push_label, push_c) = self.anim_of(State::Push);
        self.door_push = self.state == State::Push
            && self.ticks + 1 == self.len_of(push_label) as u32 * (push_c + 1);
        if self.door_push && self.is_garathe() {
            self.sound = Some(garathe::SOUND_PUSH);
        }
        // a caixa do estado vale já para a física deste tique (o BRD troca com o estado)
        self.fit_box(world);
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
        self.animate();
        self.apply_ske();
        let was_rising = self.body.vy < 0;
        let gravity = self.gravity_on();
        step(&mut self.body, world, gravity);
        match self.state {
            State::Jump(kind) if self.is_garathe() && self.body.hit_wall => {
                // 22/25 contra a parede: fala e queda (`0x429e46`, `0x429c6c`)
                self.sound = Some(if kind == JumpKind::Long {
                    garathe::SPEECH_WALL_LONG
                } else {
                    garathe::SPEECH_WALL_MEDIUM
                });
                self.body.vx = 0;
                self.set(State::Fall);
            }
            State::Jump(_) | State::Fall | State::WildFall
                if self.body.on_ground && !was_rising =>
            {
                self.body.vx = 0;
                self.body.vy = 0;
                let landing = match self.state {
                    State::Jump(JumpKind::Medium) => Landing::Medium,
                    State::Jump(JumpKind::Long) => Landing::Long,
                    State::Fall => Landing::Fall,
                    _ => Landing::Hard,
                };
                self.set(State::Land(landing));
                if landing == Landing::Long && self.is_garathe() {
                    self.sound = Some(garathe::SOUND_LONG_LANDING);
                }
            }
            State::JumpUp(_) if self.body.hit_ceiling && self.ceiling_is_bar(world) => {
                self.body.vx = 0;
                self.body.vy = 0;
                self.set(State::BarHang);
            }
            s if moves::can_fall(s) && !self.body.on_ground && self.body.vy >= FALL_SPEED => {
                self.set(State::Fall)
            }
            _ => {}
        }
        self.fit_box(world);
        self.finish_tick();
        self.prev_input = input;
    }

    /// No ar, o lado oposto inverte a velocidade horizontal.
    fn air_reverse(&mut self, side: Option<bool>) {
        let opposite = match side {
            Some(right) => (right && self.body.vx < 0) || (!right && self.body.vx > 0),
            None => false,
        };
        if opposite {
            self.body.vx = -self.body.vx;
            self.facing_right = self.body.vx > 0;
        }
    }

    fn finish_tick(&mut self) {
        self.ticks = self.ticks.saturating_add(1);
    }

    /// Tique dos estados sem entrada do jogador.
    fn tick_passive(&mut self, input: u32, env: &Env) {
        let world = env.world;
        self.fit_box(world);
        match self.state {
            State::EnteringExit(n) => {
                if n >= 10 {
                    if let Some(sc) = self.exit_scale.as_mut() {
                        *sc = (*sc - 1).max(EXIT_MIN_SCALE);
                    }
                }
                self.state = State::EnteringExit(n + 1);
                self.animate();
                self.finish_tick();
                return;
            }
            State::Teleporting(n) => {
                if n + 1 == TELEPORT_MOVE {
                    if let Some((dx, dy)) = self.teleport.take() {
                        self.body.x = self.body.x.saturating_add(dx);
                        self.body.y = self.body.y.saturating_add(dy);
                    }
                }
                self.body.vx = 0;
                self.body.vy = 0;
                if n + 1 >= TELEPORT_TICKS {
                    self.set(State::Idle);
                } else {
                    self.state = State::Teleporting(n + 1);
                }
            }
            State::Shocked(n) => {
                self.body.vx = 0;
                step(&mut self.body, world, true);
                if n <= 1 {
                    self.set(State::Idle);
                } else {
                    self.state = State::Shocked(n - 1);
                }
            }
            State::Jolted => {
                // 59: desliza pelo esqueleto (k = 2, só x), depois levanta (35)
                if self.anim_done() {
                    self.body.vx = 0;
                    self.set(State::Rise);
                } else {
                    self.animate();
                    self.ske_req = Some(SkeReq {
                        k: 2,
                        x: true,
                        y: false,
                        diag: 0,
                    });
                    self.apply_ske();
                    step(&mut self.body, world, true);
                    self.fit_box(world);
                    self.finish_tick();
                    return;
                }
            }
            State::Pushed => {
                step(&mut self.body, world, true);
                if self.anim_done() {
                    self.body.vx = 0;
                    self.set(State::Idle);
                }
            }
            State::KnockedDown(_) => {
                // arremessado (81/82): agarra a corda como nos pulos (`jake-escalada.md`)
                if self.try_grab_in_air(input, env) {
                    self.fit_box(world);
                    self.animate();
                    self.finish_tick();
                    return;
                }
                let was_rising = self.body.vy < 0;
                step(&mut self.body, world, true);
                if self.body.on_ground && !was_rising {
                    self.body.vx = 0;
                    self.body.vy = 0;
                    let next = match self.state {
                        // 32 → 30 no chão; 33 → 35 (`0x42a115`, `0x42a1b3`)
                        State::KnockedDown(right) if self.is_garathe() => {
                            if garathe::knocked_label(right, self.facing_right) == 32 {
                                State::Land(Landing::Hard)
                            } else {
                                State::Rise
                            }
                        }
                        _ => State::Land(Landing::Fall),
                    };
                    self.set(next);
                }
            }
            State::Dying(_) => {
                step(&mut self.body, world, true);
                if self.body.on_ground {
                    self.body.vx = 0;
                }
                if self.body.on_ground && self.anim_done() {
                    self.set(State::Dead);
                }
            }
            _ => {}
        }
        self.fit_box(world);
        self.animate();
        self.finish_tick();
    }

    /// Rótulo em que o cadáver ficou.
    pub fn death_label(&self) -> u16 {
        self.death_label
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{JUMP, LEFT, RIGHT, SHOOT};
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
            (168, 8),
            (81, 10),
            (82, 10),
            (15, 4),
            (173, 16),
            (77, 13),
            (171, 16),
            (12, 20),
            (23, 6),
            (56, 6),
            (54, 6),
            (55, 6),
            (22, 4),
            (18, 3),
            (19, 9),
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
    fn leaving_the_level_respawns_at_start() {
        let w = World::new(vec![]);
        let mut j = jake_on(&w, 100);
        for _ in 0..30 {
            j.tick(0, &w);
        }
        assert!(j.body.y > 300);
        assert!(j.respawn_if_outside((2000, 300)));
        assert_eq!((j.body.x, j.body.y, j.body.vy), (100, 53, 0));
        assert_eq!(j.state(), State::Idle);
        assert!(!j.respawn_if_outside((2000, 300)));
    }

    #[test]
    fn damage_reactions_and_death() {
        let w = long_floor();
        // choque: 10 tiques parado, sem entrada
        let mut j = jake_on(&w, 0).with_health(50, 0);
        j.hurt(Pending {
            amount: 20,
            electric: true,
            ..Default::default()
        });
        j.tick(RIGHT, &w);
        assert_eq!(
            (j.state(), j.label(), j.health.hp),
            (State::Shocked(9), 15, 30)
        );
        let x = j.body.x;
        for _ in 0..8 {
            j.tick(RIGHT, &w);
        }
        assert_eq!(j.state(), State::Shocked(1));
        j.tick(0, &w);
        assert_eq!((j.state(), j.body.x), (State::Idle, x));
        // derrubada para a direita: voa e aterrissa
        j.hurt(Pending {
            amount: 5,
            explosion: true,
            push: (8, -8),
            knockdown: true,
            ..Default::default()
        });
        j.tick(0, &w);
        assert_eq!((j.state(), j.label()), (State::KnockedDown(true), 81));
        assert!(j.body.x > x && j.body.vy < 0);
        for _ in 0..40 {
            j.tick(0, &w);
        }
        assert_eq!(j.state(), State::Idle);
        assert_eq!(j.health.hp, 25);
        // morte: animação, depois cadáver que ignora a entrada
        j.hurt(Pending {
            amount: 99,
            ..Default::default()
        });
        j.tick(LEFT, &w);
        assert_eq!((j.state(), j.label()), (State::Dying(173), 173));
        assert!(j.is_dead());
        let x = j.body.x;
        for _ in 0..40 {
            j.tick(LEFT, &w);
        }
        assert_eq!((j.state(), j.label(), j.body.x), (State::Dead, 173, x));
        assert!(!j.advance());
        // derrubado e morto na mesma pancada: rótulo 177/179 pelo lado
        let mut j = jake_on(&w, 0).with_health(1, 0);
        j.hurt(Pending {
            amount: 9,
            push: (-8, -8),
            knockdown: true,
            ..Default::default()
        });
        j.tick(0, &w);
        assert_eq!(j.state(), State::Dying(DIE_KNOCKED_LEFT));
    }

    #[test]
    fn dead_jake_outside_the_level_stays_dead() {
        let w = World::new(vec![]);
        let mut j = jake_on(&w, 100).with_health(1, 0);
        j.hurt(Pending {
            amount: 5,
            ..Default::default()
        });
        for _ in 0..40 {
            j.tick(0, &w);
        }
        assert!(j.is_dead() && j.body.y > 300);
        assert!(!j.respawn_if_outside((2000, 300)));
        assert!(j.is_dead());
        let x = j.body.x;
        for _ in 0..20 {
            j.tick(LEFT, &w);
        }
        assert_eq!(j.body.x, x);
    }

    fn pistol() -> Inventory {
        Inventory::from_points(&[(0, 0), (1, 20), (1, 80)])
    }

    #[test]
    fn aim_from_input_bits() {
        use crate::input::{DOWN, UP};
        assert_eq!(Aim::from_input(0), Aim::Forward);
        assert_eq!(Aim::from_input(UP | DOWN), Aim::Forward);
        assert_eq!(Aim::from_input(LEFT | RIGHT), Aim::Forward);
        assert_eq!(Aim::from_input(UP), Aim::Up);
        assert_eq!(Aim::from_input(UP | LEFT | RIGHT), Aim::Up);
        assert_eq!(Aim::from_input(RIGHT), Aim::Forward);
        assert_eq!(Aim::from_input(UP | RIGHT), Aim::DiagUp);
        assert_eq!(Aim::from_input(DOWN | LEFT), Aim::DiagDown);
        assert_eq!(Aim::from_input(DOWN), Aim::DiagDown);
        let r = |a: Aim| weapons::PISTOL_W.states[weapons::aim_slot(a)][0];
        assert_eq!((r(Aim::Forward).0, r(Aim::Forward).1.dx.1), (23, 25));
        assert_eq!(r(Aim::Up).1.v, (0, -20));
    }

    #[test]
    fn burst_of_three_then_repeat_every_six_ticks() {
        let w = long_floor();
        let mut j = jake_on(&w, 0).with_inventory(pistol());
        assert_eq!(j.inventory.pistol_ammo(), 80);
        let mut fired = vec![];
        for t in 0..20 {
            j.tick(SHOOT, &w);
            let bullets = j.shots.iter().filter(|s| !s.is_casing()).count();
            if bullets > 0 {
                fired.push((t, bullets, j.shots.iter().any(|s| s.is_casing())));
            }
            j.shots.clear();
        }
        // tique 0 entra no estado (t = 0); dispara em t = 1, 2, 3; recomeça em t = 7
        assert_eq!(
            fired,
            vec![
                (1, 1, true),
                (2, 1, false),
                (3, 1, false),
                (8, 1, true),
                (9, 1, false),
                (10, 1, false),
                (15, 1, true),
                (16, 1, false),
                (17, 1, false),
            ]
        );
        assert_eq!(j.inventory.pistol_ammo(), 71);
        assert!(j.is_firing());
        assert_eq!(j.label(), 23);
    }

    #[test]
    fn shots_carry_muzzle_velocity_and_spread() {
        let w = long_floor();
        let mut j = jake_on(&w, 0).with_inventory(pistol());
        j.tick(RIGHT, &w); // olha para a direita
        j.tick(SHOOT, &w);
        let (x, y) = (j.body.x, j.body.y);
        let mut rounds = vec![];
        for _ in 0..3 {
            j.tick(SHOOT, &w);
            rounds.extend(j.shots.drain(..).filter(|s| !s.is_casing()));
        }
        assert_eq!(rounds.len(), 3);
        assert_eq!((rounds[0].x, rounds[0].y), (x + 25, y + 2));
        assert_eq!((rounds[0].vx, rounds[0].vy), (20, 1)); // 1ª desce
        assert_eq!((rounds[1].vx, rounds[1].vy), (20, 0));
        assert_eq!((rounds[2].vx, rounds[2].vy), (20, -1)); // 3ª sobe
    }

    #[test]
    fn muzzle_inside_a_wall_fires_nothing() {
        // parede logo à direita do Jake: a boca (x + 25) fica dentro dela
        let w = World::new(vec![
            Rect {
                x: -1000,
                y: 100,
                w: 2000,
                h: 20,
            },
            Rect {
                x: 24,
                y: -100,
                w: 20,
                h: 200,
            },
        ]);
        let mut j = jake_on(&w, 0).with_inventory(pistol());
        j.tick(RIGHT, &w);
        for _ in 0..5 {
            j.tick(SHOOT, &w);
        }
        assert!(j.shots.iter().all(|s| s.is_casing()), "{:?}", j.shots);
        assert_eq!(j.inventory.pistol_ammo(), 80, "não gasta munição");
    }

    #[test]
    fn release_recovers_then_ready_then_holsters() {
        let w = long_floor();
        let mut j = jake_on(&w, 0).with_inventory(pistol());
        j.tick(SHOOT, &w);
        let mut labels = vec![j.label()];
        for _ in 0..40 {
            j.tick(0, &w);
            if labels.last() != Some(&j.label()) {
                labels.push(j.label());
            }
        }
        assert_eq!(labels, vec![23, 22, 18, 19, 1]);
        assert_eq!(j.state(), State::Idle);
    }

    #[test]
    fn running_stops_to_shoot_and_no_ammo_does_not_fire() {
        let w = long_floor();
        let mut j = jake_on(&w, 0).with_inventory(pistol());
        j.tick(RIGHT, &w);
        j.tick(RIGHT, &w);
        assert_eq!(j.state(), State::Run);
        let x = j.body.x;
        j.tick(RIGHT | SHOOT, &w);
        assert!(j.is_firing());
        j.tick(RIGHT | SHOOT, &w);
        assert_eq!(j.body.x, x, "parou para atirar");
        // sem munição na pistola (só ela e os punhos): aviso, troca para os punhos e soca
        let mut empty =
            jake_on(&w, 0).with_inventory(Inventory::from_points(&[(0, 0), (1, 20), (1, 0)]));
        empty.inventory.weapon = PISTOL;
        empty.tick(SHOOT | RIGHT, &w);
        assert!(empty.no_ammo && empty.shots.is_empty());
        assert_eq!(empty.inventory.weapon, FISTS);
        assert_eq!(empty.label(), 146, "pose \"pronto\" dos punhos");
        empty.tick(SHOOT, &w);
        assert!(!empty.no_ammo, "aviso repetido");
        assert!(empty.is_firing());
        assert!(matches!(empty.label(), 150 | 151), "{}", empty.label());
        // acabou a munição no meio: passa aos punhos
        let mut j =
            jake_on(&w, 0).with_inventory(Inventory::from_points(&[(0, 0), (1, 20), (1, 2)]));
        for _ in 0..30 {
            j.tick(SHOOT, &w);
        }
        assert_eq!(j.inventory.pistol_ammo(), 0);
        assert_eq!(j.inventory.weapon, FISTS);
    }

    #[test]
    fn entering_the_exit_shrinks_after_ten_ticks() {
        let w = long_floor();
        let mut j = jake_on(&w, 0);
        j.enter_exit(47);
        assert_eq!((j.state(), j.label()), (State::EnteringExit(0), 12));
        for _ in 0..10 {
            j.tick(LEFT, &w);
        }
        assert_eq!(j.scale_override(), Some(47));
        j.tick(0, &w);
        assert_eq!(j.scale_override(), Some(46));
        for _ in 0..100 {
            j.tick(0, &w);
        }
        assert_eq!(j.scale_override(), Some(30));
        assert!(!j.is_active());
    }

    #[test]
    fn teleport_hides_moves_halfway_and_ignores_input_and_damage() {
        let w = long_floor();
        let mut j = jake_on(&w, 0);
        let (x0, y0) = (j.body.x, j.body.y);
        j.start_teleport((68, -256));
        assert!(!j.is_visible() && !j.is_active());
        for t in 1..TELEPORT_TICKS {
            j.hurt(Pending {
                amount: 10,
                ..Default::default()
            });
            j.tick(RIGHT | JUMP, &w);
            let moved = (j.body.x - x0, j.body.y - y0);
            if t < TELEPORT_MOVE {
                assert_eq!(moved, (0, 0), "tique {t}");
            } else {
                assert_eq!(moved, (68, -256), "tique {t}");
            }
        }
        j.tick(0, &w);
        assert_eq!(j.state(), State::Idle);
        assert!(j.is_visible());
        assert_eq!(j.health.hp, 100, "invulnerável na viagem");
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

    fn arsenal() -> Inventory {
        // punhos, pistola 10, espingarda 5, foguetes 3, estrelas 4, granadas 2
        Inventory::from_points(&[(0, 0), (1, 20), (1, 10), (1, 5), (1, 3), (1, 4), (1, 2)])
    }

    #[test]
    fn best_weapon_and_cycling_follow_the_list() {
        use crate::input::{NEXT_WEAPON, PREV_WEAPON, SELECT_SHIFT};
        let mut inv = arsenal();
        assert_eq!(inv.weapon, ROCKETS, "melhor: foguetes");
        // lista 1, 6, 5, 2, 3, 4: a seguinte dos foguetes dá a volta aos punhos
        assert_eq!(inv.cycle(true), FISTS);
        assert_eq!(inv.cycle(false), SHOTGUN);
        inv.slots[SHOTGUN].1 = 0;
        assert_eq!(inv.cycle(false), PISTOL, "pula a espingarda vazia");
        let inv = Inventory::from_points(&[(0, 0), (1, 20), (0, 90)]);
        assert_eq!((inv.weapon, inv.cycle(true)), (FISTS, FISTS));
        // ganhar uma arma melhor que a melhor disponível a torna corrente (`0x43af6c` roda
        // antes de a arma entrar no inventário)
        let mut inv = Inventory::from_points(&[(0, 0), (1, 20), (1, 10)]);
        inv.gained(GRENADES);
        assert_eq!(inv.weapon, PISTOL);
        inv.gained(SHOTGUN);
        assert_eq!(inv.weapon, SHOTGUN);
        // no Jake: PRÓXIMA na borda troca, com aviso e pose "pronto"; segurar não repete
        let w = long_floor();
        let mut j = jake_on(&w, 0).with_inventory(arsenal());
        j.tick(PREV_WEAPON, &w);
        assert!(j.switched);
        assert_eq!((j.inventory.weapon, j.label()), (SHOTGUN, 108));
        j.tick(PREV_WEAPON, &w);
        assert!(!j.switched);
        assert_eq!(j.inventory.weapon, SHOTGUN);
        j.tick(NEXT_WEAPON, &w);
        assert_eq!(j.inventory.weapon, ROCKETS);
        // escolha direta (extensão do port): 5 → estrelas
        j.tick((STARS as u32) << SELECT_SHIFT, &w);
        assert_eq!(j.inventory.weapon, STARS);
        for _ in 0..8 {
            j.tick(0, &w);
        }
        assert_eq!(j.label(), 1, "a pose da troca dura 7 tiques");
        // SELECT = 7 cai na primeira bandeira (par 7) e não é arma: ignorado
        j.inventory.slots[7] = (1, 1);
        j.tick(7 << SELECT_SHIFT, &w);
        assert_eq!(j.inventory.weapon, STARS, "o par 7 é bandeira, não arma");
    }

    /// Disparos de cada arma com ATIRAR seguro por 30 tiques: (tiques com tiro, tipos).
    fn fire_log(w: usize, ammo: i32) -> (Vec<u32>, Vec<ShotKind>, i32) {
        let world = long_floor();
        let mut inv = Inventory::from_points(&[(0, 0), (1, 20)]);
        inv.slots[w] = (1, ammo);
        inv.weapon = w;
        let mut j = jake_on(&world, 0).with_inventory(inv);
        let (mut ticks, mut kinds) = (vec![], vec![]);
        for t in 0..30 {
            j.tick(SHOOT, &world);
            for s in j.shots.drain(..) {
                if !s.is_casing() {
                    ticks.push(t);
                }
                kinds.push(s.kind);
            }
        }
        (ticks, kinds, j.inventory.slots[w].1)
    }

    #[test]
    fn each_weapon_has_its_cadence_and_projectile() {
        // espingarda: um tiro a cada 9 tiques (t > 8 recomeça), com o cartucho
        let (t, k, left) = fire_log(SHOTGUN, 20);
        assert_eq!(t, vec![1, 10, 19, 28]);
        assert_eq!(&k[..2], &[ShotKind::Pellet, ShotKind::Shell]);
        assert_eq!(left, 16);
        // foguetes: a cada 25
        let (t, k, _) = fire_log(ROCKETS, 20);
        assert_eq!(t, vec![1, 26]);
        assert!(k.iter().all(|&k| k == ShotKind::Rocket));
        // estrelas e granadas: passo 2, dispara em t = 8, recomeça com t > 11 (6 tiques)
        let (t, k, _) = fire_log(STARS, 20);
        assert_eq!(t, vec![4, 10, 16, 22, 28]);
        assert!(k.iter().all(|&k| k == ShotKind::Star));
        let (t, _, left) = fire_log(GRENADES, 2);
        assert_eq!(&t[..2], &[4, 10]);
        assert_eq!(left, 0, "munição nunca negativa");
    }

    #[test]
    fn rockets_aim_up_diagonally_and_velocities_follow_the_side() {
        let w = long_floor();
        let mut inv = Inventory::from_points(&[(0, 0), (1, 20)]);
        inv.slots[ROCKETS] = (1, 5);
        inv.weapon = ROCKETS;
        let mut j = jake_on(&w, 0).with_inventory(inv);
        j.tick(UP | SHOOT, &w);
        assert_eq!(j.label(), 67);
        j.tick(UP | SHOOT, &w);
        let s = j.shots[0];
        assert_eq!((s.vx, s.vy), (-30, -30), "olhando à esquerda");
        assert_eq!((s.x - j.body.x, s.y - j.body.y), (-20, -23));
    }

    #[test]
    fn punch_strikes_once_per_blow_in_front() {
        use crate::input::PUNCH;
        let w = long_floor();
        let mut j = jake_on(&w, 0).with_inventory(arsenal());
        j.tick(RIGHT, &w);
        let mut strikes = vec![];
        for t in 0..12 {
            j.tick(PUNCH, &w);
            if let Some(r) = j.punch {
                strikes.push((t, r));
            }
        }
        assert!(j.shots.is_empty(), "o soco não dispara");
        assert_eq!(j.inventory.slots[ROCKETS].1, 3, "nem gasta munição");
        assert!(!strikes.is_empty());
        let b = j.body.bounds();
        let (_, r) = strikes[0];
        assert_eq!(
            (r.x, r.w, r.h),
            (b.x + b.w / 2, b.w / 2 + PUNCH_REACH, b.h / 3)
        );
        assert_eq!(r.y, b.y + b.h / 3, "terço do meio");
        // um acerto por golpe: tiques distintos e espaçados
        assert!(strikes.windows(2).all(|p| p[1].0 > p[0].0 + 1));
    }

    #[test]
    fn four_directions_taunt_then_stand() {
        let w = long_floor();
        let mut j = jake_on(&w, 0);
        j.tick(TAUNT_INPUT, &w);
        assert_eq!(
            (j.state(), j.label()),
            (State::Taunt, 18),
            "estado 4, rótulo 18"
        );
        assert_eq!(j.body.vx, 0);
        // sem entrada a pose toca até o fim (3 quadros) e volta a parado
        let mut n = 0;
        while j.state() == State::Taunt && n < 10 {
            j.tick(0, &w);
            n += 1;
        }
        assert_eq!((j.state(), n), (State::Idle, 3));
        // outra entrada passa pela decisão em pé
        j.tick(TAUNT_INPUT, &w);
        j.tick(RIGHT, &w);
        assert_eq!(j.state(), State::Run);
        j.tick(TAUNT_INPUT, &w);
        assert_eq!(j.state(), State::Taunt, "da corrida também");
        j.tick(SHOOT, &w);
        assert!(j.is_firing(), "atira da pose");
    }
}
