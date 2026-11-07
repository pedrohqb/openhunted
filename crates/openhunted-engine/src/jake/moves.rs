// SPDX-License-Identifier: GPL-3.0-or-later
//! Movimentos da M6b: deslocamento pelo esqueleto (`docs/re/ske.md`), zonas PR das células
//! (`0x004075af`), cordas e escadas (estados 3, 8, 30, 32), troca de camada (11, 12), andar
//! furtivo (2), esconder-se (164–166), agachar e engatinhar (85–87), barra (103–107),
//! aterrissagens (26, 34, 35, 77–79) e o especial BAIXO+ATIRAR (9).

use std::rc::Rc;

use openhunted_formats::Skeleton;

use super::{AnimMode, Climb, Jake, JumpKind, Landing, SkeReq, State};
use crate::damage::Pending;
use crate::input::{DOWN, JUMP, LEFT, RIGHT, SHOOT, STEALTH, UP};
use crate::physics::{CONTACT_SCENERY, Rect, SUB, World};

/// Bits do tipo das zonas PR (`docs/re/jake-escalada.md`).
pub mod zone {
    /// Porta de camada: CIMA → camada − 1 (estado 12).
    pub const LAYER_UP: u16 = 0x01;
    /// Porta de camada: BAIXO → camada + 1 (estado 11).
    pub const LAYER_DOWN: u16 = 0x02;
    /// Corda, escada ou rede.
    pub const ROPE: u16 = 0x04;
    /// Empurrar (estado 101).
    pub const PUSH: u16 = 0x20;
    /// Diante da saída.
    pub const EXIT: u16 = 0x40;
}

/// Caixa de engatinhar (`JK_CRAWL.BRD`) e de puxar o corpo na barra (`JK_BARUP.BRD`), de
/// reserva quando os BRDs não estão nos dados.
pub const CRAWL_BOX: Rect = Rect {
    x: 2,
    y: 24,
    w: 18,
    h: 23,
};
pub const BAR_BOX: Rect = Rect {
    x: 2,
    y: 3,
    w: 18,
    h: 16,
};

/// Vida mínima para o especial (`tiro-jake.md`: "vida > 25").
pub const SPECIAL_MIN_HP: i32 = 25;
// Janela e custo do especial (Jake: 1..5 e 3 por tique, `+0x24c = 3`): no perfil.
/// Dano do especial nos "demais" por tique (linha `(1, 0, 9)` do `DAMAGE.TBL`).
pub const SPECIAL_DAMAGE: i32 = 15;
/// Alcance do especial: distância entre centros < 70 px (`0x00427643`).
pub const SPECIAL_RADIUS: i32 = 70;

/// Espaço livre acima para a decisão em pé no 77 (`0x0040718d(0, −25)`).
const HEADROOM_77: i32 = 25;
/// Sondas de 4 px (corda, barra, engatinhar).
const PROBE: i32 = 4;
/// Controle no ar dos pulos parado e para cima (`0x00408205`): 8 px/tique.
const AIR_CONTROL: i32 = 8;
/// Quadro até o qual o pulo parado/para cima pode virar pulo lançado.
const LAUNCH_WINDOW: usize = 10;

/// Zona PR de uma célula, em px da fase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Zone {
    pub layer: i32,
    /// Máscara de bits (`zone::*`).
    pub kind: u16,
    pub rect: Rect,
}

/// O que o Jake enxerga da fase num tique.
#[derive(Debug, Clone, Copy)]
pub struct Env<'a> {
    /// Sólidos da camada dele (cenário e objetos).
    pub world: &'a World,
    /// Zonas PR de todas as camadas.
    pub zones: &'a [Zone],
    /// Número de camadas da fase (`grid_z`).
    pub layers: i32,
}

impl<'a> Env<'a> {
    /// Só os sólidos: sem zonas, uma camada.
    pub fn new(world: &'a World) -> Self {
        Self {
            world,
            zones: &[],
            layers: 1,
        }
    }
}

/// O especial disparado: centro da caixa do Jake e a camada (a cena aplica nos objetos).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpecialBlast {
    pub center: (i32, i32),
    pub layer: i32,
}

/// Efeito do especial num objeto pelo tipo (`furtividade.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecialEffect {
    /// Jogadores (1, 2): derrubados.
    Knockdown,
    /// Projéteis e afins: destruídos.
    Destroy,
    /// Os demais: dano da variante 9.
    Damage,
}

impl SpecialBlast {
    pub fn effect(kind: u32) -> SpecialEffect {
        match kind {
            1 | 2 => SpecialEffect::Knockdown,
            19..=21 | 25..=27 | 29 | 36 | 99 | 100 | 106 | 108 => SpecialEffect::Destroy,
            _ => SpecialEffect::Damage,
        }
    }

    /// Dano aos "demais" (`0x0043d629(jake, alvo, 9)`): a linha `(1, 0, 9)` do `DAMAGE.TBL`
    /// (natureza 0, reação 2, 15 de dano, empurrão fixo (2, 0) pelo lado). O módulo 20 do
    /// empurrão (`+0x20`) não é modelado.
    pub fn damage(&self, c: (i32, i32)) -> Pending {
        let dir = if c.0 >= self.center.0 { 1 } else { -1 };
        Pending {
            amount: SPECIAL_DAMAGE,
            shove: true,
            push: (2 * dir, 0),
            ..Default::default()
        }
    }

    /// O centro `c` está ao alcance (distância < 70 px).
    pub fn reaches(&self, c: (i32, i32)) -> bool {
        let (dx, dy) = (
            i64::from(c.0) - i64::from(self.center.0),
            i64::from(c.1) - i64::from(self.center.1),
        );
        dx * dx + dy * dy < i64::from(SPECIAL_RADIUS * SPECIAL_RADIUS)
    }
}

/// Esqueleto do Jake com a escala: `S` = tamanho do quadro de referência do sprite já
/// escalado (22 × 47 no Jake padrão) e `K` = caixa do esqueleto (20 × 57, do cabeçalho).
#[derive(Debug, Clone)]
pub struct Skel {
    pub ske: Rc<Skeleton>,
    pub s: (i32, i32),
    pub k: (i32, i32),
    /// Fator da compensação do desenho (Jake 12/20; Garathe 587/1000, `0x427a3d`).
    pub draw: (i32, i32),
}

impl Skel {
    pub fn new(ske: Rc<Skeleton>, s: (i32, i32)) -> Self {
        let k = (i32::from(ske.width).max(1), i32::from(ske.height).max(1));
        Self {
            ske,
            s,
            k,
            draw: (12, 20),
        }
    }

    fn roots(&self, label: u16) -> &[(i16, i16)] {
        self.ske.entry(label).map_or(&[], |e| &e.roots)
    }

    /// Escala um vetor do esqueleto (x espelhado olhando à direita) para px da tela.
    fn scale(&self, v: (i32, i32), right: bool) -> (i32, i32) {
        let x = if right { -v.0 } else { v.0 };
        (
            x.saturating_mul(self.s.0) / self.k.0,
            v.1.saturating_mul(self.s.1) / self.k.1,
        )
    }

    /// Δ da raiz no quadro `frame` em px/tique com k = 1 (`0x00406ea2` com 0x10): x > 0 é
    /// para a direita, y > 0 para baixo. No primeiro quadro (no último, de trás para a
    /// frente), zero; além da entrada, o último quadro dela.
    pub fn delta(&self, label: u16, frame: usize, back: bool, right: bool) -> (i32, i32) {
        let p = self.roots(label);
        if p.is_empty() {
            return (0, 0);
        }
        let f = frame.min(p.len() - 1);
        let prev = if back { f + 1 } else { f.wrapping_sub(1) };
        let Some(&q) = p.get(prev) else {
            return (0, 0);
        };
        let v = (
            i32::from(p[f].0) - i32::from(q.0),
            i32::from(p[f].1) - i32::from(q.1),
        );
        let (x, y) = self.scale(v, right);
        (x, -y)
    }

    /// Deslocamento do desenho (`+0x16c`, `+0x170`) no quadro `frame`: a raiz acumulada,
    /// escalada e vezes o fator de desenho; `+0x170` com o sinal trocado.
    pub fn offset(&self, label: u16, frame: usize, right: bool) -> (i32, i32) {
        let p = self.roots(label);
        if p.is_empty() {
            return (0, 0);
        }
        let r = p[frame.min(p.len() - 1)];
        let (x, y) = self.scale((i32::from(r.0), i32::from(r.1)), right);
        let (n, d) = self.draw;
        (x * n / d, -(y * n / d))
    }
}

/// BAIXO sozinho (ou com as duas setas de lado) + ATIRAR = especial (`0x43ae4c`).
pub fn wants_special(input: u32) -> bool {
    let sides = input & (LEFT | RIGHT);
    input & SHOOT != 0
        && input & DOWN != 0
        && input & UP == 0
        && (sides == 0 || sides == LEFT | RIGHT)
}

