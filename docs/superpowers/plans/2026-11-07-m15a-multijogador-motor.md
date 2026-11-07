# M15a — núcleo de dois jogadores no motor: plano de implementação

**Goal:** a cena com 1 ou 2 humanos nos modos solo, coop e disputa, com as regras do
original, sem mudar o solo.

**Spec:** `docs/superpowers/specs/2026-11-07-m15a-multijogador-motor-design.md`

## Global Constraints

- GPL-3.0-or-later/SPDX; português; nada de dados do jogo; `/home/claude/check.sh` verde
  antes de cada commit; constantes com endereço.
- Solo idêntico: rastro `--trace` das 100 fases antes/depois (script no rascunho).
- M14 corre em paralelo na casca (`crates/openhunted/src`): API do motor só cresce.

## Review Focus

1. Ordem do tique do solo intacta (o assento 0 nunca é trocado).
2. Crédito: só o golpe fatal; herança por tiro e explosão.
3. Vagas: máscaras e ordem do coop.

---

### Task 1: `--trace` com resumo da cena; rastro de base; spec e plano
### Task 2: `scene/players.rs`: `Mode`, máscaras, atribuição das vagas (TDD) e `build_with`
### Task 3: `Seat`, `with_seat`, `tick_players`; desenho e foco por jogador
### Task 4: alvos por entidade (`Out` por jogador); IA com dois alvos; humano × humano
### Task 5: crédito, mortos por jogador, F/S (TDD por arma)
### Task 6: regras: coop (veredito), disputa (renascer, cadáver, itens, fins), cores
### Task 7: testes com dados reais (`M_*`, `C_*`, disputa roteirizada, fuzz ignorado);
rastro depois = antes; README
