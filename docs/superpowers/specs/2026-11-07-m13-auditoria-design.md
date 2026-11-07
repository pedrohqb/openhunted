# OpenHunted M13 — auditoria de vencibilidade da campanha: design

Data: 2026-11-07. Base: `docs/re/logica-fase.md` (veredito), `itens-campanha.md` (chaves,
portas, bandeiras), `minotauro-ia.md` (grade de células), `menu.md` (lista solo).

## Objetivo

Achar toda missão solo (as 65 de `p=1`/`p=2` do `ENGLEVEL.TXT`) que não se vence no porte e
por quê, corrigir as causas e documentar o que sobrar. Não é jogabilidade: nada disto roda na
partida.

## Arquitetura

- `openhunted-engine::audit` (novo, atrás da feature `audit`, que o `ohtool` liga; o teste
  `tests/real_audit.rs` exige a feature): só usa a API pública da `Scene` (corpo e camada do
  jogador, `set_jake_layer`, `entities`, `prop_zones`, `cell_doors`, `ai_grid`) e os
  caminhos normais de dano (`behaviour::enemy::hurt`, `behaviour::interact::hurt`). **Nenhum
  gancho novo na `Scene`.**
- `audit::inspect(&Scene) -> Report`: personagem; limite de tempo (campo 1004); cada
  objetivo (`+0x1ac` & 2 "acionar" / 4 "destruir", fora a saída e o jogador) com tipo, nome
  do comportamento, se age (`Static` = não implementado), posição, camada e de que modo se
  cumpre (`coletar` itens e peça; `acionar` interruptor e zona; `destruir`; `explodir`
  barril); portas de célula com as chaves pedidas, os gatilhos e se a chave está na fase;
  objetos cujos gatilhos apontam para ids inexistentes.
- `audit::god_mode(&mut Scene) -> Verdict`: jogador invulnerável (vida, veneno e cronômetro
  repostos a cada tique) e teletransportado. Para cada objetivo: itens → põe o jogador sobre
  o item; zona → dentro dela até ligar; interruptor → diante dele, sobre a zona PR 0x40, e
  CIMA; destruir → dano letal pelo caminho normal (bala do jogador, natureza comum) até a
  vida ≤ 0; barril → idem até explodir. As zonas ficam por último (podem desligar). Depois,
  diante da saída sobre a zona PR 0x40, CIMA, e espera o veredito (`Outcome::Victory`).
  Resultado `Win { tiques }` ou `Fail(motivo)` (objetivo que não cumpriu, saída sem zona,
  choque de "objetivos incompletos", derrota, tempo).
- `audit::reach(&Scene) -> Vec<aviso>`: busca em largura na grade da IA (`CellGrid::allowed`
  — cair, andar, subir só por corda/escada, portas de camada), com as portas de célula
  fechadas que têm a chave na fase (ou nenhuma chave) tratadas como abertas. Avisa
  objetivos e saída fora do alcance. **Heurística**: pulos e quedas laterais não estão na
  grade; aviso não é falha.
- `ohtool audit [--level X] [--god] [--reach]`: uma seção por missão (a ordem do
  `ENGLEVEL.TXT`), e no fim a tabela `missão → WIN/FAIL(motivo)` e as contagens.

## Testes

- Unidade (`audit`): cenas sintéticas para cada modo de cumprir e para as falhas.
- `real_audit.rs`: `god_mode` em todas as missões solo (`#[ignore]`, longo) com a lista de
  falhas conhecidas fixada; uma missão por causa corrigida em teste rápido.
- Vitórias roteirizadas novas (só entrada do jogador) em `real_missions.rs`/`real_garathe.rs`
  onde couber.

## Correções

Cada causa achada vira uma correção com teste sobre dados reais (TDD); o que for grande
demais fica em `docs/campanha.md` (missão → estado → causa) e no "Em aberto" do documento de
engenharia reversa correspondente.

## Fora

Navegação real (rota, pulos, inimigos), equilíbrio de dificuldade e o tempo-limite como
desafio: a auditoria só confere que a lógica de objetivos, gatilhos e veredito deixa vencer.
