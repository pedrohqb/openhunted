# OpenHunted M0+M1 (fundação e formatos): plano de implementação

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Criar o repositório do OpenHunted e a crate `openhunted-formats`, que lê todos os recursos do Hunter Hunted 1.00 (volume, compressão, BMP, PAL, WAV, RES, ANM, BRD, ITF, PFT, fases do `.hls`), mais o `ohtool`, que os lista e extrai.

**Architecture:** Workspace Cargo com `crates/openhunted-formats` (biblioteca pura, sem SDL) e `crates/ohtool` (CLI). Os dados vêm de uma pasta ou de um `.iso` por meio de um trait `DataSource`. Formatos ainda desconhecidos (compressão, ANM, BRD, ITF, PFT, HLS/LEV) são decifrados no Ghidra a partir do `Hunter.exe`. Cada um ganha a sua página em `docs/formats/` e as notas com endereços em `docs/re/`.

**Tech Stack:** Rust (edition 2024), `thiserror` 2, `sha2` 0.11, `png` 0.18, `clap` 4, `assert_cmd` 2, `tempfile` 3, Ghidra (headless) para a engenharia reversa.

**Spec:** `docs/superpowers/specs/2026-10-31-openhunted-design.md`

**Escopo:** este plano cobre só as etapas M0 e M1 da spec. As crates `openhunted-engine` e `openhunted` (SDL3) nascem no plano da M2. As tarefas 10 a 13 dependem do que a tarefa 7 revelar sobre a compressão. Se a engenharia reversa contrariar os fatos fixados aqui, corrija o plano antes de seguir e registre a divergência em `docs/re/`.

## Global Constraints

- Licença GPL-3.0-or-later; cabeçalho `// SPDX-License-Identifier: GPL-3.0-or-later` em todo arquivo `.rs`.
- `edition = "2024"`, `rust-version = "1.85"` no workspace.
- `openhunted-formats` não depende de SDL nem de nenhuma crate de I/O de imagem; erros tipados com `thiserror`; **nenhum `panic!`/`unwrap`/indexação sem checagem sobre dados lidos do disco**.
- Nenhum dado original (recursos, executável, pseudocódigo do Ghidra) entra no repositório. Código reescrito a partir das notas de `docs/re/`. A saída do Ghidra fica em `re-out/`, que vai no `.gitignore`.
- Testes sobre dados reais leem `OPENHUNTED_DATA` (pasta do CD, pasta `Hunter/` ou `.iso`) e são **pulados com aviso** quando a variável não existe; a CI roda sem dados.
- Versão conhecida 1.00 (sha256): `RESOURCE.MAP` `8eb654c4093b3c013705f109af4c205f4854871f220cb7d7a6965ed9557abd7d`; `RESOURCE.001` `0d101d471c4b85a129bfa2f3875b77ab8f4d418c8ad8e3fb57318f31df02651a`; `RESOURCE.002` `55e1aa4ab1be58e619df64a90528d7be5be58eb1772d35ad4bcf1775cda6835e`; `hunter.hls` `02ef9276e0686e900e277004b65a6fb51e54ee3e8ba13f35b04c4ceb47d1c83a`; `Hunter.exe` `663e22cd9938ba6a5f4288c001e77f49558c7a8e76ed0a6d1ccf69472a44a365`.
- Nomes de recurso e caminhos são comparados sem diferenciar maiúsculas de minúsculas.
- Textos dos recursos são decodificados como Latin-1.
- Prosa da documentação em português; identificadores de código em inglês.
- Commits terminam com as linhas de atribuição da sessão.

## Review Focus

1. **Caminho de dados em formatos diferentes.** `--data` pode apontar para a raiz do CD montado, para a pasta `Hunter/`, para o próprio `RESOURCE.MAP` ou para o `.iso`; todos devem abrir os mesmos dados. Teste na Tarefa 4.
2. **CD montado sem Joliet.** Nomes em maiúsculas com sufixo `;1` (`RESOURCE.MAP;1`) e `hunter.hls` em minúsculas no mapa versus maiúsculas no disco; a busca deve achar. Teste na Tarefa 2.
3. **Arquivo truncado ou corrompido** (cópia interrompida, CD riscado): erro tipado com o nome do recurso, nunca pânico. Testes de truncamento em todos os comprimentos nas Tarefas 3, 5 e 8.
4. **Outra edição do jogo** (alemã ou outra tradução, hashes diferentes): abre com `GameVersion::Unknown` e um aviso, sem recusar. Teste na Tarefa 4.
5. **Nomes de recurso perigosos na extração** (barras, `..`, caracteres fora de `[A-Za-z0-9._-]`, pasta de saída já existente): o `ohtool extract` sanitiza e nunca escreve fora de `--out`. Teste na Tarefa 9.

---

## Estrutura de arquivos

