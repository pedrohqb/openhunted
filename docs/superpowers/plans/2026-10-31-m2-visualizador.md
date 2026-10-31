# OpenHunted M2 (visualizador de fase): plano de implementação

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Abrir a `T_J_001.LEV` numa janela SDL3 com cenário, rolagem e objetos animados.

**Architecture:** O corpo das fases é decodificado em `openhunted-formats`; a nova crate
`openhunted-engine` monta a cena e desenha por software num framebuffer indexado 640×480; o
binário `openhunted` é uma camada SDL3 fina. Fatos desconhecidos (índice da célula, tabela
tipo → animação, transparência, ritmo) são decifrados com o radare2 e documentados em
`docs/re/` antes do código que os usa.

**Tech Stack:** Rust 2024, crates da M1, `sdl3` 0.20 (feature `bundled-sdl` →
`sdl3/build-from-source-static`), radare2 + r2pipe.

**Spec:** `docs/superpowers/specs/2026-10-31-m2-visualizador-design.md`

**Modo de execução:** nativo, sem consulta a Pedro (pedido dele). Decisões vão para o ledger
como `Ruling:` e para a mensagem final.

## Global Constraints

- Todas as restrições globais do plano da M1 continuam valendo (licença, SPDX, nada de dados
  originais no git, sem pânico com dados de disco, testes reais via `OPENHUNTED_DATA`).
- `openhunted-engine` não depende de SDL nem de relógio de parede.
- A CI compila o binário com `--features bundled-sdl`.

## Review Focus

1. Fase com tipo de objeto desconhecido: marcador e aviso, sem pânico.
2. Câmera fora dos limites da fase (rolagem até o fim e além): recorte correto, sem pânico.
3. Sprite parcialmente fora do framebuffer (coordenadas negativas ou além de 640×480): recorte.
4. Fase sem objetos ou com célula vazia/sem animação: renderiza o resto.
5. Executável sem SDL3 no sistema: mensagem de erro de link clara na documentação; com
   `bundled-sdl`, funciona.

---

### Task 1: Corpo das fases

**Files:** `crates/openhunted-formats/src/level_body.rs`, `src/level.rs`, `docs/formats/lev.md`,
`docs/re/lev-corpo.md`, testes em `tests/real_data.rs`.

**Interfaces:** `pub struct LevelObject { kind: u32, x: i32, y: i32, name_a: String, name_b: String, fields: …}`,
`pub struct Cell { animation: String, border: String, index: i32, objects: Vec<LevelObject>, props: Vec<…>, decals: Vec<…>, … }`;
`Level { objects: Vec<LevelObject>, cells: Vec<Cell>, body: Vec<u8> /* sobra */ }`.

- [ ] Desmontar por completo `0x00432319`, `0x0042ff4c`, `0x00430278` e sub-rotinas; documentar.
- [ ] Testes: sintético mínimo; truncamento sem pânico; real: as 100 fases com sobra 0; fatos da T_J_001.
- [ ] Implementar; rodar; commit `feat(formats): corpo das fases`.

### Task 2: Cenário, transparência e ritmo

**Files:** `docs/re/cenario.md`; ajustes em `anim.rs` se necessário.

- [ ] Decifrar como o índice da célula escolhe o quadro/rótulo da animação do cenário e a
      posição de cada célula; a cor transparente nos blits de sprite; o período do timer.
- [ ] Teste real: o índice de cada célula da T_J_001 resolve para um quadro existente.
- [ ] Commit `docs(re): cenário, transparência e ritmo`.

### Task 3: Tipo de objeto → animação

**Files:** `docs/re/objetos.md`, `crates/openhunted-engine/src/object_types.rs`.

- [ ] Localizar a criação de objeto por tipo (`0x00433d80` e o despacho por `[ebx+0x1bc]`) e a
      escolha do ANM; tabelar os tipos das 100 fases.
- [ ] Teste: todo tipo da T_J_001 tem animação existente no arquivo de recursos.
- [ ] Commit `feat(engine): tabela de tipos de objeto`.

### Task 4: Engine — assets, intérprete de ANM e cena

**Files:** `crates/openhunted-engine/{Cargo.toml,src/lib.rs,src/assets.rs,src/anim.rs,src/scene.rs}`.

- [ ] Testes sintéticos: avanço de quadro por tick, laço no opcode 1, instâncias com posição.
- [ ] Teste real: cena da T_J_001 com 14 células e 13 objetos com animação.
- [ ] Commit `feat(engine): assets, animação e cena`.

### Task 5: Engine — render por software e `ohtool render`

**Files:** `crates/openhunted-engine/src/{render.rs,camera.rs}`, `crates/ohtool/src/main.rs`.

- [ ] Testes: blit com transparência, recorte em todas as bordas, câmera fora dos limites.
- [ ] `ohtool render T_J_001.LEV --out x.png [--camera X,Y]`; inspeção visual; hash de referência.
- [ ] Commit `feat(engine): render por software`.

### Task 6: Executável SDL3

**Files:** `crates/openhunted/{Cargo.toml,src/main.rs}`, `.github/workflows/ci.yml`, `README.md`.

- [ ] Janela, rolagem, Tab para colisão, Esc; laço de tempo fixo; caixa de mensagem de erro.
- [ ] Compila com `--features bundled-sdl`; teste de fumaça com `SDL_VIDEO_DRIVER=dummy`
      abrindo e fechando depois de N quadros (`--frames N`).
- [ ] Commit `feat: visualizador SDL3`.

### Task 7: Fechamento

- [ ] README e `docs/formats/README.md` atualizados; revisão final por revisor independente.
