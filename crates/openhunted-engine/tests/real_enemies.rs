// SPDX-License-Identifier: GPL-3.0-or-later
//! Os inimigos da M7a (tipos 7, 8, 10, 11 e 103) sobre os dados reais (OPENHUNTED_DATA):
//! animações, percepção e ataques nas fases da campanha e um fuzz sem pânico.

use openhunted_engine::behaviour::{
    bruiser, enemy, hop_mine, mine_generator, security_robot, sentinel,
};
use openhunted_engine::input::{DOWN, JUMP, LEFT, RIGHT, SHOOT, UP};
use openhunted_engine::{AnimPlayer, Assets, Behaviour, Body, Scene};
use openhunted_formats::{Level, open_data};

fn data() -> Option<openhunted_formats::GameData> {
    let Some(p) = std::env::var_os("OPENHUNTED_DATA") else {
        eprintln!("pulado: OPENHUNTED_DATA não definido");
        return None;
    };
    Some(open_data(std::path::Path::new(&p)).unwrap())
}

fn load(level: &str) -> Option<Scene> {
    let gd = data()?;
    let l = Level::parse(&gd.levels().unwrap().get(level).unwrap()).unwrap();
    let mut a = Assets::new(gd.resources().unwrap());
    Some(Scene::build(&l, &mut a).unwrap())
}

const ENEMIES: [u32; 5] = [7, 8, 10, 11, 103];

/// Põe o Jake em (x, y) na camada `layer`, parado.
fn place_jake(s: &mut Scene, x: i32, y: i32, layer: i32) {
    let j = s.jake.as_mut().unwrap();
    j.body = Body::new(x, y, j.body.boxr);
    j.set_layer(layer);
    s.set_jake_layer(layer);
}

/// Põe o Jake de pé no chão da camada do objeto `i`, a `dx` px (de 60 em diante, dos dois
/// lados) do centro dele e com os pés na base dele; devolve a posição do corpo.
fn stand_near(s: &mut Scene, i: usize, near: i32) -> (i32, i32) {
    let b = s.entities[i].bounds().unwrap();
    let layer = s.entities[i].layer;
    place_jake(s, 0, 0, layer);
    let boxr = s.jake.as_ref().unwrap().body.boxr;
    let c = b.x + b.w / 2;
    for d in (near..near + 200).step_by(4) {
        for side in [-1, 1] {
            for dy in -40..120 {
                let feet = b.y + b.h + dy;
                let (x, y) = (c + side * d - boxr.x - boxr.w / 2, feet - boxr.y - boxr.h);
                let r = openhunted_engine::Rect {
                    x: x + boxr.x,
                    y: y + boxr.y,
                    ..boxr
                };
                if s.world.floor_at(&r, 2) {
                    place_jake(s, x, y, layer);
                    return (x, y);
                }
            }
        }
    }
    panic!("sem chão perto do objeto {i}");
}

/// Afasta da fase os inimigos que não são o do teste (para o Jake não morrer antes).
fn only(s: &mut Scene, keep: usize) {
    for (i, e) in s.entities.iter_mut().enumerate() {
        if i != keep && (ENEMIES.contains(&e.kind) || e.kind == 102 || e.kind == 104) {
            e.alive = false;
        }
    }
    s.rebuild_world();
}

#[test]
fn enemy_labels_match_the_real_animations() {
    let Some(gd) = data() else {
        return;
    };
    let mut a = Assets::new(gd.resources().unwrap());
    let tables: [(&str, &[enemy::Label]); 5] = [
        ("OSR.ANM", &security_robot::LABELS),
        ("OSE.ANM", &sentinel::LABELS),
        ("OHS.ANM", &hop_mine::LABELS),
        ("OHG.ANM", &mine_generator::LABELS),
        ("OBR.ANM", &bruiser::LABELS),
    ];
    for (name, table) in tables {
        let anim = a.animation(name).unwrap();
        for &(id, start, len) in table {
            let p = AnimPlayer::at_label(&anim, id).unwrap_or_else(|| panic!("{name} {id}"));
            assert_eq!((p.frame(), p.len()), (start, len), "{name} rótulo {id}");
        }
    }
}

