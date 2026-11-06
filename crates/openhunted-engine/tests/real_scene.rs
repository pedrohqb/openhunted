// SPDX-License-Identifier: GPL-3.0-or-later
//! Cena montada a partir dos dados reais (OPENHUNTED_DATA).

use openhunted_engine::{Assets, Behaviour, Event, Layer, Scene};
use openhunted_formats::{Level, open_data};

pub fn load(level: &str) -> Option<(Scene, Assets)> {
    let Some(p) = std::env::var_os("OPENHUNTED_DATA") else {
        eprintln!("pulado: OPENHUNTED_DATA não definido");
        return None;
    };
    let gd = open_data(std::path::Path::new(&p)).unwrap();
    let l = Level::parse(&gd.levels().unwrap().get(level).unwrap()).unwrap();
    let mut a = Assets::new(gd.resources().unwrap());
    Some((Scene::build(&l, &mut a).unwrap(), a))
}

#[test]
fn tutorial_scene_has_all_cells_and_objects() {
    let Some((s, _)) = load("T_J_001.LEV") else {
        return;
    };
    assert_eq!((s.width, s.height), (14 * 128, 128));
    assert_eq!(
        s.sprites
            .iter()
            .filter(|p| p.layer == Layer::Scenery)
            .count(),
        14
    );
    assert_eq!(
        s.entities.iter().filter(|e| e.sprite.is_some()).count(),
        10 // 8 globais (o Jake fica à parte) + 2 de células
    );
    assert_eq!(s.entities.len(), 14); // + 2 zonas e 2 dicas (sem sprite)
    assert!(s.jake.is_some() && s.jake_sprite.is_some());
    assert_eq!(s.markers.len(), 4);
    assert!(s.warnings.is_empty(), "{:?}", s.warnings);
    // Decorações: o primeiro campo é o quadro da animação (DECALW1.ANM, quadros 41, 187, 185).
    let decos: Vec<usize> = s
        .sprites
        .iter()
        .filter(|p| p.layer == Layer::Decoration)
        .map(|p| p.player.frame())
        .collect();
    assert_eq!(decos, vec![41, 187, 185]);
}

fn fnv1a(data: &[u8]) -> u64 {
    data.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3)
    })
}

/// Render de referência da T_J_001 na câmera (0, 0), conferido visualmente em 2026-10-31.
#[test]
fn tutorial_render_matches_reference() {
    use openhunted_engine::{Framebuffer, SCREEN_H, SCREEN_W, background_index};
    let Some((s, mut a)) = load("T_J_001.LEV") else {
        return;
    };
    let pal = openhunted_formats::Palette::parse(a.archive().get("WORLD.PAL").unwrap()).unwrap();
    let mut fb = Framebuffer::new(SCREEN_W, SCREEN_H);
    fb.clear(background_index(&pal));
    s.draw(&mut a, &mut fb, (0, 0), false);
    let h = fnv1a(&fb.pixels);
    eprintln!("hash do render: {h:#018x}");
    assert_eq!(h, REFERENCE);
}

/// Atualizado na M4a: a saída mostra o rótulo 8 (`osw08`, o portão "EXIT").
const REFERENCE: u64 = 0x6d24_9829_bdba_056f;

/// Camadas z ≥ 1 usam o índice 0 como transparência: nada de índice 0 pode chegar à tela.
#[test]
fn multilayer_level_has_no_transparent_holes() {
    use openhunted_engine::{Framebuffer, SCREEN_H, SCREEN_W};
    for lev in ["I_J_101.LEV", "H_J_001.LEV", "C_J_027.LEV"] {
        let Some((s, mut a)) = load(lev) else { return };
        let mut fb = Framebuffer::new(SCREEN_W, SCREEN_H);
        fb.clear(37);
        s.draw(&mut a, &mut fb, (0, 0), false);
        let holes = fb.pixels.iter().filter(|&&p| p == 0).count();
        assert_eq!(holes, 0, "{lev}: {holes} pixels de índice 0");
    }
}

/// Objetos do tipo 6 (OGF.ANM, uma folha de itens num rótulo só) ficam parados.
#[test]
fn generic_item_objects_do_not_cycle() {
    let Some((mut s, _)) = load("H_J_027.LEV") else {
        return;
    };
    let frames = |s: &openhunted_engine::Scene| -> Vec<usize> {
        s.entities
            .iter()
            .filter_map(|e| e.sprite.as_ref())
            .filter(|p| p.anim.frames.len() == 256)
            .map(|p| p.player.frame())
            .collect()
    };
    let before = frames(&s);
    assert!(!before.is_empty());
    for _ in 0..40 {
        s.tick(0);
    }
    assert_eq!(frames(&s), before);
}

