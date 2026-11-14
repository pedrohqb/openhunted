# Auditoria M17: movimentos do Jake (correções e pendências)

Continuação de [`jake-movimentos.md`](jake-movimentos.md). Testes:
`crates/openhunted-engine/tests/real_jake_moves.rs` (Jake real, cenários sintéticos; cada
um falhou antes da correção) e os testes de unidade de `jake.rs`/`jake/moves.rs`.

## Correções (comportamentos que mudaram)

Commit `f196ef2`:

1. **PULAR segurado** (`0x200`): segurar pula de novo; o fim do 27/16 decide no mesmo tique
   (`holding_jump_jumps_again`). O Garathe segue com a borda.
2. **Controle no ar** `0x408205` na queda 25 e nos pulos 16/17/27, depois da velocidade do
   esqueleto: no chão nada (sem deslize no agachar do 27/16); a queda ganha ±8 px/tique e o
   lado segue a seta (`stand_jump_air_control_only_in_the_air`, `fall_has_air_control`).
3. **Ataque no ar** nos 16/17/25/27 (`attacks_from_jumps_and_falls`).
4. **Pulo da corda 17**: parado e sem gravidade até o quadro 9, vy −20, voo até o chão
   (26); pico 90 px como no oráculo (`rope_jump_waits_nine_frames_then_vy_minus_20`).
5. **Caixa padrão** `JK_DEFLT.BRD` para todo Jake (a M_2_048 trazia `JK_EXTND`, 32 px, e o
   teletransporte o punha dentro da parede). A troca de caixa do Jake ignora sólidos que a
   caixa já cruzava (o fuzz da `real_scene` achou um Jake nascido dentro de um pilar).

Commit `eee0478`:

6. **Pulo correndo sem seta** → escolha (29); **PULAR+CIMA+lado** → longo 28, parado ou
   correndo (`up_side_jump_is_long_and_running_jump_needs_no_arrow`).
7. **Limite do controle no ar** pelo deslocamento real do tique anterior: 8 px a cada 2
   tiques no 27 (`air_control_alternates_after_moving_eight_pixels`).
8. **Empurrar**: 100 uma vez (6 tiques) a 4 px/tique, som 5043, laço 101, PULAR ignorado,
   102 ao soltar, BAIXO agacha (`push_plays_100_then_loops_101_and_ends_with_102`).
   Estado novo `PushEnd` (102).
9. **28/29** param no último quadro e só então olham chão, parede (3712 + queda) e teto
   (pose 1); **80** com contato no ar → pose 1
   (`launched_jumps_hold_the_last_frame_and_land_only_after_it`,
   `extended_jump_hitting_a_wall_goes_to_idle_pose`).
10. **Derrubada** pelo lado relativo (81 para a frente, 82 para trás); 81 só no fim do
    rótulo: 77 ou 76; 82 → **83** (estado novo `SitUp`) → **84** (`Rise` no Jake); mortes
    177/179 pelo mesmo critério (constantes `DIE_KNOCKED_FORWARD/BACK`)
    (`knockdown_follows_facing_and_gets_up_through_83_84`).
11. **Tranco no Jake**: 169 (rótulo 173, k = 2) → 84; na corda, queda
    (`jolt_knocks_jake_to_169_then_84`).
12. **Empurrão** só parado/escondido (1, 6, 79, 164–166) (`shove_only_when_standing_or_hiding`).
13. 77/78/83/84 com a sonda de 25 px.

Commit `ef1e824`:

14. **Sair pelo alto da corda**: 8 decide como `0x423b2b` (CIMA na zona → 30 direto), e o
    fim do 3 também (`climbing_past_the_top_regrabs_from_let_go`); SOCO solta a corda.
15. **Especial** sem tiro → pose pronta (`special_ends_in_the_ready_pose`).
16. **Choque** com o quadro 0 parado; **atrasos** `+0x18` das poses de arma (146, 147, 109,
    112, 128, 131, 187, 188, 49, 73) (`shock_holds_frame_zero`,
    `weapon_poses_follow_the_table_delay`).
17. **Escolha do pulo** pela faixa de `0x425144`
    (`jump_choice_probes_a_thin_strip_under_the_feet`).

Pilotos de `real_missions.rs`: `settle` espera o parado (o 102 leva 6 tiques),
`jump_side` repete o pulo cancelado; **`i_j_002_can_be_won` fica pulado** (só roda com `OH_ROTA_I_J_002`): a rota
roteirizada depende do empurrão na corrida e do empurrar antigo (o Jake fica preso aos
raios dos robôs de cima e morre); precisa ser refeita.

## Em aberto

- **Parado com punhos (6, rót. 146)**: `0x42dc43` procura jogador/inimigo (tipos 1–4, 7,
  8, 10, 11, 89, 102, 104, 105, 107, 109) vivo na camada a < 40 px; o porte precisa de
  um campo no `Env` com essa busca.
- **Beirada 162/163**: entrada do 77 quando a faixa de 3 px sob os pés não fica toda
  sobre chão (`0x407d09`, com a junção de duas células); 162 (43 q, `JK_EXTND`, k = 2 em
  x, decisão com 25 px) → 163 (15 q, `JK_COLPS`; teto ou BAIXO → 86) → parado.
- **Caixas `JK_EXTND` (76, 77, 162) e `JK_COLPS` (78, 81–84, 163, 169–179)**: falta
  carregá-las e decidir o caso da caixa larga (32 px) que não cabe junto à parede.
- **Física (área própria)**, medida no oráculo: no chão com vy guardado 0 (furtivo,
  engatinhar, deslizes 34/35) o original anda ≈ metade do pedido (Δ/16 em vez de Δ/8:
  a varredura (vx, +16) toca o chão no meio do tique); correndo para a direita, 11 px/tique
  (à esquerda 12); ricochete na parede vx −29 contra −40; contato com o teto 1 px acima.
- **Núcleo** (já com o `audit/nucleo`): quadro de entrada dos estados de dois quadros por
  tique (85, 164, 34, 35: o original troca e avança no mesmo tique, entra no q1; 35 dura
  1 tique e 34 4), além da defasagem de um tique da velocidade.
- **Armas**: guardar a arma (147, 19…) toca ao contrário; o tiro/soco (150) para no último
  quadro em vez do laço; a entrada de 0x2000…0x80000 (bordas) não é usada.
- **Cambalear do terremoto** (`+0x274`, `0x426f6a`): rótulo 15 com atraso 1 em todos os
  estados não isentos; o porte o faz com o choque elétrico (área do Bruiser).
- **Duas setas** no motor (decisão): o original dá prioridade à ESQ (`0x422a62`) e treme
  no ar (inverte todo tique); o teclado nunca entrega as duas (eixo R−L), o porte mantém a
  mais recente.
- Ordem de desenho `+0x1b4` (0x28 escondido/corda, 0x6e barra, 0x78/0x79): sem campo.

## Notas para o Garathe

`0x408205` é a mesma rotina: o limite de 8 px pelo deslocamento real (`+0x15c`) vale para
ele também (não mexido aqui; `air_control` só o aplica ao Jake).