/// I_J_001: o Jake a ~100 px diante de uma sentinela, na mesma altura: ela abre e atira.
#[test]
fn sentinel_opens_and_shoots_at_jake() {
    let Some(mut s) = load("I_J_001.LEV") else {
        return;
    };
    let i = s
        .entities
        .iter()
        .position(|e| e.kind == 10 && (e.x, e.y) == (655, 336))
        .unwrap();
    only(&mut s, i);
    stand_near(&mut s, i, 90);
    let mut shot = None;
    for t in 0..80 {
        s.tick(0);
        if s.entities.iter().any(|e| e.alive && e.kind == 106) {
            shot = Some(t);
            break;
        }
    }
    let t = shot.expect("a sentinela não atirou");
    assert!(t < 40, "demorou {t} tiques");
    let Behaviour::Sentinel(st) = &s.entities[i].behaviour else {
        panic!()
    };
    let jx = s.focus().unwrap().0;
    assert_eq!(
        st.facing_right,
        jx > s.entities[i].center().0,
        "abre para o lado do Jake"
    );
    // as balas chegam ao Jake
    let hp = s.jake.as_ref().unwrap().health.hp;
    for _ in 0..20 {
        s.tick(0);
    }
    assert!(
        s.jake.as_ref().unwrap().health.hp < hp,
        "as balas não feriram o Jake"
    );
}

/// I_J_103: a pistola mata uma mina saltadora e a cena conta a morte.
#[test]
fn pistol_kills_a_hop_mine_and_counts() {
    let Some(mut s) = load("I_J_103.LEV") else {
        return;
    };
    let i = s
        .entities
        .iter()
        .position(|e| e.kind == 11 && (e.x, e.y) == (672, 447))
        .unwrap();
    only(&mut s, i);
    stand_near(&mut s, i, 110);
    let kills = s.kills;
    // a pistola atira na altura do peito: a mina (17 px) só é alcançada em diagonal para
    // baixo, quando chega a uns 70 px
    for t in 0..200 {
        let c = s.focus().unwrap();
        let m = s.entities[i].center();
        let side = if m.0 < c.0 { LEFT } else { RIGHT };
        let input = if (m.0 - c.0).abs() <= 70 && t % 14 < 12 {
            DOWN | side | SHOOT
        } else {
            0
        };
        s.tick(input);
        if !s.entities[i].alive {
            break;
        }
    }
    assert!(!s.entities[i].alive, "a mina saltadora não morreu");
    assert_eq!(s.kills, kills + 1);
}

/// I_J_005: o gerador perto do início solta minas flutuantes (no máximo 4 vivas por gerador).
#[test]
fn generator_releases_mines_in_i_j_005() {
    let Some(mut s) = load("I_J_005.LEV") else {
        return;
    };
    let mut max = 0;
    let mut released = false;
    for _ in 0..200 {
        s.tick(0);
        let mut per: std::collections::HashMap<u32, usize> = Default::default();
        for e in &s.entities {
            if let Behaviour::Mine(m) = &e.behaviour {
                if e.alive && e.health > 0 && m.owner != 0 {
                    released = true;
                    *per.entry(m.owner).or_default() += 1;
                }
            }
        }
        max = max.max(per.values().copied().max().unwrap_or(0));
        if s.jake.as_ref().unwrap().is_dead() {
            break;
        }
    }
    assert!(released, "nenhuma mina solta");
    assert!(
        max <= mine_generator::SLOTS,
        "{max} minas vivas de um gerador"
    );
}

/// I_J_002: diante de um robô de segurança (a ~130 px), ele vira e atira raios; escondido
/// (CIMA contra a parede), um robô sem alvo não o vê.
#[test]
fn security_robot_shoots_at_jake() {
    let Some(mut s) = load("I_J_002.LEV") else {
        return;
    };
    let i = s
        .entities
        .iter()
        .position(|e| e.kind == 8 && (e.x, e.y) == (481, 316))
        .unwrap();
    only(&mut s, i);
    stand_near(&mut s, i, 120);
    let mut fired = false;
    for _ in 0..150 {
        s.tick(0);
        fired |= s
            .entities
            .iter()
            .any(|e| e.alive && matches!(e.behaviour, Behaviour::Laser(_)));
        if fired {
            break;
        }
    }
    assert!(fired, "o robô não atirou");
}

/// T_J_007: o Bruiser (camada diferente da do Jake) fica no chão dele por 300 tiques, com o
/// Jake parado na camada dele.
#[test]
fn t_j_007_bruiser_stays_on_its_floor() {
    let Some(mut s) = load("T_J_007.LEV") else {
        return;
    };
    let i = s.entities.iter().position(|e| e.kind == 7).unwrap();
    assert_ne!(
        s.entities[i].layer, s.jake_layer,
        "o teste quer camadas diferentes"
    );
    let y0 = s.entities[i].y;
    for t in 0..300 {
        s.tick(0);
        let e = &s.entities[i];
        assert!(
            (e.y - y0).abs() < 64 && e.alive,
            "tique {t}: Bruiser em y {} (começou em {y0})",
            e.y
        );
    }
}