/// O Jake da T_J_001 é desenhado a 47/88 (tamanho-alvo 22×47 nos campos do objeto).
#[test]
fn jake_is_drawn_scaled() {
    let Some((s, _)) = load("T_J_001.LEV") else {
        return;
    };
    let jake = s.jake_sprite.as_ref().unwrap();
    assert_eq!(jake.scale, (47, 88));
}

/// Na T_J_001 o Jake pousa no chão e corre para a esquerda sem atravessar o cenário.
#[test]
fn jake_lands_and_runs_left_in_tutorial() {
    use openhunted_engine::input::LEFT;
    let Some((mut s, _)) = load("T_J_001.LEV") else {
        return;
    };
    let x0 = s.jake.as_ref().unwrap().body.x;
    for _ in 0..20 {
        s.tick(0);
    }
    assert!(s.jake.as_ref().unwrap().body.on_ground, "não pousou");
    for _ in 0..150 {
        s.tick(LEFT);
        let b = s.jake.as_ref().unwrap().body.bounds();
        assert!(b.y + b.h <= 128 && b.x >= 0, "saiu do cenário: {b:?}");
    }
    assert!(s.jake.as_ref().unwrap().body.x < x0 - 400);
}

/// Nenhum objeto é ampliado mais de 2×: o quadro de referência da escala é o do rótulo do
/// tipo do objeto (ou, numa folha, o de proporção igual ao alvo), não o quadro 0.
#[test]
fn object_scale_reference_matches_object_kind() {
    let Some(p) = std::env::var_os("OPENHUNTED_DATA") else {
        return;
    };
    let gd = open_data(std::path::Path::new(&p)).unwrap();
    let la = gd.levels().unwrap();
    let missions =
        openhunted_formats::parse_mission_list(&la.get("ENGLEVEL.TXT").unwrap()).unwrap();
    let mut a = Assets::new(gd.resources().unwrap());
    let mut bad = Vec::new();
    for m in &missions {
        let l = Level::parse(&la.get(&m.file).unwrap()).unwrap();
        let s = Scene::build(&l, &mut a).unwrap();
        for sp in s.entities.iter().filter_map(|e| e.sprite.as_ref()) {
            let (num, den) = sp.scale;
            if num > 2 * den {
                bad.push(format!("{} {}x{} {:?}", m.file, sp.x, sp.y, sp.scale));
            }
        }
    }
    assert!(
        bad.is_empty(),
        "{} objetos ampliados demais: {:?}",
        bad.len(),
        &bad[..bad.len().min(10)]
    );
}

/// O Jake controlável é o início do jogador no modo solo (na D_J_009 o primeiro objeto de
/// tipo 1 é outro); numa fase de vários jogadores, sem início solo, é o primeiro tipo 1.
#[test]
fn player_start_is_jake() {
    let Some((s, _)) = load("D_J_009.LEV") else {
        return;
    };
    let j = s.jake.as_ref().unwrap();
    assert_eq!((j.body.x, j.body.y), (573, 581));
    let (s, _) = load("C_J_006.LEV").unwrap();
    let j = s.jake.as_ref().unwrap();
    assert_eq!((j.body.x, j.body.y), (110, 709));
}

/// Na T_J_007 o Jake nasce sobre peças OPL (objetos): parado, fica de pé.
#[test]
fn jake_stands_on_objects_in_t_j_007() {
    let Some((mut s, _)) = load("T_J_007.LEV") else {
        return;
    };
    let y0 = s.jake.as_ref().unwrap().body.y;
    for _ in 0..40 {
        s.tick(0);
    }
    let j = s.jake.as_ref().unwrap();
    assert!(j.body.y - y0 <= 16, "caiu {} px", j.body.y - y0);
    assert!(j.body.on_ground);
}

/// Todas as fases do `ENGLEVEL.TXT`.
fn mission_files() -> Vec<String> {
    let p = std::env::var_os("OPENHUNTED_DATA").unwrap();
    let gd = open_data(std::path::Path::new(&p)).unwrap();
    let la = gd.levels().unwrap();
    openhunted_formats::parse_mission_list(&la.get("ENGLEVEL.TXT").unwrap())
        .unwrap()
        .into_iter()
        .map(|m| m.file)
        .collect()
}