/// Estados de arte espelhada (`0x00422fce`) tratados aqui: o andar furtivo (149).
pub fn mirrored_art(s: State) -> bool {
    matches!(s, State::Stealth)
}

/// Com gravidade (sem o sinalizador 0x8 de `+0x178`).
pub fn gravity(s: State) -> bool {
    !matches!(
        s,
        State::Climb(_)
            | State::Hidden
            | State::BarHang
            | State::BarMove
            | State::BarPull
            | State::BarUp
            | State::BarDown
    )
}

/// Estados em que o Garathe sente o tranco (`0x42859e`): 1, 10, 28, 31, 35–38, 44–46, 58, 76,
/// os de arma e a escalada (52/53, que viram queda). A barra (47, 49–51) não reage.
pub fn joltable(s: State) -> bool {
    matches!(
        s,
        State::Idle
            | State::Run
            | State::Land(Landing::Fall)
            | State::GetUp
            | State::Rise
            | State::Crawl
            | State::Crouch
            | State::StandUp
            | State::Push
            | State::Pushed
            | State::Fire { .. }
            | State::Ready(..)
            | State::Holster(_)
            | State::Recover(_)
            | State::Climb(_)
            | State::Taunt
    )
}

/// Estados "no chão" que viram queda quando o chão some (`0x0042415c`).
pub fn can_fall(s: State) -> bool {
    matches!(
        s,
        State::Idle
            | State::Run
            | State::Push
            | State::RunUp
            | State::Stealth
            | State::Grab
            | State::LetGo
            | State::Crouch
            | State::Crawl
            | State::StandUp
            | State::Hide
            | State::Unhide
            | State::Land(_)
            | State::GetUp
            | State::Rise
            | State::Shake
            | State::Special(_)
            | State::Taunt
    )
}

impl Jake {
    /// Gravidade neste tique: a do estado, salvo o pulo da corda do Garathe (19), parado e
    /// sem gravidade até o quadro 9.
    pub(super) fn gravity_on(&self) -> bool {
        match self.state {
            State::JumpUp(true) if self.is_garathe() => {
                self.ticks as usize >= super::garathe::ROPE_JUMP_FRAME
            }
            s => gravity(s),
        }
    }

    /// OU dos tipos das zonas PR da camada do Jake que cruzam a caixa dele (`0x004075af`).
    pub fn zones_under(&self, env: &Env) -> u16 {
        let b = self.body.bounds();
        env.zones
            .iter()
            .filter(|z| z.layer == self.layer && z.rect.overlaps(&b))
            .fold(0, |acc, z| acc | z.kind)
    }

    /// Há `h` px livres acima da caixa.
    pub(super) fn free_above(&self, world: &World, h: i32) -> bool {
        let b = self.body.bounds();
        !world.blocked(&Rect {
            y: b.y.saturating_sub(h),
            h,
            ..b
        })
    }

    /// Sonda de `d` px para o lado em que o Jake olha.
    fn free_ahead(&self, world: &World, d: i32) -> bool {
        let b = self.body.bounds();
        let x = if self.facing_right {
            b.x.saturating_add(b.w)
        } else {
            b.x.saturating_sub(d)
        };
        !world.blocked(&Rect { x, w: d, ..b })
    }

    /// Contato de uma sonda de `PROBE` px na direção (dx, dy) (`0x0040718d`).
    fn probe(&self, world: &World, dx: i32, dy: i32) -> u32 {
        let b = self.body.bounds();
        let r = match (dx.signum(), dy.signum()) {
            (0, -1) => Rect {
                y: b.y.saturating_sub(PROBE),
                h: PROBE,
                ..b
            },
            (0, 1) => Rect {
                y: b.y.saturating_add(b.h),
                h: PROBE,
                ..b
            },
            (-1, _) => Rect {
                x: b.x.saturating_sub(PROBE),
                w: PROBE,
                ..b
            },
            _ => Rect {
                x: b.x.saturating_add(b.w),
                w: PROBE,
                ..b
            },
        };
        world.contact(&r)
    }

    /// Bateu num teto de cenário puro (`+0x220 = 2`): é uma barra (`0x0040712f`).
    pub(super) fn ceiling_is_bar(&self, world: &World) -> bool {
        self.probe(world, 0, -1) == CONTACT_SCENERY
    }

    /// Caixa do estado (BRD da coluna `+0x01`): engatinhando, `JK_CRAWL`; puxando o corpo
    /// na barra, `JK_BARUP`; senão a padrão.
    fn state_box(&self) -> Rect {
        if self.is_garathe() {
            // `MN_CRAWL` em 36–38, `MN_BARUP` em 49–51
            return match self.state {
                State::Crouch | State::Crawl | State::StandUp => self.box_crawl,
                State::BarPull | State::BarMove | State::BarDown => self.box_bar,
                _ => self.box_default,
            };
        }
        match self.state {
            State::Crouch | State::Crawl | State::StandUp => self.box_crawl,
            State::BarPull | State::BarUp | State::BarDown => self.box_bar,
            _ => self.box_default,
        }
    }

    /// A caixa corrente é a do estado, salvo no ar ou nos estados sem controle (lá a
    /// caixa padrão espera caber; ver `fit_box`).
    pub fn box_matches_state(&self) -> bool {
        self.body.boxr == self.state_box() || !self.controllable() || !self.body.on_ground
    }

    /// Ajusta a caixa ao estado, antes da física (o original troca o BRD com o estado).
    /// Só cresce se couber. Regra (Ruling) para manter "em pé ⇒ caixa padrão": num estado
    /// controlável no chão em que a caixa padrão não cabe (sob um teto baixo), o Jake fica
    /// engatinhando (86), como o original faz no 77/78/86 com teto baixo; no ar e nos
    /// estados sem controle a caixa menor fica até a padrão caber (o pouso decide).
    pub(super) fn fit_box(&mut self, world: &World) {
        let want = self.state_box();
        if self.body.boxr == want {
            return;
        }
        let old = self.body.bounds();
        let mut b = self.body.clone();
        b.boxr = want;
        let new = b.bounds();
        let inside = new.x >= old.x
            && new.y >= old.y
            && new.x + new.w <= old.x + old.w
            && new.y + new.h <= old.y + old.h;
        if inside || !world.blocked(&new) {
            self.body.boxr = want;
        } else if self.body.boxr == self.box_crawl && self.controllable() && self.body.on_ground {
            self.body.vx = 0;
            self.ske_req = None;
            self.set(State::Crawl);
        }
    }

    /// Decisão em pé (`0x00423940`): tiro (tratado antes), pulo, lados (correr, furtivo,
    /// empurrar), CIMA/BAIXO nas zonas PR (camadas, corda), agachar e esconder. Devolve
    /// `false` se a entrada não pede nada (quem chama decide o repouso).
    pub(super) fn decide(
        &mut self,
        input: u32,
        env: &Env,
        jump_edge: bool,
        side: Option<bool>,
    ) -> bool {
        let pressed = |bit: u32| input & bit != 0;
        let world = env.world;
        if input == super::TAUNT_INPUT {
            self.body.vx = 0;
            self.set(State::Taunt);
            return true;
        }
        if jump_edge && self.body.on_ground {
            self.body.vx = 0;
            if side.is_some() {
                match self.choose_jump(world) {
                    // Garathe: PULAR+CIMA+lado → longo direto (`0x428f56`)
                    _ if self.is_garathe() && pressed(UP) => self.launch(JumpKind::Long),
                    JumpKind::Long if self.is_garathe() => self.set(State::RunUp),
                    JumpKind::Extended => self.set(State::RunUp),
                    k => self.launch(k),
                }
            } else if pressed(UP) {
                self.set(State::JumpUp(false));
            } else {
                self.set(State::StandJump);
            }
            return true;
        }
        if side.is_some() {
            if pressed(STEALTH) && self.free_ahead(world, PROBE) {
                self.set(State::Stealth);
                self.ske_req = Some(SkeReq {
                    k: self.profile().k.walk,
                    x: true,
                    y: false,
                    diag: 0,
                });
            } else {
                let next = if self.body.hit_wall || !self.free_ahead(world, PROBE) {
                    State::Push
                } else {
                    State::Run
                };
                let speed = if next == State::Push {
                    self.profile().push
                } else {
                    self.profile().run
                };
                self.body.vx = speed * SUB * self.dir();
                self.set(next);
            }
            return true;
        }
        if !pressed(JUMP) && (pressed(UP) || pressed(DOWN)) {
            self.body.vx = 0;
            if self.zone_action(input, env) {
                return true;
            }
            if !self.body.on_ground {
                return false;
            }
            if pressed(DOWN) {
                self.set(State::Crouch);
            } else {
                self.set(State::Hide);
            }
            return true;
        }
        false
    }

