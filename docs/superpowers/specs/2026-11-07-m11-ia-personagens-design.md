# OpenHunted M11 — personagens controlados pelo computador: design

Data: 2026-11-07. Base: `docs/re/minotauro-ia.md` (`0x42ec1c`), `garathe*.md`,
`jake-movimentos.md`, `dano-vida.md`, `projeteis.md`, `armas-2.md`.

## Objetivo

Os objetos de tipo 1 (Jake) ou 2 (Garathe) que não são a vaga do jogador (`modos & 1 = 0`,
presentes no modo, ex. `0xfe`) passam a ser personagens da IA: Minotauros inimigos nas fases
do Jake e Jakes inimigos nas do Garathe (hoje estátuas que travam o "destruir"). Fora: dois jogadores, o especial (a IA nunca o usa), falas.

## Arquitetura: o mesmo corpo, outra entrada

- **Ruling:** um personagem da IA é uma entidade `Behaviour::AiPlayer(Box<AiPlayer>)` com
  `AiPlayer { body: Jake, brain: Brain, … }`. O corpo é a máquina inteira do jogador
  (`jake::Player`), montada pelo código do `Scene::build` (ANM, BRDs, `.SKE`, perfil, vida,
  inventário), extraído para `scene/ai.rs`. O caminho do jogador humano não muda.
- Cada tique (antes das entidades): o cérebro devolve os bits de entrada e o corpo roda
  `tick_in(bits, Env)` com o mundo da camada dele (células, portas e objetos sólidos dessa
  camada). Depois: sprite (rótulo, quadro, espelho, deslocamento do esqueleto) e `e.x/e.y/
  e.boxes/e.health` copiados do corpo; fora da fase volta ao início (`0x404716`).
- **Cérebro puro** (`src/ai.rs`, `src/ai/path.rs`): `Brain::think(&Me, Option<&Target>,
  &mut Senses) -> u32`; `Senses`: mundo, grade de células, zonas PR, perigos, sorteio.
- **Dono dos tiros:** os `Shot` do corpo viram as mesmas entidades das armas do jogador; a
  cena guarda `(índice do tiro, índice do dono)` (as entidades nunca saem da lista). Em
  `bullet_hits`/`weapon_hits` o dono não é vítima e o jogador humano passa a ser vítima dos
  tiros da IA (mesma camada, caixa varrida). As balas da IA não dão crédito.
- Dano à IA: `enemy::hurt` encaminha ao `body.hurt` (bala, chumbo, foguete, estrela, faca,
  chicote, soco, investida, explosões; o fogo e a cerca por `apply_hazards`). A armadura vale;
  **sem** fator de dificuldade (`0x4283b7`). O especial do jogador derruba a IA a < 70 px.
- Acertos entre personagens pela linha do `DAMAGE.TBL` (`damage::row`) com o tipo da vítima
  (1 ou 2): bala 19 → 2 e empurrão; chumbo 20 → Jake derrubado, Garathe empurrado; faca,
  estrela, chicote e foguete pelas linhas deles; o soco pelo `Strike` do corpo. Os tiros dos
  inimigos acertam a IA como a qualquer objeto sólido.
- **Ruling:** personagens não se barram (todos têm `0x4000`, `blocks_flags`).
- Morte pelo código do jogador; o cadáver fica (`alive`), com `solid_shapes = false` e
  `flags |= 0x8000`; vida ≤ 0 cumpre o "destruir". Tipos 1 e 2 não contam mortes.

## Cérebro (`0x42ec1c`)

- **Ritmo:** pensa a cada 2 tiques (`+0xdc`); no tique ímpar repete a última entrada.
- **Alvo** (`0x42c5cc`): o jogador humano, se vivo; senão nenhum (modo 3).
- **Vê:** distância centro a centro < 400, mesma camada e linha livre (16 amostras no mundo
  da camada), ou caixas se tocando; no modo 3, alvo escondido (classe 10) ou morrendo (14) não
  é visto. Classes (`0x42c7b6`) pelo estado semântico do corpo (tabela da RE).
- **Modo 1 caçar:** dano pendente > 10, ou dano com vida < 30 → modo 2. Classe própria 2, 6
  ou 0xf → 0. Vendo: alvo 10/14 → modo 3; no ar (5/7) CIMA se o alvo está acima e
  `vy > −16`; na barra (11) PULAR+BAIXO com o alvo logo abaixo, lado à altura, senão BAIXO;
  escalando (12) lado e CIMA/BAIXO; em golpe (0x10) só ataque; senão ataque, senão
  aproximação. Sem ver (ou sem aproximação): nos 2 pensamentos após perder de vista, 50 % de
  pose (0xf); depois o caminho; sem caminho espera 10 tiques (1/50 pose, 1/1000 modo 3). Por
  fim os perigos.
- **Modo 2 recuar:** corre para o lado oposto; parede a 20 px ou buraco sem pouso → modo 1;
  sem ver → modo 1. **Modo 3 vagar:** dano e alvo → modo 2; alvo não escondido → modo 0
  (= caçar) e repete a entrada; senão passos de caminho 3/4 sorteados com FURTIVO, 1/20 de
  pose; na barra ou escada, PULAR.
