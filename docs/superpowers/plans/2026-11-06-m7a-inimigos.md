# M7a — inimigos principais: plano de implementação

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement
> this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** o robô de segurança (8), a sentinela (10), a mina saltadora (11), o gerador de
minas (103) e o Bruiser (7) agem como no original: veem o Jake, se movem, atacam, levam
dano, morrem explodindo e contam como mortos do jogador.

**Architecture:** um módulo por tipo em `behaviour/`, o comum em `behaviour/enemy.rs`, os
projéteis inimigos em `behaviour/enemy_shot.rs`; a cena só cria os objetos pedidos, filtra
sons globais e aplica o terremoto.

**Tech Stack:** Rust 2024 (MSRV 1.85), SDL3 no viewer.

**Spec:** `docs/superpowers/specs/2026-11-06-m7a-inimigos-design.md`

## Global Constraints

- GPL-3.0-or-later/SPDX; português; nada de dados do jogo no repo; `/home/claude/check.sh`
  verde antes de cada commit; constantes com endereço (dos docs `docs/re/`).
- Não tocar `jake.rs`, `jake/moves.rs`, `pickup.rs`, `door.rs` nem os projéteis do Jake da
  M7b; as edições em `scene.rs` ficam em funções novas e em chamadas pontuais.

## Review Focus

1. Projétil inimigo nascendo dentro de parede ou fora da fase: some sem panicar.
2. Inimigo morto não atira nem fere; destroço não barra balas.
3. Gerador: nunca mais de 4 minas vivas; estoque cai uma vez por abertura.
4. Crédito de morte só por bala do jogador; cada inimigo conta uma vez.
5. Fases sem Jake ou com o Jake morto: nenhum inimigo panica sem alvo.

---

### Task 1: base comum e encanamento na cena

**Files:** Create `behaviour/enemy.rs`; Modify `behaviour/mod.rs` (`Out.kills`,
`Out.enemy_spawns`, `Out.quake`), `entity.rs` (variantes de `Behaviour`, `from_object`,
sprites congelados), `scene.rs` (`spawn_enemy_objects`, `throttle_sounds`, terremoto,
crédito), `behaviour/interact.rs` (explosões e raios).

**Interfaces (Produces):**
```rust
pub struct Clip { pub label: u16, pub frame: usize, pub reverse: bool }
impl Clip { pub fn new(label: u16, reverse: bool) -> Self;
    pub fn advance(&mut self, len: usize) -> bool /* acabou */; pub fn offset(&self, len) -> usize }
pub struct Wound { pub pending: Pending, pub by_player: bool }
pub enum Spawn { Shot { kind: u32, at: (i32, i32), vel: (i32, i32), layer: i32 },
    Mine { at: (i32, i32), variant: usize, layer: i32, owner: u32 },
    Fire { base: (i32, i32), height: i32, layer: i32 } }
pub fn line_of_sight(ctx: &Ctx, a: (i32, i32), b: (i32, i32), skip: &[Rect]) -> bool;
pub fn jake_target(ctx: &Ctx, layer: i32) -> Option<Rect>;
pub fn hurt(e: &mut Entity, p: &Pending, by_player: bool) -> bool;
pub fn frame_cue(table: &[(usize, u16, u16)], frame: usize, at, layer) -> Option<Cue>;
```
- [ ] Testes: `clip_plays_forward_and_back_and_reports_the_end`; `line_of_sight_skips_own_box`.

### Task 2: projéteis inimigos (`enemy_shot.rs`)

- Linhas `(106, 0)`, `(100, 0)`, `(21, 0)`, `(99, 0)` do `DAMAGE.TBL`; voo reto contra as
  células; acerto varrido no Jake; parede → efeito (100 e 21 com área).
- [ ] Testes: `sentinel_bullet_hurts_jake_with_a_shove`; `bruiser_bullet_and_rocket_blast`;
  `shot_dies_in_walls_and_outside`.

### Task 3: sentinela (OSE, 10)

- Estados 1/4/2/3/−1/5 com os rótulos do `ose.anm`; fim de animação `0x41b77c`; percepção
  `0x41bacb`; disparo `0x41bcdb` (3 balas 106); fechada é invulnerável; morte `(10, 0)`.
- [ ] Testes: `sentinel_opens_and_fires_three_bullets`; `closed_sentinel_is_invulnerable`;
  `sentinel_sees_only_ahead_when_open`; `sentinel_ignores_hiding`.

### Task 4: mina saltadora (OHS, 11)

- Pulinhos (5) e pulo grande `0x41c5aa`/`0x41c543`; sem alvo, sorteio virar/parar/andar;
  percepção para a frente até 450 px; contato → explosão `(11, 0)` com 20 diretos.
- [ ] Testes: `hop_pattern_five_small_one_big`; `hop_mine_explodes_on_jake`;
  `hop_mine_sees_only_ahead`.

### Task 5: gerador de minas (OHG, 103)

- Variantes `0x472ad0`; intervalo sorteado; estoque; 4 vagas; minas na posição + (15, 8),
  escala 25; dano × %; Delta exige visão.
- [ ] Testes: `generator_opens_and_releases_mines_up_to_four`; `generator_damage_percent`;
  `only_delta_needs_line_of_sight`.

### Task 6: robô de segurança (OSR, 8)

- Tabela `0x472a18`; atenção; esconderijo; giro `0x41d9c8`; perseguição a 80–200 px;
  patrulha; flutuação; canhões alternados com rajada/pausa; raio 36 com a variante; teto de
  45; queda e 10 explosões; pulso 3157.
- [ ] Testes: `robot_ignores_hidden_jake`; `robot_cannons_alternate_and_pause`;
  `robot_health_caps_at_45`; `robot_falls_and_explodes`; `robot_keeps_its_distance`.

### Task 7: Bruiser (OBR, 7)

- Ativação por gatilhos; máquina de estados `0x417061`; passos pela tabela `0x472498`;
  escolha do ataque `0x41750b`; tiros, fogo, terremoto, esmagar; metade do dano, imune a
  fogo, tranco; morte em 30 explosões; fala.
- [ ] Testes: `bruiser_sleeps_until_triggers`; `bruiser_takes_half_and_ignores_fire`;
  `bruiser_attacks_and_quakes`; `bruiser_dies_in_thirty_blasts`.

### Task 8: dados reais, fuzz e documentação

- [ ] `enemy_labels_match_the_real_animations`; OSE da `I_J_001` abre e atira; pistola mata
  OHS e conta; gerador da `I_J_005` solta minas; OSR atira; fuzz 2000 tiques em toda fase
  solo com inimigos; tutoriais seguem vencíveis; uma vitória roteirizada com inimigos.
- [ ] README (linha M7a) e `docs/re/README.md` se houver doc novo.
