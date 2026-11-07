// SPDX-License-Identifier: GPL-3.0-or-later
//! `ohtool audit`: vencibilidade das missões solo (M13; `openhunted_engine::audit`).

use std::io::Write;

use openhunted_engine::audit::{self, Verdict};
use openhunted_engine::{Assets, Scene};
use openhunted_formats::{GameData, Level, parse_mission_list};

/// Missões solo do `ENGLEVEL.TXT` (p = 1 ou 2), na ordem do arquivo.
pub fn solo_missions(gd: &GameData) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let la = gd.levels()?;
    Ok(parse_mission_list(&la.get("ENGLEVEL.TXT")?)?
        .into_iter()
        .filter(|m| matches!(m.player, 1 | 2))
        .map(|m| m.file)
        .collect())
}

fn scene(gd: &GameData, name: &str) -> Result<Scene, Box<dyn std::error::Error>> {
    let la = gd.levels()?;
    let lv = Level::parse(&la.get(name)?).map_err(|e| e.in_resource(name))?;
    let mut assets = Assets::new(gd.resources()?);
    Ok(Scene::build(&lv, &mut assets)?)
}

pub fn run(
    gd: &GameData,
    only: Option<&str>,
    god: bool,
    (reach, strict): (bool, bool),
    out: &mut dyn Write,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut names = solo_missions(gd)?;
    if let Some(l) = only {
        names.retain(|n| n.eq_ignore_ascii_case(l));
        if names.is_empty() {
            names.push(l.to_string());
        }
    }
    let mut table = Vec::new();
    for name in &names {
        let mut s = scene(gd, name)?;
        let r = audit::inspect(&s);
        writeln!(
            out,
            "== {name}: {} | tempo {} s | chaves na fase {:#x}",
            r.character, r.time_limit, r.keys
        )?;
        for o in &r.objectives {
            writeln!(
                out,
                "  objetivo {:<16} tipo {:3} bits {:#x} {:<14} em {:?} camada {}{}",
                o.how.name(),
                o.kind,
                o.bits,
                o.behaviour,
                o.at,
                o.layer,
                o.note.as_ref().map_or(String::new(), |n| format!(" — {n}"))
            )?;
        }
        for (at, layer, zone) in &r.exits {
            writeln!(
                out,
                "  saída em {at:?} camada {layer}{}",
                if *zone { "" } else { " — SEM ZONA PR 0x40" }
            )?;
        }
        for d in r
            .doors
            .iter()
            .filter(|d| d.keys != 0 || openhunted_engine::triggers::armed(&d.triggers))
        {
            writeln!(
                out,
                "  porta {:?} {} chaves {:#x}{} gatilhos {:?}",
                d.cell,
                if d.closed { "fechada" } else { "aberta" },
                d.keys,
                if d.key_present {
                    ""
                } else {
                    " — CHAVE AUSENTE"
                },
                d.triggers
            )?;
        }
        for (kind, at, layer) in &r.static_blockers {
            writeln!(
                out,
                "  sólido sem comportamento: tipo {kind} em {at:?} camada {layer}"
            )?;
        }
        for t in &r.teleporters {
            let cross = t.targets.iter().any(|(_, l)| *l != t.layer);
            writeln!(
                out,
                "  teletransporte em {:?} camada {} → {:?}{}",
                t.at,
                t.layer,
                t.targets,
                if cross { " (outra camada)" } else { "" }
            )?;
        }
        for (kind, id) in &r.dangling {
            writeln!(out, "  gatilho do tipo {kind} para o id {id} inexistente")?;
        }
        if reach {
            for w in audit::reach(&s) {
                writeln!(out, "  alcance: {w}")?;
            }
            if strict {
                for w in audit::reach_strict(&s) {
                    writeln!(out, "  alcance estrito: {w}")?;
                }
            }
        }
        if god {
            let v = audit::god_mode(&mut s);
            writeln!(out, "  modo deus: {}", show(&v))?;
            table.push((name.clone(), v));
        }
    }
    if god {
        writeln!(out, "\n{:<14} resultado", "missão")?;
        for (n, v) in &table {
            writeln!(out, "{n:<14} {}", show(v))?;
        }
        let wins = table
            .iter()
            .filter(|(_, v)| matches!(v, Verdict::Win { .. }))
            .count();
        writeln!(out, "WIN {wins}, FAIL {}", table.len() - wins)?;
    }
    Ok(())
}

fn show(v: &Verdict) -> String {
    match v {
        Verdict::Win { ticks } => format!("WIN ({ticks} tiques)"),
        Verdict::Fail(why) => format!("FAIL({why})"),
    }
}
