// SPDX-License-Identifier: GPL-3.0-or-later
//! Arquivos do jogador (decisões do port, `docs/superpowers/specs/2026-11-06-m9-casca-design.md`):
//! a configuração (`hunter.ini`, formato do `HUNTER.INI`) na pasta de configuração do usuário
//! e, na pasta de dados, a lista de heróis (`hunter.fam`) e os jogos salvos (`saves/*.hhg`).

use std::io::Read;
use std::path::{Path, PathBuf};

use openhunted_engine::shell::config::Config;
use openhunted_engine::shell::heroes::name_ids;
use openhunted_formats::save::SAVE_SIZE;
use openhunted_formats::{HeroList, SaveGame};

/// Tamanho máximo do nome do jogo salvo: o do campo do `SHIGH` (`NameEntry`), para o nome
/// pré-preenchido voltar igual.
pub const SAVE_NAME_MAX: usize = name_ids::MAX;
/// Maior `hunter.fam` lido (14 linhas cabem com folga; o resto é ignorado).
pub const HEROES_MAX_BYTES: u64 = 64 * 1024;
/// Maior `hunter.ini` lido.
pub const CONFIG_MAX_BYTES: u64 = 1 << 20;

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

/// Nome de arquivo seguro: letras e dígitos ASCII, espaço, `-` e `_`, até 31.
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
        let text = read_capped(&self.config_file, CONFIG_MAX_BYTES)
            .ok()
            .map(|d| decode_ini(&d).0)
            .unwrap_or_default();
        Config::parse(&text)
    }

    /// Grava a configuração mantendo o resto do arquivo byte a byte. Ruling: um
    /// `hunter.ini` que não é UTF-8 (o do Windows é ANSI) é lido como Latin-1 e regravado
    /// assim, sem perder as outras linhas; se não dá para ler o arquivo existente (sem
    /// contar a falta dele), não grava por cima.
    pub fn save_config(&self, c: &Config) -> std::io::Result<()> {
        let old = match read_capped(&self.config_file, CONFIG_MAX_BYTES) {
            Ok(d) => d,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(e),
        };
        let (text, utf8) = decode_ini(&old);
        let new = c.write(&text);
        let bytes = if utf8 {
            new.into_bytes()
        } else {
            new.chars()
                .map(|ch| u8::try_from(u32::from(ch)))
                .collect::<Result<Vec<u8>, _>>()
                .map_err(|_| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "valor fora do Latin-1 num hunter.ini que não é UTF-8",
                    )
                })?
        };
        write(&self.config_file, &bytes)
    }

    fn heroes_file(&self) -> PathBuf {
        self.data_dir.join("hunter.fam")
    }

    /// Lista de heróis (sem arquivo ou ilegível: vazia; lê no máximo `HEROES_MAX_BYTES`).
    pub fn heroes(&self) -> HeroList {
        read_capped(&self.heroes_file(), HEROES_MAX_BYTES)
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

    /// Nome (sem extensão) do jogo salvo mais recente entre os que o port sabe nomear
    /// (nome já limpo, ver `clean_name`): o `SHIGH` o devolve e ele acha o mesmo arquivo.
    pub fn latest_save(&self) -> Option<String> {
        std::fs::read_dir(self.saves_dir())
            .ok()?
            .filter_map(|e| e.ok())
            .filter(|e| {
                let p = e.path();
                p.extension().is_some_and(|x| x == "hhg")
                    && p.file_stem()
                        .and_then(|s| s.to_str())
                        .is_some_and(|s| !s.is_empty() && clean_name(s) == s)
            })
            .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
            .max()
            .and_then(|(_, p)| Some(p.file_stem()?.to_string_lossy().into_owned()))
    }
}

/// Grava criando as pastas, de forma atômica: escreve um temporário na mesma pasta e o
/// renomeia por cima (uma queda no meio não deixa o arquivo pela metade).
pub fn write(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let dir = path.parent().filter(|d| !d.as_os_str().is_empty());
    if let Some(d) = dir {
        std::fs::create_dir_all(d)?;
    }
    let name = path
        .file_name()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "sem nome"))?;
    let mut tmp_name = std::ffi::OsString::from(".");
    tmp_name.push(name);
    tmp_name.push(format!(".{}.tmp", std::process::id()));
    let tmp = dir.map_or_else(|| PathBuf::from(&tmp_name), |d| d.join(&tmp_name));
    let r = (|| {
        use std::io::Write;
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(data)?;
        f.sync_all()?;
        std::fs::rename(&tmp, path)
    })();
    if r.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    r
}

