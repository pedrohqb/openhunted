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
    /// Renderiza um quadro 640×480 de uma fase em PNG (sem janela).
    Render {
        /// Arquivo da fase (ex.: T_J_001.LEV).
        level: String,
        #[arg(long)]
        out: PathBuf,
        /// Câmera "X,Y" em pixels da fase (limitada à cena).
        #[arg(long, default_value = "0,0", value_parser = parse_xy)]
        camera: (i32, i32),
        /// Desenha caixas de colisão e marcadores de objetos de lógica.
        #[arg(long)]
        overlay: bool,
        /// Avança N ticks de animação antes de renderizar.
        #[arg(long, default_value_t = 0)]
        ticks: u32,
    },
}

fn parse_xy(s: &str) -> Result<(i32, i32), String> {
    let (a, b) = s.split_once(',').ok_or("use X,Y")?;
    Ok((
        a.trim().parse().map_err(|e| format!("{e}"))?,
        b.trim().parse().map_err(|e| format!("{e}"))?,
    ))
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
        Cmd::Render {
            level,
            out: path,
            camera,
            overlay,
            ticks,
        } => {
            use openhunted_engine::{
                Assets, Framebuffer, SCREEN_H, SCREEN_W, Scene, background_index, clamp_camera,
            };
            let lv = Level::parse(&gd.levels()?.get(&level)?).map_err(|e| e.in_resource(&level))?;
            let palette = openhunted_formats::Palette::parse(ar.get(&lv.palette)?)?;
            let mut assets = Assets::new(gd.resources()?);
            let mut scene = Scene::build(&lv, &mut assets)?;
            for w in &scene.warnings {
                eprintln!("aviso: {w}");
            }
            for _ in 0..ticks {
                scene.tick(0);
            }
            let view = (SCREEN_W as i32, SCREEN_H as i32);
            let cam = clamp_camera((scene.width, scene.height), view, camera);
            let mut fb = Framebuffer::new(SCREEN_W, SCREEN_H);
            fb.clear(background_index(&palette));
            scene.draw(&mut assets, &mut fb, cam, overlay);
            extract::write_indexed_png(
                &path,
                fb.width as u32,
                fb.height as u32,
                &fb.pixels,
                &palette,
            )?;
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
