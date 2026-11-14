// SPDX-License-Identifier: GPL-3.0-or-later
//! O Garathe sobre os dados reais (OPENHUNTED_DATA): a `T_M_002` (primeiro tutorial do
//! Garathe) carrega com ele controlável, corre e pula pelas tabelas e é vencida por entrada
//! roteirizada, como a `I_M_102`; alturas dos pulos com os esqueletos reais; agarrar o teto
//! nas dicas da `T_M_004`/`T_M_008`; a `I_M_109` vencida e a poça `OPO` dela; fuzz nas fases
//! `p=2` solo.

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

/// Leva o jogador ao ponto `c` (centro da caixa) na camada `layer`, parado.
fn place(s: &mut Scene, c: (i32, i32), layer: i32) {
    let j = s.jake.as_mut().unwrap();
    let b = j.body.bounds();
    let (ox, oy) = (b.x - j.body.x, b.y - j.body.y);
    j.set_layer(layer);
    j.body.x = c.0 - b.w / 2 - ox;
    j.body.y = c.1 - b.h / 2 - oy;
    j.body.vx = 0;
    j.body.vy = 0;
    s.set_jake_layer(layer);
}

/// Tiques com `input` até `done` (no máximo `max`); devolve se chegou.
fn until(s: &mut Scene, input: u32, max: u32, done: impl Fn(&Scene) -> bool) -> bool {
    for _ in 0..max {
        if done(s) {
            return true;
        }
        s.tick(input);
    }
    done(s)
}

/// "JUMP + UP to grab the ceiling." (dica 3 do `WIZARD.RES`, magos da T_M_004 em (1263, 1154)
/// e da T_M_008 em (50, 390)): a sala do mago não tem teto e o andar de cima fica 128 px
/// acima do chão, com o teto dele 232 px acima. O pulo para cima (20) só chega lá no voo
/// livre depois da animação (`0x4295a8`, 216 px): agarra o teto (47), vai mão ante mão (49)
/// até cima da laje e BAIXO o solta sobre ela.
#[test]
fn grabs_the_ceiling_where_the_tutorials_ask() {
    let j = |s: &Scene| s.jake.as_ref().unwrap().clone();
    for (lev, at, ceiling, ledge_x, ledge) in [
        ("T_M_004.LEV", (1263, 1230), 1036, 1390, 1140),
        ("T_M_008.LEV", (64, 470), 268, 110, 372),
    ] {
        let Some(mut s) = load(lev) else {
            return;
        };
        s.run_ticks(3, 0);
        place(&mut s, at, 0);
        s.run_ticks(15, 0);
        let b = j(&s).body.bounds();
        assert!(j(&s).body.on_ground, "{lev}: no chão");
        assert_eq!(
            b.y + b.h - ledge,
            128,
            "{lev}: a laje fica uma célula acima"
        );
        s.tick(JUMP | UP);
        assert!(
            until(&mut s, UP, 40, |s| j(s).state() != State::JumpUp(false)),
            "{lev}: o pulo acaba"
        );
        assert_eq!(j(&s).state(), State::BarHang, "{lev}: agarra o teto");
        assert_eq!(j(&s).body.bounds().y, ceiling, "{lev}: rente ao teto");
        assert!(
            until(&mut s, RIGHT, 200, |s| j(s).body.bounds().x >= ledge_x),
            "{lev}: mão ante mão até a laje"
        );
        assert!(until(&mut s, 0, 40, |s| j(s).state() == State::BarHang));
        s.tick(DOWN);
        assert_eq!(j(&s).state(), State::Fall, "{lev}: BAIXO solta");
        assert!(until(&mut s, 0, 60, |s| j(s).body.on_ground && j(s).body.vy == 0));
        let b = j(&s).body.bounds();
        assert_eq!(b.y + b.h, ledge, "{lev}: em pé na laje");
    }
}