/// Índices dos sólidos das células que `r` intersecta (os objetos móveis, como as minas,
/// podem encostar no Jake de propósito).
fn overlapped(s: &Scene, r: &openhunted_engine::Rect) -> Vec<usize> {
    let solids = s.cell_world().solids();
    (0..solids.len())
        .filter(|&i| solids[i].overlaps(r))
        .collect()
}

/// Em toda fase com Jake, entradas aleatórias (LCG fixa) nunca o fazem entrar num sólido que
/// ele ainda não intersectava (pode nascer sobreposto, como na I_J_107, e sair).
#[test]
fn random_input_never_enters_solids() {
    use openhunted_engine::input::{JUMP, LEFT, RIGHT};
    if std::env::var_os("OPENHUNTED_DATA").is_none() {
        return;
    }
    for level in mission_files() {
        let (mut s, _) = load(&level).unwrap();
        if s.jake.is_none() {
            continue;
        }
        let mut seed: u32 = 0x1234_5678;
        let mut input = 0;
        let mut inside = overlapped(&s, &s.jake.as_ref().unwrap().body.bounds());
        for t in 0..1000 {
            if t % 7 == 0 {
                seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
                input =
                    [0, LEFT, RIGHT, LEFT | JUMP, RIGHT | JUMP, JUMP][(seed >> 16) as usize % 6];
            }
            s.tick(input);
            let now = overlapped(&s, &s.jake.as_ref().unwrap().body.bounds());
            assert!(
                now.iter().all(|i| inside.contains(i)),
                "{level}: entrou num sólido no tique {t}"
            );
            inside = now;
        }
    }
}

/// Em toda fase com Jake, ele responde: depois de assentar, anda para algum dos lados.
#[test]
fn jake_can_move_in_every_level() {
    use openhunted_engine::input::{LEFT, RIGHT};
    if std::env::var_os("OPENHUNTED_DATA").is_none() {
        return;
    }
    for level in mission_files() {
        let (s0, _) = load(&level).unwrap();
        if s0.jake.is_none() {
            continue;
        }
        let moved = [LEFT, RIGHT].iter().any(|&dir| {
            let mut s = s0.clone();
            for _ in 0..40 {
                s.tick(0);
            }
            let x0 = s.jake.as_ref().unwrap().body.x;
            for _ in 0..10 {
                s.tick(dir);
            }
            s.jake.as_ref().unwrap().body.x != x0
        });
        assert!(moved, "{level}: o Jake não sai do lugar");
    }
}

/// O Jake da T_J_001 começa com vida 50 e armadura 0 (campos do objeto).
#[test]
fn jake_health_comes_from_level() {
    let Some((s, _)) = load("T_J_001.LEV") else {
        return;
    };
    let h = s.jake.as_ref().unwrap().health;
    assert_eq!((h.hp, h.armor), (50, 0));
}

/// Na T_J_001, correr para a esquerda dispara a dica das minas; ao chegar perto da saída, a
/// dica "To exit press UP." (textos do `WIZARD.RES`, linhas 2 e 24).
#[test]
fn tutorial_hints_fire_in_order() {
    use openhunted_engine::input::{JUMP, LEFT};
    let Some((mut s, _)) = load("T_J_001.LEV") else {
        return;
    };
    let mut hints = Vec::new();
    for t in 0..900 {
        let j = s.jake.as_ref().unwrap();
        let input = if j.body.hit_wall && t % 10 == 0 {
            LEFT | JUMP
        } else {
            LEFT
        };
        s.tick(input);
        for ev in s.events.drain(..) {
            if let Event::Hint { text, left_of_jake } = ev {
                hints.push((text, left_of_jake, s.jake.as_ref().unwrap().body.x));
            }
        }
        if s.jake.as_ref().unwrap().body.x < 120 {
            break;
        }
    }
    let j = s.jake.as_ref().unwrap();
    assert_eq!(
        hints.len(),
        2,
        "{hints:?} x={} estado {:?}",
        j.body.x,
        j.state()
    );
    assert_eq!(
        hints[0].0,
        "Hovering mines explode on contact. SHOOT them before they get you."
    );
    assert!(
        !hints[0].1,
        "ao disparar, o Jake (x≈1430) já passou o mago 1 (x=1514)"
    );
    assert_eq!(hints[1].0, "To exit press UP.");
    assert!(hints[1].2 < 200);
}

