# Núcleo: ordem do tique, física de contato, animação, `rand` e ativação (M17)

Área transversal: o que vale para todos os objetos. Medido com o oráculo (`oraculo.md`) em
`T_J_001` (`20*,J,40*,30*L,J,20*`, 130 tiques), `T_J_003` (`10*,40*R,J,20*R,30*`, 100),
`I_J_002` (`10*,30*R,J,30*L`, 80), `T_M_002` (`10*,20*R,J,20*`, 60), `H_J_006`
(`20*,30*R,30*`, 80, com inimigos) e `T_J_001` furtivo/engatinhar
(`10*,20*LX,10*,6*D,20*DL,10*`, 80).

## Tabela

| comportamento | original (endereço/valor) | porte antes (medido) | veredito | teste/commit |
|---|---|---|---|---|
| Ordem do tique | `0x40433c`: entrada → ativação `0x435e7f` → física `0x4045a5` → resposta `0x404842` → atualização `0x434bf5` (máquina de estados) → efetivação → câmera `0x436386` | máquina de estados → física no mesmo tique (pulo sobe em t 29, corrida anda em t 61) | corrigido no Jake/Garathe/IA: `Jake::physics` no começo do tique, `after_physics` (pouso, barra, queda) antes do despacho; a velocidade gravada anda no tique seguinte. Pulo parado agora igual ao oráculo (69, 53, 37, 27, 26, 34, 48, 69; t 29–36) | `jake::tests::idle_runs_and_stops`, `standing_jump_follows_the_vy_table…` |
| `vy` gravada | antes da gravidade da física seguinte (−144 no pulo, corrida (±96, −8): `0x4262bf` soma `0x47892c` = −1 px) | depois da gravidade (−128; (−89, 0)) | corrigido (sai da ordem; correr soma −8 à vy, Jake e Garathe `0x47d148`) | idem |
| Contato com o chão | a varredura `0x432e5d` encurta o **vetor inteiro** até o contato | eixo a eixo, atrito depois | corrigido em `physics::step`: descendo e alcançando o chão, anda a fração `g/vy` do vetor (`g = 8·livres + 8`, 1 px de folga), +x perde 1/8 px, −x arredonda para baixo; atrito nas duas componentes depois do rebote. Reproduz: correr 11 → / 12 ← (vx 96), Garathe 14 (120), furtivo e engatinhar pela **metade** (vy guardada 0 → 16), aterrissagem 35 −40 → −3, mina saltadora −25/−17 → −2, −11/−3 → −1, 33 → 1, queda 48 a 4 px → desce 4 e vy −17 | `physics::tests::floor_contact_shortens_the_vector_like_the_original` (números do oráculo) |
| Avanço do quadro | `0x4238fa`/`0x423835`: `0x4879c4 mod (atraso+1) = 0` (contador global; durante o tique t vale t) | tempo no estado | corrigido (`Jake::set_clock`, a cena passa `tick_count`); rótulo 1 sai 1,1,2,2… como o oráculo | `T_J_001` t 46–60 iguais |
| Entrada com a rotina já no estado | fim do 27 (`0x4250f2`): estado 1 e `0x426ecc` no mesmo tique → já avança; 85 chama `0x423835` duas vezes (entra e avança 1); o 35 termina sem pôr 1 antes (quadro 0) | sempre quadro 0 | corrigido (`entry_advance`: fim do 27, agachar 85; o 85 encerra quando o 2º avanço passa do fim) | oráculo furtivo: 85 q 1, 3 → 86 |
| `rand` | `0x460a74`: aditivo atrasado de 56 palavras, tabela inicial em `0x483920` (dado do executável), índices 24/55, nunca semeado | LCG | implementado (`rng.rs`), reposto na carga e antes do tique 0 (como o oráculo) | `rng::tests::matches_the_original_sequence` |
| Sorteios da carga | cada inicialização na ordem do arquivo (globais, depois células, inclusive objetos de outro modo): genérica `0x40b2c8` 1 (`+0x184 = rand % n`), chaves 1, Morte 2, Manta 4, Anticorpo 5; 0 em 0, 1, 2, 6, 50–52 | nenhum; minas e itens no quadro 0 | implementado (`entity::init_draws`, `keeps_random_frame`): minas 11,5,10,2,8,8,8 e itens 4/6 na `T_J_001`, 10,2,8,8,8,6,7 e itens 4,0,1,10,1 na `T_J_003` — iguais | idem |
| Tiro: par de estados | `0x43b4ea`: `rand & 1` a cada início de tiro | LCG próprio | usa o `rand` da cena (emprestado ao Jake no tique); idem o 79 (`rand & 3`) | `punch_in_the_scene…` |
| Explosão | `0x40d20c`: sem direção, `a = rand` e rótulo `(rand >> 14) + 1`; `a < 0x4000` ou `> 0xc000` → espelho horizontal (`+0x194 \|= 1`, `0x405775`: bit 1 = x, 2 = y) | `1 + rand % 4` | corrigido (2 sorteios na ordem; espelho vertical `a > 0x8000` não implementado: o sprite não tem) | `scene.rs::explosion_entity` |
| Ativação por distância | `0x435e7f`: raio por tipo (∞: 1, 2, 19–21, 36, 42, 50–52, 71, 86, 99, 100, 106, 108 e bit `0x100000`; 400: 10, 11, 28, 29, 40, 89, 102, 104, 105, 107, 109; 600 os demais) do centro da vista do tique anterior; fora das camadas desenhadas, inativo | tudo ativo (minas animando longe) | implementado (`Scene::activate`, `tick_camera`: câmera 18 px/tique ao longo do vetor, `0x4365ae`; Ruling: salto do alvo > 64 px recentra, como no `viewer.rs`); inativo não atualiza nem anima. Minas da `T_J_001` agora paradas no quadro sorteado até a câmera chegar (q igual em todos os tiques) | medido abaixo |
| Atrito da resposta | `0x404842`: uma vez em cada eixo, também na normal depois do rebote | só na tangencial | corrigido (pedido da área de armas: cápsula na parede −4 px/tique) | `physics::tests` |
| Objetos de célula no tique 0 | itens no quadro sorteado no fim do tique 0, animam a partir do 1 | animavam no 0 | corrigido (`cell_objects`: dormem no tique 0; hipótese: só se atualizam depois do primeiro desenho) | `T_J_001`/`T_J_003` itens iguais |
| Pulo correndo (PULAR sem lado logo após correr → 29) | 29, vx −80 | 27 | **não feito** (área de movimentos do Jake) | — |
| Virar à direita desloca +1 px (Garathe, `0x434815`) | `T_M_002` t 10: 86 → 87 | não | **não feito** (movimentos do Garathe) | — |

