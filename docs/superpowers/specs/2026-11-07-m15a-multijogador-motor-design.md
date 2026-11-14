# OpenHunted M15a — núcleo de dois jogadores no motor: design

Data: 2026-11-07. Base: `docs/re/multijogador.md`, `logica-fase.md`, `fim-de-missao.md`,
`garathe.md`, `minotauro-ia.md`, `cores-sprites.md`, `itens.md`.

## Objetivo

A cena aceita 1 ou 2 jogadores humanos, nos modos solo, cooperativo (`p=4`) e disputa
(`p=3`), com as regras do original: vagas, presença por modo, crédito do golpe fatal, F/S,
renascimento, itens que renascem, fins de partida e cores. Só o motor: tela dividida,
entrada dupla e telas (`SEGOC`, `SHVLD`, `SH2HS`) ficam para a M15b. O solo não muda: os
rastros `ohtool render --trace` das 100 fases com a mesma entrada aleatória são idênticos
antes e depois (o `--trace` ganha um resumo da cena: hash das entidades e dos eventos).

## API

- `Scene::build_with(level, assets, Mode)`; `Scene::build` = `build_with(…, Mode::Solo)`.
  `Mode::{Solo, Coop, Deathmatch(Duel)}`, `Duel { chars: [Character; 2], colours: [u8; 2],
  points_limit, time_limit_ms }` (pontos e milissegundos já convertidos de `0x497378`,
  `0x497374`; 0 = sem limite).
- `tick_players([u32; 2])`; `tick(input)` = `tick_players([input, 0])`.
- **Ruling: o relógio real do limite de tempo vem de quem chama** (`Scene::elapsed_ms`,
  posto antes de cada tique; o motor não lê relógio). A pausa da casca decide se congela.
- `players: Vec<Player>` (número `+0x19c`, personagem, cor, `Stats { kills, treasures,
  secrets, frags (F), suicides (S) }`); `player_jake(i)`, `focus_of(i)`, `layer_of(i)`,
  `nearest_player(ponto)` (ouvinte do som: a vista mais próxima), `winner()`.
- `scene.jake`/`jake_sprite`/`jake_layer` continuam sendo o jogador 1 (casca, testes).

## Arquitetura

- **Ruling: assento ativo.** O estado por jogador que a cena já tinha (corpo, sprite,
  camada, sinalizadores, mundos, entrada anterior, confusão, terremoto, investida) forma um
  `Seat`. O do jogador 1 fica nos campos de sempre; o do 2, em `parked`. As fases por
  jogador (mover, especial, soco, tiros, saída/interruptor, aplicar as saídas das entidades)
  rodam com `with_seat(i, …)`, que troca os campos. Com um jogador nada é trocado: o
  caminho do solo é o mesmo código, na mesma ordem.
- Entidades: cada uma age contra **um** jogador por tique (o alvo); `Ctx` é montado com o
  assento dele e as saídas próprias dele (dano, cura, veneno, inventário, teletransporte,
  terremoto, confusão) vão a um `Out` por jogador; o resto se junta.
  - **Quem fere ou conta todo objeto que toca vê os dois** (`Ctx::players`/`active_in`; o
    dano a um jogador que não é o alvo vai por `Out::seat_damage` à saída dele): fogo
    (`0x4104a3` chama `0x43d629` para todo objeto presente na caixa), cerca (`0x41494b`: toda
    vítima na faísca; um acerto basta para uma recarga e um som), raio laser (o que a caixa
    varrida cruza), pistão, zona (`0x40e14e` percorre os objetos) e porta (encostado ou
    dentro).
  - **Quem lê um contato só vai contra o último jogador que o toca**: itens, peça do carro e
    estrela cravada (`0x40b3f5` lê `0x4076d0` = o `+0x21c` do item, posto por `0x437350` com
    quem se moveu contra ele; o primeiro contato do tique zera os campos, os seguintes
    sobrescrevem: vale o último a se mover), teletransporte e mina. Os assentos se movem em
    ordem (1, depois 2): com os dois na caixa, o 2 pega o item, mesmo mais longe; se o 2 não
    pode pegá-lo, ninguém pega nesse tique. [inferido: o original move os objetos na ordem
    da lista, e as vagas podem estar em qualquer ordem nela.]
  - **Ruling: os demais (minas e olhos atrás do alvo, robôs, gerador, dardos, Camaleão,
    Entidade…), e os de contato sem ninguém tocando, vão contra o jogador ativo mais perto
    da caixa da entidade (distância 0 dentro), preferindo a mesma camada; empate → o de
    menor número** (o original escolhe por tipo).
  - A passada à queima-roupa (balas e projéteis que acertam antes de voar) devolve tudo o
    que produz: a explosão do foguete, da granada e da estrela, com crédito e efeitos, entra
    na lista das explosões do tique (antes era perdida — também no solo, com os foguetes da
    IA).
