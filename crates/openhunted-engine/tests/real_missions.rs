// SPDX-License-Identifier: GPL-3.0-or-later
//! As missões tutoriais `T_J_003`, `T_J_005` e `T_J_007` jogadas de ponta a ponta sobre os
//! dados reais (OPENHUNTED_DATA), por rotas roteirizadas (`docs/re/missoes-tutorial.md`):
//! sem tirar perigos, só com a entrada do jogador.

use openhunted_engine::input::{DOWN, JUMP, LEFT, RIGHT, SHOOT, UP};
use openhunted_engine::jake::zone;
use openhunted_engine::{Assets, Outcome, Scene};
use openhunted_formats::{Level, open_data};

fn load(level: &str) -> Option<Scene> {
    let Some(p) = std::env::var_os("OPENHUNTED_DATA") else {
        eprintln!("pulado: OPENHUNTED_DATA não definido");
        return None;
    };
    let gd = open_data(std::path::Path::new(&p)).unwrap();
    let l = Level::parse(&gd.levels().unwrap().get(level).unwrap()).unwrap();
    let mut a = Assets::new(gd.resources().unwrap());
    Some(Scene::build(&l, &mut a).unwrap())
}

/// Piloto: um tique por vez, sem deixar o Jake morrer nem o tempo estourar.
struct Pilot {
    s: Scene,
    t: u32,
    name: &'static str,
}

