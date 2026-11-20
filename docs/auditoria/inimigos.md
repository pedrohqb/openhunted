# Auditoria: inimigos principais (M17)

Tipos 8 (robô de segurança), 10 (sentinela), 11 (mina saltadora), 102 (minas), 103
(gerador), 7 (Bruiser), 104 (olho-laser) e 36 (raio), com os tiros 21/100/106. Rotinas em
`docs/re/robo-seguranca.md`, `robo-sentinela.md`, `mina-saltadora.md`, `minas.md`,
`gerador-minas.md`, `bruiser.md`, `olho-laser.md`, `projeteis.md`, `dano-vida.md`.
Oráculo (sessões sem entrada salvo indicação, campos internos lidos com
`ORACULO_CAMPOS`, ver `oraculo.md`): `H_J_001` (robôs 37/42/50, sentinelas, olho 49),
`D_M_024` (sentinela no Garathe), `H_J_006` (saltadoras), `I_J_004` (minas), `H_J_013`
(gerador Alpha), `H_J_024` (Bruisers 42–44). O `t` do oráculo é o fim do tique (a mesma
contagem do `--trace-json`). **Núcleo** = diferença do núcleo comum (ordem física/atualização,
sorteios, ativação), fora desta área (`audit/nucleo`).

Placar (52 linhas): **27 igual** (1 a confirmar no oráculo), **19 corrigidas** (18 diferente +
1 não implementado), **3 núcleo**, **3 em aberto** (diferente).

## Robô de segurança (8)

| comportamento | original | porte (medido) | oráculo | veredito | teste/commit |
|---|---|---|---|---|---|
| variantes (21 `i16`) | `0x472a18` | `VARIANTS` | — | igual | `robot_cannons_alternate…` |
| 1ª física antes da compensação `0x404bc7` (gravidade 2 px) | `0x404656` | ficava na y da fase | 317 → 319 | diferente → corrigido | `first_physics_drops_two_px` 421e25e |
| giro `0x41d9c8`: −1 vale 8/24; alinhado não muda; empate `rand & 1` | `0x41d9e2` | forçava −1 ao alinhar | ori 10, 12 … 24 | diferente → corrigido | b33c25d |
| patrulha `0x41d426`: um eixo, vira no centro da célula (bits `0x2/0x8/0x10/0x40`), beira, modo, sorteio/contato, coleira dos gatilhos, `+0x268/+0x26e` | transcrita | era simplificada (passo livre) | robôs 37/50: x igual tique a tique, viram em 561 e 945/811 | diferente → corrigido | `patrol_turns_at_the_level_edge` b33c25d |
| anda girado (rótulo 2, quadro 8 espelhado), sem flutuar | `0x41cb94` | de lado, flutuando | rot 2, q 8, y fixo | diferente → corrigido | `patrol_moves_turned_and_level` |
| flutuação: só −1, sem contato e sem velocidade; `(n mod p)·(0x10000/p)`, `>> 14` | `0x41cbcd` | só `vy = 0`; arredondamento | — | diferente → corrigido | b33c25d |
| atenção: compara o valor antes do decremento | `0x41c90d` | 1 tique antes | — | diferente → corrigido | `attention_runs_out_one_tick_late` |
| busca: 256 px, esconderijo, camada, aleatório entre candidatos | `0x41db0f` | iguais (o sorteio `% n` agora consumido) | — | igual | `robot_ignores_hidden_jake` |
| perseguição vertical por `célula+0x04` (linha pelo `+0x150`) | `0x41d226` | teste de passo livre | — | diferente → corrigido | `chase_climbs_by_the_cell_bits` |
| chegada à última posição (`|dx| ≥ s/2`) e troca de camada pelas zonas PR | `0x41d2ad`, `0x41d2e6` | não trocava de camada | — | não impl. → corrigido | b33c25d |
| distância 80–180/200 px; canhões alternados a meio período, ±0x600, certeiro | `0x41ce0d` | iguais; agora atiram com alvo em −1 também indo à última posição | sem alvo nas sessões | igual (a confirmar no oráculo) | `robot_cannons…`, `robot_keeps_its_distance` |
| dano: teto 45, dificuldade, negativo → 1, 0 nada, morrendo ainda conta | `0x41ca22` | igual (só com dano ≠ 0) | — | igual | `robot_health_caps…` |
| queda (−2) e 10 explosões; som 3103 | `0x41cc6d` | iguais | — | igual | idem |

## Sentinela (10)

