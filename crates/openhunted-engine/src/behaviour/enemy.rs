// SPDX-License-Identifier: GPL-3.0-or-later
//! O comum aos inimigos da M7a (`docs/re/robo-seguranca.md`, `robo-sentinela.md`,
//! `mina-saltadora.md`, `gerador-minas.md`, `bruiser.md`): a animação tocada quadro a quadro
//! com a rotina de fim (`0x407942`), os sons por quadro do `SOUND.TBL`, a linha de visão, o
//! alvo, o dano pendente com crédito e os pedidos de objetos novos que a cena atende.

use super::{Ctx, Event, Out};
use crate::damage::Pending;
use crate::entity::{Behaviour, Entity};
use crate::physics::Rect;
use crate::sound::{Cue, PRIORITY_FRAME};

/// Animações e BRDs que a cena carrega para os objetos criados pelos inimigos (projéteis
/// 21/99/100/106, minas do gerador, fogo do Bruiser, dardos e formigas).
pub const ANIMS: [&str; 8] = [
    "OSB00.ANM",
    "OBB00.ANM",
    "ORO.ANM",
    "ODS00.ANM",
    "OHM00.ANM",
    "OFI.ANM",
    "ODA.ANM",
    "OAN.ANM",
];
pub const BORDERS: [&str; 8] = [
    "OSB00.BRD",
    "OBB.BRD",
    "ORO00.BRD",
    "ORO01.BRD",
    "ODS.BRD",
    "OHM00.BRD",
    "ODA.BRD",
    "OAN.BRD",
];

/// Intervalo mínimo em tiques entre duas vezes o mesmo som, entre todos os objetos: 3061
/// (sentinela em espera, 200 ms), 3157 (robô de segurança, 250 ms) e 3131 (terremoto do
/// Bruiser, enquanto o som toca).
pub fn throttle(id: u16) -> Option<u32> {
    match id {
        super::sentinel::SOUND_IDLE => Some(super::sentinel::SOUND_IDLE_TICKS),
        super::security_robot::SOUND_HOVER => Some(super::security_robot::SOUND_HOVER_TICKS),
        super::bruiser::SOUND_QUAKE => Some(super::bruiser::SOUND_QUAKE_TICKS),
        super::flyer::SOUND_REST_A | super::flyer::SOUND_REST_B => {
            Some(super::flyer::SOUND_REST_TICKS)
        }
        _ => None,
    }
}

/// Rótulo de uma animação: (id, primeiro quadro absoluto, número de quadros), conferidos
/// contra os `.ANM` reais nos testes com dados.
pub type Label = (u16, usize, usize);

/// Rótulo em curso, tocado um quadro por tique (para a frente ou ao contrário).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Clip {
    pub label: u16,
    /// Quadros já tocados desde o começo.
    pub step: usize,
    pub reverse: bool,
}

impl Clip {
    pub fn new(label: u16, reverse: bool) -> Self {
        Self {
            label,
            step: 0,
            reverse,
        }
    }

    /// Deslocamento do quadro mostrado dentro do rótulo de `len` quadros.
    pub fn offset(&self, len: usize) -> usize {
        let last = len.saturating_sub(1);
        if self.reverse {
            last - self.step.min(last)
        } else {
            self.step.min(last)
        }
    }

    /// Passa ao quadro seguinte; `true` se o mostrado era o último (a animação acabou e o
    /// comportamento escolhe o que vem, como a rotina de fim do original).
    pub fn advance(&mut self, len: usize) -> bool {
        if self.step + 1 >= len.max(1) {
            return true;
        }
        self.step += 1;
        false
    }
}

/// (id, início, quadros) do rótulo `id` na tabela.
pub fn label(table: &[Label], id: u16) -> Label {
    table
        .iter()
        .copied()
        .find(|l| l.0 == id)
        .unwrap_or((id, 0, 1))
}

/// Mostra o quadro do `clip` no sprite e devolve o quadro absoluto.
pub fn show(e: &mut Entity, table: &[Label], clip: &Clip) -> usize {
    let (id, start, len) = label(table, clip.label);
    let off = clip.offset(len);
    super::show(e, id, off);
    start + off
}

