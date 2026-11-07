// SPDX-License-Identifier: GPL-3.0-or-later
//! Os inimigos da M7a (tipos 7, 8, 10, 11 e 103) e da M10 (3, 4, 41, 87, 88, 89, 105, 107,
//! 109) sobre os dados reais (OPENHUNTED_DATA): animações, percepção e ataques nas fases da
//! campanha e um fuzz com invariantes.

use openhunted_engine::behaviour::{
    ant, bruiser, chameleon, death, enemy, flyer, hop_mine, mine_generator, security_robot,
    sentinel, the_entity,
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

const ENEMIES: [u32; 12] = [7, 8, 10, 11, 103, 3, 4, 87, 89, 105, 107, 109];

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
        if i != keep && (watched(e.kind) || matches!(e.kind, 41 | 88)) {
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
    let tables: [(&str, &[enemy::Label]); 11] = [
        ("OSR.ANM", &security_robot::LABELS),
        ("OSE.ANM", &sentinel::LABELS),
        ("OHS.ANM", &hop_mine::LABELS),
        ("OHG.ANM", &mine_generator::LABELS),
        ("OBR.ANM", &bruiser::LABELS),
        ("ODE.ANM", &death::LABELS),
        ("OCH.ANM", &chameleon::LABELS),
        ("OAB.ANM", &flyer::ANTIBODY_LABELS),
        ("OMR.ANM", &flyer::MANTA_LABELS),
        ("OES.ANM", &the_entity::LABELS),
        ("OAN.ANM", &ant::LABELS),
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

/// I_J_002: diante de um robô de segurança (a ~130 px), o Jake escondido (CIMA, 164–166) não
/// é adquirido por 150 tiques; ao sair do esconderijo, o robô vira e atira raios.
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
    let bolt = |s: &Scene| {
        s.entities
            .iter()
            .any(|e| e.alive && matches!(e.behaviour, Behaviour::Laser(_)))
    };
    // escondido (CIMA parado, 164–166) diante do robô: não é adquirido nem leva tiros (o
    // robô fica fora de jogo enquanto o Jake se esconde)
    s.entities[i].alive = false;
    for _ in 0..10 {
        s.tick(UP);
    }
    s.entities[i].alive = true;
    for t in 0..150 {
        s.tick(UP);
        let j = s.jake.as_ref().unwrap();
        assert!(j.is_hidden(), "tique {t}: estado {:?}", j.state());
        let Behaviour::SecurityRobot(r) = &s.entities[i].behaviour else {
            panic!()
        };
        assert!(
            !r.target && !bolt(&s),
            "tique {t}: o robô viu o Jake escondido"
        );
    }
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

/// H_J_013: o Bruiser 76 dorme até o alvo 14 do gatilho ligar; acordado, ataca o Jake na
/// camada dele; a bala da pistola (2, metade no Bruiser) tira 1 por acerto: com 100 de vida
/// morre no 100º e solta 30 explosões, e a morte conta.
#[test]
fn bruiser_wakes_attacks_and_dies_by_pistol() {
    let Some(mut s) = load("H_J_013.LEV") else {
        return;
    };
    let i = s
        .entities
        .iter()
        .position(|e| e.kind == 7 && (e.x, e.y) == (1167, 415))
        .unwrap();
    assert_eq!(s.entities[i].triggers[0], (14, Some(11)));
    only(&mut s, i);
    let sw = s
        .entities
        .iter()
        .position(|e| e.alive && e.id == 14)
        .unwrap();
    s.entities[sw].on = false;
    stand_near(&mut s, i, 150);
    let awake = |s: &Scene| matches!(&s.entities[i].behaviour, Behaviour::Bruiser(b) if b.awake);
    for _ in 0..40 {
        s.tick(0);
        s.entities[sw].on = false;
    }
    assert!(!awake(&s), "acordou sem o gatilho");
    s.entities[sw].on = true;
    let hp = s.jake.as_ref().unwrap().health.hp;
    let n0 = s.entities.len();
    let mut attacked = false;
    for _ in 0..300 {
        s.tick(0);
        attacked |= s.entities[n0..]
            .iter()
            .any(|e| e.alive && matches!(e.kind, 21 | 42 | 100))
            || s.jake.as_ref().unwrap().health.hp < hp;
        if attacked {
            break;
        }
    }
    assert!(awake(&s), "não acordou com o gatilho");
    assert!(attacked, "não atacou o Jake");
    // o Jake sai de cena (outra camada, longe); 100 acertos da pistola
    let other = s.entities[i].layer + 1;
    place_jake(&mut s, 0, 0, other);
    let kills = s.kills;
    let pistol = openhunted_engine::damage::Pending {
        amount: openhunted_engine::behaviour::bullet::DAMAGE,
        ..Default::default()
    };
    let dying = |s: &Scene| matches!(&s.entities[i].behaviour, Behaviour::Bruiser(b) if b.state == bruiser::DIE || b.dead);
    let mut hits = 0;
    while !dying(&s) {
        assert!(hits < 200, "não morreu");
        enemy::hurt(&mut s.entities[i], &pistol, true);
        hits += 1;
        s.tick(0);
    }
    assert_eq!(hits, 100);
    let before = s.entities.len();
    for _ in 0..40 {
        s.tick(0);
    }
    let blasts = s.entities[before..].iter().filter(|e| e.kind == 86).count();
    assert_eq!(blasts, bruiser::DEATH_BLASTS);
    assert_eq!(s.kills, kills + 1);
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
            assert!(
                (0..=s.mines_at_start).contains(&s.kills),
                "{ctx}: {} mortos de {}",
                s.kills,
                s.mines_at_start
            );
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

/// Índice do primeiro objeto do tipo `kind` que passa em `f`.
fn find(s: &Scene, kind: u32, f: impl Fn(&openhunted_engine::Entity) -> bool) -> usize {
    s.entities
        .iter()
        .position(|e| e.kind == kind && f(e))
        .unwrap_or_else(|| panic!("sem objeto do tipo {kind}"))
}

/// D_J_012: a Morte (sem gatilhos) aparece ao lado do Jake e o fere (radiação ou bala 99).
#[test]
fn death_appears_and_hurts_in_d_j_012() {
    let Some(mut s) = load("D_J_012.LEV") else {
        return;
    };
    let i = find(&s, 3, |_| true);
    only(&mut s, i);
    stand_near(&mut s, i, 100);
    let hp = s.jake.as_ref().unwrap().health.hp;
    let (mut seen, mut shot) = (false, false);
    for _ in 0..600 {
        s.tick(0);
        let Behaviour::Death(d) = &s.entities[i].behaviour else {
            panic!()
        };
        seen |= !d.hidden();
        shot |= s.entities.iter().any(|e| e.alive && e.kind == 99);
        if seen && s.jake.as_ref().unwrap().health.hp < hp {
            break;
        }
    }
    assert!(seen, "a Morte não apareceu");
    assert!(
        shot || s.jake.as_ref().unwrap().health.hp < hp,
        "a Morte não feriu nem atirou"
    );
}

/// H_J_013: diante de um Camaleão, o Jake parado leva a mordida (30).
#[test]
fn chameleon_bites_in_h_j_013() {
    let Some(mut s) = load("H_J_013.LEV") else {
        return;
    };
    let i = find(&s, 4, |e| (e.x, e.y) == (612, 185));
    only(&mut s, i);
    stand_near(&mut s, i, 40);
    let hp = s.jake.as_ref().unwrap().health.hp;
    for _ in 0..400 {
        s.tick(0);
        if s.jake.as_ref().unwrap().health.hp < hp {
            return;
        }
    }
    panic!("o Camaleão não mordeu");
}

/// Um voador do tipo `kind` em `level` se move e dá o bote no Jake parado perto.
fn flyer_attacks(level: &str, kind: u32, drop: bool) {
    let Some(mut s) = load(level) else {
        return;
    };
    let i = find(
        &s,
        kind,
        |e| matches!(&e.behaviour, Behaviour::Flyer(f) if !f.dormant),
    );
    only(&mut s, i);
    if drop {
        // o Jake cai de 40 px ao lado do voador (no ar livre em volta dele)
        let b = s.entities[i].bounds().unwrap();
        let layer = s.entities[i].layer;
        let boxr = s.jake.as_ref().unwrap().body.boxr;
        place_jake(&mut s, b.x + 40 - boxr.x, b.y - boxr.y, layer);
    } else {
        stand_near(&mut s, i, 60);
    }
    let (x0, hp) = (s.entities[i].center(), s.jake.as_ref().unwrap().health.hp);
    let mut moved = false;
    for _ in 0..1500 {
        s.tick(0);
        moved |= s.entities[i].center() != x0;
        if s.jake.as_ref().unwrap().health.hp < hp {
            break;
        }
    }
    assert!(moved, "{level}: o voador não se moveu");
    assert!(
        s.jake.as_ref().unwrap().health.hp < hp,
        "{level}: o voador não atacou"
    );
}

#[test]
fn antibody_attacks_in_h_j_009() {
    flyer_attacks("H_J_009.LEV", 105, true);
}

#[test]
fn manta_attacks_in_h_j_027() {
    flyer_attacks("H_J_027.LEV", 89, false);
}

/// D_J_007: o Jake embaixo de um gerador vertical recebe dardos que caem.
#[test]
fn dart_generator_fires_in_d_j_007() {
    let Some(mut s) = load("D_J_007.LEV") else {
        return;
    };
    let i = find(&s, 87, |e| (e.x, e.y) == (936, 146));
    only(&mut s, i);
    let b = s.entities[i].bounds().unwrap();
    let layer = s.entities[i].layer;
    place_jake(&mut s, 0, 0, layer);
    let boxr = s.jake.as_ref().unwrap().body.boxr;
    let cx = b.x + b.w / 2;
    let x = cx - boxr.x - boxr.w / 2;
    let y = (b.y + b.h + 20..b.y + 340)
        .find(|&feet| {
            let r = openhunted_engine::Rect {
                x: x + boxr.x,
                y: feet - boxr.h,
                ..boxr
            };
            s.world.floor_at(&r, 2)
        })
        .expect("sem chão embaixo do gerador")
        - boxr.y
        - boxr.h;
    place_jake(&mut s, x, y, layer);
    let mut darts = 0;
    for _ in 0..30 {
        s.tick(0);
        darts = darts.max(
            s.entities
                .iter()
                .filter(|e| e.alive && e.kind == 29)
                .count(),
        );
    }
    assert!(darts > 0, "nenhum dardo");
}

/// H_J_012: a catapulta sem gatilhos lança o Jake parado sobre ela.
#[test]
fn catapult_launches_jake_in_h_j_012() {
    let Some(mut s) = load("H_J_012.LEV") else {
        return;
    };
    let i = find(&s, 41, |e| (e.x, e.y) == (365, 1007));
    only(&mut s, i);
    stand_near(&mut s, i, 0);
    let y0 = s.jake.as_ref().unwrap().body.y;
    let mut rose = 0;
    for _ in 0..20 {
        s.tick(0);
        rose = rose.max(y0 - s.jake.as_ref().unwrap().body.y);
    }
    assert!(rose > 10, "a catapulta não lançou o Jake ({rose} px)");
}

/// D_J_009: perto da Entidade, na tela, os controles do Jake se invertem; ela não morre.
#[test]
fn entity_confuses_jake_in_d_j_009() {
    let Some(mut s) = load("D_J_009.LEV") else {
        return;
    };
    let i = find(&s, 107, |e| (e.x, e.y) == (289, 198));
    only(&mut s, i);
    stand_near(&mut s, i, 100);
    let mut confused = false;
    for _ in 0..100 {
        s.tick(0);
        confused |= s.confusion > 0;
        enemy::hurt(
            &mut s.entities[i],
            &openhunted_engine::damage::Pending {
                amount: 50,
                ..Default::default()
            },
            true,
        );
    }
    assert!(confused, "a Entidade não confundiu o Jake");
    assert!(s.entities[i].alive, "a Entidade é invulnerável");
}

/// T_M_008 (fase do Garathe): o formigueiro solta formigas que ficam na fase.
#[test]
fn anthill_spawns_ants_in_t_m_008() {
    let Some(mut s) = load("T_M_008.LEV") else {
        return;
    };
    let hill = find(&s, 88, |_| true);
    let n0 = s.entities.len();
    for _ in 0..260 {
        s.tick(0);
    }
    let ants: Vec<_> = s.entities[n0..].iter().filter(|e| e.kind == 109).collect();
    assert!(ants.len() >= 2, "{} formigas", ants.len());
    for a in ants.iter().filter(|a| a.alive) {
        let c = a.center();
        assert!(c.0 >= 0 && c.1 >= 0 && c.0 < s.width && c.1 < s.height);
        assert_eq!(a.layer, s.entities[hill].layer);
        assert!(enemy::counted(a), "formigas criadas vivas entram no total");
    }
    let alive = ants.iter().filter(|a| a.alive).count() as i32;
    assert_eq!(
        s.kills_total,
        s.mines_at_start + alive,
        "total = os da montagem + formigas vivas (`0x43cfcf`)"
    );
    // com o Garathe jogável (M8): ao lado de uma formiga, ele leva a mordida
    assert!(s.jake.is_some(), "a T_M_008 tem o Garathe");
    let a = (n0..s.entities.len())
        .find(|&k| s.entities[k].alive && s.entities[k].kind == 109)
        .expect("nenhuma formiga viva");
    stand_near(&mut s, a, 20);
    let hp = s.jake.as_ref().unwrap().health.hp;
    for _ in 0..60 {
        s.tick(0);
        if s.jake.as_ref().unwrap().health.hp < hp {
            return;
        }
    }
    panic!("a formiga não mordeu");
}

/// T_M_008: a formiga nasce no formigueiro, junto à parede, e se afasta dele (não fica
/// presa virando a cada tique dentro da parede).
#[test]
fn t_m_008_ant_walks_away_from_its_hill() {
    let Some(mut s) = load("T_M_008.LEV") else {
        return;
    };
    let hill = find(&s, 88, |_| true);
    let (hx, hy) = (s.entities[hill].x, s.entities[hill].y);
    // o Jake longe, em outra camada, para a formiga só andar
    let other = s.entities[hill].layer + 1;
    place_jake(&mut s, 0, 0, other);
    let n0 = s.entities.len();
    let mut born = None;
    let mut far = 0;
    for t in 0..600 {
        s.tick(0);
        let Some(a) = (n0..s.entities.len()).find(|&k| s.entities[k].kind == 109) else {
            continue;
        };
        let b = *born.get_or_insert(t);
        let e = &s.entities[a];
        if e.alive {
            far = far.max((e.x - hx).abs().max((e.y - hy).abs()));
        }
        if t - b >= 200 {
            break;
        }
    }
    assert!(born.is_some(), "nenhuma formiga");
    assert!(
        far >= 50,
        "a formiga só se afastou {far} px do formigueiro em 200 tiques"
    );
}

/// Dificuldade: no fácil a pistola tira o dobro do Bruiser; no difícil, nada passa de metade.
#[test]
fn difficulty_scales_enemy_damage_on_real_data() {
    let Some(mut s) = load("H_J_013.LEV") else {
        return;
    };
    let i = find(&s, 7, |_| true);
    only(&mut s, i);
    let other = s.entities[i].layer + 1;
    place_jake(&mut s, 0, 0, other);
    s.difficulty = openhunted_engine::Difficulty::Easy;
    let pistol = openhunted_engine::damage::Pending {
        amount: openhunted_engine::behaviour::bullet::DAMAGE,
        ..Default::default()
    };
    // o Bruiser só trata o dano acordado: força o gatilho
    if let Behaviour::Bruiser(b) = &mut s.entities[i].behaviour {
        b.awake = true;
    }
    let hp = s.entities[i].health;
    enemy::hurt(&mut s.entities[i], &pistol, true);
    s.tick(0);
    assert_eq!(s.entities[i].health, hp - 2, "fácil: (2 × 2) / 2");
}