/// Lê no máximo `max` bytes do arquivo (mais um, para quem quiser saber se passou).
fn read_capped(path: &Path, max: u64) -> std::io::Result<Vec<u8>> {
    let mut d = Vec::new();
    std::fs::File::open(path)?
        .take(max + 1)
        .read_to_end(&mut d)?;
    Ok(d)
}

/// Texto do `hunter.ini`: UTF-8 se for (`true`), senão Latin-1 (cada byte um caractere).
fn decode_ini(d: &[u8]) -> (String, bool) {
    match std::str::from_utf8(d) {
        Ok(s) => (s.to_string(), true),
        Err(_) => (d.iter().map(|&b| char::from(b)).collect(), false),
    }
}

/// Lê um jogo salvo (`None` se falta ou não é um `.hhg` válido); lê no máximo 1 961 bytes.
pub fn read_save(path: &Path) -> Option<SaveGame> {
    SaveGame::parse(&read_capped(path, SAVE_SIZE as u64).ok()?).ok()
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
    fn a_non_utf8_ini_keeps_its_other_bytes() {
        let root = temp("storage-ini");
        let st = Storage::at(&root);
        // `HUNTER.INI` do Windows em ANSI: "Caminho=C:\\Música" com o é em Latin-1
        let mut old =
            b"[Outro]\r\nCaminho=C:\\M\xfasica\r\n[Hunter]\r\nMusicVolume=30\r\n".to_vec();
        old.extend_from_slice(b"Extra=\xe9\r\n");
        write(&st.config_file, &old).unwrap();
        assert_eq!(st.load_config().music_volume, 30, "lê mesmo sem ser UTF-8");
        let c = Config {
            music_volume: 60,
            ..Default::default()
        };
        st.save_config(&c).unwrap();
        let new = std::fs::read(&st.config_file).unwrap();
        assert!(
            new.windows(14).any(|w| w == b"Caminho=C:\\M\xfas"),
            "byte a byte"
        );
        assert!(new.windows(7).any(|w| w == b"Extra=\xe9"));
        assert_eq!(st.load_config().music_volume, 60);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn reads_are_bounded_and_writes_atomic() {
        let root = temp("storage-bounds");
        let st = Storage::at(&root);
        // um .hhg com um byte a mais não é lido inteiro nem aceito
        let p = st.save_path("grande");
        let mut b = SaveGame::default().to_bytes();
        b.extend(std::iter::repeat_n(0u8, 1 << 20));
        write(&p, &b).unwrap();
        assert_eq!(read_save(&p), None);
        assert_eq!(
            read_capped(&p, SAVE_SIZE as u64).unwrap().len(),
            SAVE_SIZE + 1
        );
        // hunter.fam enorme: lê só o começo (as 14 linhas cabem; o resto é ignorado)
        let mut l = HeroList::default();
        for i in 0..14 {
            l.record(&format!("H{i}"), 10 + i, 0);
        }
        let mut fam = l.to_bytes(0);
        fam.extend(std::iter::repeat_n(0u8, 1 << 20));
        write(&st.heroes_file(), &fam).unwrap();
        assert_eq!(st.heroes(), l);
        // a gravação não deixa temporários na pasta
        write(&p, b"novo").unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), b"novo");
        let names: Vec<_> = std::fs::read_dir(st.saves_dir())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, ["grande.hhg"]);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn names_are_cleaned() {
        assert_eq!(clean_name(" ../Meu jogo:1! "), "Meu jogo1");
        assert_eq!(clean_name(&"x".repeat(50)).len(), SAVE_NAME_MAX);
        assert_eq!(SAVE_NAME_MAX, 31, "o mesmo limite do campo do SHIGH");
        assert_eq!(clean_name("é"), "");
    }
}
