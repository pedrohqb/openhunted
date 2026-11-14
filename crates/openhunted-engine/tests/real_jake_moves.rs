// SPDX-License-Identifier: GPL-3.0-or-later
//! Auditoria M17, movimentos do Jake (`docs/auditoria/jake-movimentos.md`): o Jake real
//! (`JAKE.ANM`, `JAKE.SKE`, BRDs da T_J_005, dados em OPENHUNTED_DATA) em cenários
//! sintéticos, medindo alturas, distâncias, durações em tiques e velocidades. Os valores
//! esperados são os do executável (endereços nos comentários).

use openhunted_engine::input::{DOWN, JUMP, LEFT, PUNCH, RIGHT, SHOOT, STEALTH, UP};
use openhunted_engine::jake::{Env, Zone, zone};
use openhunted_engine::{Assets, Jake, Rect, Scene, State, World};
use openhunted_formats::{Level, open_data};

/// O Jake da T_J_005 (escala 22/47, esqueleto e BRDs reais).
fn real_jake() -> Option<Jake> {
    let Some(p) = std::env::var_os("OPENHUNTED_DATA") else {
        eprintln!("pulado: OPENHUNTED_DATA não definido");
        return None;
    };
    let gd = open_data(std::path::Path::new(&p)).unwrap();
    let l = Level::parse(&gd.levels().unwrap().get("T_J_005.LEV").unwrap()).unwrap();
    let mut a = Assets::new(gd.resources().unwrap());
    let s = Scene::build(&l, &mut a).unwrap();
    s.jake
}

/// Chão (topo em `y`) de `x0` a `x1`.
fn slab(x0: i32, x1: i32, y: i32) -> Rect {
    Rect {
        x: x0,
        y,
        w: x1 - x0,
        h: 40,
    }
}

const FLOOR: i32 = 400;

/// Cenário com o Jake parado no chão, pés em `FLOOR`, caixa começando em `x`.
struct Run {
    j: Jake,
    world: World,
    zones: Vec<Zone>,
    layers: i32,
    /// (estado, x, y, quadro) depois de cada tique.
    log: Vec<(State, i32, i32, usize)>,
}