    /// Decisão em pé ou repouso.
    fn decide_or_idle(&mut self, input: u32, env: &Env, jump_edge: bool, side: Option<bool>) {
        if !self.decide(input, env, jump_edge, side) {
            self.body.vx = 0;
            self.set(State::Idle);
        }
    }

    /// Estados parados, correr e empurrar.
    pub(super) fn decide_standing(
        &mut self,
        input: u32,
        env: &Env,
        jump_edge: bool,
        side: Option<bool>,
    ) {
        self.decide_or_idle(input, env, jump_edge, side);
    }

    /// CIMA/BAIXO sobre as zonas PR (`0x004239f5`). A zona de saída (0x40) só segura o Jake
    /// parado: quem a usa é a cena (`Scene::try_exit`).
    fn zone_action(&mut self, input: u32, env: &Env) -> bool {
        let z = self.zones_under(env);
        let (up, down) = (input & UP != 0, input & DOWN != 0);
        if up && z & zone::LAYER_UP != 0 && self.layer > 0 {
            self.state_layer(State::LayerUp(7));
            self.tick_layer(input, env);
            true
        } else if down && z & zone::LAYER_DOWN != 0 && self.layer < env.layers - 1 {
            self.state_layer(State::LayerDown(0));
            self.tick_layer(input, env);
            true
        } else if up && z & zone::ROPE != 0 {
            self.set(State::Grab);
            true
        } else if up && z & zone::EXIT != 0 {
            self.set(State::Idle);
            true
        } else {
            false
        }
    }

    fn state_layer(&mut self, s: State) {
        self.set(s);
        self.body.vx = 0;
        self.body.vy = 0;
    }

    /// Troca de camada (`0x004262c9` / `0x004263ee`), 8 tiques pelo contador `+0x268`.
    fn tick_layer(&mut self, input: u32, env: &Env) {
        self.body.vx = 0;
        let z = self.zones_under(env);
        match self.state {
            State::LayerDown(c) => {
                if input & UP != 0 && z & zone::LAYER_UP != 0 && self.layer > 0 {
                    self.state = State::LayerUp(c);
                    self.label_changed = true;
                    self.frame = 0;
                    return;
                }
                if c == 3 && self.layer < env.layers - 1 {
                    self.layer += 1;
                }
                if c + 1 > 7 {
                    self.set(State::Idle);
                } else {
                    self.state = State::LayerDown(c + 1);
                }
            }
            State::LayerUp(c) => {
                if input & DOWN != 0 && z & zone::LAYER_DOWN != 0 && self.layer < env.layers - 1 {
                    self.state = State::LayerDown(c);
                    self.label_changed = true;
                    self.frame = 0;
                    return;
                }
                if c == 4 && self.layer > 0 {
                    self.layer -= 1;
                }
                if c - 1 < 0 {
                    self.set(State::Idle);
                } else {
                    self.state = State::LayerUp(c - 1);
                }
            }
            _ => {}
        }
    }

    /// No ar, sem PULAR, CIMA ou BAIXO sobre uma corda: agarra.
    pub(super) fn try_grab_in_air(&mut self, input: u32, env: &Env) -> bool {
        if input & JUMP == 0 && input & (UP | DOWN) != 0 && self.zones_under(env) & zone::ROPE != 0
        {
            self.body.vx = 0;
            self.body.vy = 0;
            self.set(State::Climb(Climb::Vertical));
            true
        } else {
            false
        }
    }

    fn fall(&mut self) {
        self.body.vx = 0;
        self.body.vy = 0;
        self.set(State::Fall);
    }

    /// Pulo lançado a partir do pulo parado ou da corda: com CIMA o longo, senão o da sonda.
    fn launch_from(&mut self, input: u32, world: &World, run_up: bool) {
        if input & UP != 0 {
            self.launch(JumpKind::Long);
        } else {
            match self.choose_jump(world) {
                JumpKind::Long if run_up && self.is_garathe() => self.set(State::RunUp),
                JumpKind::Extended if run_up => self.set(State::RunUp),
                k => self.launch(k),
            }
        }
    }

    /// Estados da M6b (os de chão, ar e tiro da M3/M4 ficam em `tick_in`).
    pub(super) fn tick_move(&mut self, input: u32, env: &Env, jump_edge: bool, side: Option<bool>) {
        let pressed = |bit: u32| input & bit != 0;
        let world = env.world;
        match self.state {
            State::JumpUp(true) if self.is_garathe() => {
                // 19: parado até o quadro 9, então vy = −30 com gravidade (`0x4296df`)
                let n = self.ticks as usize;
                self.body.vx = 0;
                if n < super::garathe::ROPE_JUMP_FRAME {
                    self.body.vy = 0;
                } else if n == super::garathe::ROPE_JUMP_FRAME {
                    self.body.vy = super::garathe::ROPE_JUMP_VY * SUB;
                } else if self.anim_done() {
                    self.set(State::Idle);
                }
            }
            State::Land(Landing::Medium | Landing::Long) if self.is_garathe() => {
                // 23 e 26: deslizam pelo esqueleto com k = 1; no fim, parado
                if self.anim_done() {
                    self.body.vx = 0;
                    self.set(State::Idle);
                } else {
                    self.ske_req = Some(SkeReq {
                        k: 1,
                        x: true,
                        y: false,
                        diag: 0,
                    });
                }
            }
            State::Land(Landing::Hard) if self.is_garathe() => {
                // 30 (`0x42951f`): com 20 px livres, decisão em pé; sem eles só anima; fim → 31
                self.body.vx = 0;
                let head = self.free_above(world, self.profile().headroom);
                if head && self.decide(input, env, jump_edge, side) {
                } else if self.anim_done() {
                    self.set(State::GetUp);
                }
            }
            State::GetUp | State::Rise if self.is_garathe() => {
                // 31/35 (`0x429555`, `0x42a248`): teto a menos de 20 px ou BAIXO → 36; senão
                // decisão em pé; fim → parado
                self.body.vx = 0;
                let head = self.free_above(world, self.profile().headroom);
                if !head || pressed(DOWN) {
                    self.set(State::Crawl);
                } else if self.decide(input, env, jump_edge, side) {
                } else if self.anim_done() {
                    self.set(State::Idle);
                }
            }
            State::BarHang | State::BarMove | State::BarPull | State::BarDown | State::BarUp
                if self.is_garathe() =>
            {
                self.tick_bar_garathe(input, world, side)
            }
            State::StandJump | State::JumpUp(_) => {
                if self.frame < LAUNCH_WINDOW && pressed(JUMP) && side.is_some() {
                    self.launch_from(input, world, self.body.on_ground);
                    return;
                }
                if self.state == State::StandJump && pressed(JUMP) && pressed(UP) {
                    self.set_keep_frame(State::JumpUp(false));
                }
                if self.try_grab_in_air(input, env) {
                    return;
                }
                if self.anim_done() {
                    self.body.vx = 0;
                    self.body.vy = 0;
                    self.set(State::Idle);
                    return;
                }
                self.ske_req = Some(SkeReq {
                    k: if self.state == State::StandJump {
                        self.profile().k.stand_jump
                    } else {
                        self.profile().k.jump_up
                    },
                    x: false,
                    y: true,
                    diag: 0,
                });
                self.body.vx = match side {
                    Some(_) if self.free_ahead(world, AIR_CONTROL) => {
                        AIR_CONTROL * SUB * self.dir()
                    }
                    _ => 0,
                };
            }
            State::Land(Landing::Fall) => {
                if !self.decide(input, env, jump_edge, side) && self.anim_done() {
                    self.set(State::Idle);
                }
                if matches!(self.state, State::Land(_)) {
                    self.body.vx = 0;
                }
            }
            State::Land(Landing::Medium | Landing::Long) => {
                // dois quadros por tique; a velocidade usa o Δ do último (k = 1, só x). Se
                // um dos dois `0x423835` passa do último quadro, parado (`0x425946`).
                let n = self.len_of(self.anim_of(self.state).0);
                let t = self.ticks as usize;
                if t >= 1 && 2 * t > n {
                    self.body.vx = 0;
                    self.set(State::Idle);
                    return;
                }
                self.anim_mode = AnimMode::Target((2 * t).saturating_sub(1));
                self.ske_req = Some(SkeReq {
                    k: 1,
                    x: true,
                    y: false,
                    diag: 0,
                });
            }
            State::Land(Landing::Hard) => {
                self.body.vx = 0;
                if self.free_above(world, HEADROOM_77) && self.decide(input, env, jump_edge, side) {
                    return;
                }
                if self.anim_done() {
                    if self.free_above(world, self.profile().headroom) {
                        self.set(State::GetUp);
                    } else {
                        self.set(State::Crawl);
                    }
                }
            }
            State::GetUp => {
                self.body.vx = 0;
                if !self.free_above(world, self.profile().headroom) || pressed(DOWN) {
                    self.set(State::Crawl);
                } else if self.decide(input, env, jump_edge, side) {
                } else if self.anim_done() {
                    self.rng ^= self.rng << 13;
                    self.rng ^= self.rng >> 17;
                    self.rng ^= self.rng << 5;
                    if self.rng & 3 == 0 {
                        self.set(State::Shake);
                    } else {
                        self.set(State::Idle);
                    }
                }
            }
            State::Shake => {
                self.body.vx = 0;
                if !self.decide(input, env, jump_edge, side) && self.anim_done() {
                    self.set(State::Idle);
                }
            }
            State::Stealth => self.decide_or_idle(input, env, jump_edge, side),
            State::Special(_) => self.tick_special(input, world),
            State::Grab => {
                self.body.vx = 0;
                if pressed(SHOOT) {
                    self.set(State::Idle);
                } else if self.anim_done() {
                    if pressed(UP) && self.zones_under(env) & zone::ROPE != 0 {
                        self.set(State::Climb(Climb::Vertical));
                    } else {
                        self.set(State::LetGo);
                    }
                }
            }
            State::LetGo => {
                if !self.decide(input, env, jump_edge, side) {
                    self.body.vx = 0;
                    if self.anim_done() {
                        self.set(State::Idle);
                    }
                }
            }
            State::Climb(_) => self.tick_climb(input, env, side),
            State::LayerDown(_) | State::LayerUp(_) => self.tick_layer(input, env),
            State::Crouch | State::Crawl | State::StandUp => {
                self.tick_crouch(input, env, jump_edge, side)
            }
            State::Hide | State::Hidden | State::Unhide => {
                self.tick_hide(input, env, jump_edge, side)
            }
            State::BarHang | State::BarMove | State::BarPull | State::BarUp | State::BarDown => {
                self.tick_bar(input, world, jump_edge, side)
            }
            _ => self.decide_or_idle(input, env, jump_edge, side),
        }
    }

