// SPDX-License-Identifier: GPL-3.0-or-later
//! Máquina de estados do Jake: parado, correr, pulos e quedas (M3a, `docs/re/personagem.md`)
//! e, na M6b, o deslocamento pelo esqueleto (`docs/re/ske.md`), cordas, camadas, esconder,
//! agachar, barra e o especial (`docs/re/jake-movimentos.md`, `jake-escalada.md`,
//! `escada-camadas.md`, `furtividade.md`). Os estados novos estão em `jake/moves.rs`.

pub mod garathe;
mod moves;
pub mod weapons;

use std::collections::HashMap;

pub use moves::{
    BAR_BOX, CRAWL_BOX, Env, PlayerHit, Skel, SpecialBlast, SpecialEffect, Zone, zone,
};

pub use crate::character::Character;
use crate::character::Profile;
use crate::damage::{Health, Pending, Strike, apply};
use crate::input::{DOWN, JUMP, LEFT, NEXT_WEAPON, PREV_WEAPON, PUNCH, RIGHT, SHOOT, STEALTH, UP};
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
    /// Derrubado em voo (estados 81/82, `0x004235fc`). Jake: `true` = 81, empurrado para o
    /// lado em que olha (`+0x250 |= 8`); `false` = 82, para trás (`0x10`). Garathe: `true` =
    /// jogado para a direita (o rótulo sai de `garathe::knocked_label`).
    KnockedDown(bool),
    /// Só o Jake: levantar da derrubada para trás (83, depois 84; `0x425b99`/`0x425bdf`).
    KnockRise(bool),
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
    /// Só o Garathe: cambaleando no terremoto (121 `MNQUAKE` do quadro 0, `0x42bd73`)
    /// enquanto dura o contador `+0x274` (`0x42bcf8`).
    Quake,
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
    /// Desvio da rajada da pistola somado à velocidade já criada (`0x0043bab7`: `+0x164`/
    /// `+0x168`), em 1/8 px por tique.
    pub nudge: (i32, i32),
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
/// Trava do tiro ao sair dele soltando ATIRAR com uma direção, pulo ou troca (`0x43b5a4`).
const FIRE_LOCK_RELEASE: u32 = 8;

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
pub const ALL_LABELS: [u16; 50] = [
    83,
    84,
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
        State::KnockRise(false) => (83, 0),
        State::KnockRise(true) => (84, 0),
        State::Shocked(_) => (15, 0),
        State::Dying(label) => (label, 0),
        State::Dead => (0, 0),
        State::EnteringExit(_) => (12, 0),
        State::Fire { aim, w, alt, .. } => (
            weapon(w as usize).states[weapons::aim_slot(aim)][usize::from(alt)].0,
            0,
        ),
        State::Recover(w) => pose(weapon(w as usize).states[2][0].1.recover),
        State::Ready(_, w) => pose(weapon(w as usize).ready),
        State::Holster(w) => pose(weapon(w as usize).holster),
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
        // o Jake não entra aqui (o terremoto dele é o da cena)
        State::Quake => (15, 0),
    }
}

/// Estados que tocam o rótulo de trás para a frente (`0x00423894`).
fn jake_reversed(s: State) -> bool {
    matches!(
        s,
        State::LetGo | State::StandUp | State::Unhide | State::BarDown | State::Holster(_)
    )
}

