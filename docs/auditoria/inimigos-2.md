# Auditoria M17: inimigos secundários e personagens da IA

Método em [`README.md`](README.md). Original: endereço do `Hunter.exe` e o oráculo
(`tools/oraculo/`, entrada parada, 300 tiques salvo indicação; comparado com
`ohtool render --trace-json`, `--defasagem 1`). O `rand`, a ordem do tique e a ativação por
distância são do núcleo: só distribuições e trechos determinísticos contam aqui.

**Placar** (39 linhas): igual 11 · diferente 20 (14 corrigidos) · não implementado 4 (3
corrigidos) · a confirmar 4.

Fases medidas: `D_J_012` (Morte, 600 t.), `I_J_107` e `I_M_011` (Anticorpo), `D_M_024`
(Manta), `H_J_009` (Camaleão), `I_J_013` (dardos), `H_J_039` (catapulta), `I_M_014`
(Entidade), `D_M_010` e `D_J_001` (IA).

| comportamento | original (endereço/valor) | porte (medido) | oráculo | veredito | teste/commit |
|---|---|---|---|---|---|
| Morte: tique que solta os gatilhos | `0x41588b`: liga `+0x278` e sai | seguia o tique (tentava aparecer no t0) | t0 parada, tentativa no t1 | diferente → corrigido | 4f2cd3b |
| Morte: lugar da aparição | `0x416517` alinha a borda da caixa com a do alvo e os pés − 5; `0x432e5d` varre `d` = ±(10 + rand%10) e **`+0x15c` (1/8 px) é somado à posição em px** (`0x4163ac`); chão (`0x416646`, 15 px) recuando 10 px enquanto a ≥ 20 px da origem do alvo; 2 tentativas | ao lado da caixa (10–49 px), colada ao Jake: a radiação o matava em 260 t. | x 1083 (d −11), 1302 (16), 1254 (10) com o Jake em 1202; y 452 → 457 | diferente → corrigido | `death_appears_eight_times_the_shift_away` |
| Morte: aparição que falha | fica na origem do alvo, na camada dele | não se movia | t1: (1202, 309), oculta, `vy` 120 | diferente → corrigido | `failed_appearance_leaves_death_on_the_target` |
| Morte: tique da aparição | segue no estado 1 (anima, radia); `vy = 0x78` da sonda | saía; `vy` 0 | 1.ª linha visível q1 | diferente → corrigido | idem |
| Morte: rearme `+0x264` | na tentativa oculta (sucesso ou falha) e na queda rápida; o fim do −2 não rearma | rearmava ao sumir | — | diferente → corrigido | 4f2cd3b |
| Morte: fins de animação | `0x415d63` (tabela `0x415d84`): 1/4/6 ⅓ −2/5/7; 8 ⅓ −2/7/5; 5,7 atiram; 2 ½ 4/3 | idem | 1→7→8→5→6→7… | igual | `end_of_clip` |
| Morte: tiro 99 | `(x+30 \| x−20, y+2)`, vx ±20, vy sorteado | idem | bala em (1113, 459), vx 160 | igual | — |
| Morte: radiação | `0x416a48`: `|Δ+0x154|`, `|Δ+0x158|` < 45 (posições) | centros | — | diferente → corrigido | `radiation_measures_origins` |
| Morte: radiação na IA | percorre a lista; linha `(3, 1)` = tipos 1 e 2 | só o humano | — | não implementado → corrigido | `radiation_reaches_ai_jakes` |
| Morte: sons 3051/3764 | `0x43c93d(id, 0, …)`, sem posição | posicionais, prioridade 9 | — | diferente → corrigido | 4f2cd3b |
| Morte: falas ocultas | `rand%300 < 2`, `+0x264 > 30`; 40 % a 3763, senão fila de 2–3 (`0x43c868`) | uma fala só | — | diferente | em aberto |
| Morte: falha "sem célula" | `0x4165d5`: fora da grade **ou célula vazia** | só os limites da fase | — | a confirmar | Ctx sem a grade |
| Morte: dano | `0x416979`, com dificuldade, só com vida > 0 | idem | — | igual | `death_dies_*` |
| Jake: veneno inicial | `values[4]` → `+0x270` | 0 | `D_J_012`: vida 99 no t0, 98 no t20 | diferente → corrigido | `d_j_012_jake_starts_poisoned`, ff61329 |
| Veneno: fase | `tique % 20 = 0` antes de avançar `0x4879c4` | um tique depois | 150 t. idênticos depois | diferente → corrigido | ff61329 |
| Voadores sem alvo | descanso: sem `+0x27c` nada (`0x411504`, `0x421705`) | passeava (todos) | `I_J_107`: 0 movimentos em 9 × 300 t.; `I_M_011`: 0 em 6 | diferente → corrigido | `without_target_flyers_stay_resting` |
| Anticorpo: ataque | descanso → modo 1 (rótulo 2, laços, `0x411538`) → mergulho ou perseguição pelo `+0x272` (`0x4115c3`); sem alvo no fim → 5 | descanso → 2/3 por sorteio | — | diferente → corrigido | `antibody_winds_up_in_loops_before_attacking` |
| Manta: ataque | descanso → 2 (`+0x272`) ou 3 direto (`0x421739`) | idem por sorteio | `D_M_024`: 49 t. no 3, depois 2→4→5→1 | igual | idem |
| Voadores: espera do descanso | `+0x279 > 0` sai antes da animação (`0x41148a`) | animava | q parado em 1 por 6–12 t. | diferente → corrigido | cc1ce4a |
| Voadores: fim do ataque | mergulho/perseguição/volta até `+0x30` tiques → descanso, casa nova (`0x41184a`, `0x42250b`) | 2×/4× `+0x30`, depois passeio | — | diferente → corrigido | idem |
| Anticorpo: percepção | só no fim de cada animação (`0x411a03` → `0x411a53`), centros, alcance, linha | a cada tique | — | diferente | em aberto |
| Anticorpo: tempo da perseguição | sobe só com `rand & 1` (`0x4117ae`) | todo tique | — | diferente | em aberto |
| Voadores: som 3070 | 10 % por tique nos modos 1–5 (`0x41157e`) | ausente | — | não implementado | em aberto |
| Voador descansando no ar | `I_J_107` id 35 desce 2 px/t. até o chão; os demais ficam | parado | 1 de 15 | a confirmar | `0x404bc7` não lido |
| Mordidas 105/89 | linhas `(105\|89, 0, v)` 15/20/25/30 | idem | — | igual | `antibody_dives_*`, `manta_*` |
| Camaleão: estados | `H_J_009`: 34, 26, 27, 4, 6, 28, 7, 29, 1, 35, 36, 23, 25, 3, 37, 38 | só 34, 26, 27, 28, 4 | 300 t. | diferente | em aberto |
| Camaleão: queda inicial | 34 → 26 (t2) → 27 (t12) | idem (defasagem 1) | — | igual | — |
| Geradores de dardos | sem alvo, parados | idem | `I_J_013`: 3 parados | igual | — |
| Dardo × personagem da IA | contato sólido → linha `(29, …)` | `enemy_shot::object_hits` | — | igual | — |
| Catapulta: rótulo inicial | rot 0 (quadro absoluto) | rot 1 | `H_J_039` | diferente | em aberto (rastro/núcleo) |
| Entidade `values[2] ≠ 0` | parada, anima o rótulo 1 | idem | `I_M_014` id 44 | igual | — |
| Entidade: altura de repouso | y 203 (`vy` −16, no chão) | cai a 219 | `I_M_014` id 44 | diferente | em aberto (escala da caixa?) |
| Entidade: vaguear (`values[2] = 0`) | `0x412a1c` | vagueia | as três inativas (camada) | a confirmar | — |
| Formigas/formigueiro | nenhum no solo perto do início | — | — | a confirmar | — |
| IA apanha itens | `0x40b3f5`: tipo 1/2 com `+0x19c = 0` → 0 | não apanha | — | igual | — |
| Gota (71) × IA | `0x40f6f0`: tocado 1/2 → `+0x270 = 1`, fala, `0x43d629` | atravessava | — | diferente → corrigido | `the_drop_poisons_an_ai_character_it_lands_on` |
| Especial da IA | a mesma rotina do jogador (`0x427643`/`0x42c348`) | `special` nunca lido | — | não implementado → corrigido | `ai_special_knocks_down_the_human` |
| Terremoto do Bruiser × IA | `0x418229`: tipos 1/2 vivos e em jogo a < 300 px, `+0x274 = 20 + rand%20` | só o humano | — | não implementado → corrigido | `quake_reaches_ai_characters`, `bruiser_quake_shakes_ai_characters` |
| Jakes da IA: queda e pouso | física do jogador | idem | `D_M_010` (3), `D_J_001`: rótulos e y iguais | igual | — |

