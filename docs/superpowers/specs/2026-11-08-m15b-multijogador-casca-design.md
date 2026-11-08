# OpenHunted M15b — dois jogadores na casca: design

Data: 2026-11-08. Base: `docs/re/multijogador.md`, `menus-restantes.md`, `casca-acabamento.md`,
`hud.md`, `fim-de-missao.md` e a M15a (`2026-11-07-m15a-multijogador-motor-design.md`).

## Objetivo

Os modos de dois jogadores jogáveis de ponta a ponta numa máquina: `SMULT` → lista do modo →
(disputa) `SEGOC` × 2 → `SHVLD` / (coop) briefing → partida em tela dividida com duas
entradas → `SH2HS` → NEXT/REPLAY/MAIN. O solo não muda.

## Fluxo (`0x43f0bb`, `0x4413aa`, `0x443386`, `0x445501`, `0x443c9c`)

- `SMULT`: 13001 disputa, 13002 coop → `SLEVL` com a lista do modo (`0x5601`: `p=3`;
  `0x5602`: `p=4`, na ordem do `ENGLEVEL.TXT`); 13004/Esc → menu. Na `SLEVL`, 22016–22018
  trocam de lista. **Ruling: a ordem é a do RE (lista antes da `SEGOC`; a `SHVLD` mostra o
  nome da fase), não "SEGOC → SHVLD → lista".**
- Coop: LOAD → briefing → partida. Disputa: LOAD → `SEGOC` do jogador 1, OK → jogador 2,
  OK → carrega a fase (`Mode::Deathmatch`) e abre `SHVLD` sobre o 1º quadro, GO → partida.
- `SEGOC` (`0x443386`): padrão J1 Jake/35, J2 Garathe/36 (`0x43d33b`; cor < 35 → 36, tipo
  fora de 1–2 → 1). Retratos 15010/15011 (rótulos 18/19, "fade"): o escolhido toca o fade e
  depois um rótulo de espera sorteado (`0x4803d4`, 13 do Jake; `0x480408`, 8 do Garathe); o
  outro toca o fade de trás para frente e para. Cada retrato guarda a sua cor
  (`0x48038c/0x480390`), desenhada pela tabela do `DLS.TBL`; trocar a cor apaga e reacende
  o retrato. 15009 mostra o dispositivo do jogador da vez (`0x480394[7·p + dev]`: J1 10/10/6/
  9/8/11, J2 12/12/22/23/24/25 para teclado 1/2, joystick 1–4) e abre `SINPU`, que volta à
  `SEGOC`. CANCEL/Esc: no 2º → o 1º; no 1º → `SLEVL`. Navegação nos 8 botões de `0x480428`.
- `SHVLD`: abre com os limites anteriores (`0x497374/78` × 20); setas ±20 em 0…1980 (tempo
  com piso 0, pontos com teto 1980); botão preso repete com esperas 5, 4, 3, 2, 1, 0
  (`0x4458a1`); contadores com os dígitos de rolo da `INVBAR01` (2 dígitos, unidades em
  `x + w − 26`, `y + 4`, −22 px, quadro `193 − 10d − r`), que rolam para o alvo em passos de
  `diff/5` (mín. 1); nome da fase em 32011 (cor 162, `x + 16`, `y + 9`). GO → minutos e
  pontos, partida. CANCEL/Esc → `0x47f53c`: **Ruling: a `SEGOC` do jogador 1, ou o `SH2HS`
  quando veio de REPLAY/NEXT.**
- `SH2HS`: cursor em NEXT; REPLAY → mesma fase (coop: briefing; disputa: `SHVLD`); NEXT →
  próxima da lista do modo; MAIN → menu; Esc → menu salvo vindo do jogo. **Ruling: NEXT é a
  entrada seguinte da lista do modo, voltando ao início, sem pular nenhuma** (o RE deixa em
  aberto o uso de `0x47fa8c`).
- **Ruling: partidas de dois jogadores não tocam a campanha solo** (recordes, peças, herói,
  jogo salvo; a troca de modo na `SLEVL` não chama `0x440fe0`); sem `S_CAR`, recorde,
  heróis nem gravação (`multijogador.md`).

