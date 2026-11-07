# M10 — inimigos secundários e dificuldade: plano de implementação

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement
> this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Morte, Camaleão, Anticorpo, Manta, dardos, catapulta, Entidade e formigueiro agem
como no original; a dificuldade escala o dano recebido pelos inimigos e a munição.

**Spec:** `docs/superpowers/specs/2026-11-06-m10-inimigos-secundarios-design.md`

## Global Constraints

- GPL-3.0-or-later/SPDX; português; nada de dados do jogo; `/home/claude/check.sh` verde antes
  de cada commit; constantes com endereço.
- Não tocar `jake.rs`/`jake/`; `pickup.rs` só na linha da quantidade; `scene.rs` em funções
  novas e chamadas pontuais.
- Todo objeto que se move anda contra `ctx.cells_for(camada própria)`.

## Review Focus

1. Nenhum novo tipo sai da fase nem entra nas células da própria camada.
2. Morte oculta e Camaleão invisível ficam vivos; clones e formigas criadas não contam.
3. Confusão: só com a Entidade perto e na vista; some 3 tiques depois.
4. Dificuldade Normal não muda nada do que a M7a fazia.

---

### Task 1: dificuldade

**Files:** Create `difficulty.rs`; Modify `lib.rs`, `behaviour/mod.rs` (`Ctx.difficulty`),
`scene.rs` (campo e `Ctx`), `pickup.rs` (`take_weapon_at`), `bruiser.rs`,
`security_robot.rs`, `mine_generator.rs` (dano escalado).

```rust
pub enum Difficulty { Easy, Normal, Hard }
impl Difficulty { pub fn from_index(i: i32) -> Self; pub fn damage(self, d: i32) -> i32;
    pub fn ammo(self, n: i32) -> i32; pub fn death_spread(self) -> i32;
    pub fn death_wait_base(self) -> i32 }
```
- [ ] Testes: `damage_doubles_halves_with_minimum_one`; `ammo_scales`;
  `easy_bruiser_takes_double` (e OSR, OHG); `pickup_ammo_scales_with_difficulty`.

### Task 2: dardos (87, 29)

- `dart_generator.rs`: variantes `0x473080`, orientação, alcance, tolerância, munição,
  intervalo, linha de visão no Delta, % de dano; `Spawn::Dart`.
- `enemy_shot.rs`: tipo 29 com gravidade, 1 de dano com empurrão, sem efeito.
- [ ] Testes: `generator_fires_darts_at_interval_until_ammo`; `dart_falls_and_hurts_one`.

### Task 3: catapulta (41)

- [ ] Testes: `catapult_launches_jake`; `catapult_launches_barrel_once`.

### Task 4: Entidade (107) e confusão

- [ ] Testes: `entity_confuses_near_and_on_screen`; `entity_is_pushed_not_hurt`;
  `scene_swaps_directions_while_confused`.

### Task 5: Anticorpo e Manta (105, 89)

- [ ] Testes: `wander_loops`; `dive_and_bite`; `manta_sweeps_and_shoves`;
  `dormant_antibody_does_nothing`; `one_bullet_kills`.

### Task 6: Morte (3)

- [ ] Testes: `death_waits_triggers_and_appears_beside_target`; `radiation_hurts_within_45`;
  `death_shoots_bullets_99`; `hidden_death_has_no_collision`; `death_dies_with_3764`.

### Task 7: Camaleão (4)

- [ ] Testes: `bites_for_30`; `flinches_and_dies`; `clones_have_half_life_and_die_with_it`;
  `invisible_stays_present`.

### Task 8: formigueiro e formiga (88, 109)

- [ ] Testes: `anthill_spawns_on_interval_until_count`; `ant_bites_for_5_and_dies_to_any_hit`.

### Task 9: dados reais, fuzz e README

- [ ] `real_enemies.rs`: rótulos, um teste por tipo, fuzz com os novos tipos; README (M10).
