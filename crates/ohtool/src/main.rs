// SPDX-License-Identifier: GPL-3.0-or-later
//! `ohtool`: lista, inspeciona e extrai os recursos do Hunter Hunted.

mod extract;

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use openhunted_formats::{Bitmap, GameVersion, Level, chunks, open_data, parse_mission_list};

#[derive(Parser)]
#[command(
    version,
    about = "Inspeciona e extrai os dados do Hunter Hunted (CD original)"
)]
struct Cli {
    /// Raiz do CD, pasta Hunter/, RESOURCE.MAP ou imagem .iso do CD.
    #[arg(long)]
    data: PathBuf,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Lista os recursos: NOME, VOLUME e TAMANHO separados por tabulação.
    List {
        /// Só a extensão dada (ex.: BMP).
        #[arg(long = "type")]
        kind: Option<String>,
    },
    /// Mostra o tipo, os blocos e, para bitmaps, os quadros de um recurso.
    Info { name: String },
    /// Extrai recursos: bitmaps em PNG, o resto como está.
    Extract {
        #[arg(long)]
        out: PathBuf,
        #[arg(long = "type")]
        kind: Option<String>,
        /// Paleta usada nos PNG.
        #[arg(long, default_value = "WORLD.PAL")]
        palette: String,
    },
    /// Lista as missões (ARQUIVO, JOGADOR, TÍTULO) do hunter.hls; com --out, grava o
    /// cabeçalho de cada fase em JSON.
    Levels {
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) if is_broken_pipe(e.as_ref()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("erro: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Saída fechada antes do fim (ex.: `ohtool list | head`) não é erro.
fn is_broken_pipe(e: &(dyn std::error::Error + 'static)) -> bool {
    e.downcast_ref::<std::io::Error>()
        .is_some_and(|io| io.kind() == std::io::ErrorKind::BrokenPipe)
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    let gd = open_data(&cli.data)?;
    if gd.version() == GameVersion::Unknown {
        eprintln!(
            "aviso: versão do jogo não reconhecida (esperada a 1.00 do Windows 95); continuando"
        );
    }
    let ar = gd.resources()?;
    let mut out = std::io::stdout().lock();
    match cli.cmd {
        Cmd::List { kind } => {
            for e in ar.entries() {
                if extract::matches_type(&e.name, kind.as_deref()) {
                    writeln!(out, "{}\t{}\t{}", e.name, e.volume, e.size)?;
                }
            }
        }
        Cmd::Info { name } => {
            let data = ar.get(&name)?;
            writeln!(
                out,
                "{name}: tipo {}, {} bytes",
                extract::extension(&name),
                data.len()
            )?;
            let tags: Vec<String> = chunks(data)
                .map_while(Result::ok)
                .map(|c| format!("{}({})", String::from_utf8_lossy(&c.tag), c.data.len()))
                .collect();
            if !tags.is_empty() {
                writeln!(out, "blocos: {}", tags.join(" "))?;
            }
            if let Some(json) = extract::structured_json(&name, data) {
                writeln!(out, "{}", json?)?;
            }
            if extract::extension(&name) == "BMP" {
                let bmp = Bitmap::parse(data)?;
                writeln!(out, "quadros: {}", bmp.frames.len())?;
                for (i, f) in bmp.frames.iter().enumerate() {
                    writeln!(out, "  {i:3}: {}x{}", f.width, f.height)?;
                }
            }
        }
        Cmd::Levels { out: dir } => {
            let la = gd.levels()?;
            let missions = parse_mission_list(&la.get("ENGLEVEL.TXT")?)?;
            if let Some(dir) = &dir {
                std::fs::create_dir_all(dir)?;
            }
            for m in &missions {
                writeln!(out, "{}\t{}\t{}", m.file, m.player, m.title)?;
                if let Some(dir) = &dir {
                    let level =
                        Level::parse(&la.get(&m.file)?).map_err(|e| e.in_resource(&m.file))?;
                    let name = format!("{}.json", extract::safe_file_name(&m.file));
                    std::fs::write(dir.join(name), serde_json::to_string_pretty(&level)?)?;
                }
            }
        }
        Cmd::Extract {
            out: dir,
            kind,
            palette,
        } => {
            let n = extract::extract(&ar, &dir, kind.as_deref(), &palette)?;
            eprintln!("{n} arquivos escritos em {}", dir.display());
        }
    }
    Ok(())
}