/// Alturas dos pulos com o esqueleto e a escala reais, num chão sem teto: Garathe (`MINOTAUR.SKE`,
/// S = 39 × 58, K = 28 × 50) 20 → 216 px, 21 → 122 px, 22 → 30 px, 25 → 42 px; Jake (`JAKE.SKE`,
/// 22 × 47 e 20 × 57) 16 → 79 px, 27 → 43 px. O 19 sobe 210 px (`vy = −30`).
#[test]
fn jump_heights_with_the_real_skeletons() {
    let w = openhunted_engine::physics::World::new(vec![openhunted_engine::physics::Rect {
        x: -10_000,
        y: 1000,
        w: 20_000,
        h: 20,
    }]);
    let peak = |lev: &str, first: u32, rest: u32| -> Option<i32> {
        let mut j = load(lev)?.jake.unwrap();
        j.body.x = 0;
        j.body.y = 900;
        for _ in 0..60 {
            j.tick(0, &w);
        }
        assert!(j.body.on_ground);
        let y0 = j.body.y;
        let mut top = y0;
        j.tick(first, &w);
        for _ in 0..80 {
            j.tick(rest, &w);
            top = top.min(j.body.y);
        }
        assert!(j.body.on_ground, "{lev}: pousou");
        Some(y0 - top)
    };
    let Some(up) = peak("T_M_002.LEV", JUMP | UP, UP) else {
        return;
    };
    assert_eq!(up, 216, "Garathe, pulo para cima (20)");
    assert_eq!(
        peak("T_M_002.LEV", JUMP, 0),
        Some(122),
        "Garathe, parado (21)"
    );
    assert_eq!(
        peak("T_M_002.LEV", JUMP | RIGHT, RIGHT),
        Some(30),
        "Garathe, médio (22)"
    );
    assert_eq!(
        peak("T_M_002.LEV", JUMP | UP | RIGHT, RIGHT),
        Some(42),
        "Garathe, longo (25)"
    );
    assert_eq!(
        peak("T_J_001.LEV", JUMP | UP, UP),
        Some(79),
        "Jake, para cima (16)"
    );
    assert_eq!(peak("T_J_001.LEV", JUMP, 0), Some(43), "Jake, parado (27)");
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
    /// Rota, não luta: a vida e o relógio são repostos a cada tique (como o modo deus da
    /// auditoria).
    god: bool,
}

