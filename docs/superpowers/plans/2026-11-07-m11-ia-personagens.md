# M11 — personagens da IA: plano de implementação

**Goal:** Minotauros e Jakes sem jogador agem pela IA de `0x42ec1c`, ferem e são feridos.

**Spec:** `docs/superpowers/specs/2026-11-07-m11-ia-personagens-design.md`

## Global Constraints

- GPL-3.0-or-later/SPDX; português; nada de dados do jogo; constantes com endereço;
  `/home/claude/check.sh` verde antes de cada commit (os 454 testes continuam verdes).
- Em paralelo: M10 (`entity.rs`, `scene.rs` nos caminhos de criação, `behaviour/enemy*.rs`,
  `pickup.rs`), correção da M9 (`crates/openhunted`) e talvez da M8 (`jake/garathe.rs`):
  em arquivos alheios, só pontos de entrada; o grosso fica em `ai.rs`, `ai/path.rs`,
  `scene/ai.rs`.

## Review Focus

1. O caminho do jogador humano não muda (fora a pose 0xf, que é do original).
2. O dono nunca é vítima dos próprios tiros; o humano é vítima dos tiros da IA.
3. Cadáver: nem vítima nem sólido; "destruir" cumprido; nada de pânico com índices.
4. Busca de caminho limitada (Manhattan ≤ 10), sem custo exponencial.
5. Fuzz: a IA nunca dentro das células da camada dela nem fora da fase.

---

### Task 1: pose 0xf no corpo comum

`State::Taunt` (Jake 18, Garathe 76): entrada exatamente 0xf na decisão em pé; outra entrada
roda a decisão; fim → parado; tiro permitido. Testes Jake e Garathe. Commit.

### Task 2: cérebro puro (`src/ai.rs`)

`Class` pelo estado; `Me`/`Target`/`Senses`; ritmo 2; modos 1/2/3; ataque; mira; aproximação;
sonda de buraco; perigos. Testes sintéticos com mundo e grade de mentira. Commit.

### Task 3: grade e caminho (`src/ai/path.rs`)

`CellGrid` (bits `fields[0]`, zonas PR por célula, portas fechadas), BFS por direção, passo
`0x42d721` no cérebro (centrar, lados, camadas, recomeço). Testes com cada bit. Commit.

### Task 4: personagem da IA na cena (`scene/ai.rs`)

Extrair a montagem do jogador; `Behaviour::AiPlayer`; tique antes das entidades (mundo da
camada com objetos sólidos), sprite, sincronia de caixa/vida, cadáver; tiros com dono
(`ai_shots`), vítima humana em `bullet_hits`/`weapon_hits`, linhas do `DAMAGE.TBL` por
personagem; soco, investida e especial; `enemy::hurt` → corpo; `apply_hazards`. Testes de
cena sintéticos (aproxima e fere; pistola mata; dono imune). Commit.

### Task 5: dados reais e fuzz; README

`tests/real_ai.rs`: `H_J_006` (aproxima em 200 tiques, nunca no cenário); fuzz em todas as
fases com IA. README (linha da M11). Merge da main, check, commit; relatório.