/// Som de um quadro absoluto pela lista do tipo no `SOUND.TBL`: (quadro, som, prioridade).
pub fn frame_sound(table: &[(usize, u16, u16)], frame: usize) -> Option<(u16, u16)> {
    table
        .iter()
        .find(|(f, _, _)| *f == frame)
        .map(|&(_, id, p)| (id, p))
}

/// Emite o som do quadro `frame` se ele mudou desde `last` (`0x004354dc`: só quando o
/// quadro absoluto muda em relação ao último sonorizado).
pub fn sound_frame(
    table: &[(usize, u16, u16)],
    frame: usize,
    last: &mut Option<usize>,
    at: (i32, i32),
    layer: i32,
    out: &mut Out,
) {
    if *last == Some(frame) {
        return;
    }
    *last = Some(frame);
    if let Some((id, p)) = frame_sound(table, frame) {
        let p = if p == 0 { PRIORITY_FRAME } else { p };
        out.events.push(Event::Sound(Cue::at(id, p, at, layer)));
    }
}

/// Dano pendente (`+0x24c` e vizinhos) e se veio de uma bala do jogador (crédito).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Wound {
    pub pending: Pending,
    pub by_player: bool,
}

impl Wound {
    pub fn add(&mut self, p: &Pending, by_player: bool) {
        self.pending.add(p);
        self.by_player |= by_player;
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    /// Esvazia (o original zera natureza, reação e dano depois de aplicar).
    pub fn take(&mut self) -> Wound {
        std::mem::take(self)
    }
}

/// Objeto que a cena cria a pedido de um inimigo (precisa de animações e BRDs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Spawn {
    /// Projétil inimigo (tipos 21, 99, 100, 106) com velocidade em px/tique.
    Shot {
        kind: u32,
        at: (i32, i32),
        vel: (i32, i32),
        layer: i32,
    },
    /// Mina flutuante de um gerador (`0x41e497`).
    Mine {
        at: (i32, i32),
        variant: usize,
        layer: i32,
        owner: u32,
    },
    /// Fogo no chão do Bruiser (`0x4180d0`): base (centro x, y do chão) e altura.
    Fire {
        base: (i32, i32),
        height: i32,
        layer: i32,
    },
    /// Dardo (tipo 29) de um gerador, velocidade em px/tique; rótulo 2 na vertical.
    Dart {
        at: (i32, i32),
        vel: (i32, i32),
        layer: i32,
        vertical: bool,
    },
    /// Formiga (109) de um formigueiro (`0x414bd4`).
    Ant {
        at: (i32, i32),
        variant: i32,
        layer: i32,
        mirror: bool,
    },
}

/// Soma o dano a um inimigo desta fatia; `false` se não é um deles.
pub fn hurt(e: &mut Entity, p: &Pending, by_player: bool) -> bool {
    let w = match &mut e.behaviour {
        // personagem da IA: o dano vai ao corpo (armadura, reações e morte do jogador)
        Behaviour::AiPlayer(a) => {
            a.body.hurt(*p);
            return true;
        }
        Behaviour::Sentinel(s) => &mut s.wound,
        Behaviour::HopMine(s) => &mut s.wound,
        Behaviour::MineGenerator(s) => &mut s.wound,
        Behaviour::SecurityRobot(s) => &mut s.wound,
        Behaviour::Bruiser(s) => &mut s.wound,
        Behaviour::Death(s) => &mut s.wound,
        Behaviour::Chameleon(s) => &mut s.wound,
        Behaviour::Flyer(s) => &mut s.wound,
        Behaviour::DartGenerator(s) => &mut s.wound,
        Behaviour::TheEntity(s) => &mut s.wound,
        Behaviour::Ant(s) => &mut s.wound,
        _ => return false,
    };
    w.add(p, by_player);
    true
}