## Medições (`compare.py --objetos`, tiques que diferem; objetos = soma dos campos)

| fase | jogador antes | jogador depois | objetos antes (q / x / y) | objetos depois (q / x / y) |
|---|---|---|---|---|
| T_J_001 | x 68, y 21, vx 34, vy 55, rot 31, q 47 | x 38, y 15, vx 4, vy 17, rot 31, q 30 (tudo igual até t 90; dali o pulo correndo) | 919 / 55 / 122 | 0 / 53 / 121 |
| T_J_003 | x 90, y 84, vx 53, vy 55, rot 61, q 72 | x 84, y 76, vx 51, vy 55, rot 49, q 62 (igual até t 15: beirada, caixa de 18 px × octógono) | 1080 / 52 / 343 | 534 / 80 / 394 |
| I_J_002 | x 70, y 14, vx 61, vy 63, rot 27, q 26 | x 39, y 15, vx 26, vy 26, rot 26, q 22 (igual até t 40: pulo correndo) | 2323 / 537 / 680 | 1120 / 354 / 353 |
| T_M_002 | x 50, y 12, vx 32, vy 33, rot 12, q 5 | x 50, y 13, vx 12, vy 15, rot 16, q 21 (+1 px da virada em t 10; 14 px/tique iguais) | 989 / 32 / 178 | 326 / 30 / 179 |
| H_J_006 | x 59, y 15, vx 25, vy 30, rot 25, q 35 | x 40, y 12, vx 8, vy 11, rot 21, q 24 (igual até t 38: quique na parede) | 7447 / 1929 / 3023 | 4686 / 587 / 579 |

