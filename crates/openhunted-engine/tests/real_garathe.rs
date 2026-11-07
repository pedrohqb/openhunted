// SPDX-License-Identifier: GPL-3.0-or-later
//! O Garathe sobre os dados reais (OPENHUNTED_DATA): a `T_M_002` (primeiro tutorial do
//! Garathe) carrega com ele controlável, corre e pula pelas tabelas e é vencida por entrada
//! roteirizada; fuzz nas fases `p=2` solo.

use openhunted_engine::character::Character;
use openhunted_engine::input::{DOWN, JUMP, LEFT, RIGHT, SHOOT, UP};
use openhunted_engine::jake::{JumpKind, garathe, zone};
use openhunted_engine::physics::CONTACT_SCENERY;
use openhunted_engine::scene::player_character;
use openhunted_engine::{Assets, Outcome, Scene, State};
use openhunted_formats::{GameData, Level, open_data};

fn data() -> Option<GameData> {
    let Some(p) = std::env::var_os("OPENHUNTED_DATA") else {
        eprintln!("pulado: OPENHUNTED_DATA não definido");
        return None;
    };
    Some(open_data(std::path::Path::new(&p)).unwrap())
}

fn level(gd: &GameData, name: &str) -> Level {
    Level::parse(&gd.levels().unwrap().get(name).unwrap()).unwrap()
}

fn load(name: &str) -> Option<Scene> {
    let gd = data()?;
    let l = level(&gd, name);
    let mut a = Assets::new(gd.resources().unwrap());
    Some(Scene::build(&l, &mut a).unwrap())
}

#[test]
fn t_m_002_has_a_controllable_garathe_that_runs_and_jumps() {
    let Some(mut s) = load("T_M_002.LEV") else {
        return;
    };
    assert!(s.warnings.is_empty(), "{:?}", s.warnings);
    let j = s.jake.as_ref().expect("jogador");
    assert_eq!(j.character(), Character::Garathe);
    assert_eq!((j.health.hp, j.health.armor), (100, 100));
    // os rótulos dos estados existem no `MINOTAUR.ANM`
    let anim = s.jake_sprite.as_ref().unwrap().anim.clone();
    for l in garathe::ALL_LABELS
        .iter()
        .copied()
        .chain(openhunted_engine::jake::weapons::labels_of(
            Character::Garathe,
        ))
    {
        assert!(
            openhunted_engine::AnimPlayer::at_label(&anim, l).is_some(),
            "rótulo {l} ausente"
        );
    }
    s.run_ticks(5, 0);
    let x0 = s.jake.as_ref().unwrap().body.x;
    s.run_ticks(2, RIGHT);
    let x1 = s.jake.as_ref().unwrap().body.x;
    s.run_ticks(1, RIGHT);
    let j = s.jake.as_ref().unwrap();
    assert_eq!(j.state(), State::Run);
    assert_eq!(j.body.x - x1, 15, "corrida a 15 px/tique (de {x0})");
    s.run_ticks(1, RIGHT | JUMP);
    let j = s.jake.as_ref().unwrap();
    assert_eq!(
        j.state(),
        State::Jump(JumpKind::Medium),
        "chão adiante: médio"
    );
    assert_eq!(j.body.vx, 13 * 8);
    assert_eq!(j.label(), 22);
}

trait Ticks {
    fn run_ticks(&mut self, n: u32, input: u32);
}

impl Ticks for Scene {
    fn run_ticks(&mut self, n: u32, input: u32) {
        for _ in 0..n {
            self.tick(input);
        }
    }
}

/// Piloto: um tique por vez, sem deixar o Garathe morrer nem o tempo estourar.
struct Pilot {
    s: Scene,
    t: u32,
    /// Tiques em escalada (52/53).
    climbed: u32,
}