/// Na T_J_001, correndo para a esquerda o Jake apanha a vida (50 → 75) e a armadura
/// (0 → 25) do caminho; os itens somem.
#[test]
fn tutorial_items_are_picked_up() {
    use openhunted_engine::input::{JUMP, LEFT};
    let Some((mut s, _)) = load("T_J_001.LEV") else {
        return;
    };
    let mut best = (0, 0);
    for t in 0..600 {
        let j = s.jake.as_ref().unwrap();
        let input = if j.body.hit_wall && t % 10 == 0 {
            LEFT | JUMP
        } else {
            LEFT
        };
        s.tick(input);
        let h = s.jake.as_ref().unwrap().health;
        best = (best.0.max(h.hp), best.1.max(h.armor));
        if s.jake.as_ref().unwrap().body.x < 1300 {
            break;
        }
    }
    // apanhou os dois (as minas podem ter tirado um pouco depois)
    assert!(
        s.entities
            .iter()
            .filter(|e| matches!(e.kind, 32 | 79))
            .all(|e| !e.alive)
    );
    assert!(
        best.0 >= 75 && best.1 >= 25,
        "melhor vida/armadura vistas: {best:?}"
    );
}

/// A T_J_001 tem 7 minas Alpha com vida 1; correndo às cegas para a esquerda, o Jake esbarra
/// nelas: explodem (aparece a explosão `OEX.ANM`), ele perde vida mas chega vivo à saída.
#[test]
fn tutorial_mines_explode_on_jake() {
    use openhunted_engine::input::{JUMP, LEFT};
    let Some((mut s, _)) = load("T_J_001.LEV") else {
        return;
    };
    let mines: Vec<_> = s
        .entities
        .iter()
        .filter(|e| e.kind == 102)
        .map(|e| {
            (
                e.health,
                matches!(&e.behaviour, Behaviour::Mine(m) if m.variant == 0),
            )
        })
        .collect();
    assert_eq!(mines, vec![(1, true); 7]);
    let mut saw_explosion = false;
    let mut hurt = false;
    let mut last_hp = 50;
    for t in 0..900 {
        let j = s.jake.as_ref().unwrap();
        let input = if j.body.hit_wall && t % 10 == 0 {
            LEFT | JUMP
        } else {
            LEFT
        };
        s.tick(input);
        saw_explosion |= s.entities.iter().any(|e| e.alive && e.kind == 86);
        let hp = s.jake.as_ref().unwrap().health.hp;
        hurt |= hp < last_hp;
        last_hp = hp;
        if s.jake.as_ref().unwrap().body.x < 120 || s.jake.as_ref().unwrap().is_dead() {
            break;
        }
    }
    let j = s.jake.as_ref().unwrap();
    assert!(saw_explosion);
    // a explosão toca os sons dos quadros do `OEX.ANM` (3080–3087), no ponto e na camada
    assert!(
        s.events.iter().any(|e| matches!(e,
            openhunted_engine::Event::Sound(c)
                if (3080..=3087).contains(&c.id) && c.at.is_some_and(|a| a.layer == s.jake_layer)))
    );
    assert!(hurt, "nenhuma mina feriu o Jake");
    assert!(!j.is_dead(), "o Jake morreu em x={}", j.body.x);
    assert!(j.body.x < 120, "parou em x={}", j.body.x);
    assert!(
        s.entities
            .iter()
            .filter(|e| e.kind == 102)
            .any(|e| !e.alive)
    );
}

/// A T_J_001 de ponta a ponta: correr para a esquerda até a saída (pulando quando bate em
/// algo), parar e apertar CIMA; 40 tiques depois é vitória, com o Jake vivo.
#[test]
fn t_j_001_can_be_won() {
    use openhunted_engine::input::{JUMP, LEFT, UP};
    let Some((mut s, _)) = load("T_J_001.LEV") else {
        return;
    };
    let mut t = 0;
    loop {
        let j = s.jake.as_ref().unwrap();
        assert!(!j.is_dead(), "o Jake morreu no tique {t} em x={}", j.body.x);
        assert!(t < 900, "não chegou à saída: x={}", j.body.x);
        if j.body.x < 100 {
            break;
        }
        let input = if j.body.hit_wall && t % 10 == 0 {
            LEFT | JUMP
        } else {
            LEFT
        };
        s.tick(input);
        t += 1;
    }
    for _ in 0..5 {
        s.tick(0);
    }
    let j = s.jake.as_ref().unwrap();
    assert!(j.can_use_exit(), "{:?}", j.state());
    s.tick(UP);
    assert!(
        s.entities.iter().any(|e| e.kind == 5 && e.on),
        "a saída não ligou"
    );
    for _ in 0..60 {
        s.tick(0);
    }
    assert_eq!(s.outcome, Some(openhunted_engine::Outcome::Victory));
    assert!(s.jake.as_ref().unwrap().health.hp > 0);
}