    /// Especial (`0x0043b895`): CIMA ou um lado só (`test [0x487a2c], 0xd`) voltam na hora
    /// ao `0x0043b43e`; senão `t = +0x260 + 1` e, na janela 0..5 do registro `0x47d7bc`, o
    /// efeito `0x00427643` a cada tique (3 de vida ao Jake por tique, alvos a cada tique);
    /// com `t > 5` (`+0x18`) volta ao `0x0043b43e`, que com BAIXO+ATIRAR recomeça o 9 no
    /// mesmo tique. As duas setas de lado juntas contam como nenhuma (como em
    /// `wants_special`), senão o original recursaria sem fim.
    pub(super) fn tick_special(&mut self, input: u32, world: &World) {
        let State::Special(t) = self.state else {
            return;
        };
        self.body.vx = 0;
        let sides = input & (LEFT | RIGHT);
        let one_side = sides == LEFT || sides == RIGHT;
        let t = t + 1;
        if input & UP != 0 || one_side || t > self.profile().special.end {
            self.state = State::Special(t);
            let shoot = input & SHOOT != 0 && self.body.on_ground;
            if !(shoot && self.start_fire(input, world)) {
                self.set(State::Idle);
            }
            return;
        }
        self.state = State::Special(t);
        let sp = self.profile().special;
        if t < sp.first || t > sp.last {
            return;
        }
        let b = self.body.bounds();
        self.special = Some(SpecialBlast {
            center: (b.x + b.w / 2, b.y + b.h / 2),
            layer: self.layer,
        });
        self.hurt(Pending {
            amount: self.profile().special.self_damage,
            ..Default::default()
        });
    }

    /// Escalar (`0x0042710a`).
    fn tick_climb(&mut self, input: u32, env: &Env, side: Option<bool>) {
        let pressed = |bit: u32| input & bit != 0;
        let world = env.world;
        self.body.vx = 0;
        self.body.vy = 0;
        if self.zones_under(env) & zone::ROPE == 0 || pressed(SHOOT) {
            self.set(State::LetGo);
            return;
        }
        if pressed(JUMP) {
            if side.is_some() {
                self.launch_from(input, world, false);
            } else if pressed(UP) {
                self.set(State::JumpUp(true));
            } else {
                self.set(State::LetGo);
            }
            return;
        }
        let free = |c: u32| c & CONTACT_SCENERY == 0;
        let up_contact = self.probe(world, 0, -1);
        if pressed(UP) && up_contact == CONTACT_SCENERY {
            self.set(State::BarHang);
            return;
        }
        let u = pressed(UP) && free(up_contact);
        let d = pressed(DOWN) && free(self.probe(world, 0, 1));
        let l = pressed(LEFT) && free(self.probe(world, -1, 0));
        let r = pressed(RIGHT) && free(self.probe(world, 1, 0));
        // Ruling: CIMA+BAIXO com um lado (máscara 7, fora da tabela) anda de lado (32)
        let lateral = match (l, r) {
            (true, false) => -1,
            (false, true) => 1,
            _ => 0,
        };
        let vertical = match (u, d) {
            (true, false) => -1,
            (false, true) => 1,
            _ => 0,
        };
        if vertical != 0 {
            self.set(State::Climb(Climb::Vertical));
            if vertical > 0 {
                self.anim_mode = AnimMode::Reverse;
            }
            // diagonal: vx = ±vy (CIMA+ESQ e BAIXO+DIR: vx = vy)
            self.ske_req = Some(SkeReq {
                k: self.profile().k.rope,
                x: false,
                y: true,
                diag: lateral * vertical,
            });
        } else if lateral != 0 {
            self.set(State::Climb(Climb::Side));
            self.ske_req = Some(SkeReq {
                k: self.profile().k.rope,
                x: true,
                y: false,
                diag: 0,
            });
        } else {
            self.anim_mode = AnimMode::Hold;
        }
    }

    /// Agachar, engatinhar e levantar (`0x00426519`, `0x004266a4`, `0x004265c6`).
    fn tick_crouch(&mut self, input: u32, env: &Env, jump_edge: bool, side: Option<bool>) {
        let pressed = |bit: u32| input & bit != 0;
        let world = env.world;
        let head = self.free_above(world, self.profile().headroom);
        self.body.vx = 0;
        match self.state {
            State::Crouch => {
                if pressed(UP) {
                    self.set(State::StandUp);
                } else if jump_edge && self.decide(input, env, jump_edge, side) {
                } else if self.anim_done() {
                    self.set(State::Crawl);
                } else {
                    self.anim_mode = AnimMode::Double;
                }
            }
            State::Crawl => {
                if head && jump_edge && self.decide(input, env, jump_edge, side) {
                } else if !pressed(DOWN) && head {
                    self.set(State::StandUp);
                } else if side.is_some() && self.free_ahead(world, PROBE) {
                    self.ske_req = Some(SkeReq {
                        k: self.profile().k.crawl,
                        x: true,
                        y: false,
                        diag: 0,
                    });
                } else {
                    self.anim_mode = AnimMode::Hold;
                }
            }
            _ => {
                // levantar: ATIRAR (na cena), PULAR e os lados interrompem
                if (jump_edge || side.is_some()) && self.decide(input, env, jump_edge, side) {
                } else if pressed(DOWN) {
                    let z = self.zones_under(env);
                    if z & zone::LAYER_DOWN != 0 && self.layer < env.layers - 1 {
                        self.state_layer(State::LayerDown(0));
                        self.tick_layer(input, env);
                    } else {
                        self.set(State::Crouch);
                    }
                } else if self.anim_done() {
                    self.set(State::Idle);
                }
            }
        }
    }

    /// Esconder-se (`0x00425e82`, `0x00426020`, `0x00425f8a`).
    fn tick_hide(&mut self, input: u32, env: &Env, jump_edge: bool, side: Option<bool>) {
        let pressed = |bit: u32| input & bit != 0;
        self.body.vx = 0;
        if (jump_edge || side.is_some()) && self.decide(input, env, jump_edge, side) {
            return;
        }
        match self.state {
            State::Hide => {
                if pressed(DOWN) {
                    self.set(State::Unhide);
                } else if self.anim_done() {
                    self.set(State::Hidden);
                } else {
                    self.anim_mode = AnimMode::Double;
                }
            }
            State::Hidden => {
                self.body.vy = 0;
                if !pressed(UP) {
                    self.set(State::Unhide);
                }
            }
            _ => {
                if pressed(UP) {
                    self.set(State::Hide);
                } else if self.anim_done() {
                    self.set(State::Idle);
                }
            }
        }
    }

    /// Há barra (cenário) no segmento de 4 px acima de `x`.
    fn bar_at(&self, world: &World, x: i32) -> bool {
        let b = self.body.bounds();
        world.contact(&Rect {
            x,
            y: b.y.saturating_sub(PROBE),
            w: 1,
            h: PROBE,
        }) & CONTACT_SCENERY
            != 0
    }

