// SPDX-License-Identifier: GPL-3.0-or-later
//! Arquivos do jogador (decisões do port, `docs/superpowers/specs/2026-11-06-m9-casca-design.md`):
//! a configuração (`hunter.ini`, formato do `HUNTER.INI`) na pasta de configuração do usuário
//! e, na pasta de dados, a lista de heróis (`hunter.fam`) e os jogos salvos (`saves/*.hhg`).

use std::path::{Path, PathBuf};

use openhunted_engine::shell::config::Config;
use openhunted_formats::{HeroList, SaveGame};

/// Tamanho máximo do nome do jogo salvo.
pub const SAVE_NAME_MAX: usize = 32;

/// Onde ficam os arquivos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Storage {
    pub config_file: PathBuf,
    pub data_dir: PathBuf,
}

/// Pasta base do XDG: a variável, se absoluta; senão `$HOME/<padrão>`.
fn xdg(var: &str, default: &str) -> Option<PathBuf> {
    std::env::var_os(var)
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(default)))
}

/// Nome de arquivo seguro: letras e dígitos ASCII, espaço, `-` e `_`, até 32.
pub fn clean_name(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '-' | '_'))
        .take(SAVE_NAME_MAX)
        .collect::<String>()
        .trim()
        .to_string()
}

impl Storage {
    /// `$XDG_CONFIG_HOME/openhunted/hunter.ini` e `$XDG_DATA_HOME/openhunted/`.
    pub fn from_env() -> Option<Self> {
        Some(Self {
            config_file: xdg("XDG_CONFIG_HOME", ".config")?
                .join("openhunted")
                .join("hunter.ini"),
            data_dir: xdg("XDG_DATA_HOME", ".local/share")?.join("openhunted"),
        })
    }

    /// Tudo dentro de `root` (testes).
    pub fn at(root: &Path) -> Self {
        Self {
            config_file: root.join("config").join("hunter.ini"),
            data_dir: root.join("data"),
        }
    }

    pub fn load_config(&self) -> Config {
        Config::parse(&std::fs::read_to_string(&self.config_file).unwrap_or_default())
    }

    /// Grava a configuração mantendo o resto do arquivo.
    pub fn save_config(&self, c: &Config) -> std::io::Result<()> {
        let old = std::fs::read_to_string(&self.config_file).unwrap_or_default();
        write(&self.config_file, c.write(&old).as_bytes())
    }

    fn heroes_file(&self) -> PathBuf {
        self.data_dir.join("hunter.fam")
    }

    /// Lista de heróis (sem arquivo ou ilegível: vazia).
    pub fn heroes(&self) -> HeroList {
        std::fs::read(self.heroes_file())
            .ok()
            .and_then(|d| HeroList::parse(&d).ok())
            .unwrap_or_default()
    }

    /// Grava a lista com uma chave sorteada (como o original, a cada gravação).
    pub fn save_heroes(&self, l: &HeroList) -> std::io::Result<()> {
        let key = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0x5a, |d| (d.subsec_nanos() >> 8) as u8);
        write(&self.heroes_file(), &l.to_bytes(key))
    }

    pub fn saves_dir(&self) -> PathBuf {
        self.data_dir.join("saves")
    }

    /// Caminho do jogo salvo de nome `name` (já limpo).
    pub fn save_path(&self, name: &str) -> PathBuf {
        self.saves_dir().join(format!("{name}.hhg"))
    }

    /// Nome (sem extensão) do jogo salvo mais recente.
    pub fn latest_save(&self) -> Option<String> {
        std::fs::read_dir(self.saves_dir())
            .ok()?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .is_some_and(|x| x.eq_ignore_ascii_case("hhg"))
            })
            .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
            .max()
            .and_then(|(_, p)| Some(p.file_stem()?.to_string_lossy().into_owned()))
    }
}

/// Grava criando as pastas.
pub fn write(path: &Path, data: &[u8]) -> std::io::Result<()> {
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d)?;
    }
    std::fs::write(path, data)
}

/// Lê um jogo salvo (`None` se falta ou não é um `.hhg` válido).
pub fn read_save(path: &Path) -> Option<SaveGame> {
    SaveGame::parse(&std::fs::read(path).ok()?).ok()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Pasta temporária única para um teste (apagada no fim pelo chamador, se quiser).
    pub(crate) fn temp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("openhunted-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn config_heroes_and_saves_round_trip_in_their_folders() {
        let root = temp("storage");
        let st = Storage::at(&root);
        assert_eq!(st.load_config(), Config::default());
        let c = Config {
            music_volume: 60,
            ..Default::default()
        };
        st.save_config(&c).unwrap();
        assert_eq!(st.load_config(), c);
        assert!(st.config_file.ends_with("config/hunter.ini"));
        let mut l = st.heroes();
        l.record("ANA", 10, 0);
        st.save_heroes(&l).unwrap();
        assert_eq!(st.heroes(), l);
        assert_eq!(st.latest_save(), None);
        let p = st.save_path("um");
        let s = SaveGame {
            name: "ANA".into(),
            ..Default::default()
        };
        write(&p, &s.to_bytes()).unwrap();
        assert_eq!(read_save(&p), Some(s));
        assert_eq!(st.latest_save().as_deref(), Some("um"));
        assert_eq!(read_save(&st.save_path("nada")), None);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn names_are_cleaned() {
        assert_eq!(clean_name(" ../Meu jogo:1! "), "Meu jogo1");
        assert_eq!(clean_name(&"x".repeat(50)).len(), 32);
        assert_eq!(clean_name("é"), "");
    }
}