| comportamento | original | porte (medido) | oráculo | veredito | teste/commit |
|---|---|---|---|---|---|
| estados 1/4/2/3/−1/5, um quadro por tique | `0x41b640` | iguais | `D_M_024`: 2 q0–7, 3 q0–6 (×2), −1 q7–0, 4 | igual | `sentinel_*` |
| quadro inicial sorteado (decide no 1º fim de rótulo) | `0x40b2c8` | quadro 1 | q 1–3 no t 0 | núcleo (sorteios da carga) | — |
| visão `|dy| ≤ 50`, `|dx| ≤ 150`, lado, linha; sem esconderijo | `0x41bacb` | iguais | abre ao ver o Garathe a 28 px | igual | — |
| 30 % em alerta, 45 % repete, 50 % 1/4 | `0x41b77c` | iguais | sequência aleatória compatível | igual | — |
| 3 balas 106: x+`rand%5`±10, 1–12 px acima, vx ±20, vy 0–5 | `0x41bcdb` | iguais | x 869–873, vy 0/1/3 px | igual | — |
| bala 15, empurrão; três no mesmo tique valem uma | `0x43d6c5` (atribui) | somava 45 | 3 balas → 2 × 7/8 | diferente → corrigido | `direct_hits_assign…` 44e6dab |
| fechado invulnerável; vida 4; tiro 106 não fere | `0x41b9c9` | iguais | — | igual | — |

## Mina saltadora (11)

| comportamento | original | porte (medido) | oráculo | veredito | teste/commit |
|---|---|---|---|---|---|
| pulinho 2–5/−8, grande 6–11/−12…−9, virar −7, 5 pulinhos | `0x41c5aa/543/5f9` | iguais | vx −40/−32/−24, vy −64/−56/−88 | igual | `hop_pattern…` |
| decisão no fim do rótulo; 33/33/34 %; visão 450/50/lado | `0x41c093` | iguais | — | igual | `hop_mine_sees_only_ahead` |
| quique ao pousar (vy −25), atrito 7/tique | física | sem quique | `H_J_006` | núcleo | — |
| virar: `0x434815` desloca x ±4 (caixa fixa) | `0x434815` | x fixo | 366 ↔ 370 | diferente (aberto) | — |
| fora da fase pela posição ou sem célula → some sem explodir | `0x41beee`, `0x41bf5d` | pelo centro | — | diferente → corrigido | `leaves_by_position…` 41942e6 |
| contato de qualquer objeto fora de 6, 9, 11, 22–25, 30, 46, 50–52, 56 detona | `0x41c49a` | só o Jake e tiros do jogador | — | diferente (aberto) | — |
| explosão (11, 0): 20 direto (atribui), raio 12 | `DAMAGE.TBL` | iguais | — | igual | `hop_mine_explodes_on_jake` |

## Minas (102) e gerador (103)

| comportamento | original | porte (medido) | oráculo | veredito | teste/commit |
|---|---|---|---|---|---|
| variantes Alpha–Delta | `0x472bfc` | iguais | — | igual | — |
| 1ª física 2 px | `0x404656` | na y da fase | 290 → 292; do gerador 147 → 149 | diferente → corrigido | `first_physics_drops_two_px` |
| sem alvo, parada | `0x41e8c4` | igual | `I_J_004` | igual | — |
| oscilação `((t + x) mod p)·(0x10000/p)`, `>> 14` | `0x41e996` | divisão depois; truncava | — | diferente → corrigido | 421e25e |
| alvo: visão nas células **e objetos** (`0x43735d`) | `0x41ed72` | 16 amostras, só células | mina 30 sem alvo no original, com alvo no porte | diferente (aberto) | — |
| fora da fase/sem célula → some | `0x41e884` | pelo centro | — | diferente → corrigido | `leaves_by_position…` |
| gerador: conta `iv`, abre no `iv`º tique | `0x41de45` | igual | Alpha abre no t 19 | igual | `opens_on_the_twentieth…` |
| abrir mostra já o q1 (q1…q7), fechar q7…q0 | `0x407942` | q0…q7 | `H_J_013` | diferente → corrigido | idem b45d795 |
| recarga `iv + (rand%iv)/2 − iv/4`; conta ao fechar | `0x41e1c3` | iguais | 17 | igual | — |
| mina em `(x+15, y+8)`, vida da variante, ≤ 4 | `0x41e497` | iguais | 390,147 | igual | `generator_opens…` |
| dano ×% e dificuldade | `0x41dfaf` | iguais | — | igual | `generator_damage…` |

## Bruiser (7)