/// Todos os rótulos que a máquina de estados do Jake usa existem no `JAKE.ANM`.
#[test]
fn jake_state_labels_exist_in_the_real_animation() {
    let Some((_, mut a)) = load("T_J_001.LEV") else {
        return;
    };
    let anim = a.animation("JAKE.ANM").unwrap();
    let missing: Vec<u16> = openhunted_engine::jake::ALL_LABELS
        .iter()
        .copied()
        .filter(|id| !anim.labels.iter().any(|l| l.id == *id))
        .collect();
    assert!(missing.is_empty(), "rótulos ausentes: {missing:?}");
}

/// O Jake da T_J_001 começa com a pistola e 80 tiros (`points[2]`).
#[test]
fn tutorial_jake_starts_with_the_pistol() {
    let Some((s, _)) = load("T_J_001.LEV") else {
        return;
    };
    assert_eq!(s.jake.as_ref().unwrap().inventory.pistol_ammo(), 80);
}

/// Parado no início da T_J_001, olhando para a esquerda, uma rajada mata a mina mais
/// próxima (que vem ao encontro dele) e deixa a faísca `oex15`; a cápsula cai e some.
#[test]
fn tutorial_shooting_kills_the_nearest_mine() {
    use openhunted_engine::input::{LEFT, SHOOT};
    let Some((mut s, _)) = load("T_J_001.LEV") else {
        return;
    };
    assert_eq!(s.mines_at_start, 7, "as 7 minas da T_J_001");
    for _ in 0..20 {
        s.tick(0);
    }
    s.tick(LEFT); // vira para a esquerda
    let alive = |s: &Scene| {
        s.entities
            .iter()
            .filter(|e| e.alive && e.kind == 102)
            .count()
    };
    assert_eq!(alive(&s), 7);
    let mut saw_casing = false;
    let mut saw_spark = false;
    let mut t = 0;
    while alive(&s) == 7 && t < 200 {
        let input = if t % 6 == 0 { SHOOT } else { 0 };
        s.tick(input);
        saw_casing |= s.entities.iter().any(|e| e.alive && e.kind == 22);
        saw_spark |= s.entities.iter().any(|e| {
            e.alive
                && e.kind == 86
                && e.sprite.as_ref().is_some_and(|sp| {
                    sp.anim
                        .labels
                        .iter()
                        .any(|l| l.id == 15 && l.frame as usize <= sp.player.frame())
                        && sp
                            .anim
                            .labels
                            .iter()
                            .all(|l| l.id <= 15 || l.frame as usize > sp.player.frame())
                })
        });
        t += 1;
    }
    assert!(alive(&s) < 7, "nenhuma mina morreu em {t} tiques");
    assert!(saw_casing);
    assert!(saw_spark, "sem faísca oex15");
    let positioned = |id: u16| {
        s.events.iter().any(
            |e| matches!(e, openhunted_engine::Event::Sound(c) if c.id == id && c.at.is_some()),
        )
    };
    assert!(
        positioned(3040) && positioned(3046),
        "tiro e faísca posicionais"
    );
    assert!(s.jake.as_ref().unwrap().inventory.pistol_ammo() < 80);
    for _ in 0..60 {
        s.tick(0);
    }
    assert!(
        !s.entities.iter().any(|e| e.alive && e.kind == 22),
        "a cápsula não sumiu"
    );
}

