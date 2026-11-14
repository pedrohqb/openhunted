// SPDX-License-Identifier: GPL-3.0-or-later
//! Auditoria M17, movimentos do Jake (`docs/auditoria/jake-movimentos.md`): o Jake real
//! (`JAKE.ANM`, `JAKE.SKE`, BRDs da T_J_005, dados em OPENHUNTED_DATA) em cenários
//! sintéticos, medindo alturas, distâncias, durações em tiques e velocidades. Os valores
//! esperados são os do executável (endereços nos comentários).

use openhunted_engine::damage::Pending;
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

/// PULAR + CIMA + lado é o pulo longo direto (28, `0x00423f35`), parado ou correndo;
/// correndo (ou andando), PULAR sem CIMA escolhe pela sonda mesmo sem seta
/// (`0x00423f4d` → `0x425144`): no oráculo, `5*L,J` dá o 29 com vx −80.
#[test]
fn up_side_jump_is_long_and_running_jump_needs_no_arrow() {
    use openhunted_engine::JumpKind;
    let Some(mut r) = Run::flat(0) else { return };
    r.tick(LEFT | UP | JUMP);
    assert_eq!(r.j.state(), State::Jump(JumpKind::Long), "{}", r.dump());
    let Some(mut r) = Run::flat(0) else { return };
    r.ticks(LEFT, 5);
    r.tick(LEFT | UP | JUMP);
    assert_eq!(r.j.state(), State::Jump(JumpKind::Long), "{}", r.dump());
    let Some(mut r) = Run::flat(0) else { return };
    r.ticks(LEFT, 5);
    r.tick(JUMP);
    assert_eq!(r.j.state(), State::Jump(JumpKind::Medium), "{}", r.dump());
    assert_eq!(r.j.body.vx, -80);
}

/// O controle no ar compara a seta com o deslocamento real do tique anterior (`+0x15c`):
/// depois de andar 8 px no ar ele não age e a velocidade do esqueleto (0) fica; no
/// oráculo (`J,9*,15*L` na T_J_001) o Jake anda 8 px a cada dois tiques no pulo 27.
#[test]
fn air_control_alternates_after_moving_eight_pixels() {
    let Some(mut r) = Run::flat(1000) else { return };
    r.tick(JUMP);
    r.ticks(0, 9);
    r.ticks(LEFT, 6);
    let xs: Vec<i32> = r.log[10..].iter().map(|e| e.1).collect();
    assert_eq!(xs, vec![992, 992, 984, 984, 976, 976], "{}", r.dump());
}

/// Empurrar (`0x00425c34`, `0x00425d04`, `0x00425ced`), medido no oráculo (`25*R` contra a
/// parede da T_J_001): 100 uma vez (6 tiques) com vx = 4 px/tique, então som 5043
/// (`0x13b3`) e o laço 101 (5 quadros); PULAR não faz nada; soltar a seta toca o 102 (o
/// 100 ao contrário, 6 tiques) e só então parado.
#[test]
fn push_plays_100_then_loops_101_and_ends_with_102() {
    let wall = Rect {
        x: 200,
        y: FLOOR - 200,
        w: 40,
        h: 200,
    };
    let Some(mut r) = Run::new(vec![slab(-5000, 5000, FLOOR), wall], 150) else {
        return;
    };
    let mut sounds = Vec::new();
    for t in 0..30 {
        r.tick(if (12..18).contains(&t) {
            RIGHT | JUMP
        } else {
            RIGHT
        });
        sounds.extend(r.j.sound.take());
    }
    let push = r.first(|s| s == State::Push).expect("empurra");
    assert!(
        r.log[push..].iter().all(|e| e.0 == State::Push),
        "PULAR não pula: {}",
        r.dump()
    );
    assert_eq!(sounds, vec![5043]);
    assert_eq!(r.j.label(), 101);
    r.log.clear();
    r.ticks(0, 8);
    let st: Vec<State> = r.log.iter().map(|e| e.0).collect();
    assert_eq!(&st[..6], &[State::PushEnd; 6], "{}", r.dump());
    assert_eq!(r.log[0].3, 5, "102 = o 100 ao contrário");
    assert_eq!(st[6], State::Idle, "{}", r.dump());
}