- Personagens da IA (`0x42c5cc`): candidatos os dois primeiros jogadores vivos (no
  original, os dois primeiros objetos com `+0x19c`, `0x80003 → 0x80001`, presentes e sem
  `+0x178 & 0x8000`; **Ruling: na ordem dos assentos**); o mais perto pela distância do
  centro da caixa da IA à **origem** do candidato (empate → o segundo) fica se está na mesma
  célula ou a busca o alcança, senão o outro nas mesmas condições, **senão nenhum**. O
  escolhido fica enquanto vive (`Brain::target_of`). Golpes, investida e tiros da IA ferem qualquer
  humano.
- Entre humanos (fogo amigo também no coop, `multijogador.md`): balas e projéteis do outro,
  explosões (inclusive as próprias), soco, investida e especial (derruba, como na IA).

## Modos e vagas (`0x43c95c`, `0x43cbb0`, `0x43cb28`)

- Máscara: solo 2; disputa 4 (Jake×Jake), 8 (Jake×Garathe), 0x10 (Garathe×Garathe); coop
  0xe0. Fica só o que está no modo (`objects::present`) e as vagas atribuídas.
- Candidatos do jogador `i`: bit 0 e `modos & m ≠ 0` (solo 2, disputa 0x1c, coop 0xe0), tipo
  = personagem escolhido (coop: qualquer), menos os tomados. Disputa: sorteio pelo gerador da
  cena, `(rand · n) >> 16` (`0x43ccdc`, `rand` de 16 bits). Coop: `n` vira 1 — o primeiro da
  lista montada = **o último na ordem dos objetos** —, mas o `rand` é tirado do mesmo jeito.
  Sem candidato: erro "Unable to find a human object to assign to player N!".
- Solo: como antes (vaga solo; sem ela, o primeiro objeto tipo 1).

## Crédito e pontos

- `credits[entidade]` (`+0x1b0`): os tiros nascem com o número do dono; quem leva dano de
  um tiro, explosão, soco, especial ou investida herda o crédito do atacante (vivo). A
  explosão herda o de quem a criou (granada, foguete, barril). Todo golpe passa o crédito
  do atacante (`0x43d708`–`0x43d71b`), também os de quem não tem crédito: fogo, cerca, raio
  laser e tiros inimigos põem o deles (0) na vítima viva (barril, personagem da IA,
  objetos) — o barril que um jogador marcou e o fogo queimou deixa de ser dele. Dos objetos
  que uma entidade com crédito cria, só os projéteis (bala, chumbo, foguete, granada,
  estrela: o chumbo que se divide) herdam o crédito e contam como tiros do jogador; o par
  da cerca, os raios do olho e afins não.
- Jogador: o crédito é o do último dano posto na fila com ele vivo; inimigos põem 0 (com dois
  jogadores, 0 também sobrescreve). Vivo depois do próprio tique, zera (só o golpe fatal
  conta: a fila é aplicada no tique seguinte).
- Mortos por jogador: os mortos contados (`enemy::killed`) com o crédito dele; no solo,
  todos (como antes). **Ruling: o corpo clonado não entra em `kills`** (o `[inferido]` do
  RE fica para a M15b com o `SH2HS`). Tesouros e segredos: 0, como no solo (não contados).
- Fim da morte na disputa (`Dying → Dead`): `c` = 0 nada; `c` = a vítima → `S = min(S+1,
  F)`; outro → `F_c += 1`.

## Disputa