/// Atirar ajuda: andando para a esquerda e disparando uma rajada quando há mina à frente na
/// mesma altura, o Jake vence com pelo menos a vida do roteiro sem tiros e mata minas a tiro.
#[test]
fn t_j_001_shooting_helps() {
    use openhunted_engine::input::{JUMP, LEFT, SHOOT, UP};
    let run = |shoot: bool| -> (i32, usize) {
        let (mut s, _) = load("T_J_001.LEV").unwrap();
        let mut t = 0;
        let mut killed_by_bullet = 0;
        let mut burst = 0;
        loop {
            let j = s.jake.as_ref().unwrap();
            assert!(!j.is_dead(), "morreu no tique {t} (atirando: {shoot})");
            assert!(t < 1500, "não chegou (atirando: {shoot}) x={}", j.body.x);
            if j.body.x < 100 {
                break;
            }
            let jb = j.body.bounds();
            let jc = (jb.x + jb.w / 2, jb.y + jb.h / 2);
            let mine_ahead = s.entities.iter().any(|e| {
                e.alive
                    && e.kind == 102
                    && e.center().0 < jc.0
                    && jc.0 - e.center().0 <= 160
                    && (e.center().1 - jc.1).abs() <= 20
            });
            let mines_before = s
                .entities
                .iter()
                .filter(|e| e.alive && e.kind == 102)
                .count();
            let input = if shoot && (mine_ahead || burst > 0) {
                if burst == 0 {
                    burst = 4;
                }
                burst -= 1;
                SHOOT
            } else if j.body.hit_wall && t % 10 == 0 {
                LEFT | JUMP
            } else {
                LEFT
            };
            s.tick(input);
            let mines_after = s
                .entities
                .iter()
                .filter(|e| e.alive && e.kind == 102)
                .count();
            if mines_after < mines_before && s.entities.iter().any(|e| e.alive && e.kind == 19) {
                killed_by_bullet += mines_before - mines_after;
            }
            t += 1;
        }
        for _ in 0..5 {
            s.tick(0);
        }
        s.tick(UP);
        for _ in 0..60 {
            s.tick(0);
        }
        assert_eq!(s.outcome, Some(openhunted_engine::Outcome::Victory));
        (s.jake.as_ref().unwrap().health.hp, killed_by_bullet)
    };
    if std::env::var_os("OPENHUNTED_DATA").is_none() {
        return;
    }
    let (hp_plain, _) = run(false);
    let (hp_shoot, killed) = run(true);
    assert!(killed >= 3, "só {killed} minas mortas a tiro");
    assert!(
        hp_shoot >= hp_plain,
        "atirando {hp_shoot} < sem atirar {hp_plain}"
    );
}

/// Em toda fase com Jake e pistola: empurrando uma parede e atirando para a frente, nenhuma
/// bala aparece do outro lado (a boca dentro da parede não dispara; a que nasce encostada
/// faísca na hora).
#[test]
fn bullets_never_cross_cell_walls() {
    use openhunted_engine::input::{LEFT, RIGHT, SHOOT};
    if std::env::var_os("OPENHUNTED_DATA").is_none() {
        return;
    }
    let mut checked = 0;
    for level in mission_files() {
        let (s0, _) = load(&level).unwrap();
        let Some(j0) = s0.jake.as_ref() else { continue };
        if j0.inventory.pistol_ammo() <= 0 {
            continue;
        }
        for dir in [LEFT, RIGHT] {
            let mut s = s0.clone();
            for _ in 0..30 {
                s.tick(0);
            }
            // anda até bater numa parede (ou desiste)
            let mut hit = false;
            for _ in 0..200 {
                s.tick(dir);
                if s.jake.as_ref().unwrap().body.hit_wall {
                    hit = true;
                    break;
                }
            }
            if !hit {
                continue;
            }
            checked += 1;
            for _ in 0..6 {
                s.tick(0);
            }
            for t in 0..30 {
                s.tick(if t < 8 { SHOOT } else { 0 });
                let jb = s.jake.as_ref().unwrap().body.bounds();
                let jc = (jb.x + jb.w / 2, jb.y + jb.h / 2);
                for b in s.entities.iter().filter(|e| e.alive && e.kind == 19) {
                    let bb = b.bounds().unwrap();
                    let bc = (bb.x + bb.w / 2, bb.y + bb.h / 2);
                    // o segmento do centro do Jake ao centro da bala não cruza sólido algum
                    let crossed = (1..16).any(|i| {
                        let p = openhunted_engine::Rect {
                            x: jc.0 + (bc.0 - jc.0) * i / 16,
                            y: jc.1 + (bc.1 - jc.1) * i / 16,
                            w: 1,
                            h: 1,
                        };
                        s.cell_world().blocked(&p)
                    });
                    assert!(!crossed, "{level}: bala atravessou a parede em {bb:?}");
                }
            }
        }
    }
    assert!(checked > 10, "poucas paredes testadas: {checked}");
}