```
Cargo.toml                          workspace
rust-toolchain.toml                 channel = "stable"
LICENSE  README.md  .gitignore
.github/workflows/ci.yml
crates/openhunted-formats/
  Cargo.toml
  src/lib.rs                        reexporta módulos
  src/error.rs                      FormatError, Result
  src/source/mod.rs                 trait DataSource, normalize_path
  src/source/dir.rs                 DirSource
  src/source/iso.rs                 IsoSource (ISO 9660 + Joliet)
  src/volume.rs                     ResourceArchive (RESOURCE.MAP + volumes)
  src/game_data.rs                  open_data, GameData, GameVersion
  src/chunk.rs                      blocos TAG: + u32
  src/palette.rs  src/sound.rs  src/text.rs
  src/compress.rs                   decompress (Tarefa 7)
  src/bitmap.rs                     Bitmap, Frame
  src/anim.rs  src/border.rs  src/interface.rs  src/font.rs
  src/hls.rs  src/level.rs
  src/test_support.rs               mkiso(), mkarchive() (feature "test-support")
  tests/support/mod.rs              data_or_skip!()
  tests/real_data.rs                testes sobre OPENHUNTED_DATA
crates/ohtool/
  Cargo.toml  src/main.rs  src/extract.rs
  tests/cli.rs
tools/ghidra/analyze.sh             análise headless reprodutível
tools/ghidra/ExportIndex.java       exporta funções, RTTI e xrefs de strings para re-out/
docs/formats/*.md   docs/re/*.md
```

---

### Task 1: Fundação do repositório

**Files:**
- Create: `Cargo.toml`, `rust-toolchain.toml`, `LICENSE` (texto oficial da GPL-3.0), `README.md`, `.gitignore`, `.github/workflows/ci.yml`
- Create: `crates/openhunted-formats/Cargo.toml`, `src/lib.rs`, `src/error.rs`
- Test: `crates/openhunted-formats/src/error.rs` (módulo `#[cfg(test)]`)

**Interfaces:**
- Produces: `openhunted_formats::{FormatError, Result}`:
  ```rust
  pub enum FormatError {
      Io { path: String, source: std::io::Error },
      NotFound(String),
      Truncated { what: &'static str, offset: usize },
      BadMagic { what: &'static str, expected: &'static [u8], found: Vec<u8> },
      Unsupported { what: &'static str, detail: String },
      InResource { name: String, source: Box<FormatError> },
  }
  pub type Result<T> = std::result::Result<T, FormatError>;
  impl FormatError { pub fn in_resource(self, name: &str) -> FormatError }
  ```