impl Run {
    fn new(solids: Vec<Rect>, x: i32) -> Option<Run> {
        let mut j = real_jake()?;
        let world = World::new(solids);
        let b = j.body.bounds();
        j.body.x += x - b.x;
        j.body.y += FLOOR - (b.y + b.h);
        j.body.vx = 0;
        j.body.vy = 0;
        let mut r = Run {
            j,
            world,
            zones: vec![],
            layers: 1,
            log: vec![],
        };
        r.ticks(0, 6);
        assert!(r.j.body.on_ground, "assentou");
        assert_eq!(r.j.state(), State::Idle);
        r.log.clear();
        Some(r)
    }
    fn flat(x: i32) -> Option<Run> {
        Run::new(vec![slab(-5000, 5000, FLOOR)], x)
    }
    fn tick(&mut self, input: u32) {
        let env = Env {
            world: &self.world,
            zones: &self.zones,
            layers: self.layers,
        };
        self.j.tick_in(input, &env);
        let b = self.j.body.bounds();
        self.log
            .push((self.j.state(), b.x, b.y + b.h, self.j.frame()));
    }
    fn ticks(&mut self, input: u32, n: usize) {
        for _ in 0..n {
            self.tick(input);
        }
    }
    /// Pés (base da caixa) agora.
    fn feet(&self) -> i32 {
        let b = self.j.body.bounds();
        b.y + b.h
    }
    /// Tiques (desde o início do log) até o primeiro estado que satisfaz `f`.
    fn first(&self, f: impl Fn(State) -> bool) -> Option<usize> {
        self.log.iter().position(|e| f(e.0))
    }
    fn dump(&self) -> String {
        self.log
            .iter()
            .enumerate()
            .map(|(i, e)| format!("{i}:{:?}@({},{})q{}", e.0, e.1, e.2, e.3))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Sonda de medição (rodar com `--nocapture --ignored`): imprime os números da tabela.
#[test]
#[ignore]
fn medir() {
    let Some(mut r) = Run::flat(0) else { return };
    let b = r.j.body.bounds();
    println!("caixa padrão {b:?}");
    // correr
    r.ticks(RIGHT, 10);
    println!("correr: {}", r.dump());
    // pulo parado
    let mut r = Run::flat(0).unwrap();
    r.tick(JUMP);
    r.ticks(0, 40);
    println!("pulo parado: {}", r.dump());
    let mut r = Run::flat(0).unwrap();
    r.tick(JUMP | UP);
    r.ticks(0, 40);
    println!("pulo para cima: {}", r.dump());
    let mut r = Run::flat(0).unwrap();
    r.ticks(RIGHT | STEALTH, 12);
    println!("furtivo: {}", r.dump());
    let mut r = Run::flat(0).unwrap();
    r.ticks(DOWN, 4);
    r.ticks(DOWN | RIGHT, 13);
    r.ticks(0, 8);
    println!("agachar/engatinhar: {}", r.dump());
    let mut r = Run::flat(0).unwrap();
    r.ticks(UP, 6);
    r.ticks(0, 6);
    println!("esconder: {}", r.dump());
    let _ = (LEFT, SHOOT, zone::ROPE);
}

/// PULAR é o botão segurado (`0x200`, `0x00407f4b`), não a borda: a decisão em pé
/// (`0x00423b02`) pula de novo enquanto ele estiver apertado. O pulo parado dura os 26
/// quadros do rótulo 27 e o seguinte começa no mesmo tique em que ele acaba.
#[test]
fn holding_jump_jumps_again() {
    let Some(mut r) = Run::flat(0) else { return };
    r.ticks(JUMP, 60);
    let starts: Vec<usize> = (0..r.log.len())
        .filter(|&i| r.log[i].0 == State::StandJump && r.log[i].3 == 0)
        .collect();
    assert_eq!(starts, vec![0, 26, 52], "{}", r.dump());
}

/// Controle no ar (`0x00408205`) nos pulos 27/16: no chão (o agachar dos quadros 0–8) não
/// faz nada, logo não há deslize (o Δx do esqueleto é 0); no ar, o lado apertado põe
/// ±8 px/tique.
#[test]
fn stand_jump_air_control_only_in_the_air() {
    let Some(mut r) = Run::flat(100) else { return };
    r.tick(JUMP);
    r.ticks(RIGHT, 20);
    let xs: Vec<i32> = r.log.iter().map(|e| e.1).collect();
    assert_eq!(&xs[..10], &[100; 10], "agachado não desliza: {}", r.dump());
    assert_eq!(xs[10] - xs[9], 8, "{}", r.dump());
    assert_eq!(xs[13] - xs[12], 8, "{}", r.dump());
}

/// Queda controlada (25, `0x0042424d`): chama o controle no ar, ±8 px/tique para o lado
/// apertado (sem lado, a horizontal fica), e o lado do Jake segue a seta (`0x00422a62`).
#[test]
fn fall_has_air_control() {
    let Some(mut r) = Run::flat(100) else { return };
    r.j.body.y -= 200;
    r.ticks(0, 4);
    assert_eq!(r.j.state(), State::Fall);
    r.log.clear();
    r.ticks(RIGHT, 3);
    r.ticks(0, 2);
    r.ticks(LEFT, 2);
    let xs: Vec<i32> = r.log.iter().map(|e| e.1).collect();
    assert_eq!(xs, vec![108, 116, 124, 132, 140, 132, 124], "{}", r.dump());
    assert!(!r.j.facing_right());
}

/// Ataque no ar: os pulos 27/16 (`0x00424f89`, `0x00424ce3`, zerando a vertical pedida),
/// o pulo da corda 17 (`0x00424df3`) e a queda 25 (`0x00424263`) chamam o ataque
/// (`0x0043b43e`) com ATIRAR/SOCO.
#[test]
fn attacks_from_jumps_and_falls() {
    for (start, wait) in [(JUMP, 11), (JUMP | UP, 12)] {
        let Some(mut r) = Run::flat(100) else { return };
        r.tick(start);
        r.ticks(0, wait);
        assert!(!r.j.body.on_ground);
        r.tick(PUNCH);
        assert!(r.j.is_firing(), "{}", r.dump());
        assert!(!r.j.body.on_ground);
    }
    let Some(mut r) = Run::flat(100) else { return };
    r.j.body.y -= 200;
    r.ticks(0, 4);
    assert_eq!(r.j.state(), State::Fall);
    r.tick(PUNCH);
    assert!(r.j.is_firing(), "{}", r.dump());
}

/// Corda (zona PR 4) em volta de `x`, de `y0` a `y1`.
fn rope(x: i32, y0: i32, y1: i32) -> Zone {
    Zone {
        layer: 0,
        kind: zone::ROPE,
        rect: Rect {
            x: x - 8,
            y: y0,
            w: 32,
            h: y1 - y0,
        },
    }
}

/// Jake pendurado na corda a `alto` px do chão.
fn on_rope(alto: i32) -> Option<Run> {
    let mut r = Run::flat(100)?;
    r.zones.push(rope(100, FLOOR - 400, FLOOR));
    r.j.body.y -= alto + 26;
    r.ticks(0, 4);
    r.tick(UP);
    assert_eq!(
        r.j.state(),
        State::Climb(openhunted_engine::jake::Climb::Vertical)
    );
    r.log.clear();
    Some(r)
}

/// Pulo da corda (17, `0x00424dd0`): parado e sem gravidade até o quadro 9, então
/// vy = −20 px/tique (`0x00424ee2`) com gravidade; o rótulo para no fim e o voo segue até o
/// chão (aterrissagem 26), sem voltar ao parado no ar.
#[test]
fn rope_jump_waits_nine_frames_then_vy_minus_20() {
    let Some(mut r) = on_rope(150) else { return };
    let y0 = r.feet();
    r.tick(JUMP | UP);
    assert_eq!(r.j.state(), State::JumpUp(true));
    r.ticks(0, 60);
    let ys: Vec<i32> = r.log.iter().map(|e| e.2).collect();
    assert_eq!(&ys[..9], &[y0; 9], "{}", r.dump());
    assert_eq!(ys[9], y0 - 18, "{}", r.dump());
    assert_eq!(ys.iter().min(), Some(&(y0 - 90)), "{}", r.dump());
    let land = r.first(|s| s == State::Land(openhunted_engine::jake::Landing::Fall));
    let idle = r.first(|s| s == State::Idle);
    assert!(land.is_some() && land < idle, "{}", r.dump());
}

#[test]
#[ignore]
fn scratch_fuzz() {
    use openhunted_engine::input::{DOWN, JUMP, LEFT, RIGHT, SHOOT, STEALTH, UP};
    let p = std::env::var_os("OPENHUNTED_DATA").unwrap();
    let gd = open_data(std::path::Path::new(&p)).unwrap();
    let l = Level::parse(
        &gd.levels()
            .unwrap()
            .get(&std::env::var("LV").unwrap())
            .unwrap(),
    )
    .unwrap();
    let mut a = Assets::new(gd.resources().unwrap());
    let mut s = Scene::build(&l, &mut a).unwrap();
    let choices = [
        0,
        LEFT,
        RIGHT,
        UP,
        DOWN,
        UP | LEFT,
        UP | RIGHT,
        DOWN | LEFT,
        DOWN | RIGHT,
        LEFT | STEALTH,
        RIGHT | STEALTH,
        JUMP,
        UP | JUMP,
        LEFT | JUMP,
        RIGHT | UP | JUMP,
        DOWN | SHOOT,
        JUMP | DOWN,
    ];
    let mut seed: u32 = 0x9e37_79b9;
    let mut input = 0;
    let from: usize = std::env::var("FROM").unwrap().parse().unwrap();
    for t in 0..1500 {
        if t % 9 == 0 {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            input = choices[(seed >> 16) as usize % choices.len()];
        }
        s.tick(input);
        let j = s.jake.as_ref().unwrap();
        if t >= from {
            println!(
                "{t} {input:#x} {:?} q{} {:?} {:?}",
                j.state(),
                j.frame(),
                j.body,
                j.character()
            );
        }
        if t == from {
            for r in s.world.solids() {
                if r.x < 1300 && r.x + r.w > 1200 && r.y < 140 && r.y + r.h > 40 {
                    println!("  sólido {r:?}");
                }
            }
        }
    }
}

/// A caixa do Jake vem da tabela de estados (`0x00422878`: `0x42318e(jake, 1)` põe o
/// `JK_DEFLT.BRD`), não do BRD do objeto da fase (`JK_EXTND.BRD` na M_2_048).
#[test]
fn default_box_is_jk_deflt_even_if_the_level_says_otherwise() {
    let Some(p) = std::env::var_os("OPENHUNTED_DATA") else {
        return;
    };
    let gd = open_data(std::path::Path::new(&p)).unwrap();
    let l = Level::parse(&gd.levels().unwrap().get("M_2_048.LEV").unwrap()).unwrap();
    let mut a = Assets::new(gd.resources().unwrap());
    let s = Scene::build(&l, &mut a).unwrap();
    let j = s.jake.as_ref().unwrap();
    assert_eq!(
        j.body.boxr,
        Rect {
            x: 2,
            y: 3,
            w: 18,
            h: 44
        }
    );
}