impl Pilot {
    fn tick(&mut self, input: u32) {
        if self.god {
            self.s.jake.as_mut().unwrap().health.hp = 10_000;
            self.s.timer = self.s.time_limit * openhunted_engine::scene::TIMER_TICKS_PER_SECOND;
        }
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
    fn state(&self) -> State {
        self.s.jake.as_ref().unwrap().state()
    }
    /// PULAR+CIMA parado, CIMA até o fim do pulo (20): agarra o teto (47).
    fn grab_ceiling(&mut self) {
        self.tick(JUMP | UP);
        for _ in 0..40 {
            if self.state() != State::JumpUp(false) {
                break;
            }
            self.tick(UP);
        }
        assert_eq!(
            self.state(),
            State::BarHang,
            "não agarrou o teto (em {:?})",
            self.center()
        );
    }
    /// PULAR+CIMA parado, CIMA na animação e, no voo livre, o lado (controle no ar,
    /// `0x408205`) que leva o centro a `x` (até 4 px), até pousar.
    fn jump_up_to(&mut self, x: i32) {
        self.tick(JUMP | UP);
        for _ in 0..80 {
            let j = self.s.jake.as_ref().unwrap();
            if j.body.on_ground && j.state() != State::JumpUp(false) {
                break;
            }
            let c = self.center().0;
            let steer = if j.body.vy == 0 || (c - x).abs() <= 4 {
                0
            } else if c > x {
                LEFT
            } else {
                RIGHT
            };
            self.tick(UP | steer);
        }
        self.settle();
    }
    /// Mão ante mão para `side` até o teto acabar e cair (27), segurando `side` no ar até o
    /// chão.
    fn bar_off(&mut self, side: u32) {
        for _ in 0..600 {
            if self.on_ground() && self.state() != State::BarHang {
                break;
            }
            self.tick(side);
        }
        self.settle();
    }
    /// Mão ante mão (49) até o centro passar de `x`, de volta ao 47 e BAIXO: cai.
    fn bar_to(&mut self, x: i32) {
        let right = self.center().0 < x;
        for _ in 0..600 {
            let c = self.center().0;
            if (right && c >= x) || (!right && c <= x) {
                break;
            }
            self.tick(if right { RIGHT } else { LEFT });
        }
        for _ in 0..40 {
            if self.state() == State::BarHang {
                break;
            }
            self.tick(0);
        }
        assert_eq!(
            self.state(),
            State::BarHang,
            "na barra em {:?}",
            self.center()
        );
        self.tick(DOWN);
        self.settle();
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
        god: false,
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

/// I_M_102 (só a saída): cai pelo poço da sala do início (camada 2, x 406–490) até a sala
/// de baixo e passa à camada 1 pela porta de x ≈ 703; sobe a rede da esquerda (x 430–463)
/// até a saliência de y 756, anda até a rede de x 562–595 e sobe à sala de cima, saltando
/// dela para a direita; passa à camada 0 pela porta de x ≈ 711, cai pelo vão de x 278–362,
/// empurra a porta de x 128–152 (id 2; ela abre a de baixo pelo gatilho) e cai pela coluna
/// da esquerda até o corredor da saída.
#[test]
fn i_m_102_can_be_won() {
    let Some(s) = load("I_M_102.LEV") else {
        return;
    };
    let mut p = Pilot {
        s,
        t: 0,
        climbed: 0,
        god: false,
    };
    p.run(10, 0);
    p.go_x(448);
    p.settle();
    assert_eq!(p.feet(), 884, "desceu o poço: {:?}", p.center());
    p.go_x(703);
    p.tick(UP);
    p.run(10, 0);
    assert_eq!(p.s.jake_layer, 1);
    p.go_x(446);
    p.climb(UP, 740);
    p.run(6, RIGHT);
    p.settle();
    assert_eq!(p.feet(), 756, "saliência: {:?}", p.center());
    p.go_x(578);
    p.climb(UP, 495);
    p.tick(JUMP | RIGHT);
    p.run(30, RIGHT);
    p.settle();
    assert_eq!(p.feet(), 500, "sala de cima: {:?}", p.center());
    p.go_x(711);
    p.tick(UP);
    p.run(10, 0);
    assert_eq!(p.s.jake_layer, 0);
    p.go_x(320);
    p.settle();
    assert_eq!(p.feet(), 628, "caiu pelo vão: {:?}", p.center());
    p.go_x(65);
    assert_eq!(p.feet(), 884, "caiu pela coluna: {:?}", p.center());
    p.win();
}

/// I_M_109 ("Garathe Tutorial 5"): a porta de célula (5, 3, 0) diante da saída pede a chave
/// 62, no corredor de cima da camada 0, e não há cordas. Rota (vida e relógio repostos, sem
/// lutar com olhos laser e barris), agarrando o teto em vãos de uma célula (pulo 20 em voo
/// livre, 47, mão ante mão 49, BAIXO sobre a laje 128 px acima):
/// - camada 1: do início, pula o fosso de x 662–746 e sobe pelo vão do teto de x 406–490;
/// - camada 0 (posto na sala de x 278–618, que fica sob o corredor da porta de camada de
///   x ≈ 195): sobe ao corredor da chave (barris a caminho dela) e, de volta, ao da porta.
///
/// O poço entre as duas (camada 1, x 278–362) e a vitória estão em `i_m_109_can_be_won`.
#[test]
fn i_m_109_climbs_with_the_ceiling_grab() {
    let Some(s) = load("I_M_109.LEV") else {
        return;
    };
    let mut p = Pilot {
        s,
        t: 0,
        climbed: 0,
        god: true,
    };
    p.run(10, 0);
    assert_eq!((p.s.jake_layer, p.feet()), (1, 372));
    p.go_x(782);
    p.tick(JUMP | LEFT);
    p.run(20, LEFT);
    p.settle();
    assert_eq!(p.feet(), 372, "pulou o fosso: {:?}", p.center());
    p.go_x(448);
    p.grab_ceiling();
    p.bar_to(380);
    assert_eq!(p.feet(), 244, "subiu pelo vão do teto: {:?}", p.center());
    place(&mut p.s, (604, 200), 0);
    p.run(5, 0);
    p.settle();
    assert_eq!((p.s.jake_layer, p.feet()), (0, 244));
    p.grab_ceiling();
    p.bar_to(660);
    assert_eq!(
        p.feet(),
        116,
        "subiu ao corredor da chave: {:?}",
        p.center()
    );
    // de volta, da sala ao corredor da porta de camada
    p.go_x(590);
    assert_eq!(p.feet(), 244, "de volta à sala: {:?}", p.center());
    p.grab_ceiling();
    p.bar_to(240);
    assert_eq!(
        p.feet(),
        116,
        "corredor da porta de camada: {:?}",
        p.center()
    );
}

/// I_M_109 ("Garathe Tutorial 5") vencida só pela entrada, sem repor vida nem relógio. A
/// saída fica atrás da porta de célula (5, 3, 0), que pede a chave 62 do corredor de cima da
/// camada 0, e não há cordas: as subidas são o pulo para cima (216 px), agarrando o teto ou,
/// no poço de x 278–362, pousando direto no corredor de cima com o controle no ar. O gerador
/// de minas do canto do teto do poço fica para trás (não é objetivo) e a poça `OPO` do pé
/// dele envenena na descida (1 de vida a cada 20 tiques até a saída).
#[test]
fn i_m_109_can_be_won() {
    let Some(s) = load("I_M_109.LEV") else {
        return;
    };
    let mut p = Pilot {
        s,
        t: 0,
        climbed: 0,
        god: false,
    };
    p.run(10, 0);
    // camada 1: pula o fosso e sobe pelo vão do teto da sala do início
    p.go_x(782);
    p.tick(JUMP | LEFT);
    p.run(20, LEFT);
    p.settle();
    assert_eq!(p.feet(), 372, "pulou o fosso: {:?}", p.center());
    p.go_x(448);
    p.grab_ceiling();
    // mão ante mão para a esquerda até o teto acabar (x 362) e cai segurando ESQUERDA,
    // longe da poça de veneno (x 331–397)
    p.bar_off(LEFT);
    assert_eq!(p.feet(), 244, "no chão do poço: {:?}", p.center());
    p.go_x(288);
    p.run(3, LEFT);
    p.settle();
    assert_eq!(p.center().0, 288, "encostado na parede");
    assert_eq!(p.s.jake.as_ref().unwrap().poison, 0, "não pisou na poça");
    // pulo para cima rente à parede da esquerda; no voo, ESQUERDA tira o Garathe de baixo
    // do gerador de minas (x 290–320, no canto do teto) e o leva ao corredor de cima
    p.jump_up_to(280);
    assert_eq!(
        p.feet(),
        116,
        "corredor da porta de camada: {:?}",
        p.center()
    );
    // porta de camada (zona 1, x 179–212): para a camada 0
    p.go_x(195);
    p.tick(UP);
    p.run(10, 0);
    assert_eq!(p.s.jake_layer, 0, "camada 0: {:?}", p.center());
    // cai na sala de x 278–618 e sobe ao corredor da chave pelo vão do teto
    p.go_x(604);
    assert_eq!(p.feet(), 244, "na sala: {:?}", p.center());
    p.grab_ceiling();
    p.bar_to(660);
    assert_eq!(p.feet(), 116, "corredor da chave: {:?}", p.center());
    // os barris de x 921–1061 cercam a chave (x 976–986): pulo médio sobre os dois
    // primeiros (tombam ao toque e rolam, mas não passam um pelo outro), corre por cima e
    // cai no vão da chave; volta do mesmo jeito
    p.go_x(866);
    p.tick(JUMP | RIGHT);
    p.run(25, RIGHT);
    p.settle();
    assert_ne!(
        p.s.jake.as_ref().unwrap().inventory.slots[13].1,
        0,
        "chave 62"
    );
    p.tick(JUMP | LEFT);
    p.run(30, LEFT);
    p.settle();
    assert!(p.center().0 < 900, "de volta dos barris: {:?}", p.center());
    // de volta à sala, ao corredor da porta de camada e à camada 1
    p.go_x(590);
    assert_eq!(p.feet(), 244, "de volta à sala: {:?}", p.center());
    p.grab_ceiling();
    p.bar_to(240);
    assert_eq!(p.feet(), 116, "corredor da porta: {:?}", p.center());
    p.go_x(195);
    p.tick(DOWN);
    p.run(10, 0);
    assert_eq!(p.s.jake_layer, 1, "camada 1: {:?}", p.center());
    // a catapulta (tipo 41, x 236–261) joga para a esquerda quem a pisa: pulo médio sobre
    // ela; segurando DIREITA, cai no poço, cruza a poça (envenena) e cai pelo vão do teto
    // na sala do início
    p.go_x(212);
    p.tick(JUMP | RIGHT);
    p.run(40, RIGHT);
    p.settle();
    assert_eq!(p.feet(), 372, "sala do início: {:?}", p.center());
    // pelo fosso (x 662–746) à sala de baixo e à porta de camada (zona 1, x 563–587)
    p.go_x(704);
    assert_eq!(p.feet(), 500, "sala de baixo: {:?}", p.center());
    p.go_x(575);
    p.tick(UP);
    p.run(10, 0);
    assert_eq!(p.s.jake_layer, 0, "camada 0 de baixo: {:?}", p.center());
    // a porta de célula (5, 3, 0, x 700–718) abre com a chave ao empurrar; depois dela, a
    // pilha de três barris (x 807–849): pulo parado e DIREITA no ar até cima deles, e por
    // cima (tombam e rolam) até a saída (x 900–912)
    p.go_x(778);
    p.tick(JUMP);
    p.run(3, 0);
    p.run(40, RIGHT);
    p.win();
    let j = p.s.jake.as_ref().unwrap();
    assert_eq!(j.poison, 1, "envenenado na poça, sem antídoto na fase");
    assert!(j.health.hp > 50, "vida {:?}", j.health);
    assert!(p.t < 20 * 300, "dentro dos 300 s: {} tiques", p.t);
}

/// Ataque na barra (`0x42a29b`/`0x42a4af` → `0x43b43e`) sobre os dados: no poço da `I_M_109`,
/// mão ante mão até encostar no gerador de minas do canto do teto e CIMA+ATIRAR com as garras:
/// o Garathe solta, golpeia no ar (t = 3) e tira 7 dos 15 de vida do gerador.
#[test]
fn claws_hit_the_mine_generator_from_the_ceiling() {
    let Some(mut s) = load("I_M_109.LEV") else {
        return;
    };
    let gen_hp = |s: &Scene| s.entities.iter().find(|e| e.kind == 103).unwrap().health;
    s.run_ticks(3, 0);
    place(&mut s, (345, 210), 1);
    s.run_ticks(15, 0);
    s.tick(openhunted_engine::input::PREV_WEAPON);
    s.tick(0);
    assert_eq!(
        s.jake.as_ref().unwrap().inventory.weapon,
        openhunted_engine::jake::weapons::CLAWS
    );
    let j = |s: &Scene| s.jake.as_ref().unwrap().clone();
    s.tick(JUMP | UP);
    assert!(until(&mut s, UP, 40, |s| j(s).state() == State::BarHang));
    assert!(until(&mut s, LEFT, 80, |s| j(s).body.bounds().x <= 320));
    assert_eq!(
        j(&s).body.bounds().x,
        320,
        "encostado no gerador (x 290–320)"
    );
    assert_eq!(gen_hp(&s), 15);
    s.tick(SHOOT | UP);
    assert!(matches!(j(&s).state(), State::Fire { .. }), "solta e ataca");
    s.run_ticks(4, SHOOT | UP);
    assert_eq!(gen_hp(&s), 8, "golpe das garras no ar");
    assert!(until(&mut s, 0, 40, |s| j(s).body.on_ground));
}

/// A poça `OPO` (tipo 68, 66 × 8) do pé do poço da `I_M_109` (camada 1, x 331–397, sobre o
/// chão de y 244): a cada 10 tiques (`values[0]`) quem a cruza leva 1 de veneno e fica
/// envenenado (`+0x270 = 1`, `0x40fa16`); o Garathe fala 3737 ("Oh no it's poison") só na
/// primeira vez. Fora dela (x < 331), nada.
#[test]
fn the_shaft_puddle_poisons() {
    let Some(mut s) = load("I_M_109.LEV") else {
        return;
    };
    let puddle = s.entities.iter().find(|e| e.kind == 68).unwrap();
    assert_eq!((puddle.x, puddle.y, puddle.layer), (331, 237, 1));
    assert_eq!(puddle.values.first().copied().unwrap_or(0), 10);
    s.run_ticks(3, 0);
    place(&mut s, (300, 210), 1);
    s.run_ticks(30, 0);
    let j = s.jake.as_ref().unwrap();
    assert_eq!((j.poison, j.body.bounds().y + j.body.bounds().h), (0, 244));
    place(&mut s, (360, 210), 1);
    s.events.clear();
    let mut speech = 0;
    let mut first = None;
    for t in 0..40 {
        s.tick(0);
        speech += s
            .events
            .drain(..)
            .filter(|e| e.sound_id() == Some(3737))
            .count();
        if first.is_none() && s.jake.as_ref().unwrap().poison > 0 {
            first = Some(t);
        }
    }
    assert!(first.is_some_and(|t| t < 11), "envenenado em até 10 tiques");
    let j = s.jake.as_ref().unwrap();
    assert_eq!(j.poison, 1);
    assert_eq!(speech, 1, "a fala só na primeira vez");
}

/// Entrada aleatória em todas as fases `p=2` solo: sem pânico e nunca dentro do cenário.
#[test]
#[ignore = "fuzz longo: rode com check.sh --full"]
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