/// Rótulo de pose de arma (pronto, guardar, recuperação) e o atraso de quadro da tabela
/// `0x473300` (`+0x18`): 1 nas recuperações 73/112/131/188, guardar 109/128/147/187 e no
/// "pronto" dos punhos 146 (oráculo: o 112 anda um quadro a cada 2 tiques).
fn pose(label: u16) -> (u16, u32) {
    let slow = matches!(label, 73 | 112 | 131 | 188 | 109 | 128 | 147 | 187 | 146);
    (label, u32::from(slow))
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
    /// `n` quadros por tique, sem passar do último (estados de arma: `0x423835` uma vez por
    /// passo, no modo 0xa que para no fim).
    Step(usize),
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
    teleport: Option<((i32, i32), i32)>,
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
    /// Garathe caído (`MN_COLPS`: 29–33, 35, 59 e as mortes); no Jake, a padrão.
    box_collapse: Rect,
    /// Gerador para o sorteio do 79 (`rand & 3`).
    rng: u32,
    /// Especial disparado neste tique (a cena aplica nos objetos).
    pub special: Option<SpecialBlast>,
    /// Caixa do soco no primeiro tique da janela do golpe (a cena fere quem cruza).
    pub punch: Option<Rect>,
    /// Esse golpe (atacante e variante da arma e da mira; a linha do `DAMAGE.TBL` sai por
    /// vítima).
    pub blow: Option<Strike>,
    /// Trocou de arma neste tique (a cena toca 3090).
    pub switched: bool,
    /// Tiques restantes da pose "pronto" depois da troca (`+0x26c`).
    switch_pose: u32,
    /// A pose da troca vale neste tique (o contador andou no repouso).
    pose_now: bool,
    /// Trava do tiro (`+0x294`): sair do tiro soltando ATIRAR põe 8; cai 1 por tique e,
    /// enquanto ≠ 0, `0x0043b43e` não começa tiro nenhum.
    fire_lock: u32,
    /// Modo soco (`0x47d7b8`): 1 com SOCO, 2 com a boca bloqueada, 0 com ATIRAR.
    punch_mode: u8,
    /// Veneno (`+0x270`): quanto perde a cada 20 tiques.
    pub poison: i32,
    /// Piscar da cor (`+0x28c/+0x290`, `docs/re/cores-sprites.md`).
    pub flash: crate::tint::Flash,
    /// Chegou ao fim do rótulo 100 empurrando neste tique (`0x00425d04`: a cena tenta a
    /// porta da célula numa zona PR 0x20).
    pub door_push: bool,
    /// Garathe empurrando: já passou do 44 ao laço 46.
    push_loop: bool,
    /// Tiques desde a criação (o veneno age em `tique % 20 = 0`).
    clock: u32,
    /// Quem é (perfil de movimentos, rótulos e armas).
    character: Character,
    /// Som posicional pedido neste tique (Garathe: pouso do pulo longo, empurrar, pulo
    /// contra a parede; a cena toca).
    pub sound: Option<u16>,
    /// Cor-base (`+0x280`): 0 no solo e no coop (35/36); na disputa, a da `SEGOC`.
    colour: i32,
    /// Garathe: tiques de terremoto (`+0x274`, `0x42bcf8`); o do Jake fica na cena.
    quake: u32,
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
            box_collapse: boxr,
            rng: 0x4a41_4b45,
            special: None,
            punch: None,
            blow: None,
            switched: false,
            switch_pose: 0,
            pose_now: false,
            fire_lock: 0,
            punch_mode: 0,
            poison: 0,
            flash: crate::tint::Flash::default(),
            door_push: false,
            push_loop: false,
            clock: 0,
            character: Character::Jake,
            sound: None,
            colour: 0,
            quake: 0,
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

    /// Cor-base `+0x280` (e `+0x27c`) da disputa (`0x43cd2f`); 0 = a do personagem.
    pub fn with_colour(mut self, colour: i32) -> Self {
        self.colour = colour;
        self
    }

    /// Cor-base (`+0x280`; 0 = a do personagem).
    pub fn colour(&self) -> i32 {
        self.colour
    }

    /// Renasce na disputa (estados 181 `0x426b09` / 80 `0x42b861`): de pé com a origem em
    /// `at` (`0x43d300`: `+0x14c/+0x150` = o ponto) ou, sem ponto (`None`), onde está 20 px
    /// acima (`0x426a84`), na camada `layer`, sem armas, munição, chaves, bandeiras, veneno,
    /// dano pendente nem piscar, com a melhor arma (punhos/garras) e vida 100; a armadura fica.
    pub fn respawn_at(&mut self, at: Option<(i32, i32)>, layer: i32) {
        let bx = self.box_default;
        let (x, y) = at.unwrap_or((self.body.x, self.body.y - 20));
        let armor = self.health.armor;
        self.body = Body::new(x, y, bx);
        self.start = (x, y);
        self.layer = layer;
        self.health = Health { hp: 100, armor };
        self.pending = Pending::default();
        self.poison = 0;
        self.quake = 0;
        self.flash = crate::tint::Flash::default();
        self.teleport = None;
        self.exit_scale = None;
        self.switch_pose = 0;
        self.push_loop = false;
        self.inventory = Inventory::for_character(&[], self.character);
        self.inventory.weapon = self.inventory.best();
        self.shots.clear();
        self.set(State::Idle);
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
            // empurrar: 44 até o fim, depois o laço 46
            (Character::Garathe, State::Push) if !self.push_loop => (garathe::PUSH_START, 0),
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

    /// Caixa do Garathe caído (`MN_COLPS.BRD`).
    pub fn with_collapse_box(mut self, r: Rect) -> Self {
        self.box_collapse = r;
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

    /// Começa (ou recomeça) um tiro ou soco com a mira da entrada (`0x0043b43e`): nada com
    /// a trava `+0x294` ligada; BAIXO (com ATIRAR ou SOCO) e vida > 25 → especial; senão um
    /// dos dois estados da mira (`rand & 1`) com `t = 0` e o primeiro tique dele no mesmo
    /// tique (`0x0043b50b`): a pistola dispara na hora em que se aperta ATIRAR.
    fn start_fire(&mut self, input: u32, world: &World) -> bool {
        if self.fire_lock > 0 {
            return false;
        }
        if self.wants_special_now(input) {
            self.enter_special(input, world);
            return true;
        }
        let aim = Aim::from_input(input);
        let w = self.desc_weapon();
        self.stop_for_fire();
        let alt = self.coin();
        self.state = State::Fire {
            aim,
            t: 0,
            w: w as u8,
            alt,
        };
        self.ticks = 0;
        self.frame = 0;
        self.label_changed = true;
        self.tick_fire(input, world, (aim, 0, w as u8, alt));
        true
    }

    /// BAIXO com ATIRAR ou SOCO (sem CIMA nem um lado só) e vida > 25 (`0x43ae4c`).
    fn wants_special_now(&self, input: u32) -> bool {
        let shoot = if input & PUNCH != 0 { SHOOT } else { 0 };
        moves::wants_special(input | shoot) && self.health.hp > moves::SPECIAL_MIN_HP
    }

    fn enter_special(&mut self, input: u32, world: &World) {
        self.body.vx = 0;
        self.set(State::Special(0));
        self.tick_special(input, world);
    }

    /// Sorteio `rand & 1` de um dos dois estados do par.
    fn coin(&mut self) -> bool {
        self.rng = self.rng.wrapping_mul(1_103_515_245).wrapping_add(12345);
        (self.rng >> 16) & 1 != 0
    }

    /// Arma do descritor corrente (`0x0043ab05`): a do soco no modo soco, senão a corrente.
    fn desc_weapon(&self) -> usize {
        if self.punch_mode != 0 {
            self.melee_weapon()
        } else {
            self.inventory.weapon
        }
    }

    /// O ataque para o Jake; o Garathe no ar segue com a horizontal (o `0x43b43e` e o
    /// `0x43b50b` não gravam `+0x164`; no chão a decisão já zerou com `0x407009`).
    fn stop_for_fire(&mut self) {
        if !(self.is_garathe() && !self.body.on_ground) {
            self.body.vx = 0;
        }
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
    /// Centro da caixa do estado corrente (`0x42318e` troca o BRD já na entrada do estado de
    /// tiro; a do corpo só se ajusta no fim do tique).
    fn fire_center(&self) -> (i32, i32) {
        let mut body = self.body.clone();
        body.boxr = self.state_box();
        let b = body.bounds();
        (b.x + b.w / 2, b.y + b.h / 2)
    }

    fn muzzle_ok(&self, world: &World, w: usize, muzzle: (i32, i32)) -> bool {
        if self.weapon(w).half_line {
            let c = self.fire_center();
            self.muzzle_clear(world, ((c.0 + muzzle.0) / 2, (c.1 + muzzle.1) / 2))
        } else {
            self.muzzle_clear(world, muzzle)
        }
    }

    /// Golpe corpo a corpo do estado de tiro corrente: o tipo do jogador e a variante da
    /// arma e da mira.
    fn blow_damage(&self, w: u8, aim: Aim) -> Option<Strike> {
        let variant = self.weapon(w as usize).blow?[weapons::aim_slot(aim)];
        Some(Strike {
            attacker: self.character.kind(),
            variant,
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

    /// Som de arma do quadro corrente, quando o quadro mudou neste tique (rótulo novo ou
    /// avanço; `SOUND.TBL` pelo quadro, `0x4354dc`).
    pub fn fire_sound(&self) -> Option<u16> {
        let State::Fire { aim, w, .. } = self.state else {
            return None;
        };
        if !(self.label_changed || self.advance) {
            return None;
        }
        self.weapon(w as usize).sounds[weapons::aim_slot(aim)]
            .iter()
            .find(|&&(f, _)| f == self.frame)
            .map(|&(_, id)| id)
    }

    /// A linha do centro da caixa até a boca da arma está livre de sólidos
    /// (`docs/re/tiro-jake.md`, passo 4: senão o tiro é pulado).
    fn muzzle_clear(&self, world: &World, muzzle: (i32, i32)) -> bool {
        let c = self.fire_center();
        // um ponto por pixel do segmento (`0x433aa3`/`0x43735d` testam a linha toda): uma
        // parede fina não escapa entre as amostras
        let n = (muzzle.0 - c.0).abs().max((muzzle.1 - c.1).abs()).max(1);
        (1..=n).all(|i| {
            let p = Rect {
                x: c.0 + (muzzle.0 - c.0) * i / n,
                y: c.1 + (muzzle.1 - c.1) * i / n,
                w: 1,
                h: 1,
            };
            !world.blocked(&p)
        })
    }

    /// Tique de um estado de tiro ou soco (`0x0043b50b`).
    fn tick_fire(&mut self, input: u32, world: &World, st: (Aim, u32, u8, bool)) {
        let (aim0, t0, w0, alt0) = st;
        let pressed = |bit: u32| input & bit != 0;
        // modo soco (`0x47d7b8`): SOCO liga, ATIRAR desliga; boca bloqueada → 2
        if pressed(PUNCH) {
            self.punch_mode = 1;
        } else if pressed(SHOOT) {
            self.punch_mode = 0;
        }
        let right = self.facing_right;
        let mut w = self.desc_weapon();
        let cur_label =
            self.weapon(w0 as usize).states[weapons::aim_slot(aim0)][usize::from(alt0)].0;
        if self.punch_mode == 0 {
            // registro do estado corrente na arma do descritor (`0x43ac05`; fora dela, o da
            // mira da entrada)
            let slot = if w == w0 as usize {
                weapons::aim_slot(aim0)
            } else {
                weapons::aim_slot(Aim::from_input(input))
            };
            let r = self.weapon(w).states[slot][0].1;
            let muzzle = (
                self.body.x + if right { r.dx.1 } else { r.dx.0 },
                self.body.y + r.dy,
            );
            if !self.muzzle_ok(world, w, muzzle) {
                self.punch_mode = 2;
                w = self.melee_weapon();
            }
        }
        // soltou ATIRAR/SOCO mexendo (direção, pulo, troca de arma): sai na hora e trava o
        // tiro por 8 tiques (`0x43b591`)
        if !pressed(SHOOT | PUNCH)
            && input & (UP | DOWN | LEFT | RIGHT | JUMP | PREV_WEAPON | NEXT_WEAPON) != 0
        {
            self.fire_lock = FIRE_LOCK_RELEASE;
            self.set(State::Idle);
            return;
        }
        // a mira é refeita a cada tique (`0x43ae4c`): BAIXO → especial; outra mira (ou outra
        // arma) troca o estado sem zerar `t`, no mesmo quadro
        if self.wants_special_now(input) {
            self.enter_special(input, world);
            return;
        }
        let (mut aim, mut alt) = (aim0, alt0);
        let want = Aim::from_input(input);
        let pair = self.weapon(w).states[weapons::aim_slot(want)];
        if pair[0].0 != cur_label && pair[1].0 != cur_label {
            aim = want;
            alt = self.coin();
            // o quadro do rótulo antigo já andou `passo` neste tique (`0x43b5ed`) e o novo
            // rótulo fica nesse deslocamento (`0x43b6c3`)
            let f = self.frame + self.weapon(w0 as usize).step as usize;
            let next = State::Fire {
                aim,
                t: t0,
                w: w as u8,
                alt,
            };
            self.set(next);
            self.frame = f.min(self.len_of(self.anim_of(next).0) - 1);
        }
        let wp = self.weapon(w);
        let r = wp.states[weapons::aim_slot(aim)][usize::from(alt)].1;
        let mut t = t0 + wp.step;
        let mut frames = wp.step as usize;
        // golpe: a rotina de contato `0x43afe1` vale em cada tique com `+0x04 < t ≤ +0x08`
        if r.hit.0 < t && t <= r.hit.1 {
            self.punch = Some(self.punch_box(aim));
            self.blow = self.blow_damage(w as u8, aim);
        }
        let muzzle = (
            self.body.x + if right { r.dx.1 } else { r.dx.0 },
            self.body.y + r.dy,
        );
        // chicote: algo até 3/4 da linha da boca → `t` e a animação andam um passo a mais
        // (`0x43bcf2`)
        if wp.half_line {
            let c = self.fire_center();
            let q = (
                c.0 + (muzzle.0 - c.0) * 3 / 4,
                c.1 + (muzzle.1 - c.1) * 3 / 4,
            );
            if !self.muzzle_clear(world, q) {
                t += wp.step;
                frames += wp.step as usize;
            }
        }
        self.anim_mode = AnimMode::Step(frames);
        self.state = State::Fire {
            aim,
            t,
            w: w as u8,
            alt,
        };
        if r.fire.0 <= t && t < r.fire.1 + wp.step && self.discharge(world, w, r, muzzle, t) {
            return;
        }
        if t > r.again && pressed(SHOOT | PUNCH) {
            self.start_fire(input, world);
        } else if t > r.reeval {
            self.reevaluate(input, world, w, r);
        }
    }

    /// Disparo (`0x0043b1d0`). Sem munição na arma corrente: aviso 3090, a melhor arma e a
    /// pose "pronto" dela (devolve `true`: o estado acabou). Linha do centro à boca
    /// bloqueada: nada. Senão gasta 1 e cria o projétil (e a cápsula).
    fn discharge(
        &mut self,
        world: &World,
        w: usize,
        r: weapons::Reg,
        muzzle: (i32, i32),
        t: u32,
    ) -> bool {
        let wp = self.weapon(w);
        let Some(kind) = wp.shot else {
            return false;
        };
        let current = self.inventory.weapon;
        if self.inventory.ammo(current) <= 0 {
            self.no_ammo = true;
            self.inventory.weapon = self.inventory.best();
            self.stop_for_fire();
            let nw = self.inventory.weapon as u8;
            self.set(State::Ready(0, nw));
            return true;
        }
        if !self.muzzle_clear(world, muzzle) {
            return false;
        }
        let slot = &mut self.inventory.slots[current].1;
        *slot = (*slot - 1).max(0);
        let sign = if self.facing_right { 1 } else { -1 };
        let (vx, vy) = (r.v.0 * sign, r.v.1);
        // desvio da rajada (`0x0043bab7`): `n = t − +0x0c`; n = 0 soma (−vy, vx)·8/|v|,
        // n = 2 o oposto (1/8 px por tique), negados olhando para a esquerda
        let n = t - r.fire.0;
        let nudge = match n {
            0 | 2 if wp.spread => {
                let len = crate::behaviour::mine::distance((vx, vy), (0, 0)).max(1);
                let s = if n == 0 { sign } else { -sign };
                (s * (-vy * 8 / len), s * (vx * 8 / len))
            }
            _ => (0, 0),
        };
        self.shots.push(Shot {
            x: muzzle.0,
            y: muzzle.1,
            vx,
            vy,
            kind,
            nudge,
        });
        if let (Some(ck), Some(c), 0) = (wp.casing, r.casing, n) {
            self.shots.push(Shot {
                x: self.body.x + if self.facing_right { c.dx.1 } else { c.dx.0 },
                y: self.body.y + c.dy,
                vx: c.v.0 * sign,
                vy: c.v.1,
                kind: ck,
                nudge: (0, 0),
            });
        }
        false
    }

    /// `0x0043b43e` sem novo disparo: com ATIRAR/SOCO recomeça; senão a recuperação do
    /// registro (`+0x1c`) ou a pose "pronto". Com a trava ligada, nada.
    fn reevaluate(&mut self, input: u32, world: &World, w: usize, r: weapons::Reg) {
        if self.fire_lock > 0 {
            return;
        }
        if input & (SHOOT | PUNCH) != 0 {
            self.start_fire(input, world);
        } else if r.recover != 0 {
            self.set(State::Recover(w as u8));
        } else {
            self.set(State::Ready(0, w as u8));
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
    /// Tabela de cor do desenho (`0x42315f`/`0x4282d3`): a do piscar ou a do personagem.
    pub fn tint(&self) -> crate::tint::Tint {
        crate::tint::player_coloured(self.is_garathe(), self.colour, &self.flash)
    }

    pub fn is_visible(&self) -> bool {
        !matches!(self.state, State::Teleporting(_))
    }

    /// Entra no teletransporte: some, e na metade da viagem anda `delta` px.
    pub fn start_teleport(&mut self, delta: (i32, i32)) {
        self.start_teleport_to(delta, self.layer);
    }

    /// Como `start_teleport`, indo à camada `layer` na metade da viagem (`0x41347e`: o
    /// passageiro vai à camada do destino).
    pub fn start_teleport_to(&mut self, delta: (i32, i32), layer: i32) {
        if self.is_dead() {
            return;
        }
        self.body.vx = 0;
        self.body.vy = 0;
        self.pending = Pending::default();
        self.teleport = Some((delta, layer));
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

    /// Dano pendente ainda não aplicado (`+0x24c`, lido pela IA).
    pub fn pending_damage(&self) -> i32 {
        self.pending.amount
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
        // o 58 do Garathe decide em pé a cada tique (`0x42b93e`)
        if self.state == State::Pushed {
            return self.is_garathe();
        }
        !matches!(
            self.state,
            State::Jolted
                | State::KnockedDown(_)
                | State::KnockRise(_)
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
        if p.poisons {
            self.poison = 1;
        }
        if p.amount != 0 && (p.knockdown || p.shove || p.jolt || p.explosion) {
            self.flash.start(crate::tint::FLASH_HIT, self.colour);
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
            // Jake (`0x423657`): 81 se empurrado para o lado em que olha, senão 82
            let right = if g {
                p.push.0 >= 0
            } else if self.facing_right {
                p.push.0 > 0
            } else {
                p.push.0 < 0
            };
            self.body.vx = p.push.0 * SUB;
            self.body.vy = p.push.1 * SUB;
            self.set(State::KnockedDown(right));
        } else if p.shove && shovable {
            // o Garathe não é deslocado (`0x428531` não grava velocidade)
            if !g {
                self.body.vx = p.push.0 * SUB;
            }
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
            self.flash.start(crate::tint::FLASH_SHOCK, self.colour);
            self.set(State::Shocked(SHOCK_TICKS));
            if g {
                self.frame = garathe::SHOCK_FRAME.min(self.len_of(121) - 1);
            }
        }
        if apply(&mut self.health, &p, tick) == 0 {
            if g && garathe::death_waits(self.state) {
                // `0x428863`: nada agora; a vida a 0 é vista a cada tique
                return false;
            }
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
                State::Land(Landing::Fall) if g => garathe::DIE_DEFAULT,
                _ if g && p.poison => garathe::DIE_POISON,
                _ if g => garathe::DIE_DEFAULT,
                State::KnockedDown(true) => DIE_KNOCKED_RIGHT,
                State::KnockedDown(false) | State::KnockRise(_) => DIE_KNOCKED_LEFT,
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
            State::Idle if self.pose_now => self.weapon(self.inventory.weapon).ready,
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
            self.push_loop = false;
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
            AnimMode::Step(k) => (before + k).min(n - 1),
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
        // veneno (`0x004231f9`, passo 3): a cada 20 tiques, `+0x270` de dano de veneno; o
        // contador global ainda não avançou (`tique % 20 = 0` no tique 0, oráculo `D_J_012`)
        if self.poison > 0 && self.clock.wrapping_sub(1) % POISON_PERIOD == 0 && self.is_active() {
            self.pending.add(&Pending {
                amount: self.poison,
                poison: true,
                ..Default::default()
            });
        }
        self.take_damage(self.ticks);
        if self.is_garathe() {
            self.garathe_late_death();
        }
        // piscar: envenenado (`0x42328e`), e o passo do fim da atualização (`0x408129`)
        // adiantado para cá (o do especial, posto depois, conta a partir do próximo tique)
        if self.poison > 0 {
            self.flash.start(crate::tint::FLASH_POISON, self.colour);
        }
        self.flash.tick(self.poison > 0);
        // trava do tiro (`0x422a98`) e troca de arma (`0x43b0cb`) antes dos estados, em
        // qualquer estado salvo morrendo (`+0x178 & 0x200000`), inclusive derrubado
        if !matches!(self.state, State::Dying(_) | State::Dead) {
            self.fire_lock = self.fire_lock.saturating_sub(1);
            self.switch_weapon(input);
        }
        self.pose_now = false;
        if !self.controllable() {
            self.tick_passive(input, env);
            return;
        }
        // a pose da troca anda e aparece só no repouso (`0x422ce4` → `0x43b15c`); nos outros
        // estados o contador espera (oráculo: trocada no 84, a pose sai no repouso seguinte)
        if self.state == State::Idle && self.switch_pose > 0 {
            self.switch_pose -= 1;
            self.pose_now = self.switch_pose > 0;
        }
        let pressed = |bit: u32| input & bit != 0;
        let jump_edge = pressed(JUMP) && self.prev_input & JUMP == 0;
        let side = self.wanted_side(input);
        // Garathe: terremoto (`0x42bcf8`), antes da rotina do estado
        let quaking = self.is_garathe() && self.tick_quake();
        // o Garathe vira com a entrada em todo tique (`0x427c9c`); o Jake não no ar
        let airborne = !self.is_garathe()
            && matches!(self.state, State::Jump(_) | State::Fall | State::WildFall);
        if let (false, Some(right)) = (airborne, side) {
            self.facing_right = right;
        }
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
        ) || (self.is_garathe() && self.state == State::Pushed)
            || (self.state == State::Crawl && self.free_above(world, self.profile().headroom));
        let fired = !quaking
            && !matches!(self.state, State::Fire { .. })
            && can_fire
            && shooting
            && self.start_fire(input, world);
        // Garathe: a animação do pulo 22/25 acabou neste tique
        let mut jump_over = false;
        match self.state {
            _ if quaking => {
                // 121 toca uma vez; no fim, a rotina do parado, e o próximo tique recomeça
                if self.anim_done() {
                    self.decide_or_idle(input, env, jump_edge, side);
                }
            }
            _ if fired => {}
            State::Fire { aim, t, w, alt } => {
                self.tick_fire(input, world, (aim, t, w, alt));
                if self.state == State::Idle {
                    // saída pelo repouso `+0x7c` (`0x426ecc`): o tratamento em pé decide já
                    self.decide_standing(input, env, jump_edge, side);
                }
            }
            State::Recover(_) | State::Ready(..) | State::Holster(_) => {
                // `0x43b94e`: direção, pulo, furtivo ou troca seguros (`0x1e0f`) → repouso
                if input & (UP | DOWN | LEFT | RIGHT | JUMP | STEALTH | PREV_WEAPON | NEXT_WEAPON)
                    != 0
                {
                    self.set(State::Idle);
                    self.decide_standing(input, env, jump_edge, side);
                } else {
                    match self.state {
                        State::Recover(w) if self.anim_done() => self.set(State::Ready(0, w)),
                        State::Ready(n, w) if n + 1 >= READY_TICKS => self.set(State::Holster(w)),
                        State::Ready(n, w) => self.state = State::Ready(n + 1, w),
                        State::Holster(_) if self.anim_done() => self.set(State::Idle),
                        _ => {}
                    }
                    self.body.vx = 0;
                }
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
                        // 22/25: a animação toca uma vez e para no último quadro; só então
                        // vê chão, parede e contato (`0x429e46`, `0x429c6c`)
                        self.anim_mode = AnimMode::Hold;
                        jump_over = true;
                    }
                    self.air_reverse(side);
                }
            }
            State::Fall if self.is_garathe() => {
                // 27 (`0x4293c2`): ataque (`0x4293ef`); zona 4 agarra; senão controle no ar
                // (`0x408205`)
                if self.air_attack(input, world, false) {
                } else if !self.try_grab_in_air(input, env) {
                    self.air_control(world, side);
                }
            }
            State::Fall | State::WildFall => {
                if !self.try_grab_in_air(input, env) {
                    self.air_reverse(side);
                }
            }
            _ => self.tick_move(input, env, jump_edge, side),
        }
        if self.is_garathe() {
            // fim do 44: som 5043 e entrada no 46 (rótulo novo), que pede a porta
            self.door_push = self.state == State::Push && !self.push_loop && self.anim_done();
            if self.door_push {
                self.sound = Some(garathe::SOUND_PUSH);
                self.push_loop = true;
                self.frame = 0;
                self.label_changed = true;
            }
        } else {
            let (push_label, push_c) = self.anim_of(State::Push);
            self.door_push = self.state == State::Push
                && self.ticks + 1 == self.len_of(push_label) as u32 * (push_c + 1);
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
        let y0 = self.body.y;
        let gravity = self.gravity_on();
        step(&mut self.body, world, gravity);
        let attacking = pressed(SHOOT | PUNCH);
        match self.state {
            // Garathe no meio do pulo: nada decide até a animação acabar
            State::Jump(_) if self.is_garathe() && !jump_over => {}
            State::Jump(kind)
                if self.is_garathe() && self.body.hit_wall && !self.body.on_ground =>
            {
                // 22/25 contra a parede (o chão vem antes): fala e queda (`0x429e46`,
                // `0x429c6c`)
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
            // estados de arma no ar (o ataque da barra): a queda (`0x0042415c`/`0x004292d8`)
            // só os leva se o jogador não está atacando; atacando, ao tocar o chão vindo
            // a 5 px ou mais por tique, aterrissa (`0x00424220`/`0x00429395` → 26/28)
            State::Fire { .. } if attacking => {
                if self.body.on_ground && self.body.y - y0 >= FALL_SPEED / SUB {
                    self.body.vx = 0;
                    self.body.vy = 0;
                    self.set(State::Land(Landing::Fall));
                }
            }
            State::Fire { .. } | State::Ready(..) | State::Recover(_) | State::Holster(_)
                if !self.body.on_ground && self.body.vy >= FALL_SPEED =>
            {
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
                    if let Some(((dx, dy), layer)) = self.teleport.take() {
                        self.body.x = self.body.x.saturating_add(dx);
                        self.body.y = self.body.y.saturating_add(dy);
                        self.layer = layer;
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
                if self.garathe_passive_fall() {
                } else if n <= 1 {
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
                    self.garathe_passive_fall();
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
                if !self.is_garathe() {
                    // Jake (`0x425a70`/`0x425b1e`): o rótulo toca uma vez (modo 0xa) e só no
                    // fim decide: no chão, 81 aterrissa e 82 levanta (83 → 84); no ar, 81 cai
                    self.anim_mode = AnimMode::Step(1);
                    let len = self.len_of(self.anim_of(self.state).0);
                    if !self.label_changed && self.frame + 1 >= len {
                        let fwd = self.state == State::KnockedDown(true);
                        if self.body.on_ground {
                            self.body.vx = 0;
                            self.body.vy = 0;
                            self.set(if fwd {
                                State::Land(Landing::Fall)
                            } else {
                                State::KnockRise(false)
                            });
                        } else if fwd {
                            self.set(State::Fall);
                        }
                    }
                } else if self.body.on_ground && !was_rising {
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
            State::KnockRise(second) => {
                // 83 e 84 tocam até o fim; então o tratamento em pé decide (`0x425bdf`,
                // `0x426ecc`); sem decisão, 83 → 84 → repouso
                step(&mut self.body, world, true);
                self.anim_mode = AnimMode::Step(1);
                let len = self.len_of(self.anim_of(self.state).0);
                if !self.label_changed && self.frame + 1 >= len {
                    let jump_edge = input & JUMP != 0 && self.prev_input & JUMP == 0;
                    let side = self.wanted_side(input);
                    self.set(State::Idle);
                    self.decide_standing(input, env, jump_edge, side);
                    if !second && self.state == State::Idle {
                        self.set(State::KnockRise(true));
                    }
                }
            }
            State::Dying(label)
                if self.is_garathe()
                    && matches!(label, garathe::DIE_POISON | garathe::DIE_DEFAULT)
                    && !self.anim_done() =>
            {
                // 61 e 63 deslizam pelo esqueleto (k = 2, só x: `0x42bc84`, `0x42bb62`)
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
            State::Dying(_) if self.is_garathe() => {
                // o fim (`0x42b604`) pede `vx = 0`, `0 ≤ vy ≤ 16` e chão; até lá o último
                // quadro fica e o atrito freia
                let done = self.anim_done();
                if done {
                    self.anim_mode = AnimMode::Hold;
                }
                step(&mut self.body, world, true);
                if self.body.on_ground && done && self.body.vx == 0 {
                    self.set(State::Dead);
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

    /// Morte adiada do Garathe (`0x428863`): com a vida a 0 fora dos estados que esperam,
    /// a regra pelo estado: 30 → 67, 35 → 69, os demais → 63 (o veneno e as marcas da
    /// derrubada já foram zerados no fim do tique do dano).
    fn garathe_late_death(&mut self) {
        if !self.health.is_dead()
            || matches!(self.state, State::Dying(_) | State::Dead)
            || garathe::death_waits(self.state)
        {
            return;
        }
        self.death_label = match self.state {
            State::Land(Landing::Hard) => garathe::DIE_FORWARD,
            State::Rise => garathe::DIE_BACK,
            _ => garathe::DIE_DEFAULT,
        };
        self.set(State::Dying(self.death_label));
    }

    /// Garathe sem controle (59, 121 do choque) no ar a 5 px/tique: a queda (`0x4292d8`).
    fn garathe_passive_fall(&mut self) -> bool {
        let fall = self.is_garathe() && !self.body.on_ground && self.body.vy >= FALL_SPEED;
        if fall {
            self.set(State::Fall);
        }
        fall
    }

    /// Terremoto no Garathe (`+0x274`): o abalo do brutamontes (`0x418369`, 20–39 tiques) e
    /// o especial de outro Garathe a 70–139 px (30).
    pub fn start_quake(&mut self, ticks: u32) {
        self.quake = ticks;
    }

    /// Tiques de terremoto que restam ao Garathe.
    pub fn quake_ticks(&self) -> u32 {
        self.quake
    }

    /// `0x42bcf8`: com `+0x274 > 0`, desconta um tique; com gravidade e fora dos estados
    /// isentos, a rotina do 121 (`0x42bd73`) toma o tique: entra no 121 (velocidade
    /// intacta) ou segue nele; zerado o contador, a rotina do parado. `true` = tomou o tique.
    fn tick_quake(&mut self) -> bool {
        if self.quake == 0 {
            return false;
        }
        self.quake -= 1;
        if !self.gravity_on() || moves::quake_exempt(self.state) {
            return false;
        }
        if self.quake == 0 {
            if self.state == State::Quake {
                self.set(State::Idle);
            }
            return false;
        }
        if self.state != State::Quake {
            self.set(State::Quake);
        }
        true
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
    fn color_flashes_on_hits_poison_and_shock() {
        let w = long_floor();
        let mut j = jake_on(&w, 0).with_health(100, 0);
        assert_eq!(j.tint().table, crate::tint::JAKE);
        // dano comum, sem reação: não pisca
        j.hurt(Pending {
            amount: 1,
            ..Default::default()
        });
        j.tick(0, &w);
        assert!(!j.flash.active());
        // explosão: 0x2e em tiques alternados (4)
        j.hurt(Pending {
            amount: 1,
            explosion: true,
            ..Default::default()
        });
        let seen: Vec<i32> = (0..5)
            .map(|_| {
                let t = j.tint().table;
                j.tick(0, &w);
                t
            })
            .collect();
        assert_eq!(seen[1..], [0x2e, 0x23, 0x2e, 0x23]);
        // choque: 0x25 por 10
        j.hurt(Pending {
            amount: 1,
            electric: true,
            ..Default::default()
        });
        j.tick(0, &w);
        assert_eq!(j.tint().table, 0x25);
        for _ in 0..10 {
            j.tick(0, &w);
        }
        // veneno: 0x2f rearmado a cada tique enquanto envenenado
        j.poison = 1;
        let poisoned = (0..12)
            .filter(|_| {
                j.tick(0, &w);
                j.tint().table == 0x2f
            })
            .count();
        assert!((5..=7).contains(&poisoned), "{poisoned}");
        j.poison = 0;
        for _ in 0..5 {
            j.tick(0, &w);
        }
        assert_eq!(j.tint().table, crate::tint::JAKE);
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
        // derrubada para a direita olhando para a esquerda: para trás (82); voa, levanta
        // (83, 84) e volta ao repouso
        j.hurt(Pending {
            amount: 5,
            explosion: true,
            push: (8, -8),
            knockdown: true,
            ..Default::default()
        });
        j.tick(0, &w);
        assert_eq!((j.state(), j.label()), (State::KnockedDown(false), 82));
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
        // derrubado e morto na mesma pancada: rótulo 177 (`J077`) para a frente (`+0x250 & 8`),
        // 179 para trás (`0x10`); olhando para a esquerda, empurrão à esquerda é para a frente
        let mut j = jake_on(&w, 0).with_health(1, 0);
        j.hurt(Pending {
            amount: 9,
            push: (-8, -8),
            knockdown: true,
            ..Default::default()
        });
        j.tick(0, &w);
        assert_eq!(j.state(), State::Dying(DIE_KNOCKED_RIGHT));
        let mut j = jake_on(&w, 0).with_health(1, 0);
        j.hurt(Pending {
            amount: 9,
            push: (8, -8),
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
        // o tique 0 entra no estado e já anda (`0x43b50b`: t = 1, dispara); t = 1, 2, 3
        // disparam; em t = 7 (> 6) recomeça e dispara no mesmo tique: rajadas a cada 6
        assert_eq!(
            fired,
            vec![
                (0, 1, true),
                (1, 1, false),
                (2, 1, false),
                (6, 1, true),
                (7, 1, false),
                (8, 1, false),
                (12, 1, true),
                (13, 1, false),
                (14, 1, false),
                (18, 1, true),
                (19, 1, false),
            ]
        );
        assert_eq!(j.inventory.pistol_ammo(), 69);
        assert!(j.is_firing());
        assert_eq!(j.label(), 23);
    }

    #[test]
    fn shots_carry_muzzle_velocity_and_spread() {
        let w = long_floor();
        let mut j = jake_on(&w, 0).with_inventory(pistol());
        j.tick(RIGHT, &w); // olha para a direita
        let (x, y) = (j.body.x, j.body.y);
        let mut rounds = vec![];
        for _ in 0..3 {
            j.tick(SHOOT, &w);
            rounds.extend(j.shots.drain(..).filter(|s| !s.is_casing()));
        }
        assert_eq!(rounds.len(), 3);
        assert_eq!((rounds[0].x, rounds[0].y), (x + 25, y + 2));
        // desvio em 1/8 px por tique: (−vy, vx)·8/|v| na 1ª (desce), o oposto na 3ª (sobe)
        let v = |s: &Shot| (s.vx * SUB + s.nudge.0, s.vy * SUB + s.nudge.1);
        assert_eq!(v(&rounds[0]), (160, 8));
        assert_eq!(v(&rounds[1]), (160, 0));
        assert_eq!(v(&rounds[2]), (160, -8));
        // na diagonal (20, 20), |v| = 28: (−20, 20)·8/28 = (−5, 5) — não 1 px inteiro
        let mut j = jake_on(&w, 0).with_inventory(pistol());
        j.tick(RIGHT, &w);
        j.tick(SHOOT | UP | RIGHT, &w);
        let s = j.shots.iter().find(|s| !s.is_casing()).copied().unwrap();
        assert_eq!((s.vx, s.vy, s.nudge), (20, -20, (5, 5)));
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

    /// A viagem a um teletransporte de outra camada leva o Jake a ela na metade.
    #[test]
    fn teleport_to_another_layer_changes_layer_halfway() {
        let w = long_floor();
        let mut j = jake_on(&w, 0);
        j.start_teleport_to((0, 256), 2);
        for t in 1..TELEPORT_TICKS {
            j.tick(0, &w);
            assert_eq!(
                j.layer(),
                if t < TELEPORT_MOVE { 0 } else { 2 },
                "tique {t}"
            );
        }
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
        // espingarda: um tiro a cada 8 tiques (t > 8 recomeça e dispara), com o cartucho
        let (t, k, left) = fire_log(SHOTGUN, 20);
        assert_eq!(t, vec![0, 8, 16, 24]);
        assert_eq!(&k[..2], &[ShotKind::Pellet, ShotKind::Shell]);
        assert_eq!(left, 16);
        // foguetes: a cada 24
        let (t, k, _) = fire_log(ROCKETS, 20);
        assert_eq!(t, vec![0, 24]);
        assert!(k.iter().all(|&k| k == ShotKind::Rocket));
        // estrelas e granadas: passo 2, dispara em t = 8, recomeça com t > 11 já em t = 2
        // (5 tiques)
        let (t, k, _) = fire_log(STARS, 20);
        assert_eq!(t, vec![3, 8, 13, 18, 23, 28]);
        assert!(k.iter().all(|&k| k == ShotKind::Star));
        let (t, _, left) = fire_log(GRENADES, 2);
        assert_eq!(&t[..2], &[3, 8]);
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