/// I_J_110: as minas saltadoras das outras camadas não caem para fora da fase enquanto o
/// Jake fica parado na camada dele.
#[test]
fn i_j_110_hop_mines_on_other_layers_survive() {
    let Some(mut s) = load("I_J_110.LEV") else {
        return;
    };
    let mines: Vec<usize> = s
        .entities
        .iter()
        .enumerate()
        .filter(|(_, e)| e.kind == 11 && e.layer != s.jake_layer)
        .map(|(i, _)| i)
        .collect();
    assert!(mines.len() >= 4, "{} minas em outras camadas", mines.len());
    for _ in 0..300 {
        s.tick(0);
    }
    let alive = mines.iter().filter(|&&i| s.entities[i].alive).count();
    assert_eq!(alive, mines.len(), "minas sumidas");
}

/// Inimigos vigiados pelo fuzz: os da M7a, as minas e os olhos-laser.
fn watched(kind: u32) -> bool {
    ENEMIES.contains(&kind) || kind == 102 || kind == 104
}

/// Invariantes de um inimigo vivo: dentro da fase e fora das células da própria camada (se
/// começou fora delas).
fn check_enemy(s: &Scene, i: usize, started_inside: bool, ctx: &str) {
    let e = &s.entities[i];
    let Some(b) = e.bounds() else {
        return;
    };
    let c = (b.x + b.w / 2, b.y + b.h / 2);
    assert!(
        c.0 >= 0 && c.1 >= 0 && c.0 < s.width && c.1 < s.height,
        "{ctx}: objeto {i} (tipo {}) saiu da fase em {c:?}",
        e.kind
    );
    // o destroço do Bruiser e do OSR fica onde caiu
    let wreck = matches!(&e.behaviour, Behaviour::Bruiser(st) if st.dead);
    if !started_inside && !wreck {
        assert!(
            !s.layer_cells(e.layer).blocked(&b),
            "{ctx}: objeto {i} (tipo {}, camada {}) dentro das células em {b:?}",
            e.kind,
            e.layer
        );
    }
}

/// Em toda fase solo com estes inimigos, 2000 tiques de entrada aleatória: sem pânico, nenhum
/// inimigo sai da fase, entra nas células da própria camada ou some sem morrer.
#[test]
fn enemies_fuzz_without_panics() {
    let Some(gd) = data() else {
        return;
    };
    let la = gd.levels().unwrap();
    let missions =
        openhunted_formats::parse_mission_list(&la.get("ENGLEVEL.TXT").unwrap()).unwrap();
    let mut tested = 0;
    for m in missions {
        let l = Level::parse(&la.get(&m.file).unwrap()).unwrap();
        let mut a = Assets::new(gd.resources().unwrap());
        let mut s = Scene::build(&l, &mut a).unwrap();
        if s.jake.is_none() || !s.entities.iter().any(|e| ENEMIES.contains(&e.kind)) {
            continue;
        }
        tested += 1;
        // os inimigos da fase (os criados depois, como as minas dos geradores, ficam de fora)
        let start: Vec<(usize, bool)> = s
            .entities
            .iter()
            .enumerate()
            .filter(|(_, e)| e.alive && watched(e.kind))
            .map(|(i, e)| {
                let inside = e
                    .bounds()
                    .is_some_and(|b| s.layer_cells(e.layer).blocked(&b));
                (i, inside)
            })
            .collect();
        let mut seed: u32 = 0x5eed_0001 ^ tested;
        let mut input = 0;
        for t in 0..2000 {
            if t % 9 == 0 {
                seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
                input = [0, LEFT, RIGHT, LEFT | SHOOT, RIGHT | SHOOT, JUMP, UP, SHOOT]
                    [(seed >> 16) as usize % 8];
            }
            s.tick(input);
            if t % 10 != 9 {
                continue;
            }
            let ctx = format!("{} tique {t}", m.file);
            for &(i, inside) in &start {
                let e = &s.entities[i];
                if e.alive {
                    check_enemy(&s, i, inside, &ctx);
                } else {
                    assert!(
                        e.health <= 0,
                        "{ctx}: objeto {i} (tipo {}) sumiu com vida {}",
                        e.kind,
                        e.health
                    );
                }
            }
        }
    }
    assert!(tested >= 30, "só {tested} fases com inimigos");
}