## Correções

- 4f2cd3b (`behaviour/death.rs`): aparição, falha, ativação, rearme, radiação pelas
  posições, sons.
- ff61329 (`scene/ai.rs`, `jake.rs`): `values[4]` → veneno; fase do veneno.
- e2be449: personagens da IA como alvos dos objetos: `behaviour::Npc` em `Ctx::npcs`,
  `Out::npc_hits`/`npc_quakes` aplicados por `Scene::apply_npc_out`; `AiPlayer::quake`;
  `Scene::ai_special`. Usados pela radiação, pela gota e pelo terremoto.
- cc1ce4a (`behaviour/flyer.rs`): descanso sem passeio, laços do Anticorpo antes do ataque, espera
  sem animação, fim do ataque no descanso.

## Em aberto

- Camaleão: a máquina de estados completa (`0x418704`, ~38 rótulos).
- Anticorpo: percepção no fim da animação, contagem da perseguição com `rand & 1`, som
  3070, a descida do id 35.
- Morte: fila de falas (`0x43c868`) e o teste de célula vazia.
- Entidade: altura de repouso (16 px) e o vaguear; catapulta: rótulo inicial no rastro.
- Minotauro da IA (`D_J_001`): corre 15 px/t. contra 14 (área `garathe-movimentos`).
- O empurrão do especial nos personagens vem de `jake::SPECIAL_KNOCKDOWN`; a área de armas
  troca o valor do jogador para (±15, 20) com dano: unificar na integração.
