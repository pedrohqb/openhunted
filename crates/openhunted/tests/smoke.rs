// SPDX-License-Identifier: GPL-3.0-or-later
//! Teste de fumaça: abre a janela com o driver de vídeo "dummy", roda alguns quadros e sai.

use std::process::Command;

#[test]
fn opens_tutorial_and_exits_after_frames() {
    let Some(data) = std::env::var_os("OPENHUNTED_DATA") else {
        eprintln!("pulado: OPENHUNTED_DATA não definido");
        return;
    };
    let out = Command::new(env!("CARGO_BIN_EXE_openhunted"))
        .args(["--level", "T_J_001.LEV", "--frames", "5", "--data"])
        .arg(&data)
        .env("SDL_VIDEO_DRIVER", "dummy")
        .env("SDL_AUDIO_DRIVER", "dummy")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn missing_data_fails_with_message() {
    let empty = std::env::temp_dir().join("openhunted-sem-dados");
    let _ = std::fs::create_dir_all(&empty);
    let out = Command::new(env!("CARGO_BIN_EXE_openhunted"))
        .args(["--frames", "1", "--data"])
        .arg(&empty)
        .env("SDL_VIDEO_DRIVER", "dummy")
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("RESOURCE.MAP"));
}
