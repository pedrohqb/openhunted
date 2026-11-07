// SPDX-License-Identifier: GPL-3.0-or-later
//! As missões `T_J_003`, `T_J_005`, `T_J_007` e `I_J_002` jogadas de ponta a ponta sobre os
//! dados reais (OPENHUNTED_DATA), por rotas roteirizadas (`docs/re/missoes-tutorial.md`):
//! sem tirar perigos, só com a entrada do jogador.

use openhunted_engine::input::{DOWN, JUMP, LEFT, RIGHT, SELECT_SHIFT, SHOOT, UP};
use openhunted_engine::jake::zone;
use openhunted_engine::{Assets, Entity, Outcome, Scene};
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

/// Como lutar: desistir se a vida dos alvos não cai em `patience` tiques; só alvos a até
/// `dy_max` px na vertical.
#[derive(Clone, Copy)]
struct Fight {
    patience: u32,
    dy_max: i32,
}

impl Fight {
    /// Minas e olhos: até o fim.
    const PATIENT: Fight = Fight {
        patience: u32::MAX,
        dy_max: i32::MAX,
    };
    /// Robôs: os do mesmo andar, e desiste do que não se fere (fora da mira, sem munição).
    const ROBOTS: Fight = Fight {
        patience: 80,
        dy_max: 90,
    };
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
                    .filter(|e| e.alive && matches!(e.kind, 8 | 102 | 104 | 28 | 36))
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
    /// `max` tiques): mira pelo lado e pela altura delas. Desiste do alvo inalcançável
    /// (atrás de parede): `STUCK` tiques empurrando sem nada morrer.
    fn fight(&mut self, range: i32, max: u32) {
        self.fight_with(range, max, Fight::PATIENT, |e| {
            (e.kind == 102 && e.health > 0) || e.kind == 104
        });
    }