    /// Barra (`0x00424498`, `0x004247f8`, `0x00424ab4`, `0x004246c4`, `0x00424bb8`).
    fn tick_bar(&mut self, input: u32, world: &World, jump_edge: bool, side: Option<bool>) {
        let pressed = |bit: u32| input & bit != 0;
        self.body.vx = 0;
        self.body.vy = 0;
        if self.probe(world, 0, -1) & CONTACT_SCENERY == 0 || pressed(SHOOT) || jump_edge {
            self.fall();
            return;
        }
        match self.state {
            State::BarHang | State::BarMove => {
                if pressed(JUMP) || pressed(DOWN) {
                    self.fall();
                } else if pressed(UP) {
                    self.set(State::BarPull);
                } else if side.is_some() {
                    let b = self.body.bounds();
                    let front = if self.facing_right {
                        b.x + b.w
                    } else {
                        b.x - 1
                    };
                    let ahead = front + 3 * self.dir();
                    if self.probe(world, self.dir(), 0) & CONTACT_SCENERY != 0 {
                        self.set(State::BarHang);
                    } else if !self.bar_at(world, front) && !self.bar_at(world, ahead) {
                        self.fall();
                    } else {
                        self.set(State::BarMove);
                        self.ske_req = Some(SkeReq {
                            k: self.profile().k.bar,
                            x: true,
                            y: false,
                            diag: 0,
                        });
                    }
                } else {
                    self.set(State::BarHang);
                }
            }
            State::BarPull => {
                if !pressed(UP) && (pressed(JUMP) || pressed(DOWN) || side.is_some()) {
                    self.set(State::BarDown);
                } else if self.anim_done() {
                    self.set(State::BarUp);
                }
            }
            State::BarUp => {
                if !pressed(UP) {
                    self.set(State::BarDown);
                }
            }
            _ => {
                if pressed(UP) {
                    self.set(State::BarPull);
                } else if self.anim_done() {
                    self.set(State::BarHang);
                }
            }
        }
    }

    /// Barra do Garathe (`0x42a29b`, `0x42a4af`, `0x42a7c2`, `0x42a8bb`): pendurado (47);
    /// CIMA ou lado → agarrar (50, uma vez) → mão ante mão (49, k = 6 em x); sem direção →
    /// soltar a mão (51: o 50 ao contrário) → 47. Não há "subir na barra".
    fn tick_bar_garathe(&mut self, input: u32, world: &World, side: Option<bool>) {
        let pressed = |bit: u32| input & bit != 0;
        self.body.vx = 0;
        self.body.vy = 0;
        if self.probe(world, 0, -1) & CONTACT_SCENERY == 0
            || pressed(SHOOT)
            || pressed(JUMP)
            || (pressed(DOWN) && self.state == State::BarHang)
        {
            self.fall();
            return;
        }
        match self.state {
            State::BarHang => {
                if pressed(UP) || side.is_some() {
                    self.set(State::BarPull);
                }
            }
            State::BarPull => {
                if self.anim_done() {
                    self.set(State::BarMove);
                }
            }
            State::BarMove => {
                if side.is_none() {
                    self.set(State::BarDown);
                    return;
                }
                let b = self.body.bounds();
                let front = if self.facing_right {
                    b.x + b.w
                } else {
                    b.x - 1
                };
                let ahead = front + 3 * self.dir();
                if self.probe(world, self.dir(), 0) & CONTACT_SCENERY != 0 {
                    self.anim_mode = AnimMode::Hold;
                } else if !self.bar_at(world, front) && !self.bar_at(world, ahead) {
                    self.fall();
                } else {
                    self.ske_req = Some(SkeReq {
                        k: self.profile().k.bar,
                        x: true,
                        y: false,
                        diag: 0,
                    });
                }
            }
            _ => {
                if self.anim_done() {
                    self.set(State::BarHang);
                }
            }
        }
    }

    /// Está escondido (o teste dos inimigos, `0x0041db0f`, olha os estados 164–166).
    pub fn is_hidden(&self) -> bool {
        matches!(self.state, State::Hide | State::Hidden | State::Unhide)
    }