- [ ] **Step 1:** Escrever `#[test] fn in_resource_wraps_and_displays_name()`: `FormatError::Truncated{what:"bitmap", offset:12}.in_resource("SABOUT.BMP").to_string()` contém `"SABOUT.BMP"` e `"bitmap"`.
- [ ] **Step 2:** `cargo test -p openhunted-formats`. Esperado: falha de compilação (tipo inexistente).
- [ ] **Step 3:** Criar o workspace (`members = ["crates/*"]`, `resolver = "3"`, `[workspace.package]` com edition, rust-version, `license = "GPL-3.0-or-later"`, `repository = "https://github.com/pedrohqb/openhunted"`) e implementar `error.rs` com `thiserror`.
- [ ] **Step 4:** README em português com: o que é o projeto, aviso de que **é preciso o CD original** (versão Windows 95 1.00) e de que nenhum dado do jogo é distribuído, status ("M1: formatos em andamento"), como compilar, como rodar os testes com `OPENHUNTED_DATA`. `.gitignore`: `/target`, `/re-out`, `*.iso`. CI no `ubuntu-latest`: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`.
- [ ] **Step 5:** `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`. Esperado: tudo passa.
- [ ] **Step 6:** Commit `chore: fundação do workspace, licença, README e CI`.

### Task 2: Fontes de dados (pasta e ISO)

**Files:**
- Create: `src/source/mod.rs`, `src/source/dir.rs`, `src/source/iso.rs`, `src/test_support.rs`, `tests/support/mod.rs`
- Test: `tests/source.rs`

**Interfaces:**
- Consumes: `FormatError`, `Result` (Tarefa 1).
- Produces:
  ```rust
  pub trait DataSource: Send + Sync {
      /// `path` relativo à raiz, separado por '/', sem diferenciar maiúsculas.
      fn read(&self, path: &str) -> Result<Vec<u8>>;
      fn exists(&self, path: &str) -> bool;
  }
  pub fn normalize_path(path: &str) -> String; // maiúsculas, '\\'→'/', remove ';<n>' e '/' inicial
  pub struct DirSource { /* … */ } impl DirSource { pub fn new(root: impl Into<PathBuf>) -> Self }
  pub struct IsoSource { /* … */ } impl IsoSource { pub fn open(path: &Path) -> Result<Self> }
  ```
  `IsoSource` lê a imagem inteira para memória, prefere a árvore Joliet (descritor suplementar com sequência de escape `%/@`, `%/C` ou `%/E`) e cai para a do descritor primário.
- Produces (apoio de teste): `openhunted_formats::test_support::mkiso(files: &[(&str, &[u8])], joliet: bool) -> Vec<u8>`, que monta uma imagem mínima válida (setores de 2048 bytes, PVD no setor 16, opcionalmente o SVD Joliet, terminador, um nível de subpastas). No modo sem Joliet, grava os nomes em maiúsculas com `;1`. O módulo `test_support` só é compilado com a feature `test-support` (os testes de integração da própria crate a ativam por `[dev-dependencies] openhunted-formats = { path = ".", features = ["test-support"] }`). Em `tests/support/mod.rs` fica a macro `data_or_skip!()`, que devolve o `PathBuf` de `OPENHUNTED_DATA` ou faz `eprintln!("pulado: OPENHUNTED_DATA não definido"); return;`.

- [ ] **Step 1:** Testes:
  - `dir_source_is_case_insensitive`: pasta temporária com `Hunter/hunter.hls`; `read("HUNTER/HUNTER.HLS")` devolve o conteúdo.
  - `iso_plain_strips_version_suffix`: `mkiso(&[("Hunter/RESOURCE.MAP", b"abc")], false)`; `IsoSource::read("hunter/resource.map") == b"abc"`.
  - `iso_joliet_preserves_lowercase_names`: com `joliet=true` e o arquivo `Hunter/hunter.hls`, `exists("HUNTER/HUNTER.HLS")` é `true`.
  - `missing_file_is_not_found`: `read("nada.bin")` dá `Err(FormatError::NotFound(_))`.
  - `truncated_iso_errors`: a imagem cortada em 16×2048+10 bytes dá `Err`, sem pânico.
  - `real_iso_has_resource_map` (dados reais; só roda se `OPENHUNTED_DATA` terminar em `.iso`): `read("Hunter/RESOURCE.MAP").len() == 18532`.
- [ ] **Step 2:** `cargo test -p openhunted-formats --test source`. Esperado: falha de compilação.
- [ ] **Step 3:** Implementar os três arquivos de `source/`. Em `DirSource`, resolver cada componente do caminho listando a pasta e comparando sem diferenciar maiúsculas.
- [ ] **Step 4:** `cargo test -p openhunted-formats --test source` e depois `OPENHUNTED_DATA=/caminho/Hunter_Hunted.iso cargo test -p openhunted-formats --test source`. Esperado: tudo passa.
- [ ] **Step 5:** Commit `feat(formats): fontes de dados em pasta e ISO 9660/Joliet`.

### Task 3: Volume de recursos

**Files:**
- Create: `src/volume.rs`, `docs/formats/volume.md`
- Modify: `src/test_support.rs` (adiciona `mkarchive`)
- Test: `tests/volume.rs`, `tests/real_data.rs`

**Interfaces:**
- Consumes: `FormatError`.
- Produces:
  ```rust
  pub struct Entry { pub name: String, pub volume: String, pub offset: u32, pub size: u32 }
  pub struct ResourceArchive { /* buffers dos volumes + índice por nome normalizado */ }
  impl ResourceArchive {
      pub fn from_map(map: &[u8], load_volume: impl FnMut(&str) -> Result<Vec<u8>>) -> Result<Self>;
      pub fn len(&self) -> usize;
      pub fn entries(&self) -> &[Entry];          // na ordem do mapa
      pub fn get(&self, name: &str) -> Result<&[u8]>; // sem diferenciar maiúsculas
  }
  ```
- Formato (fixado pela inspeção): **mapa**: 4 bytes de cabeçalho (`01 02 03 05` nesta versão; não validar), `u16` número de volumes; para cada volume, nome com 13 bytes (ASCIIZ), `u16` contagem e as entradas `(u32 hash, u32 offset)`. **Volume**: em cada `offset`, nome com 13 bytes (ASCIIZ), `u32` tamanho e os dados. O hash não é usado nesta fase. Anotar em `docs/formats/volume.md`, como questão aberta, que o algoritmo do hash fica para a Tarefa 6.
- Apoio de teste: `test_support::mkarchive(volumes: &[(&str, &[(&str, &[u8])])]) -> (Vec<u8> /*map*/, Vec<(String, Vec<u8>)> /*volumes*/)`.

- [ ] **Step 1:** Testes:
  - `reads_entries_across_volumes`: dois volumes sintéticos, com `get("b.wav")` devolvendo os bytes de `B.WAV` do segundo volume.
  - `duplicate_name_keeps_first`: o índice aponta para a primeira ocorrência.
  - `truncated_map_never_panics`: para cada `n in 0..map.len()`, `from_map(&map[..n], …)` dá `Err` ou `Ok`, sem pânico.
  - `entry_beyond_volume_is_truncated_error`: offset além do fim do volume dá `Err(Truncated{..})`.
  - `real_archive_inventory` (dados reais): 2312 entradas; contagem por extensão `BMP 1654, WAV 280, BRD 225, ANM 87, PAL 26, ITF 16, RES 13, PFT 5, TBL 3, SKE 2, VCR 1`; `get("SX_3000.WAV")` começa com `RIFF`; `get("avacado.pal")` começa com `RIFF`.
- [ ] **Step 2:** Rodar `cargo test -p openhunted-formats --test volume`. Esperado: falha.
- [ ] **Step 3:** Implementar `volume.rs`, com validação de limites em toda leitura (um helper `read_u16_le` / `read_u32_le` sobre a fatia, devolvendo `Truncated`). Escrever `docs/formats/volume.md`.
- [ ] **Step 4:** Rodar os testes, com e sem `OPENHUNTED_DATA`. Esperado: passam.
- [ ] **Step 5:** Commit `feat(formats): leitura do volume de recursos (RESOURCE.MAP)`.

### Task 4: Abrir os dados do jogo e identificar a versão

**Files:**
- Create: `src/game_data.rs`
- Test: `tests/game_data.rs`, `tests/real_data.rs`

**Interfaces:**
- Consumes: `DataSource`, `DirSource`, `IsoSource` (Tarefa 2); `ResourceArchive` (Tarefa 3).
- Produces:
  ```rust
  pub enum GameVersion { Win95V100, Unknown }
  pub struct GameData { /* source + prefixo da pasta do jogo ("HUNTER/" ou "") */ }
  pub fn open_data(path: &Path) -> Result<GameData>;
  impl GameData {
      pub fn version(&self) -> GameVersion;   // sha256 do RESOURCE.MAP comparado com a Global Constraint
      pub fn read(&self, file: &str) -> Result<Vec<u8>>; // relativo à pasta do jogo
      pub fn resources(&self) -> Result<ResourceArchive>;
  }
  ```
  Resolução: se `path` for um arquivo terminado em `.iso`, usa `IsoSource`; se for o `RESOURCE.MAP`, usa a pasta onde ele está; se for uma pasta, procura `RESOURCE.MAP` e depois `HUNTER/RESOURCE.MAP`. Nenhum encontrado: `NotFound("RESOURCE.MAP")`, com mensagem que cite os caminhos tentados.

- [ ] **Step 1:** Testes (sintéticos, com `mkarchive` gravado em pasta temporária):
  - `opens_cd_root_hunter_dir_map_file_and_iso`: as quatro formas de caminho dão o mesmo `resources()?.len()`.
  - `unknown_version_still_opens`: um mapa sintético dá `version() == GameVersion::Unknown` e `resources()` funciona.
  - `missing_map_lists_tried_paths`: a pasta vazia dá um erro cuja mensagem contém `RESOURCE.MAP`.
  - `real_data_is_v100` (dados reais): `version() == GameVersion::Win95V100`.
- [ ] **Step 2:** Rodar `cargo test -p openhunted-formats --test game_data`. Esperado: falha.
- [ ] **Step 3:** Implementar `game_data.rs` (dependência `sha2`).
- [ ] **Step 4:** Rodar os testes. Esperado: passam.
- [ ] **Step 5:** Commit `feat(formats): abertura dos dados e identificação da versão`.

### Task 5: Blocos, paleta, som e textos

**Files:**
- Create: `src/chunk.rs`, `src/palette.rs`, `src/sound.rs`, `src/text.rs`, `docs/formats/blocos.md`, `docs/formats/pal-wav-res.md`
- Test: módulos `#[cfg(test)]` de cada arquivo; `tests/real_data.rs`