impl Pilot {
    fn tick(&mut self, input: u32) {
        self.s.tick(input);
        self.t += 1;
        let j = self.s.jake.as_ref().unwrap();
        self.climbed += u32::from(matches!(j.state(), State::Climb(_)));
        if std::env::var_os("OH_TRACE").is_some() {
            eprintln!(
                "{:5} {input:#06x} ({}, {}) {:?} q{} vida {}",
                self.t,
                j.body.x,
                j.body.y,
                j.state(),
                j.frame(),
                j.health.hp
            );
        }
        assert!(
            !j.is_dead(),
            "o Garathe morreu no tique {} em ({}, {})",
            self.t,
            j.body.x,
            j.body.y
        );
        assert!(self.t < 6000, "tempo demais");
    }
    fn run(&mut self, n: u32, input: u32) {
        for _ in 0..n {
            self.tick(input);
        }
    }
    fn center(&self) -> (i32, i32) {
        self.s.focus().unwrap()
    }
    fn feet(&self) -> i32 {
        let b = self.s.jake.as_ref().unwrap().body.bounds();
        b.y + b.h
    }
    fn on_ground(&self) -> bool {
        self.s.jake.as_ref().unwrap().body.on_ground
    }
    fn settle(&mut self) {
        for _ in 0..80 {
            if self.on_ground() && self.s.jake.as_ref().unwrap().body.vy == 0 {
                break;
            }
            self.tick(0);
        }
        self.run(3, 0);
    }
    /// Anda até o centro ficar a até 8 px de `x`, atacando as minas que chegam perto.
    fn go_x(&mut self, x: i32) {
        for _ in 0..600 {
            let c = self.center().0;
            if (c - x).abs() <= 8 {
                self.settle();
                return;
            }
            if self.fight(110, 200) {
                continue;
            }
            self.tick(if c < x { RIGHT } else { LEFT });
        }
        panic!("não chegou a x = {x} (em {:?})", self.center());
    }
    /// Anda para `side` segurando CIMA (cai do vão) até agarrar a rede: escalada 52/53.
    fn grab_net(&mut self, side: u32) {
        for _ in 0..80 {
            if matches!(self.s.jake.as_ref().unwrap().state(), State::Climb(_)) {
                return;
            }
            self.tick(side | UP);
        }
        panic!("não agarrou a rede (em {:?})", self.center());
    }
    /// Segura `key` (CIMA ou BAIXO, na rede) até os pés passarem de `y`.
    fn climb(&mut self, key: u32, y: i32) {
        for _ in 0..400 {
            let f = self.feet();
            if (key == UP && f < y) || (key == DOWN && f > y) {
                return;
            }
            self.tick(key);
        }
        panic!("não chegou a {y} (pés em {})", self.feet());
    }
    /// Ataca as minas vivas da camada a menos de `range` px (facas, ou garras/clava sem
    /// facas). Devolve se havia alvo.
    fn fight(&mut self, range: i32, max: u32) -> bool {
        let mut any = false;
        for _ in 0..max {
            let c = self.center();
            let layer = self.s.jake_layer;
            let target = self
                .s
                .entities
                .iter()
                .filter(|e| e.alive && e.layer == layer && e.kind == 102 && e.health > 0)
                .map(|e| {
                    let m = e.center();
                    (m.0 - c.0, m.1 - c.1)
                })
                .filter(|(dx, dy)| dx * dx + dy * dy < range * range)
                .min_by_key(|(dx, dy)| dx * dx + dy * dy);
            let Some((dx, dy)) = target else {
                return any;
            };
            any = true;
            let side = if dx > 0 { RIGHT } else { LEFT };
            let aim = if dy < -2 * dx.abs() - 20 {
                UP
            } else if dy < -30 {
                UP | side
            } else if dy > 40 {
                DOWN | side
            } else {
                side
            };
            let fire = if self.t % 10 < 8 { SHOOT } else { 0 };
            self.tick(aim | fire);
        }
        any
    }
    fn win(&mut self) {
        self.settle();
        let c = self.center().0;
        let layer = self.s.jake_layer;
        let target = self
            .s
            .prop_zones
            .iter()
            .filter(|z| z.layer == layer && z.kind & zone::EXIT != 0)
            .map(|z| z.rect.x + z.rect.w / 2)
            .min_by_key(|x| (x - c).abs())
            .expect("zona de saída");
        self.go_x(target);
        self.tick(UP);
        assert!(
            self.s.entities.iter().any(|e| e.kind == 5 && e.on),
            "a saída não ligou (objetivos?)"
        );
        self.run(60, 0);
        assert_eq!(self.s.outcome, Some(Outcome::Victory));
    }
}