## Entrada (`0x43edc1`, `0x407e2d`)

Dispositivos pelos `Player1Input/Player2Input` (`devices::resolve`). Teclado: as 10 teclas do
jogador (`InputKeys1` para o 1, `InputKeys2` para o 2), lidas do mesmo estado do teclado;
joystick: o mapa `Joystick1/2` do jogador. Um `InputReader` por jogador. As batidas rápidas
(pular, CIMA) valem para os dois teclados. 1–6 (arma direta) só no jogador 1 (extensão).

## Tela dividida (`0x4361c8`, `0x40a4ef`, `0x436386`)

- `views::layout(n)`: colunas 2 se `n > 2`, senão 1; linhas `(n+1)/2` ou `n`; barra 75 px,
  37 com dois ou mais. Dois: J1 jogo (0,0,640,203) e barra (0,203,640,37); J2 jogo
  (0,240,640,203) e barra (0,443,640,37).
- Escala: **Ruling: 1:1, como o solo do port** (referência 240 para 203 px dá a mesma
  ampliação que 480 para 405).
- Câmera por vista (`0x436386`): o alvo do solo (foco no centro, presa à fase) na vista
  640×203, andando no máximo 18 px por eixo e tique. **Ruling: um salto do foco > 64 px num
  tique (renascimento, teletransporte) recentra na hora; a carga também. O solo segue
  centrando direto, como antes.**
- Desenho: um ponto só, `draw_view(scene, assets, fb, camera, layer)` no `viewer.rs`, que hoje
  desenha com `Scene::draw` num quadro 640×203 e o copia; quando a `Scene::draw_view` da
  M16 chegar, só essa função muda.
- Barra achatada (`0x471cd0`, `0x45aee4`): **Ruling: a barra inteira desenhada em 640×75 e
  copiada uma linha sim, outra não** (difere do original em no máximo 1 linha de fase).
- Disputa (`0x40b089`): no lugar do cronômetro, quadro 194 em `(x + w − largura, y + 30/2)`
  e `max(0, F − S)` no estilo 195 em `+38, +5/2` (2 dígitos, glifos 20–29, unidades em
  `x + 20, y + 4`; o `hud.md` dizia 196). Coop: o cronômetro global nas duas barras.
- Ouvinte: cada som posicional é ouvido da vista cujo centro está mais perto dele, na
  camada do jogador dela; sons sem posição pela vista 1.

## Relógio, pausa e foco

`Viewer` soma os ms que de fato fazem o jogo andar e põe `scene.elapsed_ms` antes de cada
tique. **Ruling: pausa, dica aberta e janela sem foco param o relógio do limite de tempo.**
P/Pause e a perda de foco pausam a partida inteira; Esc com personagem não faz nada.

## Resultado `SH2HS` (`0x443e7a`)

Por jogador `k` (30004–30015 e 30017–30028): mortos/×30/% do total da fase, tesouros/×65/%,
segredos/×215/% (totais 0 → 100 %), `SCORE` = 30·m + 65·t + 215·s; só na disputa `KILLS` =
S, `KILLO` = F. Cor 231, centrados; contagem 5 %/quadro com 3093, tecla ou clique salta.
30031/30032 pelo resultado (`0x497328`). **Ruling: o cadáver clonado não conta em NKILLS**
(`0x43d140` não lido; mantém a M15a). Locutor: disputa pelo `F − S` 3790/3792/3773; coop
vitória 3780 + 3788, tempo esgotado 3776 + 3787, outra 3777 + 3787.

## Testes

Telas com dados reais: `SMULT` → `SLEVL` (listas `p=3`/`p=4`) → `SEGOC` (escolhas, CANCEL,
padrões, rótulo do dispositivo) → `SHVLD` (passos, limites, repetição) → partida →
`SH2HS` (campos por modo, botões); coop → briefing → partida → `SH2HS`. Geometria
(`layout`, retângulos das barras, quadro dividido com duas câmeras). Entrada: estado de
teclado falso com teclas dos dois jogadores e joystick falso no jogador 2. Limite de tempo
real encerra a partida; pausa congela o relógio.