**Interfaces:**
- Produces:
  ```rust
  pub struct Chunk<'a> { pub tag: [u8; 4], pub data: &'a [u8] }       // tag inclui ':' (b"BMP:")
  pub fn chunks(data: &[u8]) -> impl Iterator<Item = Result<Chunk<'_>>>;
  pub fn find_chunk<'a>(data: &'a [u8], tag: &[u8; 4]) -> Result<Option<Chunk<'a>>>;
  pub struct Palette(pub [[u8; 3]; 256]);
  impl Palette { pub fn parse(data: &[u8]) -> Result<Self> }
  pub struct Sound { pub sample_rate: u32, pub channels: u16, pub bits_per_sample: u16, pub data: Vec<u8> }
  impl Sound { pub fn parse(data: &[u8]) -> Result<Self> }
  pub fn parse_text_list(data: &[u8]) -> Vec<String>;
  ```
- Bloco: tag de 4 bytes terminada em `:`, `u32` tamanho e os dados. Se o bit 31 do tamanho vier ligado, mascarar e tratar como contêiner. Nenhum foi observado; `debug_assert` não serve aqui: devolver `Unsupported`.
- PAL: RIFF `PAL ` com o bloco `data`, `u16` versão `0x0300`, `u16` contagem (256) e entradas `R G B flags`.
- WAV: RIFF `WAVE`, com `fmt ` PCM (formato 1) e `data`; outros formatos dão `Unsupported`.
- RES: linhas separadas por CRLF, Latin-1, preservando linhas vazias e descartando só a última se vier vazia.