`rot` dos objetos segue alto por representação: o original grava rótulo 0 e quadro absoluto
nas minas (102) e o porte o rótulo 1 (`ohtool --trace-json`); não é comportamento.

## Correções

Commit `feat(núcleo): ordem do tique…` e o seguinte (ativação). Testes ajustados ao novo
tempo (velocidade um tique depois; metade do vetor no chão) com o motivo no comentário:
`jake::tests` (correr, pulo médio, tiro parado, derrubada), `jake::moves::tests` (aterrissagens
34/35, impulso 31, furtivo 28 px, corda, pulo parado, engatinhar), `jake::garathe::tests` e
`auditoria.rs` (corrida 14, vy gravada antes da gravidade, 19 com gravidade depois do
quadro 9, 49 pelo contador global, 63 desliza 4 px), `physics::tests` (corpos "correndo" com
vy −8), `ant` (`0x4207c3` refaz vy = −8 ao andar), `the_entity`, `flyer`, `barrel`,
`scene::ai::tests` e as ajudas `go_x`/`walk_to_zone`/`jump_side`/`grab_net` dos roteiros
(param pela posição prevista; esperam o impulso; soltam o lado no ar).

**Enfraquecidos (dito):** `antibody_dives_and_bites_the_jake` 600 → 3000 tiques (sorteios);
`punch_in_the_scene…` e `bruiser_wakes_attacks…` fixam `Rng::new(k)` (com a sequência do
original o soco sai no par alternativo / o brutamontes se afasta); `ai_minotaur_approaches…`:
o humano espera e só chega perto quando o Minotauro ataca; `t_j_007_layer_doors`: faixa de x
440–470 → 425–470; o fuzz `m10_fuzz…` não confere inimigos inativos (a formiga nascida no
formigueiro dentro da parede fica lá); `t_m_008_ant…` põe a vista sobre o formigueiro; o modo
deus da auditoria não teleporta para fora da fase (`audit.rs`); **ignorados** com motivo: `i_m_109_can_be_won` (o Garathe pousa 4 px dentro
da poça) e `i_j_002_can_be_won` (a luta com os robôs de x ≈ 362 mata o Jake) — roteiros a
refazer pelas áreas de movimentos.

## Em aberto

- Entidades (`behaviour/*`) ainda fazem decisão → física dentro do próprio tique: só o
  Jake/Garathe/IA seguem a ordem do original; a ordem global (física de todos, depois todas
  as atualizações) não foi reestruturada.
- A matemática fina da varredura (`0x433999`, octógono `JK_DEFLT`): a regra de arredondamento
  por sentido é empírica; a caixa de 18 px do porte sai da beirada 1 tique depois (`T_J_003`
  t 15).
- Câmera: tremor (`+0x274`, consome `rand`), ponto fixo das células e "camada desenhada"
  (`0x4879e4`) aproximada por "camada ≥ a da vista"; alvo pela caixa do corpo, não pelo quadro.
- Objetos de célula: a regra exata (desenho/visibilidade da célula) é hipótese.
- "Pouso segurando o ataque" (`+0x160` em px depois da física): não conferido.
- Da área de armas: dano de área medido do centro do **quadro** da vítima (28 e (15, 7) no
  original × 21 e (10, −3)); efeitos criados no meio da atualização vivem 8 tiques (porte 7:
  ordem global das entidades); quique da granada 8 × 10 px/tique (conferir com o atrito novo).
- Para as áreas de movimentos: pulo correndo (29), +1 px na virada do Garathe, furtivo do
  Garathe; para inimigos: a formiga anda a `±0x40` (8 px) no original, o porte usa 2 px.