    /// `fight` contra os alvos de `hostile` (vivos, na camada do Jake); desiste também se a
    /// vida somada deles não cai em `patience` tiques (alvo fora da mira ou sem munição).
    fn fight_with(&mut self, range: i32, max: u32, how: Fight, hostile: fn(&Entity) -> bool) {
        const STUCK: u32 = 30;
        let alive = |s: &Scene| s.entities.iter().filter(|e| e.alive && hostile(e)).count();
        let hp = |s: &Scene| -> i32 {
            s.entities
                .iter()
                .filter(|e| e.alive && hostile(e))
                .map(|e| e.health)
                .sum()
        };
        let (mut pushing, mut left) = (0, alive(&self.s));
        let (mut best, mut idle) = (hp(&self.s), 0);
        for _ in 0..max {
            let h = hp(&self.s);
            idle = if h < best { 0 } else { idle + 1 };
            best = best.min(h);
            if idle >= how.patience {
                return;
            }
            let now = alive(&self.s);
            let push = self.s.jake.as_ref().unwrap().state() == openhunted_engine::State::Push;
            pushing = if push && now == left { pushing + 1 } else { 0 };
            left = now;
            if pushing >= STUCK {
                return;
            }
            let c = self.center();
            let layer = self.s.jake_layer;
            let target = self
                .s
                .entities
                .iter()
                .filter(|e| e.alive && e.layer == layer && hostile(e))
                .map(|e| {
                    let m = e.center();
                    (m.0 - c.0, m.1 - c.1)
                })
                .filter(|(dx, dy)| dx * dx + dy * dy < range * range && dy.abs() <= how.dy_max)
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

    /// Anda até `x` como `go_x`, parando para lutar (`fight_with`) com quem chegar a
    /// `range` px.
    /// Três pulos sem passar: parede; volta sem chegar (quem chamou confere onde está).
    fn hunt_x(&mut self, x: i32, range: i32, hostile: fn(&Entity) -> bool) {
        let mut jumps = 0;
        for _ in 0..600 {
            let before = self
                .s
                .entities
                .iter()
                .filter(|e| e.alive && hostile(e))
                .count();
            self.fight_with(range, 400, Fight::ROBOTS, hostile);
            if self
                .s
                .entities
                .iter()
                .filter(|e| e.alive && hostile(e))
                .count()
                < before
            {
                // o robô abatido cai e explode: longe dele até passar
                self.run(40, 0);
            }
            let c = self.center().0;
            if (c - x).abs() <= 6 {
                self.settle();
                return;
            }
            let side = if c < x { RIGHT } else { LEFT };
            if self.s.jake.as_ref().unwrap().state() == openhunted_engine::State::Push {
                // obstáculo baixo: recua um pouco e pula por cima; parede: desiste
                if jumps == 3 {
                    return;
                }
                jumps += 1;
                self.run(4, side ^ (LEFT | RIGHT));
                self.settle();
                self.jump_side(side);
                continue;
            }
            self.tick(side);
        }
        panic!(
            "{}: não chegou a x = {x} (em {:?})",
            self.name,
            self.center()
        );
    }

    /// Atravessa a porta 50–52 mais próxima da camada que abre sozinha (gatilho sem
    /// condição: `values[1]` tiques fechada sem o Jake encostado, `values[2]` aberta):
    /// espera a 40 px dela até abrir e passa correndo para `side`.
    fn through_door(&mut self, side: u32) {
        let c = self.center();
        let layer = self.s.jake_layer;
        let i = (0..self.s.entities.len())
            .filter(|&i| {
                let e = &self.s.entities[i];
                (50..=52).contains(&e.kind) && e.layer == layer
            })
            .min_by_key(|&i| {
                let d = self.s.entities[i].center();
                (d.0 - c.0).abs() + (d.1 - c.1).abs()
            })
            .expect("porta");
        let dx = self.s.entities[i].center().0;
        let dir = if side == RIGHT { 1 } else { -1 };
        self.go_x(dx - dir * 40);
        for _ in 0..300 {
            if self.s.entities[i].on {
                break;
            }
            self.tick(0);
        }
        assert!(
            self.s.entities[i].on,
            "{}: a porta de x {dx} não abriu",
            self.name
        );
        for _ in 0..40 {
            if (self.center().0 - dx) * dir > 40 {
                break;
            }
            self.tick(side);
        }
        assert!(
            (self.center().0 - dx) * dir > 40,
            "{}: não passou a porta de x {dx} (em {:?})",
            self.name,
            self.center()
        );
    }

    /// Pulo correndo para a direita (a sonda escolhe médio, longo ou estendido) até pousar.
    fn jump_right(&mut self) {
        self.jump_side(RIGHT);
    }

    /// Pulo correndo para `side` até pousar.
    fn jump_side(&mut self, side: u32) {
        self.tick(side);
        self.tick(side | JUMP);
        for _ in 0..80 {
            self.tick(side);
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
/// confere que os barris (x = 959) explodiram (em cadeia, por uma bala perdida; senão atira
/// neles de longe), cruza o fosso de fogo no pulo estendido e sai.
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
    // Barris: na prática já explodiram. Uma bala perdida da primeira luta (contra a mina
    // de x ≈ 261) segue pelo corredor e acerta a mina de x ≈ 935 por volta do tique 60; a
    // explosão leva os dois barris e a mina de x ≈ 1003 em cadeia. O laço abaixo é só a
    // reserva (atirar de longe, fora do raio 144) se essa cadeia deixar de acontecer.
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

/// Robôs de segurança (tipo 8) vivos abaixo do teto do corredor de cima da I_J_002 (os da
/// sala de cima descem e atiram; lá em cima, atrás do teto, não se alcançam).
fn robot(e: &Entity) -> bool {
    e.kind == 8 && e.health > 0 && e.center().1 > 128
}

/// I_J_002 (seis robôs-objetivo): apanha a espingarda e os foguetes da sala do início
/// (camada 2) e fica com a espingarda (o foguete de perto fere o próprio Jake); sobe à
/// camada 1 pela porta de x ≈ 1270 e limpa o corredor para a esquerda, passando a porta que
/// abre sozinha (x ≈ 1107); sobe a escada de x 682–721 ao corredor de cima, empurra a porta
/// de x ≈ 792 até a vida grande e a armadura (os robôs da sala de cima descem e atiram),
/// volta, pula o vão da escada da esquerda (x 278–362) até a chave 62 e torna; desce,
/// passa à camada 0 pela porta de x ≈ 581, cruza a porta que abre sozinha (x ≈ 809),
/// empurra a porta de célula (8, 2, 0) com a chave, abate o último robô e sai.
#[test]
fn i_j_002_can_be_won() {
    let Some(s) = load("I_J_002.LEV") else {
        return;
    };
    let mut p = Pilot {
        s,
        t: 0,
        name: "I_J_002",
    };
    let jake = |p: &Pilot| p.s.jake.as_ref().unwrap().clone();
    p.run(10, 0);
    p.go_x(1290);
    p.tick(3 << SELECT_SHIFT);
    p.run(20, 0);
    assert_eq!(jake(&p).inventory.weapon, 3, "espingarda");
    p.zone_key(zone::LAYER_UP, UP);
    p.run(10, 0);
    assert_eq!(p.s.jake_layer, 1);
    p.fight_with(400, 600, Fight::ROBOTS, robot);
    p.hunt_x(1160, 300, robot);
    p.through_door(LEFT);
    p.hunt_x(450, 300, robot);
    p.go_x(701);
    p.climb_to(236);
    p.leap(RIGHT);
    assert_eq!(p.feet(), 244, "corredor de cima: {:?}", p.center());
    p.go_x(960);
    assert!(jake(&p).health.armor > 100, "armadura 77");
    p.go_x(770);
    p.jump_side(LEFT);
    // se cair no vão da esquerda (uma luta, um tranco), sobe de novo pela escada dele
    let climb_back = |p: &mut Pilot, side: u32| {
        if p.feet() > 300 {
            p.go_x(323);
            p.climb_to(236);
            p.leap(side);
        }
    };
    p.hunt_x(392, 300, robot);
    if p.feet() <= 300 {
        p.jump_side(LEFT);
    }
    climb_back(&mut p, LEFT);
    p.go_x(238);
    assert_ne!(jake(&p).inventory.slots[13].1, 0, "chave 62");
    p.go_x(250);
    p.jump_side(RIGHT);
    climb_back(&mut p, RIGHT);
    p.hunt_x(701, 300, robot);
    p.settle();
    assert_eq!(p.feet(), 372, "desceu: {:?}", p.center());
    p.zone_key(zone::LAYER_UP, UP);
    p.run(10, 0);
    assert_eq!(p.s.jake_layer, 0);
    p.hunt_x(760, 300, robot);
    p.through_door(RIGHT);
    // o robô atrás da porta de célula fica para depois dela
    p.hunt_x(1000, 150, robot);
    p.go_x(1150);
    p.hunt_x(1200, 300, robot);
    assert!(
        !p.s.entities
            .iter()
            .any(|e| e.alive && e.objective & 4 != 0 && e.health > 0),
        "robôs-objetivo vivos"
    );
    p.win();
}