    /// O estado toca de trás para a frente.
    pub fn reversed(&self) -> bool {
        self.reversed_state(self.state)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use openhunted_formats::test_support::{SynthSkeletonEntry, mkske};

    use super::*;
    use crate::damage::Health;
    use crate::physics::Body;

    /// Raízes sintéticas por rótulo (S = K: 1 unidade = 1 px), com os Δ de `docs/re/ske.md`.
    fn skeleton() -> Skel {
        let ys = |v: &[i16]| v.iter().map(|&y| (0, y)).collect::<Vec<_>>();
        let xs = |v: &[i16]| v.iter().map(|&x| (x, 0)).collect::<Vec<_>>();
        let jump = {
            let mut y = vec![0i16; 9];
            y.extend([6, 12, 16, 17, 15, 11, 2, 0]);
            y.resize(26, 0);
            ys(&y)
        };
        let mut e: Vec<SynthSkeletonEntry> = vec![(0, (0, 0), vec![], 0); 197];
        e[27] = (0, (0, 0), jump.clone(), 0);
        e[16] = (0, (0, 0), jump.clone(), 0);
        e[17] = (0, (0, 0), jump, 0);
        e[31] = (0, (0, 0), xs(&[0, -8]), 0);
        e[34] = (0, (0, 0), xs(&[0, -4, -7, -9, -11, -14, -16, -18, -20]), 0);
        e[35] = (0, (0, 0), xs(&[0, -5, -10]), 0);
        e[149] = (0, (0, 0), xs(&[0, 1, 4, 9, 12, 15, 18, 21, 25]), 0);
        e[30] = (
            0,
            (0, 0),
            ys(&[0, 1, 6, 9, 9, 9, 9, 10, 17, 21, 21, 21, 21]),
            0,
        );
        e[32] = (0, (0, 0), xs(&[0, -8, -15, -17, -22, -25, -25]), 0);
        e[86] = (0, (0, 0), (0..12).map(|i| (-5 * i, 0)).collect(), 0);
        e[107] = (0, (0, 0), (0..16).map(|i| (-2 * i, 0)).collect(), 0);
        let ske = Skeleton::parse(&mkske(1, (20, 57), &e)).unwrap();
        Skel::new(Rc::new(ske), (20, 57))
    }

    fn lengths() -> HashMap<u16, usize> {
        [
            (1, 34),
            (10, 12),
            (27, 26),
            (16, 26),
            (17, 12),
            (29, 9),
            (28, 8),
            (31, 2),
            (80, 8),
            (35, 3),
            (34, 9),
            (25, 1),
            (26, 11),
            (76, 18),
            (77, 13),
            (78, 28),
            (79, 27),
            (100, 6),
            (149, 9),
            (9, 8),
            (3, 7),
            (30, 13),
            (32, 7),
            (11, 12),
            (12, 12),
            (85, 5),
            (86, 12),
            (164, 7),
            (165, 1),
            (103, 1),
            (104, 1),
            (105, 9),
            (107, 16),
            (23, 8),
            (55, 6),
        ]
        .into_iter()
        .collect()
    }

    const BOX: Rect = Rect {
        x: 2,
        y: 3,
        w: 18,
        h: 44,
    };

    fn floor(y: i32) -> Rect {
        Rect {
            x: -10_000,
            y,
            w: 20_000,
            h: 20,
        }
    }

    /// Jake parado no chão em y = 100 (pés), olhando para a esquerda.
    fn jake(world: &World, x: i32) -> Jake {
        let mut j = Jake::new(x, 53, BOX, lengths()).with_skeleton(skeleton());
        j.tick(0, world);
        assert!(j.body.on_ground);
        j
    }

    fn rope(x: i32, y: i32, h: i32) -> Zone {
        Zone {
            layer: 0,
            kind: zone::ROPE,
            rect: Rect { x, y, w: 32, h },
        }
    }

    #[test]
    fn skeleton_delta_and_offset_follow_side_and_scale() {
        let mut sk = skeleton();
        // à esquerda, "para a frente" = x negativo; à direita, espelhado
        assert_eq!(sk.delta(31, 1, false, false), (-8, 0));
        assert_eq!(sk.delta(31, 1, false, true), (8, 0));
        assert_eq!(sk.delta(31, 0, false, true), (0, 0), "primeiro quadro");
        assert_eq!(
            sk.delta(30, 2, false, false),
            (0, -5),
            "y do esqueleto para cima"
        );
        assert_eq!(
            sk.delta(30, 1, true, false),
            (0, 5),
            "de trás para a frente"
        );
        assert_eq!(
            sk.delta(30, 99, false, false),
            (0, 0),
            "além da entrada: último Δ"
        );
        assert_eq!(sk.delta(200, 3, false, false), (0, 0), "sem entrada");
        // escala do Jake padrão: S/K = 22/20 e 47/57; o desenho ainda × 12/20
        sk.s = (22, 47);
        assert_eq!(sk.delta(31, 1, false, true), (8, 0)); // trunc(8·22/20) = 8
        assert_eq!(sk.delta(30, 8, false, false), (0, -5)); // trunc(7·47/57) = 5
        assert_eq!(sk.offset(32, 4, false), (-24 * 12 / 20, 0));
        assert_eq!(sk.offset(30, 9, false), (0, -(17 * 12 / 20)));
    }

    #[test]
    fn standing_jump_follows_the_vy_table_with_air_control() {
        let w = World::new(vec![floor(100)]);
        let mut j = jake(&w, 0);
        let y0 = j.body.y;
        j.tick(JUMP, &w);
        assert_eq!(j.state(), State::StandJump);
        let mut vys = Vec::new();
        for _ in 0..17 {
            j.tick(0, &w);
            vys.push(j.body.y);
        }
        // tiques 1–8 parados; o 9º pede −18 (+2 de gravidade → sobe 16)
        assert_eq!(vys[7], y0);
        assert_eq!(vys[8], y0 - 16);
        assert_eq!(vys[9], y0 - 32);
        assert_eq!(vys[10], y0 - 42);
        assert_eq!(vys[11], y0 - 43);
        assert!(vys[16] == y0, "volta ao chão: {vys:?}");
        // controle no ar: 8 px/tique para o lado apertado
        let mut j = jake(&w, 0);
        j.tick(JUMP, &w);
        let x0 = j.body.x;
        j.tick(RIGHT, &w);
        assert_eq!(j.body.x - x0, 8);
        assert_eq!(j.state(), State::StandJump);
        for _ in 0..30 {
            j.tick(0, &w);
        }
        assert_eq!(j.state(), State::Idle);
    }

    #[test]
    fn jump_up_with_up_and_a_bar_ceiling() {
        // teto de cenário 70 px acima do chão: o pulo para cima pendura na barra (103)
        let w = World::new(vec![
            floor(100),
            Rect {
                x: -100,
                y: 20,
                w: 400,
                h: 10,
            },
        ]);
        let mut j = jake(&w, 0);
        j.tick(UP | JUMP, &w);
        assert_eq!(j.state(), State::JumpUp(false));
        for _ in 0..14 {
            j.tick(UP, &w);
            if j.state() == State::BarHang {
                break;
            }
        }
        assert_eq!(j.state(), State::BarHang);
        assert_eq!(j.body.bounds().y, 30, "encostado no teto");
        // sem gravidade: fica pendurado
        j.tick(0, &w);
        j.tick(0, &w);
        assert_eq!((j.state(), j.body.bounds().y), (State::BarHang, 30));
        // mão ante mão: 107 com k = 4 (8 px por quadro do esqueleto sintético)
        let x0 = j.body.x;
        for _ in 0..5 {
            j.tick(RIGHT, &w);
        }
        assert_eq!(j.state(), State::BarMove);
        assert!(j.body.x > x0);
        // puxar o corpo: caixa JK_BARUP
        j.tick(UP, &w);
        assert_eq!(j.state(), State::BarPull);
        assert_eq!(j.body.boxr, BAR_BOX);
        for _ in 0..12 {
            j.tick(UP, &w);
        }
        assert_eq!(j.state(), State::BarUp);
        j.tick(0, &w);
        assert_eq!(j.state(), State::BarDown);
        for _ in 0..12 {
            j.tick(0, &w);
        }
        assert_eq!((j.state(), j.body.boxr), (State::BarHang, BOX));
        // BAIXO solta
        j.tick(DOWN, &w);
        assert_eq!(j.state(), State::Fall);
        for _ in 0..20 {
            j.tick(0, &w);
        }
        assert!(j.body.on_ground);
        // teto de objeto não é barra
        let w2 = World::with_objects(
            vec![floor(100)],
            vec![Rect {
                x: -100,
                y: 20,
                w: 400,
                h: 10,
            }],
        );
        let mut j = jake(&w2, 0);
        j.tick(UP | JUMP, &w2);
        for _ in 0..30 {
            j.tick(0, &w2);
            assert_ne!(j.state(), State::BarHang);
        }
    }

    #[test]
    fn run_up_moves_by_the_skeleton_then_launches() {
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
            floor(400),
        ]);
        let mut j = jake(&w, 0);
        let x0 = j.body.x;
        j.tick(RIGHT | JUMP, &w);
        assert_eq!((j.state(), j.body.x), (State::RunUp, x0));
        j.tick(RIGHT, &w);
        assert_eq!(j.body.x - x0, 16, "2º tique: 16 px (k = 2)");
        j.tick(RIGHT, &w);
        assert_eq!(j.state(), State::Jump(JumpKind::Extended));
        assert_eq!(j.body.x - x0, 16 + 17);
    }

    #[test]
    fn long_landing_slides_two_frames_per_tick() {
        let w = World::new(vec![floor(100)]);
        let mut j = jake(&w, 0);
        // o tique do pouso (aqui o primeiro, com o estado posto à mão) fica no quadro 0
        j.set(State::Land(Landing::Long));
        let x0 = j.body.x;
        let mut xs = Vec::new();
        for _ in 0..7 {
            j.tick(0, &w);
            xs.push((x0 - j.body.x, j.frame()));
        }
        assert_eq!(
            xs[..5],
            [(0, 0), (4, 1), (6, 3), (9, 5), (11, 7)],
            "vx 4, 2, 3, 2"
        );
        // 5º tique: o segundo `0x423835` passa do último quadro → parado (11 px ao todo)
        assert_eq!(xs[5].0, 11);
        assert_eq!(j.state(), State::Idle);
        let mut j = jake(&w, 0);
        j.set(State::Land(Landing::Medium));
        let x0 = j.body.x;
        for _ in 0..2 {
            j.tick(0, &w);
        }
        assert_eq!(x0 - j.body.x, 5, "35: 5 px no 1º tique");
        j.tick(0, &w);
        assert_eq!((x0 - j.body.x, j.state()), (5, State::Idle), "parado no 2º");
    }

    #[test]
    fn extended_jump_ending_in_the_air_is_a_wild_fall() {
        let w = World::new(vec![floor(1000)]);
        let mut j = jake(&World::new(vec![floor(100)]), 0);
        j.launch(JumpKind::Extended);
        let mut seen = Vec::new();
        for _ in 0..200 {
            j.tick(0, &w);
            if seen.last() != Some(&j.state()) {
                seen.push(j.state());
            }
            if j.state() == State::Idle {
                break;
            }
        }
        assert!(seen.contains(&State::WildFall), "{seen:?}");
        assert!(seen.contains(&State::Land(Landing::Hard)), "{seen:?}");
        assert!(seen.contains(&State::GetUp), "{seen:?}");
        assert_eq!(j.state(), State::Idle);
    }

    #[test]
    fn stealth_walks_by_the_skeleton_with_mirrored_art() {
        let w = World::new(vec![floor(100)]);
        let mut j = jake(&w, 0);
        let x0 = j.body.x;
        j.tick(RIGHT | STEALTH, &w);
        assert_eq!((j.state(), j.label()), (State::Stealth, 149));
        assert!(j.facing_right() && !j.sprite_mirrored());
        for _ in 0..8 {
            j.tick(RIGHT | STEALTH, &w);
        }
        // 3·(1+3+5+3+3+3+3+4) = 75 px, contra 108 correndo
        assert_eq!(j.body.x - x0, 75);
        j.tick(RIGHT, &w);
        assert_eq!(j.state(), State::Run);
    }

    #[test]
    fn rope_grab_climb_and_let_go() {
        let w = World::new(vec![floor(100)]);
        let zones = [rope(0, -200, 300)];
        let env = Env {
            world: &w,
            zones: &zones,
            layers: 1,
        };
        let mut j = jake(&w, 0);
        // BAIXO em pé numa corda agacha
        j.tick_in(DOWN, &env);
        assert_eq!(j.state(), State::Crouch);
        let mut j = jake(&w, 0);
        j.tick_in(UP, &env);
        assert_eq!((j.state(), j.label()), (State::Grab, 3));
        for _ in 0..7 {
            j.tick_in(UP, &env);
        }
        assert_eq!(j.state(), State::Climb(Climb::Vertical));
        let y0 = j.body.y;
        for _ in 0..13 {
            j.tick_in(UP, &env);
        }
        assert_eq!(y0 - j.body.y, 42, "42 px por ciclo (k = 2)");
        // parado na corda: sem gravidade
        let y1 = j.body.y;
        for _ in 0..5 {
            j.tick_in(0, &env);
        }
        assert_eq!(j.body.y, y1);
        // descer toca ao contrário (os últimos quadros do 30 não têm Δ)
        for _ in 0..6 {
            j.tick_in(DOWN, &env);
        }
        assert!(j.body.y > y1);
        // diagonal: vx = ±vy
        let x0 = j.body.x;
        let y2 = j.body.y;
        for _ in 0..4 {
            j.tick_in(UP | RIGHT, &env);
        }
        assert!(j.facing_right());
        assert_eq!(j.body.x - x0, y2 - j.body.y, "45°");
        assert!(j.body.x > x0);
        // de lado: 32
        j.tick_in(LEFT, &env);
        assert_eq!(j.state(), State::Climb(Climb::Side));
        // PULAR+CIMA: pulo para cima saindo da corda (17)
        j.tick_in(JUMP | UP, &env);
        assert_eq!((j.state(), j.label()), (State::JumpUp(true), 17));
        // ATIRAR solta (8 = rótulo 3 ao contrário)
        let mut j2 = jake(&w, 0);
        j2.set(State::Climb(Climb::Vertical));
        j2.tick_in(SHOOT, &env);
        assert_eq!(j2.state(), State::LetGo);
        assert!(j2.reversed());
        // fora da corda: solta e cai
        let mut j3 = jake(&w, 300);
        j3.set(State::Climb(Climb::Vertical));
        j3.body.y = -100;
        j3.tick_in(UP, &env);
        assert_eq!(j3.state(), State::LetGo);
        for _ in 0..40 {
            j3.tick_in(0, &env);
        }
        assert!(j3.body.on_ground);
    }

    #[test]
    fn grabbing_the_rope_in_mid_air() {
        let w = World::new(vec![floor(100)]);
        let zones = [rope(-10, -400, 300)];
        let env = Env {
            world: &w,
            zones: &zones,
            layers: 1,
        };
        let mut j = Jake::new(0, -300, BOX, lengths()).with_skeleton(skeleton());
        j.tick_in(0, &env);
        for _ in 0..5 {
            j.tick_in(0, &env);
        }
        assert_eq!(j.state(), State::Fall);
        // PULAR segurado impede agarrar
        j.tick_in(UP | JUMP, &env);
        assert_eq!(j.state(), State::Fall);
        j.tick_in(UP, &env);
        assert_eq!(j.state(), State::Climb(Climb::Vertical));
        let y = j.body.y;
        j.tick_in(0, &env);
        assert_eq!(j.body.y, y, "parado na corda");
        // Ruling: CIMA+BAIXO com um lado (máscara 7) anda de lado (32)
        j.tick_in(UP | DOWN | RIGHT, &env);
        assert_eq!(j.state(), State::Climb(Climb::Side));
        // arremessado (81/82) também agarra
        let mut j = Jake::new(0, -300, BOX, lengths()).with_skeleton(skeleton());
        j.tick_in(0, &env);
        j.hurt(Pending {
            knockdown: true,
            push: (0, -4),
            ..Default::default()
        });
        j.tick_in(0, &env);
        assert!(matches!(j.state(), State::KnockedDown(_)));
        j.tick_in(DOWN, &env);
        assert_eq!(j.state(), State::Climb(Climb::Vertical));
    }

    #[test]
    fn layer_doors_change_layer_on_the_fourth_tick() {
        let w = World::new(vec![floor(100)]);
        let door = |layer, kind| Zone {
            layer,
            kind,
            rect: Rect {
                x: 5,
                y: 60,
                w: 5,
                h: 32,
            },
        };
        let zones = [
            door(0, zone::LAYER_DOWN),
            door(1, zone::LAYER_UP),
            door(1, zone::LAYER_DOWN),
        ];
        let env = Env {
            world: &w,
            zones: &zones,
            layers: 2,
        };
        let mut j = jake(&w, 0);
        // sem zona na camada 0, CIMA esconde
        j.tick_in(UP, &env);
        assert_eq!(j.state(), State::Hide);
        let mut j = jake(&w, 0);
        let mut layers = Vec::new();
        for _ in 0..9 {
            j.tick_in(DOWN, &env);
            layers.push(j.layer());
        }
        assert_eq!(layers, [0, 0, 0, 1, 1, 1, 1, 1, 1]);
        assert_eq!(
            j.label(),
            85,
            "BAIXO de novo, já sem camada abaixo (2 camadas): agacha"
        );
        // CIMA volta à 0 no 4º tique
        let mut j = jake(&w, 0).with_layer(1);
        let mut layers = Vec::new();
        for _ in 0..8 {
            j.tick_in(UP, &env);
            layers.push(j.layer());
        }
        assert_eq!(layers, [1, 1, 1, 0, 0, 0, 0, 0]);
        // meia-volta durante a 11: CIMA na zona 1 vira 12 com o contador onde está
        let mut j = jake(&w, 0);
        for _ in 0..4 {
            j.tick_in(DOWN, &env);
        }
        assert_eq!(j.layer(), 1);
        j.tick_in(UP, &env);
        assert!(matches!(j.state(), State::LayerUp(_)), "{:?}", j.state());
        for _ in 0..8 {
            j.tick_in(0, &env);
        }
        assert_eq!((j.layer(), j.state()), (0, State::Idle));
    }

    #[test]
    fn hide_with_up_and_no_zone() {
        let w = World::new(vec![floor(100)]);
        let mut j = jake(&w, 0);
        j.tick(UP, &w);
        assert_eq!((j.state(), j.label()), (State::Hide, 164));
        let frames: Vec<usize> = (0..4)
            .map(|_| {
                j.tick(UP, &w);
                j.frame()
            })
            .collect();
        assert_eq!(frames, [2, 4, 6, 0], "dois quadros por tique, depois 165");
        assert_eq!(j.state(), State::Hidden);
        assert!(j.is_hidden());
        j.tick(UP, &w);
        assert_eq!(j.state(), State::Hidden);
        j.tick(0, &w);
        assert_eq!((j.state(), j.label(), j.frame()), (State::Unhide, 164, 6));
        for _ in 0..7 {
            j.tick(0, &w);
        }
        assert_eq!(j.state(), State::Idle);
        // correr sai do esconderijo
        j.tick(UP, &w);
        j.tick(LEFT, &w);
        assert_eq!(j.state(), State::Run);
    }

    #[test]
    fn crouch_crawl_and_headroom() {
        // teto baixo a partir de x = 100: só passa engatinhando
        let w = World::new(vec![
            floor(100),
            Rect {
                x: 100,
                y: 0,
                w: 400,
                h: 70,
            },
        ]);
        let mut j = jake(&w, 0);
        j.tick(DOWN, &w);
        assert_eq!((j.state(), j.label()), (State::Crouch, 85));
        assert_eq!(j.body.boxr, CRAWL_BOX);
        j.tick(DOWN, &w);
        j.tick(DOWN, &w);
        j.tick(DOWN, &w);
        assert_eq!(j.state(), State::Crawl);
        // engatinhando: k = 2, 10 px por quadro (Δ 5)
        let x0 = j.body.x;
        j.tick(DOWN | RIGHT, &w);
        j.tick(DOWN | RIGHT, &w);
        assert_eq!(j.body.x - x0, 20);
        for _ in 0..20 {
            j.tick(DOWN | RIGHT, &w);
        }
        assert!(j.body.x > 120, "passou sob o teto: {}", j.body.x);
        // sob o teto não levanta nem pula
        j.tick(0, &w);
        j.tick(JUMP, &w);
        assert_eq!(j.state(), State::Crawl);
        // fora do teto, soltar BAIXO levanta (87) e volta à caixa padrão
        for _ in 0..40 {
            j.tick(LEFT | DOWN, &w);
        }
        assert!(j.body.x < 60);
        j.tick(0, &w);
        assert_eq!((j.state(), j.label()), (State::StandUp, 85));
        for _ in 0..6 {
            j.tick(0, &w);
        }
        assert_eq!((j.state(), j.body.boxr), (State::Idle, BOX));
    }

    /// Beiral com 35 px livres sobre o chão (23 < 35 < 44): engatinhando passa, em pé não.
    fn overhang() -> World {
        World::new(vec![
            floor(100),
            Rect {
                x: 100,
                y: 0,
                w: 400,
                h: 65,
            },
        ])
    }

    /// Em pé, a caixa é sempre a padrão e nunca cruza o beiral.
    fn assert_box_ok(j: &Jake, w: &World, t: usize) {
        assert!(
            j.box_matches_state(),
            "tique {t}: {:?} com {:?}",
            j.state(),
            j.body.boxr
        );
        if j.body.boxr == BOX {
            assert!(!w.blocked(&j.body.bounds()), "tique {t}: {:?}", j.body);
        }
    }

    #[test]
    fn standing_never_keeps_the_crawl_box_under_an_overhang() {
        let w = overhang();
        let mut j = jake(&w, 0);
        // correndo em pé, para no beiral
        for t in 0..40 {
            j.tick(RIGHT, &w);
            assert_box_ok(&j, &w, t);
        }
        assert!(j.body.bounds().x + j.body.bounds().w <= 100);
        // agacha, solta BAIXO e segura DIR: levanta e corre, mas não passa por baixo
        for t in 0..5 {
            j.tick(DOWN, &w);
            assert_box_ok(&j, &w, t);
        }
        assert_eq!(j.state(), State::Crawl);
        for t in 0..60 {
            j.tick(RIGHT, &w);
            assert_box_ok(&j, &w, t);
        }
        assert!(
            j.body.bounds().x + j.body.bounds().w <= 100,
            "{:?} {:?}",
            j.state(),
            j.body
        );
        // engatinhando passa; sob o beiral, soltar BAIXO e segurar DIR não o põe em pé
        for t in 0..5 {
            j.tick(DOWN, &w);
            assert_box_ok(&j, &w, t);
        }
        for t in 0..20 {
            j.tick(DOWN | RIGHT, &w);
            assert_box_ok(&j, &w, t);
        }
        assert!(j.body.bounds().x > 110, "passou engatinhando: {:?}", j.body);
        for input in [RIGHT, UP, JUMP, SHOOT, UP | RIGHT, 0, LEFT] {
            for t in 0..12 {
                j.tick(input, &w);
                assert_box_ok(&j, &w, t);
            }
        }
        // Ruling: o 85 sob o beiral (posto à mão) não atira nem pula em pé: fica no 86
        let mut j = j.with_inventory(crate::jake::Inventory::from_points(&[
            (0, 0),
            (1, 20),
            (1, 9),
        ]));
        for input in [SHOOT, JUMP] {
            j.set(State::Crouch);
            j.tick(input, &w);
            assert_eq!((j.state(), j.body.boxr), (State::Crawl, CRAWL_BOX));
        }
    }

    /// Teto a 6 px da cabeça em pé: a caixa padrão cabe, mas não há 30 px livres acima da
    /// de engatinhar. Só o 86 exige os 30 px (`jake-movimentos.md`); o 85 e o esconder
    /// atiram e pulam como em pé.
    #[test]
    fn headroom_gate_only_while_crawling() {
        let w = World::new(vec![
            floor(100),
            Rect {
                x: -500,
                y: 0,
                w: 1000,
                h: 50,
            },
        ]);
        let armed = |j: Jake| {
            j.with_inventory(crate::jake::Inventory::from_points(&[
                (0, 0),
                (1, 20),
                (1, 9),
            ]))
        };
        let mut j = armed(jake(&w, 0));
        j.tick(DOWN, &w);
        assert_eq!(j.state(), State::Crouch);
        j.tick(SHOOT, &w);
        assert!(j.is_firing(), "85 atira: {:?}", j.state());
        assert_box_ok(&j, &w, 0);
        let mut j = armed(jake(&w, 0));
        j.tick(DOWN, &w);
        j.tick(JUMP, &w);
        assert_eq!(j.state(), State::StandJump, "85 pula");
        let mut j = armed(jake(&w, 0));
        j.tick(UP, &w);
        assert_eq!(j.state(), State::Hide);
        j.tick(UP | SHOOT, &w);
        assert!(j.is_firing(), "164 atira: {:?}", j.state());
        // no 86 sem 30 px não atira nem pula
        let mut j = armed(jake(&w, 0));
        for _ in 0..5 {
            j.tick(DOWN, &w);
        }
        assert_eq!(j.state(), State::Crawl);
        j.tick(DOWN | SHOOT, &w);
        j.tick(DOWN | JUMP, &w);
        assert_eq!(j.state(), State::Crawl);
    }

    #[test]
    fn knockdown_under_an_overhang_keeps_crawling_and_in_the_open_stands() {
        let w = overhang();
        let mut j = jake(&w, 60);
        for _ in 0..5 {
            j.tick(DOWN, &w);
        }
        for _ in 0..30 {
            j.tick(DOWN | RIGHT, &w);
        }
        assert!(j.body.bounds().x > 110);
        j.hurt(Pending {
            knockdown: true,
            push: (4, -6),
            ..Default::default()
        });
        for t in 0..60 {
            j.tick(0, &w);
            assert_box_ok(&j, &w, t);
        }
        assert!(matches!(j.state(), State::Crawl), "{:?}", j.state());
        // ao ar livre, a derrubada devolve a caixa padrão
        let w = World::new(vec![floor(100)]);
        let mut j = jake(&w, 0);
        for _ in 0..5 {
            j.tick(DOWN, &w);
        }
        assert_eq!(j.state(), State::Crawl);
        j.hurt(Pending {
            knockdown: true,
            push: (4, -6),
            ..Default::default()
        });
        j.tick(0, &w);
        assert!(matches!(j.state(), State::KnockedDown(_)));
        assert_eq!(j.body.boxr, BOX, "a derrubada usa a caixa padrão");
    }

    #[test]
    fn down_shoot_special_hurts_jake_and_reaches_70_px() {
        let w = World::new(vec![floor(100)]);
        let mut j = jake(&w, 0);
        j.health = Health { hp: 50, armor: 0 };
        // o tique da entrada já aplica o efeito (`0x43b43e` chama o `0x43b895`: t = 1)
        j.tick(DOWN | SHOOT, &w);
        assert_eq!((j.state(), j.label()), (State::Special(1), 9));
        let b = j.special.expect("especial");
        let jb = j.body.bounds();
        assert_eq!(b.center, (jb.x + jb.w / 2, jb.y + jb.h / 2));
        assert!(b.reaches((b.center.0 + 69, b.center.1)));
        assert!(!b.reaches((b.center.0 + 50, b.center.1 + 50)));
        // janela 0..5 do registro `0x47d7bc`: efeito em t = 1..5, mesmo soltando as teclas
        for t in 2..=5 {
            j.tick(0, &w);
            assert_eq!(j.state(), State::Special(t));
            assert!(j.special.is_some(), "efeito no t = {t}");
        }
        // t = 6 > 5 (`+0x18`): volta ao `0x43b43e`; sem ATIRAR, parado
        j.tick(0, &w);
        assert_eq!(j.state(), State::Idle);
        assert!(j.special.is_none());
        j.tick(0, &w);
        assert_eq!(
            j.health.hp,
            50 - 5 * crate::character::JAKE.special.self_damage,
            "3 de vida por tique"
        );
        // segurando BAIXO+ATIRAR repete enquanto a vida passar de 25
        let mut j = jake(&w, 0);
        j.health = Health { hp: 50, armor: 0 };
        let mut hits = 0;
        for _ in 0..20 {
            j.tick(DOWN | SHOOT, &w);
            hits += usize::from(j.special.is_some());
        }
        assert_eq!(hits, 10, "dois ciclos de 5: no 11º tique a vida é 20");
        assert!(j.health.hp <= SPECIAL_MIN_HP);
        // CIMA ou um lado interrompem na hora
        for stop in [UP, RIGHT, LEFT] {
            let mut j = jake(&w, 0);
            j.health = Health { hp: 50, armor: 0 };
            j.tick(DOWN | SHOOT, &w);
            j.tick(stop, &w);
            assert!(!matches!(j.state(), State::Special(_)), "{stop:#x}");
            assert!(j.special.is_none());
        }
        assert_eq!(SpecialBlast::effect(19), SpecialEffect::Destroy);
        assert_eq!(SpecialBlast::effect(1), SpecialEffect::Knockdown);
        assert_eq!(SpecialBlast::effect(102), SpecialEffect::Damage);
        let d = b.damage((b.center.0 - 10, b.center.1));
        assert_eq!((d.amount, d.shove, d.push), (SPECIAL_DAMAGE, true, (-2, 0)));
        assert!(!d.explosion, "natureza 0: o barril não explode");
        // com vida ≤ 25 é o tiro para baixo comum
        let mut j = jake(&w, 0).with_inventory(crate::jake::Inventory::from_points(&[
            (0, 0),
            (1, 20),
            (1, 9),
        ]));
        j.health = Health { hp: 25, armor: 0 };
        j.tick(DOWN | SHOOT, &w);
        assert!(j.is_firing());
        assert!(!wants_special(DOWN | LEFT | SHOOT));
        assert!(wants_special(DOWN | LEFT | RIGHT | SHOOT));
    }

    #[test]
    fn draw_offset_follows_the_root() {
        let w = World::new(vec![floor(100)]);
        let mut j = jake(&w, 0);
        assert_eq!(j.draw_offset(), (0, 0));
        j.tick(DOWN, &w);
        for _ in 0..3 {
            j.tick(DOWN, &w);
        }
        assert_eq!(j.state(), State::Crawl);
        j.tick(DOWN | LEFT, &w);
        j.tick(DOWN | LEFT, &w);
        // rótulo 86, quadro 2: raiz x = −10 → +0x16c = −6, desenho em x + 6
        assert_eq!((j.frame(), j.draw_offset()), (2, (6, 0)));
        let b = Body::new(0, 0, BOX);
        assert_eq!(b.boxr, BOX);
    }
}