- [ ] **Step 1:** Testes:
  - `chunks_iterates_and_rejects_overrun`: dois blocos sintéticos são lidos; um tamanho maior que o restante dá `Truncated`; truncar em todo comprimento nunca entra em pânico.
  - `palette_parses_riff`: uma paleta sintética com a cor 0 = `(255, 0, 189)` sai igual.
  - `wav_rejects_non_pcm`.
  - `text_keeps_blank_lines`: `b"a\r\n\r\nb\r\n"` vira `["a", "", "b"]`.
  - `real_standard_formats` (dados reais): `AVACADO.PAL` cor 0 = `[0xff, 0x00, 0xbd]`; `SX_3000.WAV` com 11025 Hz, 1 canal, 8 bits; `parse_text_list(SYSTEM.RES)[0] == "OKAY"`; os 26 `.PAL` passam no `parse`; os 280 `.WAV` passam no `parse`; `SABOUT.ANM` lista as tags `ANM:`, `TAG:`, `LAB:`, `PAL:`, nessa ordem.
- [ ] **Step 2:** Rodar e ver falhar.
- [ ] **Step 3:** Implementar os quatro módulos e as duas páginas de documentação.
- [ ] **Step 4:** Rodar e ver passar.
- [ ] **Step 5:** Commit `feat(formats): blocos, paleta, som e listas de texto`.

### Task 6: Ambiente de engenharia reversa e mapa do executável

**Files:**
- Create: `tools/ghidra/analyze.sh`, `tools/ghidra/ExportIndex.java`, `docs/re/README.md`, `docs/re/executavel.md`

**Interfaces:**
- Produces (para as Tarefas 7 e 10 a 13): `docs/re/executavel.md` com:
  - base da imagem e seções;
  - a lista de classes RTTI (`.?AV…@@`);
  - os endereços das funções de abertura do `RESOURCE.MAP`, de busca por nome (com o algoritmo do hash, se aparecer), de leitura de bloco, de decodificação de quadro BMP e dos métodos de `LZHuffman` e `RLERStream`, além das funções que leem `.ANM`, `.BRD`, `.ITF`, `.PFT`, `HUNTER.HLS` e `.LEV` (localizadas pelos xrefs das strings já conhecidas: `%s\hunter.hls`, `C%d%c%02d%c0.BRD`, `garam12.pft`, `SLOADING.BMP`).
- `analyze.sh <Hunter.exe>`: baixa o Ghidra da release oficial no GitHub se `GHIDRA_HOME` não estiver definido, roda `analyzeHeadless` num projeto em `re-out/ghidra/` e executa o `ExportIndex.java`, que grava `re-out/functions.tsv` (endereço, nome, tamanho), `re-out/rtti.tsv` e `re-out/string-xrefs.tsv`.
- `docs/re/README.md` descreve o método limpo: o pseudocódigo fica em `re-out/` (fora do git), e os documentos de `docs/re/` descrevem o comportamento com palavras e endereços, sem colar código decompilado.

- [ ] **Step 1:** Escrever `analyze.sh` e `ExportIndex.java` (JDK 21; instalar no ambiente se faltar).
- [ ] **Step 2:** Rodar `tools/ghidra/analyze.sh /caminho/Hunter.exe`. Esperado: os três `.tsv` em `re-out/`, e `rtti.tsv` contém `LZHuffman` e `RLERStream`.
- [ ] **Step 3:** Escrever `docs/re/executavel.md` a partir dos `.tsv` e da navegação no decompilador. Cada função listada leva endereço, papel e as funções que a chamam.
- [ ] **Step 4:** Verificar: `git status` não mostra nada de `re-out/`; `grep -c '0x' docs/re/executavel.md` ≥ 12.
- [ ] **Step 5:** Commit `docs(re): ambiente Ghidra headless e mapa inicial do executável`.

### Task 7: Descompressão

**Files:**
- Create: `src/compress.rs`, `docs/formats/compressao.md`, `docs/re/compressao.md`
- Test: módulo `#[cfg(test)]` em `compress.rs`; `tests/real_data.rs`

**Interfaces:**
- Consumes: os endereços de `LZHuffman` e `RLERStream` em `docs/re/executavel.md` (Tarefa 6).
- Produces: `pub fn decompress(method: u8, src: &[u8], expected_len: usize) -> Result<Vec<u8>>`. Métodos desconhecidos dão `Unsupported`. O resultado com tamanho diferente de `expected_len` dá `Truncated`.
- Fatos fixados: o byte de tipo dos quadros BMP é `0x00` (cru) em 1 bitmap (`ITF.BMP`) e `0x83` nos outros 1653. No `SABOUT.BMP`, o quadro é 320×256, com tipo `0x83` e `u32` = `0xF813`, e o fluxo começa por `00 00 40 01 00 01 b3 87 02 00`. A hipótese a confirmar é de que `0x80` = comprimido e `3` = cadeia LZHuffman + RLE, e de que o fluxo traga cabeçalho com dimensões e tamanho expandido.
- Também fixar: se a mesma rotina serve aos `.hls` ou a outros tipos (registrar nos dois documentos).