- Campo 1004 ignorado. Pontos de renascimento: células com `fields[14] & 4`, até 16, na
  ordem das células, `(128·cx + 64, 128·cy + 32, camada)`; sem nenhum, o início.
- Fim da morte: cadáver clonado (`Behaviour::Corpse`, sprite no último quadro, cor do
  jogador, sem colisão nem dano) e renascer: **Ruling: na hora, de pé (estado parado;
  animação de 181/80 em aberto)**. Posição: `y −= 20` (`0x426a84`), depois, com pontos,
  `0x43d2b5` sorteia `r / (0x10000 / n)` e põe a **origem** do corpo no ponto (`0x43d300`:
  `+0x14c/+0x150`, e a camada); sem pontos, fica onde morreu, 20 px acima. O original lê além
  da lista quando `r` cai no resto da divisão; aqui, o último ponto. `0x426b09` zera armas,
  munição, chaves, bandeiras, veneno (`+0x270`), terremoto (`+0x274`), dano pendente e
  piscar; a confusão (`+0x264`) não está na lista e fica; melhor arma; vida 100; armadura
  fica.
- Qualquer personagem pega qualquer item. Itens apanhados (coleta genérica, salvo `0x10`,
  chaves, bandeiras e peça do carro) renascem: "Regenerate" (tipo 43, `ORG.ANM` rótulo 1,
  do tamanho do item) espera 300 tiques escondido, anima um quadro por tique e no quadro 8
  devolve o item; some no fim.
- Saída: interruptor sem objetivos. Fim (`0x43cdf0`, 40 tiques seguidos): o laço dos
  objetos dá "2" com os dois mortos (antes de tudo), senão "1" com os objetivos ("acionar"
  ligado, "destruir" morto) cumpridos, "0" sem eles. Limite de pontos ≠ 0 → só `F − S ≥
  limite` decide ("2" ou "0"); limite de tempo ≠ 0 → `elapsed_ms > limite` dá "2". Com "2",
  fim. Senão (`0x43cf36`): a última saída da lista desligada → "0"; ligada (ou sem saída),
  sem limite de pontos, "0" vira "2" (`0x43cf68`): **ligar a saída com objetivos pendentes
  é derrota** (`M_2_012`, `M_2_015` "acionar"; `M_M_033`, `M_M_045` "destruir"); com eles
  cumpridos, vitória. Sem saída na fase e sem objetivos pendentes, "1" (as `M_*` todas têm
  saída). `winner()`: maior `F − S` (empate → nenhum).

## Cooperativo

Morto vira cadáver (como no solo, sem renascer); derrota quando todos morreram ou o tempo
esgota; um na saída com os objetivos cumpridos (vivo quem entra) vence pelos dois. Itens não
renascem; coleta pelo bit do personagem.

## Luz (M14)

**Ruling: a luz por célula é uma só para as duas vistas** (o estado sai dos gatilhos, não do
jogador); cada jogador é desenhado com a sombra da célula dele (`look_at` no centro dele).

## Cores

Disputa: `Jake::with_colour(c)` põe a cor-base (`+0x280`) usada no desenho e no piscar (troca
35↔37, 36↔38). Solo e coop: 0 (35/36), como antes.

## Testes

Sintéticos: vagas por modo e par (e os sorteios); crédito por bala, chumbo, foguete,
granada, explosão e soco, e o 0 de fogo e raio; só projéteis herdam o crédito de quem os
cria; F/S; renascimento (cadáver, ponto, sem ponto, inventário, armadura, terremoto,
confusão); regeneração só na disputa; fins por pontos, tempo e saída (objetivos pendentes →
derrota; mortos antes da saída); derrota e vitória do coop; inimigo mira o mais perto; os
dois no fogo, na cerca e no raio; item tocado pelos dois vai ao último; zona conta qualquer
um; foguete à queima-roupa explode com dano de área; IA sem humano alcançável fica sem alvo.
Dados reais: toda `M_*` nos três pares com 2 jogadores controláveis; toda `C_*` no coop;
disputa roteirizada até o limite de pontos; saída ligada nas `M_*` com e sem objetivos; fuzz `#[ignore]` (3000 tiques, entradas
aleatórias nos dois): sem pânico, jogadores fora do cenário, entidades limitadas.