/// Pulos 28/29 (`0x004254df`, `0x0042568c`): o rótulo toca uma vez e para no último
/// quadro (no oráculo, `29/8` fica até o pouso); só então olham o chão. Pousando antes do
/// fim (num degrau), seguem no 29 até o rótulo acabar.
#[test]
fn launched_jumps_hold_the_last_frame_and_land_only_after_it() {
    use openhunted_engine::JumpKind;
    // do alto de uma plataforma: o voo passa do fim do rótulo
    let Some(mut r) = Run::new(
        vec![slab(-5000, 5000, FLOOR + 200), slab(-10, 100, FLOOR)],
        0,
    ) else {
        return;
    };
    r.tick(LEFT | JUMP);
    r.ticks(0, 20);
    let medium: Vec<usize> = r
        .log
        .iter()
        .filter(|e| e.0 == State::Jump(JumpKind::Medium))
        .map(|e| e.3)
        .collect();
    assert_eq!(medium.last(), Some(&8), "{}", r.dump());
    assert!(
        medium.windows(2).all(|w| w[1] >= w[0]),
        "sem laço: {}",
        r.dump()
    );
    // degrau 22 px acima, 20 px à frente: toca o topo antes do fim do rótulo
    let step = Rect {
        x: -400,
        y: FLOOR - 22,
        w: 380,
        h: 22,
    };
    let Some(mut r) = Run::new(vec![slab(-5000, 5000, FLOOR), step], 0) else {
        return;
    };
    r.tick(LEFT | JUMP);
    r.ticks(0, 14);
    let land = r.first(|s| matches!(s, State::Land(_))).expect("pousa");
    assert_eq!(land, 9, "{}", r.dump());
}

/// Pulo estendido (80, `0x004253d3`): contato que não é chão durante a animação → pose 1
/// (`0x004254c7`) no ar, em vez da queda descontrolada no fim.
#[test]
fn extended_jump_hitting_a_wall_goes_to_idle_pose() {
    let wall = Rect {
        x: 100,
        y: FLOOR - 300,
        w: 20,
        h: 270,
    };
    let Some(mut r) = Run::new(
        vec![slab(-5000, 0, FLOOR), slab(200, 5000, FLOOR), wall],
        -30,
    ) else {
        return;
    };
    r.tick(RIGHT | JUMP);
    assert_eq!(r.j.state(), State::RunUp, "{}", r.dump());
    r.ticks(0, 12);
    let idle = r.first(|s| s == State::Idle).expect("pose 1");
    assert!(r.log[idle].2 < FLOOR - 10, "no ar: {}", r.dump());
    assert!(r.first(|s| s == State::WildFall).is_none(), "{}", r.dump());
}

fn knock(push: (i32, i32)) -> Pending {
    Pending {
        amount: 1,
        knockdown: true,
        push,
        ..Default::default()
    }
}

/// Derrubada (`0x004235fc`): empurrado para o lado em que olha → 81, senão 82. O 81 só olha
/// o chão no fim do rótulo (`0x00425ab6`): no chão → 77; o 82 → 83 (`BK2STAND`) → 84 →
/// parado (`0x00425b99`, `0x00425bdf`).
#[test]
fn knockdown_follows_facing_and_gets_up_through_83_84() {
    let Some(mut r) = Run::flat(0) else { return };
    assert!(!r.j.facing_right());
    r.j.hurt(knock((-15, -10)));
    r.tick(0);
    assert_eq!(r.j.label(), 81, "empurrado para a frente (esquerda)");
    r.ticks(0, 40);
    assert!(
        r.first(|s| s == State::Land(openhunted_engine::jake::Landing::Hard))
            .is_some(),
        "{}",
        r.dump()
    );
    let Some(mut r) = Run::flat(0) else { return };
    r.j.hurt(knock((15, -10)));
    r.tick(0);
    assert_eq!(r.j.label(), 82, "empurrado para trás");
    r.ticks(0, 80);
    let sit = r.first(|s| s == State::SitUp).expect("83");
    let rise = r.first(|s| s == State::Rise).expect("84");
    let idle = r.first(|s| s == State::Idle).expect("parado");
    assert!(sit < rise && rise < idle, "{}", r.dump());
}

/// Tranco no Jake (`0x004233e3`): nos estados da tabela, 169 (rótulo 173, `JK_COLPS`,
/// esqueleto k = 2 em x) e no fim 84 (`0x00426c5e`); escalando (30/32), queda.
#[test]
fn jolt_knocks_jake_to_169_then_84() {
    let Some(mut r) = Run::flat(0) else { return };
    r.j.hurt(Pending {
        amount: 1,
        jolt: true,
        ..Default::default()
    });
    r.tick(0);
    assert_eq!((r.j.state(), r.j.label()), (State::Jolted, 173));
    r.ticks(0, 60);
    let rise = r.first(|s| s == State::Rise).expect("84");
    assert!(r.first(|s| s == State::Idle) > Some(rise), "{}", r.dump());
}