/// Tipos que entram nas estatísticas de fim de fase (`0x43cfcf`, `0x43d0f2`…`0x43d1e8`):
/// 3, 4, 7, 8, 10, 11, 89, 102–105 e 109.
pub const COUNTED: [u32; 12] = [3, 4, 7, 8, 10, 11, 89, 102, 103, 104, 105, 109];

/// Entra no total: tipo contado e não é criado em jogo — mina de gerador, clone do Camaleão
/// ou formiga de formigueiro (ligam `0x10` e são liberados ao morrer, `0x435b63` →
/// `0x433e2c`: não ficam nas listas que o placar percorre).
pub fn counted(e: &Entity) -> bool {
    COUNTED.contains(&e.kind)
        && !match &e.behaviour {
            Behaviour::Mine(m) => m.owner != 0,
            Behaviour::Chameleon(c) => c.clone_of != 0,
            Behaviour::Ant(a) => a.spawned,
            _ => false,
        }
}

/// Morte que conta para o jogador: no solo (`0x497300 = 1`) quem não tem dono do golpe
/// (`+0x1b0 = 0`) é creditado ao jogador 1, então todo objeto contado com vida ≤ 0 conta,
/// explosões e contato incluídos. Só os tipos com comportamento portado (os demais nunca
/// morrem aqui); o olho-laser, cuja vida não é lida, conta quando morre (some com vida 0).
pub fn killed(e: &Entity) -> bool {
    counted(e)
        && match &e.behaviour {
            Behaviour::LaserEye(_) => !e.alive && e.health <= 0,
            Behaviour::Mine(_)
            | Behaviour::Sentinel(_)
            | Behaviour::HopMine(_)
            | Behaviour::MineGenerator(_)
            | Behaviour::SecurityRobot(_)
            | Behaviour::Bruiser(_)
            | Behaviour::Death(_)
            | Behaviour::Chameleon(_)
            | Behaviour::Flyer(_)
            | Behaviour::Ant(_) => e.health <= 0,
            _ => false,
        }
}

/// Mortos da fase (`0x497348`).
pub fn kills(entities: &[Entity]) -> i32 {
    entities.iter().filter(|e| killed(e)).count() as i32
}

/// Total da fase (`0x497330`), contado na montagem.
pub fn kills_total(entities: &[Entity]) -> i32 {
    entities.iter().filter(|e| counted(e)).count() as i32
}

/// O Jake como alvo possível: vivo, em jogo e na camada `layer` (caixa).
pub fn jake_target(ctx: &Ctx, layer: i32) -> Option<Rect> {
    ctx.jake_bounds().filter(|_| ctx.jake_layer == layer)
}

pub fn center(r: &Rect) -> (i32, i32) {
    (r.x + r.w / 2, r.y + r.h / 2)
}

fn inside(r: &Rect, p: (i32, i32)) -> bool {
    p.0 >= r.x && p.0 < r.x + r.w && p.1 >= r.y && p.1 < r.y + r.h
}

/// Linha de visão (`0x433aa3`/`0x43735d`): 16 amostras do segmento entre `a` e `b` fora dos
/// sólidos do mundo do Jake (células e objetos sólidos da camada dele), ignorando as que
/// caem nas caixas `skip` (quem olha e o alvo).
pub fn line_of_sight(ctx: &Ctx, a: (i32, i32), b: (i32, i32), skip: &[Rect]) -> bool {
    line_of_sight_on(ctx, ctx.jake_layer, a, b, skip)
}

/// A linha de visão na camada `layer`: a do Jake vê o mundo dele (células e objetos); outra
/// camada (o OSR que segue a última posição vista depois que o Jake trocou de camada) vê só
/// as células dela.
pub fn line_of_sight_on(
    ctx: &Ctx,
    layer: i32,
    a: (i32, i32),
    b: (i32, i32),
    skip: &[Rect],
) -> bool {
    let world = if layer == ctx.jake_layer {
        ctx.world
    } else {
        ctx.cells_for(layer)
    };
    (1..16).all(|i| {
        let p = (a.0 + (b.0 - a.0) * i / 16, a.1 + (b.1 - a.1) * i / 16);
        if skip.iter().any(|r| inside(r, p)) {
            return true;
        }
        !world.blocked(&Rect {
            x: p.0,
            y: p.1,
            w: 1,
            h: 1,
        })
    })
}