- [ ] **Step 1:** Desmontar no Ghidra os métodos de leitura das duas classes e descrever em `docs/re/compressao.md` o algoritmo (árvore/tabela de Huffman, janela e comprimentos do LZ, codificação das corridas RLE e como os estágios se encadeiam).
- [ ] **Step 2:** Escrever os testes unitários com fluxos montados à mão a partir da descrição: `rle_literal_and_run`, `lz_backreference_overlapping`, `huffman_single_symbol_tree` (um por caminho de código descoberto); `unknown_method_is_unsupported`; `truncated_stream_never_panics` (todo comprimento). Teste com dados reais: `sabout_decompresses_to_320x256`, com o resultado de `320 * 256` bytes.
- [ ] **Step 3:** Rodar e ver falhar.
- [ ] **Step 4:** Implementar `compress.rs` (reescrito a partir do documento, não do pseudocódigo) e `docs/formats/compressao.md`.
- [ ] **Step 5:** Rodar e ver passar.
- [ ] **Step 6:** Commit `feat(formats): descompressão LZHuffman/RLE`.

### Task 8: Bitmaps

**Files:**
- Create: `src/bitmap.rs`, `docs/formats/bmp.md`
- Test: módulo `#[cfg(test)]`; `tests/real_data.rs`

**Interfaces:**
- Consumes: `find_chunk` (Tarefa 5), `decompress` (Tarefa 7).
- Produces:
  ```rust
  pub struct Frame { pub width: u16, pub height: u16, pub pixels: Vec<u8> } // índices de paleta, linha a linha, de cima para baixo
  pub struct Bitmap { pub frames: Vec<Frame> }
  impl Bitmap { pub fn parse(data: &[u8]) -> Result<Self> } // `data` = recurso inteiro, começando em "BMP:"
  ```
- Layout fixado, com offsets a partir do início do conteúdo do bloco `BMP:`: `u16` versão `1000`, `u16` número de quadros `n`, `u32` tamanho do cabeçalho (= offset do quadro 0 contado desde o início do recurso, isto é, desde a tag `BMP:`) e `n-1` `u32` com os offsets dos quadros 1 a n-1 (confirmar no Ghidra a base desses offsets). Cada quadro: `u16` largura, `u16` altura, `u8` tipo, `u32` tamanho e os dados. A orientação das linhas (de cima para baixo ou de baixo para cima) é confirmada na Tarefa 6 e normalizada aqui para cima→baixo.

- [ ] **Step 1:** Testes:
  - `raw_single_frame`: um bitmap sintético cru 2×2.
  - `truncated_bitmap_never_panics`.
  - `real_bitmaps` (dados reais): `ITF.BMP` com 1 quadro de 124×25 e pixel 0 = `0x89`; `SABOTBUT.BMP` com 6 quadros, o primeiro de 36×32; `SEGOCBAK.BMP` com 1 quadro de 640×480; `SEGOCBUT.BMP` com 63 quadros; **os 1654 `.BMP` passam no `parse`** e cada quadro tem `pixels.len() == width * height`.
- [ ] **Step 2:** Rodar e ver falhar.
- [ ] **Step 3:** Implementar `bitmap.rs` e `docs/formats/bmp.md`.
- [ ] **Step 4:** Rodar e ver passar.
- [ ] **Step 5:** Commit `feat(formats): bitmaps com múltiplos quadros`.

### Task 9: ohtool (list, info, extract)

**Files:**
- Create: `crates/ohtool/Cargo.toml`, `src/main.rs`, `src/extract.rs`
- Test: `crates/ohtool/tests/cli.rs`

**Interfaces:**
- Consumes: `open_data`, `GameData`, `ResourceArchive`, `Bitmap`, `Palette`, `Sound`, `parse_text_list`.
- Produces (CLI):
  - `ohtool --data <PATH> list [--type <EXT>]`: imprime `NOME<TAB>VOLUME<TAB>TAMANHO`, uma linha por recurso, na ordem do mapa.
  - `ohtool --data <PATH> info <NOME>`: imprime o tipo, as tags de bloco e, para BMP, o número de quadros e as dimensões.
  - `ohtool --data <PATH> extract --out <DIR> [--type <EXT>] [--palette <NOME.PAL>]`: BMP → `<nome>_<quadro:03>.png`, PNG indexado com a paleta escolhida (padrão `WORLD.PAL`); WAV/PAL/RES/demais → cópia dos bytes do recurso.
  - Na abertura, se a versão for `Unknown`, avisa em stderr e continua.
- `pub fn safe_file_name(resource: &str) -> String` (em `extract.rs`): troca todo caractere fora de `[A-Za-z0-9._-]` por `_`, transforma um nome vazio ou só com pontos em `_`, e nunca devolve separador de caminho.

- [ ] **Step 1:** Testes (com `assert_cmd` sobre um arquivo sintético gerado por `openhunted_formats::test_support::mkarchive` e gravado em pasta temporária; o `ohtool` ativa a feature `test-support` só em `[dev-dependencies]`):
  - `list_prints_all_entries`.
  - `extract_writes_png_with_dimensions`: um BMP cru sintético de 2×2 vira um PNG lido de volta com 2×2.
  - `safe_file_name_blocks_traversal`: `safe_file_name("../X\\Y.BMP") == "___X_Y.BMP"`, e o nome não contém `/`.
  - `extract_into_existing_dir_overwrites_only_own_files`.
  - `missing_data_path_exits_nonzero_with_message`.
  - `real_extract_all_bitmaps` (dados reais, `#[ignore]` por ser lento): `extract --type BMP` gera exatamente a soma dos quadros dos 1654 BMP.
