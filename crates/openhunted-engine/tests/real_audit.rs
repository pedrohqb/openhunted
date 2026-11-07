// SPDX-License-Identifier: GPL-3.0-or-later
//! Auditoria de vencibilidade das missões solo sobre os dados reais (OPENHUNTED_DATA; M13,
//! `openhunted_engine::audit`): o modo deus vence cada missão pela lógica de objetivos,
//! gatilhos e veredito.

use openhunted_engine::audit::{self, Verdict};
use openhunted_engine::{Assets, Scene};
use openhunted_formats::{GameData, Level, open_data, parse_mission_list};

fn data() -> Option<GameData> {
    let Some(p) = std::env::var_os("OPENHUNTED_DATA") else {
        eprintln!("pulado: OPENHUNTED_DATA não definido");
        return None;
    };
    Some(open_data(std::path::Path::new(&p)).unwrap())
}

fn scene(gd: &GameData, name: &str) -> Scene {
    let l = Level::parse(&gd.levels().unwrap().get(name).unwrap()).unwrap();
    let mut a = Assets::new(gd.resources().unwrap());
    Scene::build(&l, &mut a).unwrap()
}

/// Missões solo do `ENGLEVEL.TXT` (p = 1 ou 2).
fn solo(gd: &GameData) -> Vec<String> {
    let la = gd.levels().unwrap();
    parse_mission_list(&la.get("ENGLEVEL.TXT").unwrap())
        .unwrap()
        .into_iter()
        .filter(|m| matches!(m.player, 1 | 2))
        .map(|m| m.file)
        .collect()
}

fn god(gd: &GameData, name: &str) -> Verdict {
    audit::god_mode(&mut scene(gd, name))
}

/// Missões que o modo deus não vence, com a causa documentada em `docs/campanha.md`.
const KNOWN: [&str; 1] = [
    // antídoto-objetivo sem fonte de veneno na fase (como no original, ao que parece)
    "D_M_010.LEV",
];

/// Todas as missões solo vencem no modo deus, salvo as conhecidas; e nenhuma tem objetivo,
/// saída ou porta fora do alcance pela grade da IA (otimista).
/// (≈ 4 s em depuração: roda sempre.)
#[test]
fn every_solo_mission_wins_in_god_mode() {
    let Some(gd) = data() else { return };
    let names = solo(&gd);
    assert_eq!(names.len(), 65);
    let mut fails = Vec::new();
    for n in &names {
        let mut s = scene(&gd, n);
        for w in audit::reach(&s) {
            fails.push(format!("{n}: alcance: {w}"));
        }
        match audit::god_mode(&mut s) {
            Verdict::Win { .. } if KNOWN.contains(&n.as_str()) => {
                fails.push(format!("{n}: venceu (tirar de KNOWN)"));
            }
            Verdict::Fail(why) if !KNOWN.contains(&n.as_str()) => {
                fails.push(format!("{n}: {why}"));
            }
            _ => {}
        }
    }
    assert!(
        fails.is_empty(),
        "{} falhas:\n{}",
        fails.len(),
        fails.join("\n")
    );
}

/// D_M_010: a placa (zona 73 "acionar", duração 30, rearma ocupada) termina 7 px antes da
/// zona PR da saída; o Garathe em pé sobre as duas aperta CIMA e a placa segue ligada
/// enquanto ele entra na porta (o original conta o jogador na porta, `0x40e14e`). O antídoto
/// (objetivo) é recusado sem veneno e a fase não tem veneno: sem trapaça a auditoria falha
/// (`docs/campanha.md`); com o Garathe envenenado à mão, vence.
#[test]
fn d_m_010_plate_holds_while_entering_the_exit() {
    let Some(gd) = data() else { return };
    let Verdict::Fail(why) = god(&gd, "D_M_010.LEV") else {
        panic!("o antídoto passou sem veneno");
    };
    assert!(why.contains("antídoto"), "{why}");
    let mut s = scene(&gd, "D_M_010.LEV");
    s.jake.as_mut().unwrap().poison = 1;
    assert!(matches!(audit::god_mode(&mut s), Verdict::Win { .. }));
}

/// I_J_005: o Jake começa num corredor fechado da camada 0; o teletransporte de id 2
/// (473, 119) leva ao de id 4, na camada 2 (`0x41347e` muda a camada do passageiro). Antes,
/// só valiam destinos na mesma camada e a fase não tinha saída.
#[test]
fn i_j_005_teleporter_crosses_layers() {
    let Some(gd) = data() else { return };
    let mut s = scene(&gd, "I_J_005.LEV");
    let t = s
        .entities
        .iter()
        .position(|e| e.kind == 46 && e.id == 2)
        .unwrap();
    let (c, layer) = (s.entities[t].center(), s.entities[t].layer);
    assert_eq!(layer, 0);
    audit::teleport(&mut s, (c.0, c.1 - 60), layer);
    for _t in 0..80 {
        s.tick(0);
        if std::env::var_os("OH_TRACE").is_some() {
            let j = s.jake.as_ref().unwrap();
            eprintln!(
                "{_t} {:?} {:?} camada {}",
                j.body.bounds(),
                j.state(),
                j.layer()
            );
        }
    }
    let d = s
        .entities
        .iter()
        .find(|e| e.kind == 46 && e.id == 4)
        .unwrap();
    let (dc, dl) = (d.center(), d.layer);
    let j = s.jake.as_ref().unwrap();
    let b = j.body.bounds();
    assert_eq!(j.layer(), dl);
    assert_eq!(s.jake_layer, dl);
    assert!(
        (b.x + b.w / 2 - dc.0).abs() < 16 && (b.y - dc.1).abs() < 128,
        "{b:?} {dc:?}"
    );
}

/// Alcance estrito (sem a subida livre, como a IA): a saída da T_M_006 fica fora, porque o
/// Garathe pula do chão até a corda; o alcance otimista a vê. A T_J_001 não depende de pulo.
#[test]
fn strict_reach_flags_upward_jumps() {
    let Some(gd) = data() else { return };
    let s = scene(&gd, "T_M_006.lev");
    assert!(audit::reach(&s).is_empty());
    let w = audit::reach_strict(&s);
    assert!(w.iter().any(|w| w.starts_with("saída")), "{w:?}");
    assert!(audit::reach_strict(&scene(&gd, "T_J_001.lev")).is_empty());
}

/// D_J_015: a porta (1, 3, 2) da sala do início pede a chave 62, que não está no chão da
/// fase: o Jake a traz (par 13 dos `points`). O alcance parte com ela.
#[test]
fn d_j_015_starts_with_the_key_of_the_locked_room() {
    let Some(gd) = data() else { return };
    let s = scene(&gd, "D_J_015.LEV");
    assert_eq!(audit::held_keys(&s) & 1, 1);
    assert!(
        !s.entities.iter().any(|e| e.alive && e.kind == 62),
        "a 62 está na fase"
    );
    assert!(audit::reach(&s).is_empty(), "{:?}", audit::reach(&s));
}