/// Pontos sorteados em ±`spread` px em torno de `at` (as explosões em série da morte).
pub fn scatter(ctx: &mut Ctx, at: (i32, i32), spread: i32) -> (i32, i32) {
    let s = spread.max(0) as u32;
    (
        at.0 + ctx.rng.below(2 * s + 1) as i32 - spread,
        at.1 + ctx.rng.below(2 * s + 1) as i32 - spread,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviour::test_support::{ctx, ctx_with_jake};

    #[test]
    fn clip_plays_forward_and_back_and_reports_the_end() {
        let mut c = Clip::new(2, false);
        let offs: Vec<usize> = (0..3)
            .map(|_| {
                let o = c.offset(3);
                assert!(!c.advance(3) || o == 2);
                o
            })
            .collect();
        assert_eq!(offs, vec![0, 1, 2]);
        assert!(c.advance(3), "acabou depois do último quadro");
        let mut r = Clip::new(2, true);
        assert_eq!(r.offset(3), 2);
        r.advance(3);
        assert_eq!(r.offset(3), 1);
        r.advance(3);
        assert_eq!(r.offset(3), 0);
        assert!(r.advance(3));
        assert!(Clip::new(1, false).advance(1), "um quadro só acaba já");
    }

    #[test]
    fn line_of_sight_skips_own_box_and_sees_walls() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(300, 0);
        let own = Rect {
            x: 90,
            y: 0,
            w: 40,
            h: 40,
        };
        let walled = crate::World::new(
            w.solids()
                .iter()
                .copied()
                .chain([own])
                .chain([Rect {
                    x: 200,
                    y: 0,
                    w: 5,
                    h: 40,
                }])
                .collect(),
        );
        let c = ctx(&j, &w, &on);
        assert!(line_of_sight(&c, (100, 20), (300, 20), &[own]));
        let cw = ctx(&j, &walled, &on);
        assert!(
            !line_of_sight(&cw, (100, 20), (180, 20), &[]),
            "sem pular a própria caixa"
        );
        assert!(line_of_sight(&cw, (100, 20), (180, 20), &[own]));
        assert!(
            !line_of_sight(&cw, (100, 20), (300, 20), &[own]),
            "parede em x 200"
        );
    }

    #[test]
    fn every_counted_death_counts_but_generator_mines_never() {
        use crate::behaviour::test_support::logic;
        let level_mine = super::super::mine_generator::mine((0, 0), 0, 0, 0);
        let freed = super::super::mine_generator::mine((0, 0), 0, 0, 7);
        let mut hop = logic(crate::entity::HOP_MINE, 0, 0, &[]);
        hop.behaviour = Behaviour::HopMine(Default::default());
        hop.health = 10;
        let unported = logic(102, 0, 0, &[]);
        let barrel = logic(crate::entity::BARREL, 0, 0, &[]);
        let mut all = vec![level_mine, freed, hop, unported, barrel];
        assert_eq!(
            kills_total(&all),
            3,
            "mina da fase, saltadora e o 102 sem comportamento"
        );
        assert_eq!(kills(&all), 0);
        for e in all.iter_mut() {
            e.health = 0;
            e.alive = false;
        }
        assert_eq!(
            kills(&all),
            2,
            "a mina de gerador e o tipo não portado não contam"
        );
        assert!(kills(&all) <= kills_total(&all));
    }

    #[test]
    fn frame_sounds_fire_once_per_new_frame() {
        let table = [(5, 3062, 0), (9, 3060, 3)];
        let mut last = None;
        let mut out = Out::default();
        for f in [4, 5, 5, 9] {
            sound_frame(&table, f, &mut last, (1, 2), 0, &mut out);
        }
        let ids: Vec<_> = out.events.iter().filter_map(Event::sound_id).collect();
        assert_eq!(ids, vec![3062, 3060]);
    }
}