/// Empurrão (reação 2, `0x00423366`): só nos estados 1, 6, 79 e 164–166; correndo ou
/// empurrando o Jake leva o dano e segue.
#[test]
fn shove_only_when_standing_or_hiding() {
    let shove = Pending {
        amount: 1,
        shove: true,
        push: (10, 0),
        ..Default::default()
    };
    let Some(mut r) = Run::flat(0) else { return };
    r.j.hurt(shove);
    r.tick(0);
    assert_eq!(r.j.state(), State::Pushed);
    let Some(mut r) = Run::flat(0) else { return };
    r.ticks(LEFT, 3);
    r.j.hurt(shove);
    r.tick(LEFT);
    assert_eq!(r.j.state(), State::Run);
}

/// Subindo além do alto da corda: sai da zona → 8 (`0x4270e5`), cuja decisão
/// (`0x00423b2b`) com CIMA sobre a zona 4 volta direto a escalar (no oráculo, T_J_003:
/// 30 → 8 por 3 tiques → 30, e segura no alto com CIMA). Não há queda nem o 3.
#[test]
fn climbing_past_the_top_regrabs_from_let_go() {
    let Some(mut r) = on_rope(150) else { return };
    r.ticks(UP, 120);
    assert!(r.first(|s| s == State::LetGo).is_some(), "{}", r.dump());
    assert!(r.first(|s| s == State::Fall).is_none(), "{}", r.dump());
    assert!(r.first(|s| s == State::Grab).is_none(), "{}", r.dump());
    assert!(
        matches!(r.j.state(), State::Climb(_) | State::LetGo),
        "{}",
        r.dump()
    );
}

/// Fim do especial sem ATIRAR: o `0x0043b43e` põe a pose de arma pronta (no oráculo, com a
/// pistola: 9 por 5 tiques, então 18 por 10 e 19).
#[test]
fn special_ends_in_the_ready_pose() {
    let Some(mut r) = Run::flat(0) else { return };
    r.tick(DOWN | SHOOT);
    assert!(matches!(r.j.state(), State::Special(_)), "{}", r.dump());
    r.ticks(0, 6);
    assert!(matches!(r.j.state(), State::Ready(..)), "{}", r.dump());
}

/// Choque elétrico (`0x00423577` → 15, contador `+0x284 = 10`): o pré-tratamento
/// `0x004235c4` consome o tique enquanto conta, então o rótulo 15 fica no quadro 0 os 9
/// tiques e o Jake volta ao parado.
#[test]
fn shock_holds_frame_zero() {
    let Some(mut r) = Run::flat(0) else { return };
    r.j.hurt(Pending {
        amount: 1,
        electric: true,
        ..Default::default()
    });
    r.ticks(0, 10);
    let shocked: Vec<usize> = r
        .log
        .iter()
        .filter(|e| matches!(e.0, State::Shocked(_)))
        .map(|e| e.3)
        .collect();
    assert_eq!(shocked, vec![0; 9], "{}", r.dump());
    assert_eq!(r.j.state(), State::Idle);
}

/// Atrasos `+0x18` da tabela nas poses de arma do Jake (punhos: pronto 146 e guardar 147
/// com atraso 1): no oráculo (`P` na T_J_001) o 147 leva 16 tiques.
#[test]
fn weapon_poses_follow_the_table_delay() {
    let Some(mut r) = Run::flat(0) else { return };
    r.tick(PUNCH);
    r.ticks(0, 60);
    let holster = r
        .log
        .iter()
        .filter(|e| matches!(e.0, State::Holster(_)))
        .count();
    assert_eq!(holster, 16, "{}", r.dump());
}

/// Escolha do pulo (`0x00425144`): procura chão numa faixa de meia largura e 3 px logo
/// abaixo dos pés (y + h + 1), a ¼ da largura + alcance (110, 150, 255) à frente. Um chão
/// 6 px mais baixo a 110 px não conta para o médio.
#[test]
fn jump_choice_probes_a_thin_strip_under_the_feet() {
    use openhunted_engine::JumpKind;
    // a 110 px, chão 6 px abaixo (fora da faixa); a 150, no mesmo nível → longo
    let Some(mut r) = Run::new(
        vec![
            slab(-5000, 40, FLOOR),
            slab(100, 140, FLOOR + 6),
            slab(160, 5000, FLOOR),
        ],
        0,
    ) else {
        return;
    };
    r.tick(RIGHT);
    r.tick(RIGHT | JUMP);
    assert_eq!(r.j.state(), State::Jump(JumpKind::Long), "{}", r.dump());
}