- [ ] **Step 2:** Rodar e ver falhar.
- [ ] **Step 3:** Implementar o `ohtool` (`clap` com derive, `png`).
- [ ] **Step 4:** Rodar e ver passar. Inspeção manual: extrair `SEGOCBAK.BMP` com `--palette SMAIN.PAL` e um bitmap de cenário com `WORLD.PAL`, e abrir os PNG. Esperado: imagens reconhecíveis, com cores plausíveis.
- [ ] **Step 5:** Commit `feat(ohtool): listar, inspecionar e extrair recursos`.

### Task 10: Animações (ANM)

**Files:**
- Create: `src/anim.rs`, `docs/formats/anm.md`, `docs/re/anm.md`
- Modify: `crates/ohtool/src/main.rs` (o `info` e o `extract` passam a exportar o ANM como `.json`; isso adiciona `serde` e `serde_json` ao `ohtool`, e `serde` como *feature* opcional `serde` em `openhunted-formats`)
- Test: módulo `#[cfg(test)]`; `tests/real_data.rs`

**Interfaces:**
- Consumes: `chunks`, `find_chunk`; a função de leitura de ANM em `docs/re/executavel.md`.
- Produces: `pub struct Animation { … }` com campos definidos pela engenharia reversa. No mínimo: o bitmap usado (nome do `.BMP` no bloco `TAG:`), a paleta referenciada (bloco `PAL:`, guardando só o nome do arquivo, sem o caminho `J:\hunter\…`), os rótulos (bloco `LAB:`) e as sequências de quadros com tempos e deslocamentos. `impl Animation { pub fn parse(data: &[u8]) -> Result<Self> }`.
- Fatos fixados: `SABOUT.ANM` tem os blocos `ANM:`(216) `TAG:`(30) `LAB:`(25) `PAL:`(25); o `ANM:` começa por `54 42` (`"TB"`) e `ec 03` (1004).

- [ ] **Step 1:** Desmontar o leitor de ANM e escrever `docs/re/anm.md`.
- [ ] **Step 2:** Testes: `anm_parses_synthetic` (montado a partir do documento); `truncated_anm_never_panics`; com dados reais, `sabout_anm_references_sabout_bmp` (o `TAG:` aponta para um nome que começa com `sabout.`, sem diferenciar maiúsculas), além de **os 87 ANM passam no `parse`** e de todo bitmap referenciado existir no arquivo de recursos.
- [ ] **Step 3:** Rodar e ver falhar; implementar; rodar e ver passar.
- [ ] **Step 4:** Commit `feat(formats): animações ANM`.

### Task 11: Cenários (BRD) e interfaces (ITF)

**Files:**
- Create: `src/border.rs`, `src/interface.rs`, `docs/formats/brd.md`, `docs/formats/itf.md`, `docs/re/brd-itf.md`
- Modify: `crates/ohtool/src/main.rs` (exporta `.json`)
- Test: módulos `#[cfg(test)]`; `tests/real_data.rs`

**Interfaces:**
- Produces: `pub struct Border { pub name: String, … }` (`HED:` traz o nome legível, ex.: `"Climber Bord…"` em `C1A38A0.BRD`; `BDR:` traz a geometria, com campos pela engenharia reversa) e `impl Border { pub fn parse(data: &[u8]) -> Result<Self> }`. Também `pub struct Interface { … }` (`ITF:` começa por `"TBO"` e um nome de arquivo, como `sabout.A…`; `TAG:` traz os rótulos dos botões, como `AB_BACKG…`) e `impl Interface { pub fn parse(data: &[u8]) -> Result<Self> }`.
- Fatos fixados: `C1A38A0.BRD` = `HED:`(24) + `BDR:`(30); `SABOUT.ITF` = `ITF:`(89) + `TAG:`(37).

- [ ] **Step 1:** Desmontar os dois leitores; escrever `docs/re/brd-itf.md`.
- [ ] **Step 2:** Testes: sintéticos de cada formato; truncamento sem pânico; com dados reais, os 225 BRD e os 16 ITF passam no `parse`, `Border::parse(C1A38A0.BRD)?.name` começa com `"Climber"`, e todo recurso referenciado por um ITF existe.
- [ ] **Step 3:** Rodar e ver falhar; implementar; rodar e ver passar.
- [ ] **Step 4:** Commit `feat(formats): cenários BRD e interfaces ITF`.

### Task 12: Fontes (PFT)

**Files:**
- Create: `src/font.rs`, `docs/formats/pft.md`
- Modify: `crates/ohtool/src/main.rs` (extrai uma folha de glifos em PNG)
- Test: módulo `#[cfg(test)]`; `tests/real_data.rs`