| comportamento | original | porte (medido) | oráculo | veredito | teste/commit |
|---|---|---|---|---|---|
| acordar (−3, 6 tiques) sem gatilhos | `0x416c68` | igual | 42–44: −3 q5…0 | igual | `bruiser_sleeps…` |
| 13 e −2 só nos tiques ímpares | `0x41764b/77` | 1 q/tique | 13: q0,1,1,2,2…; 12 no t 15 | diferente → corrigido | `looking_around…` e152544 |
| 12 e andar (1/2, vx da tabela `0x472498`) | `0x417a68` | iguais | x 616 → 636 em 1, t 37–48 | igual | — |
| ataques, fogo, terremoto, metade do dano, imune a fogo | `0x41750b`, `0x417ae0` | iguais | — | igual | `bruiser_*` |
| terremoto no Jake: `+0x274` tiques; cada um desconta e, fora dos isentos (sem gravidade `+0x178 & 8`; estados 11, 12, 16, 17, 25, 27–30, 32, 34, 35 pela tabela `0x426fae`; 76–78, 80–87, 103–107, 162–166, 177, 179), põe o estado 15 com a rotina do parado `0x426ecc`; zerado, a rotina do parado | `0x426f6a`, `0x42705f` | choque elétrico a cada tique no chão (contador da cena) | — | diferente → corrigido | `quake_staggers_the_jake_in_state_15` |
| esmagar (15, derrubada) atribui | `0x417308` | somava | — | diferente → corrigido | 44e6dab |

## Olho-laser (104) e raio (36)

| comportamento | original | porte (medido) | oráculo | veredito | teste/commit |
|---|---|---|---|---|---|
| varredura: `+0x290` desconta e em 0 recarrega e anda (período + 1) | `0x41f5d9` | a cada `período` | Beta: passos t 4, 9, 14 | diferente → corrigido | `sweep_steps_every…` 504f8cd |
| cone 1500, alcance, linha; segue só com o Jake andando | `0x41f974` | iguais | alvo no t 15 | igual | `eye_sees_jake…` |
| rajada: intervalo, tiros, pausa (Beta 3/5/15×3) | `0x41f6..` | iguais | tiros t 17–29, 80… | igual | `burst_is_five…` |
| dois raios; dano 5/7/10/5 com empurrão | `0x41fbe6`, linha 36 | iguais | Beta: 7 (3 vida + 4 armadura) | igual | `bolt_flies…` |
| os dois raios no mesmo tique valem um | `0x43d6c5` | somava 14 | 7 por par | diferente → corrigido | 44e6dab |
| qualquer dano mata; explosão raio 48 | `0x41f898` | iguais | — | igual | `any_damage…` |

## Comum

| comportamento | original | porte (medido) | oráculo | veredito | teste/commit |
|---|---|---|---|---|---|
| ativação por distância (400/600/∞ px da vista, `0x435e7f`) | tabela `0x435efb` | tudo ativo | sentinelas e saltadoras longe paradas (`H_J_006`) | núcleo (implementado em `audit/nucleo`) | — |
| tiros 21/100/106 (linhas, velocidades) | `projeteis.md` | iguais | balas 106 | igual | `bruiser_bullet_and_rocket…` |

## Correções

b33c25d (robô: patrulha/giro/flutuação/perseguição; `Ctx` ganha `grid`, `zones` e `at`),
421e25e (1ª física de robôs e minas; oscilação), 44e6dab (`Pending::assign`/`Out::hit_player`:
golpe direto atribui), 504f8cd (olho), 41942e6 (saída da fase de 11/102), b45d795
(gerador), e152544 (Bruiser), a040d86 (terremoto no Jake;
o contador `Scene::quake` deixa de dar choque). Ferramenta: b3366e8 (`ORACULO_CAMPOS`/`ORACULO_TIPOS`).
Roteiros adaptados: `i_j_002_can_be_won` (porta, retorno do vão, robôs do vão fora da caça,
vida reposta — `TOUGH` — e o robô que patrulha atrás da porta abatido direto),
`i_m_109_can_be_won` (BAIXO solta o teto; a poça deixa de ser exigida).

## Em aberto

- Linha de visão (`0x433aa3` células, `0x43735d` objetos): o porte amostra 16 pontos e as
  minas só olham as células (mina 30 da `I_J_004`).
- Mina saltadora: contato com qualquer objeto (lista de exceções) e o deslocamento ±4 da
  virada (`0x434815`, caixa fixa); quique ao pousar (núcleo).
- Combate do robô (canhões, distância, troca de camada) sem medição: nenhuma sessão deu alvo
  a um robô; e os sorteios (rumo da patrulha, mudança de rumo) só alinham com o `rand` do
  núcleo.
- Contato `+0x224` do robô aproximado pelo passo (parede, teto, chão descendo); com contato o
  original mantém a velocidade do tique anterior.
- Dano direto no Jake de outras áreas (barril, Morte, voadores, catapulta) ainda soma.