- **Ataque** (`0x42e659`): `m` 7/12, `p` 30/50 (Jake/Garathe). `< m` nada; até
  `(p−m)/2+m` repete a entrada se tinha ATIRAR/SOCO; até `p` SOCO (o corpo escolhe punhos,
  clava ou garras), salvo alvo fugindo (sinal de dx = sinal de vx, exceto empurrando); até
  200: Garathe correndo, d > 90, mesmo sentido, ±22,5° e alvo correndo → pulo longo
  (lado+PULAR+CIMA); senão ATIRAR com a arma atual se Jake ≠ 1 ou Garathe ≥ 3. Nunca nos
  estados da lista da RE (quedas, pousos, dano, morte, camadas, especial).
- **Mira** (`0x42cca4`): perto, setores largos (frente e cima ±0x800, diagonal de baixo até
  0x4000); longe, só a ±0x200 de 0, 0x2000, 0x6000, 0x8000, 0xa000, 0xc000, 0xe000.
- **Aproximação** (`0x42dd8f`) contra alvo de classe 1, 3–5, 7–9, 11–13, 15, 16; com o alvo
  pulando e `|dy| < 64`, dy = 0. Mesma altura (`|dy| < 20`): perto (< 30/50) segue o vx do
  alvo que anda, ou recua com FURTIVO (parede a 20 px → inverte); longe e `|dx| > 10` corre
  para ele. Alvo no ângulo 0xbc00–0xc400 → PULAR+CIMA. Escalando: lado e CIMA/BAIXO.
- **Sonda de buraco** (`0x42cf70`, só com entrada = um lado, no chão): faixa de 3 px sob os
  pés a `2·largura` à frente com chão → segue; senão pouso (faixa de meia largura) a
  110/150/255 px (Jake) ou 156/350/225 (Garathe) → +PULAR (agachado: +CIMA); sem pouso →
  PULAR+CIMA no lugar.
- **Perigos** (`0x42e1a9`), o mais próximo a < 100 px: projéteis (19–21, 25, 27, 29, 99, 100,
  106, 108) vindo → BAIXO (se não rasteja); 26, 42, 68–72 → vindo, pula por cima, senão pula
  para um lado ao acaso; pistão 39 < 95 mantém PULAR/ATIRAR/SOCO, < 75 e embaixo → BAIXO;
  40/41 < 50 e indo para ele → +PULAR+CIMA; 11/89/109 < 50 → ATIRAR virado (+BAIXO se acima).
  A célula de código `(c/10)%100` ∈ {29, 30, 36, 37, 39} com `+0x40 & 1`: andando para a
  borda → BAIXO.
- **Caminho** (`0x42f88c`/`0x42f9a4`): grade com os bits de `célula+0x04` (`fields[0]`, v ≥
  1006): 1 subir (sem `0x40`, zona PR 4), 2 descer (sem `0x2`), 3/4 lados (sem `0x10`/`0x8`,
  destino sem porta de célula fechada), 5/6 camada (zona PR 1/2). **Ruling:** busca em
  largura a partir de cada direção, em ordem, com Manhattan ≤ 10 e profundidade ≤ 10: dá a
  mesma primeira direção do menor caminho que a busca em profundidade do original, sem o
  custo exponencial. Passo (`0x42d721`): célula-alvo e 10 tiques; centrar (15/25 px) antes
  de subir/descer; lado com +BAIXO se `oy < −50/−40` fora da barra e +CIMA escalando; a sonda
  de buraco; 5 CIMA; 6 BAIXO (PULAR escalando); fora da rota por mais de 1 célula recomeça.

## Pose (entrada 0xf)

Novo estado semântico `Taunt` no corpo comum (`0x426f05`, `0x42bcbc`): a decisão em pé com
entrada exatamente 0xf entra nele (Jake: estado 4, rótulo 18; Garathe: 76); com outra
entrada roda a decisão em pé; no fim do rótulo, parado. Vale também para o teclado (quatro
setas), como no original.

## Simplificações declaradas

- Robôs, sentinelas, minas, lasers e demais inimigos continuam mirando só o jogador humano;
  a IA não apanha itens nem usa saídas, interruptores e teletransportes.
- A investida do Garathe da IA fere só o humano (uma vez por pulo).
- O cálculo exato de "logo abaixo" na barra (`0x42e05d`): `|dx| < 32` e alvo abaixo.
- Sem sons próprios; o tiro e o corpo soam posicionais, como no jogador.
- Cada IA sorteia com gerador próprio (não mexe no dos demais).

## Testes

- Cérebro sintético: ritmo; modos; ataque por distância (Jake e Garathe); mira perto/longe
  sem o especial; sonda (pulo no lugar, pulo com pouso); perigos; caminho numa grade
  sintética com cada bit e zona; pose 0xf no corpo.
- Cena: um Minotauro da IA se aproxima do humano e o fere; a pistola do humano mata um Jake
  da IA (cadáver, "destruir" cumprido); os tiros da IA ferem o humano e não a ela.
- Dados reais: `H_J_006` (3 Minotauros), a IA se aproxima em 200 tiques sem entrar no
  cenário; fuzz de 2000 tiques em toda fase com IA (sem pânico, IA fora das células da camada
  dela e dentro da fase, entidades limitadas).