/// T_M_002 ("Garathe Tutorial 1"): apanha as facas (x 204), corre para a direita pelo
/// corredor de cima (facas nas minas) até o vão da rede de x ≈ 1216, cai nele segurando CIMA,
/// agarra a rede (escalada 52/53) e desce ao corredor do meio, soltando-se com PULAR (5); faz
/// o mesmo na rede de x ≈ 830 até o corredor de baixo; segue para a esquerda até a saída
/// (x ≈ 160).
#[test]
fn t_m_002_can_be_won() {
    let Some(s) = load("T_M_002.LEV") else {
        return;
    };
    let mut p = Pilot {
        s,
        t: 0,
        climbed: 0,
    };
    p.run(10, 0);
    p.go_x(225);
    assert!(
        p.s.jake.as_ref().unwrap().inventory.ammo(4) > 0,
        "pegou as facas"
    );
    p.go_x(1160);
    p.grab_net(RIGHT);
    p.climb(DOWN, 365);
    let first = p.climbed;
    assert!(first > 0, "desceu pela 1ª rede");
    p.tick(JUMP);
    p.settle();
    p.go_x(880);
    p.grab_net(LEFT);
    p.climb(DOWN, 490);
    assert!(p.climbed > first, "desceu pela 2ª rede");
    p.tick(JUMP);
    p.settle();
    p.win();
}

/// Entrada aleatória em todas as fases `p=2` solo: sem pânico e nunca dentro do cenário.
#[test]
fn fuzz_random_input_on_all_garathe_levels() {
    let Some(gd) = data() else { return };
    let mut seed: u32 = 0x4d49_4e4f;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        seed
    };
    let names: Vec<String> = gd
        .levels()
        .unwrap()
        .names()
        .into_iter()
        .filter(|n| {
            let u = n.to_ascii_uppercase();
            u.ends_with(".LEV") && u.contains("_M_")
        })
        .collect();
    let mut played = 0;
    for name in names {
        let l = level(&gd, &name);
        if player_character(&l) != Some(Character::Garathe) {
            continue;
        }
        played += 1;
        let mut a = Assets::new(gd.resources().unwrap());
        let mut s = Scene::build(&l, &mut a).unwrap();
        if let Some(j) = s.jake.as_mut() {
            for w in 2..=6 {
                j.inventory.slots[w] = (1, 99);
            }
        }
        let mut input = 0;
        let mut hist = std::collections::VecDeque::new();
        for t in 0..2000 {
            if t % 7 == 0 {
                input = next() & (UP | DOWN | LEFT | RIGHT | SHOOT | JUMP | 0x400 | 0x40000);
            }
            s.tick(input);
            let j = s.jake.as_ref().unwrap();
            hist.push_back((input, j.state(), j.body.bounds(), j.body.vx, j.body.vy));
            if hist.len() > 12 {
                hist.pop_front();
            }
            assert!(
                j.box_matches_state(),
                "{name}: caixa errada no tique {t}: {hist:#?}"
            );
            // só o cenário: um objeto sólido que se move (porta, bloco) pode encostar nele
            if j.is_active() {
                assert!(
                    s.world.contact(&j.body.bounds()) & CONTACT_SCENERY == 0,
                    "{name}: dentro de sólido no tique {t}: {hist:#?}"
                );
            }
        }
    }
    assert!(played >= 20, "fases do Garathe: {played}");
}