impl Pilot {
    fn tick(&mut self, input: u32) {
        self.s.tick(input);
        self.t += 1;
        let j = self.s.jake.as_ref().unwrap();
        if std::env::var_os("OH_TRACE").is_some() {
            if std::env::var_os("OH_MINES").is_some() {
                let m: Vec<_> = self
                    .s
                    .entities
                    .iter()
                    .filter(|e| e.alive && matches!(e.kind, 102 | 104 | 28 | 36))
                    .map(|e| (e.kind, e.center()))
                    .collect();
                eprintln!("      {m:?}");
            }
            eprintln!(
                "{:5} {input:#06x} ({}, {}) {:?} q{} camada {} vida {}",
                self.t,
                j.body.x,
                j.body.y,
                j.state(),
                j.frame(),
                j.layer(),
                j.health.hp
            );
        }
        assert!(
            !j.is_dead(),
            "{}: o Jake morreu no tique {} em ({}, {})",
            self.name,
            self.t,
            j.body.x,
            j.body.y
        );
        assert!(self.t < 6000, "{}: tempo demais", self.name);
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

    /// Espera parar no chão.
    fn settle(&mut self) {
        for _ in 0..80 {
            if self.on_ground() && self.s.jake.as_ref().unwrap().body.vy == 0 {
                break;
            }
            self.tick(0);
        }
        self.run(3, 0);
    }

    /// Anda até o centro ficar a até 6 px de `x`.
    fn go_x(&mut self, x: i32) {
        for _ in 0..400 {
            let c = self.center().0;
            if (c - x).abs() <= 6 {
                self.settle();
                return;
            }
            self.tick(if c < x { RIGHT } else { LEFT });
        }
        panic!(
            "{}: não chegou a x = {x} (em {:?})",
            self.name,
            self.center()
        );
    }

    /// Segura CIMA (agarra e sobe) até os pés ficarem acima de `y`.
    fn climb_to(&mut self, y: i32) {
        for _ in 0..400 {
            if self.feet() < y {
                return;
            }
            self.tick(UP);
        }
        panic!("{}: não subiu até {y} (pés em {})", self.name, self.feet());
    }

    /// Salta da corda para o lado com PULAR+CIMA (pulo longo) e segue até pousar.
    fn leap(&mut self, side: u32) {
        self.tick(JUMP | UP | side);
        for _ in 0..80 {
            self.tick(side);
            if self.on_ground() {
                break;
            }
        }
        self.settle();
    }

    /// Anda até a zona PR `kind` mais próxima e aperta `key` até o Jake agir.
    fn zone_key(&mut self, kind: u16, key: u32) {
        let c = self.center().0;
        let layer = self.s.jake_layer;
        let target = self
            .s
            .prop_zones
            .iter()
            .filter(|z| z.layer == layer && z.kind & kind != 0)
            .map(|z| z.rect.x + z.rect.w / 2)
            .min_by_key(|x| (x - c).abs())
            .expect("zona");
        self.go_x(target);
        self.tick(key);
    }

    /// Atira nas minas e olhos-laser vivos da camada a menos de `range` px até não sobrar nenhuma (ou
    /// `max` tiques): mira pelo lado e pela altura delas.
    fn fight(&mut self, range: i32, max: u32) {
        for _ in 0..max {
            let c = self.center();
            let layer = self.s.jake_layer;
            let target = self
                .s
                .entities
                .iter()
                .filter(|e| {
                    e.alive
                        && e.layer == layer
                        && ((e.kind == 102 && e.health > 0) || e.kind == 104)
                })
                .map(|e| {
                    let m = e.center();
                    (m.0 - c.0, m.1 - c.1)
                })
                .filter(|(dx, dy)| dx * dx + dy * dy < range * range)
                .min_by_key(|(dx, dy)| dx * dx + dy * dy);
            let Some((dx, dy)) = target else {
                return;
            };
            let side = if dx > 0 { RIGHT } else { LEFT };
            let input = if dy < -60 && dx.abs() > 6 && self.on_ground() {
                // bem acima: vai para baixo dele (a pistola não atira em ângulo fechado)
                self.tick(side);
                continue;
            } else if dy < -2 * dx.abs() - 20 {
                UP
            } else if dy < -20 {
                UP | side
            } else if dy > 30 {
                DOWN | side
            } else {
                side
            };
            // alterna para recomeçar as rajadas
            let fire = if self.t % 14 < 12 { SHOOT } else { 0 };
            self.tick(input | fire);
        }
    }

    /// Pulo correndo para a direita (a sonda escolhe médio, longo ou estendido) até pousar.
    fn jump_right(&mut self) {
        self.tick(RIGHT);
        self.tick(RIGHT | JUMP);
        for _ in 0..80 {
            self.tick(RIGHT);
            if self.on_ground() && self.s.jake.as_ref().unwrap().body.vy == 0 {
                break;
            }
        }
        self.settle();
    }

    fn win(&mut self) {
        self.settle();
        self.zone_key(zone::EXIT, UP);
        assert!(
            self.s.entities.iter().any(|e| e.kind == 5 && e.on),
            "{}: a saída não ligou (objetivos?)",
            self.name
        );
        self.run(60, 0);
        assert_eq!(self.s.outcome, Some(Outcome::Victory), "{}", self.name);
    }
}

/// T_J_003: desce correndo para a direita até o fundo; sobe as três redes em degrau
/// (x 561–593, 688–720, 815–847), saltando de cada uma para a plataforma da direita; anda
/// até a saída e aperta CIMA.
#[test]
fn t_j_003_can_be_won() {
    let Some(s) = load("T_J_003.LEV") else {
        return;
    };
    let mut p = Pilot {
        s,
        t: 0,
        name: "T_J_003",
    };
    p.run(20, 0);
    // a pistola está logo à esquerda do início
    p.go_x(70);
    p.fight(300, 200);
    for _ in 0..300 {
        if p.on_ground() && p.feet() >= 620 {
            break;
        }
        p.tick(RIGHT);
    }
    assert!(p.feet() >= 620, "não chegou ao fundo: {:?}", p.center());
    for (net_x, floor_y) in [(577, 500), (704, 372), (831, 244)] {
        // uma mina que desce do alto pode derrubar o Jake no salto: ele volta e tenta de novo
        for attempt in 0.. {
            assert!(
                attempt < 4,
                "não subiu à plataforma de {floor_y}: pés {}",
                p.feet()
            );
            p.fight(400, 300);
            p.go_x(net_x);
            p.fight(400, 300);
            p.go_x(net_x);
            p.climb_to(floor_y - 8);
            p.leap(RIGHT);
            if p.feet() <= floor_y {
                break;
            }
        }
    }
    p.fight(400, 300);
    p.win();
}

/// T_J_007: desce até o corredor de baixo, sobe a escada (x 553–585), passa à camada 1 pela
/// porta de x ≈ 451, à camada 2 pela de x ≈ 579, apanha a peça do carro, volta à camada 1
/// pela porta de x ≈ 707 e sai.
#[test]
fn t_j_007_can_be_won() {
    let Some(s) = load("T_J_007.LEV") else {
        return;
    };
    let mut p = Pilot {
        s,
        t: 0,
        name: "T_J_007",
    };
    p.run(20, 0);
    // olho-laser logo acima do início
    p.fight(300, 300);
    // cai até o chão da sala e pelo poço (x 150–235) até o corredor de baixo
    p.go_x(60);
    p.go_x(192);
    p.settle();
    assert!(p.feet() >= 490, "não desceu: {:?}", p.center());
    // olho-laser do corredor e as minas da célula (3, 3)
    p.fight(350, 600);
    p.go_x(569);
    p.fight(350, 300);
    p.go_x(569);
    // escada até a sala da linha 2; salta para a esquerda
    p.climb_to(364);
    p.leap(LEFT);
    assert!(p.feet() <= 372, "não subiu: {:?}", p.center());
    // porta da camada 1 (x ≈ 451)
    p.zone_key(zone::LAYER_DOWN, DOWN);
    p.run(10, 0);
    assert_eq!(p.s.jake_layer, 1);
    p.zone_key(zone::LAYER_DOWN, DOWN);
    p.run(10, 0);
    assert_eq!(p.s.jake_layer, 2);
    // camada 2: minas e a peça do carro
    p.fight(350, 600);
    let piece = p.s.entities.iter().position(|e| e.kind == 9).unwrap();
    let pc = p.s.entities[piece].center();
    p.go_x(pc.0);
    if p.s.entities[piece].alive {
        p.tick(JUMP);
        p.run(30, 0);
    }
    assert!(
        !p.s.entities[piece].alive,
        "não apanhou a peça: {:?}",
        p.center()
    );
    p.fight(350, 300);
    p.zone_key(zone::LAYER_UP, UP);
    p.run(10, 0);
    assert_eq!(p.s.jake_layer, 1);
    p.fight(350, 300);
    p.win();
}

/// T_J_005: corredor da linha 1 (chão em y = 244) com três fossos; pula os dois primeiros,
/// explode os barris (x = 959) de longe com a pistola, cruza o fosso de fogo no pulo
/// estendido e sai.
#[test]
fn t_j_005_can_be_won() {
    let Some(s) = load("T_J_005.LEV") else {
        return;
    };
    let mut p = Pilot {
        s,
        t: 0,
        name: "T_J_005",
    };
    assert_eq!(p.s.entities.iter().filter(|e| e.kind == 28).count(), 2);
    p.run(20, 0);
    p.fight(300, 300);
    for edge in [262, 646] {
        p.fight(300, 300);
        p.go_x(edge);
        p.jump_right();
        assert_eq!(p.feet(), 244, "caiu no fosso de x {edge}: {:?}", p.center());
    }
    // barris de longe (raio 96 da explosão); os tiros nas minas ao lado deles costumam já
    // tê-los explodido em cadeia
    p.go_x(790);
    for _ in 0..300 {
        if !p.s.entities.iter().any(|e| e.alive && e.kind == 28) {
            break;
        }
        let fire = if p.t % 14 < 12 { SHOOT } else { 0 };
        p.tick(RIGHT | fire);
        p.tick(fire);
    }
    assert!(
        !p.s.entities.iter().any(|e| e.alive && e.kind == 28),
        "barris inteiros"
    );
    p.fight(300, 300);
    p.go_x(1030);
    p.jump_right();
    assert_eq!(p.feet(), 244, "caiu no fogo: {:?}", p.center());
    p.fight(300, 300);
    p.win();
}