**Interfaces:**
- Produces: `pub struct Font { pub height: u16, pub glyphs: Vec<Option<Glyph>> }` e `pub struct Glyph { pub width: u16, pub pixels: Vec<u8> }`, indexados por byte Latin-1; `impl Font { pub fn parse(data: &[u8]) -> Result<Self> }`.
- Fatos fixados: magic `PFNT`, depois `u32` tamanho e um sub-bloco que começa por `head`. `CREDIT2.PFT` tem 305388 bytes. O executável carrega `garam12.pft`.

- [ ] **Step 1:** Desmontar o leitor de PFT (ou deduzir pelo layout, se for trivial) e documentar.
- [ ] **Step 2:** Testes: sintético; truncamento sem pânico; com dados reais, as 5 PFT passam no `parse` e a glifo `b'A'` existe e tem largura maior que 0 em todas.
- [ ] **Step 3:** Rodar e ver falhar; implementar; rodar e ver passar. Inspeção: a folha de glifos de `GARAM12.PFT` é legível.
- [ ] **Step 4:** Commit `feat(formats): fontes PFT`.

### Task 13: Arquivo de fases (`hunter.hls`) e fases (LEV)

**Files:**
- Create: `src/hls.rs`, `src/level.rs`, `docs/formats/hls.md`, `docs/formats/lev.md`, `docs/re/hls-lev.md`
- Modify: `src/game_data.rs` (adiciona `GameData::levels(&self) -> Result<LevelArchive>`), `crates/ohtool/src/main.rs` (subcomando `levels` e exportação `.json`)
- Test: módulos `#[cfg(test)]`; `tests/real_data.rs`

**Interfaces:**
- Consumes: `decompress` (Tarefa 7), `parse_text_list` (Tarefa 5).
- Produces: `pub struct LevelArchive { … }` com `pub fn from_bytes(data: Vec<u8>) -> Result<Self>`, `pub fn names(&self) -> Vec<String>` e `pub fn get(&self, name: &str) -> Result<Vec<u8>>` (já descomprimido). Também `pub struct Level { … }` (cenário: BRDs usados e posições; objetos e inimigos com tipo e posição; ponto inicial do jogador; objetivo da missão; campos pela engenharia reversa), com `impl Level { pub fn parse(data: &[u8]) -> Result<Self> }`.
- Fatos fixados: o `.hls` tem 12395047 bytes e começa por `00 01 06 07`, `u16` = 1 e o nome `RESOURCE.001` com 13 bytes. Em seguida vêm 201 entradas de 8 bytes (`u32`, `u32` offset dentro do próprio `.hls`), e os dados das entradas começam no byte 1629. Os nomes lidos nos offsets saem ilegíveis: hipótese de cabeçalho comprimido ou cifrado, a confirmar. `LEVELS.RES` lista nomes como `C_2_001.LEV`, `H_J_001.LEV`, `D_M_003.LEV` (com alguns em minúsculas, como `D_M_021.lev`); a hipótese de prefixos é C = cooperativo, D = disputa, H = um jogador, J = Jake e M = Minotauro, a confirmar.

- [ ] **Step 1:** Desmontar a abertura do `hunter.hls` e o leitor de LEV; escrever `docs/re/hls-lev.md`. Responder ali qual é a **primeira missão do Jake** no modo para um jogador (a fase-alvo da M2).
- [ ] **Step 2:** Testes: sintéticos de `LevelArchive` e de `Level`; truncamento sem pânico; com dados reais, o `.hls` abre com 201 entradas, todo nome de `LEVELS.RES` está em `names()` (sem diferenciar maiúsculas), todas as fases passam no `Level::parse`, todo BRD referenciado por uma fase existe no arquivo de recursos, e a fase-alvo tem ponto inicial do jogador.
- [ ] **Step 3:** Rodar e ver falhar; implementar; rodar e ver passar.
- [ ] **Step 4:** Commit `feat(formats): arquivo de fases HLS e fases LEV`.

### Task 14: Fechamento da M1

**Files:**
- Create: `docs/formats/README.md` (índice dos formatos, com estado de cada um)
- Modify: `README.md` (status "M1 concluída", exemplos de uso do `ohtool`), `tests/real_data.rs`

- [ ] **Step 1:** Teste `every_resource_parses_by_type` (dados reais): percorre as 2312 entradas e chama o `parse` adequado à extensão (BMP, PAL, WAV, RES, ANM, BRD, ITF, PFT); `TBL`, `SKE` e `VCR` ficam como "não decodificados" e são listados no `docs/formats/README.md` como pendentes para as etapas seguintes.
- [ ] **Step 2:** Rodar `OPENHUNTED_DATA=… cargo test --workspace -- --include-ignored`. Esperado: tudo passa. Rodar também o trio fmt/clippy/test sem a variável. Esperado: passa, com os testes reais pulados.
- [ ] **Step 3:** Atualizar o README e escrever o índice.
- [ ] **Step 4:** Commit `docs: fechamento da M1 (formatos)`.
