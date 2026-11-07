# OpenHunted M14 — acabamento da casca e entrada: design

Data: 2026-11-07. Base: `docs/re/casca-acabamento.md`, `docs/formats/vcr.md`, `menu.md`,
`menus-restantes.md`, `opcoes-salvamento.md`, `cores-sprites.md`; spec da M9.

## Objetivo

Tirar da lista "Ainda não há" o logotipo, o título, a história, o laço de demonstração, a
troca de teclas, o joystick e a luz por região, como no original.

## Abertura e demonstração

- Sequência: logotipo (`SIERRA.ANM` rótulo 1, `SIERRA.PAL`; o "tp", rótulo 3, um quadro por
  tique depois do som 3403; acaba 500 ms depois do último quadro, teto 30 s) → preto 1 000 ms
  → voz 4100 → filme `hh*.avi` → título (rótulo 4, `STITLE.PAL`, sons 3081, 3045, 3220, 3086;
  1 500 ms; voz 4101; 500 ms) → história → créditos. Regras de cada uma em
  `casca-acabamento.md`. Ruling: "o som 3403 parou" = a duração do WAV passou (o mixer não
  responde se um som toca).
- História e créditos (`Scroller`): coluna, entrelinha, velocidade, cor 255 e animação pela
  tabela do RE. A sequência do fim da campanha usa os mesmos créditos (antes 40 px/s).
- **Laço**: ao abrir o jogo (sem `--no-intro` nem `--level`) e depois de 30 000 ms de menu
  ocioso, a sequência repete até uma tecla, um clique ou um botão de joystick, que a encerra
  inteira e leva ao menu. Ruling: o relógio da ociosidade zera com o ponteiro, com qualquer
  tecla e com os eixos do joystick (o original: ponteiro e eixos). Os filmes da 7ª/15ª peça
  continuam pulando um passo por tecla (M9).
- `PLAY.VCR`: Ruling: o original não o lê e a demonstração dele é a abertura; o port lê e
  regrava o arquivo (`openhunted_formats::vcr`, `ohtool info PLAY.VCR`) mas não o mostra.
  Não há entrada a reproduzir, então não há "sincronia" a medir; o teste de determinismo é
  o da cena (mesma entrada → mesmo rastro).

## Teclas (`SKEYB`) e configuração

- `hunter.ini` ganha `InputKeys1/2` (10 VK), `Joystick1/2` (6 botões) e
  `Player1Input/Player2Input` (`Keyboard1`, `Keyboard2`, `Joystick1–4`), com os padrões do
  original; valores ilegíveis → padrão. **O pular padrão do jogador 1 é Shift** (como no
  original; antes do port, espaço).
- `SKEYB`: clique numa ação espera a tecla (nome piscando a cada 6 tiques, RESET com o
  rótulo 34, retrato fazendo a ação, descrição da ação); Esc ou clique cancela (Ruling:
  clique no RESET volta só a ação, como o rótulo "reset" sugere); outra tecla
  troca com a ação que a usava (nos dois jogadores). RESET: na espera, a ação; fora, tudo.
  DONE/Esc gravam e voltam ao `SINPU`. Ruling: a descrição de soco e de próxima arma é a
  certa (o original as troca).
- Ruling: a tecla é o código VK do Windows; o port traduz o scancode da SDL (letras,
  dígitos, F1–F12, setas, navegação, Shift/Ctrl/Alt de qualquer lado = 16/17/18, teclado
  numérico, pontuação OEM 186–192 e 219–222). Teclas sem VK não se atribuem. P/Pause, Tab,
  Esc e 1–6 seguem como extensões fixas.
- Na partida, o jogador 1 lê o dispositivo dele: bits de entrada pela regra de `0x407e2d`
  (eixos ±128 do teclado, limiar 50, meia inclinação anda, combinação de pulo com < 4
  botões). As batidas rápidas do pulo e do CIMA (`tap`) seguem as teclas atribuídas.

## Dispositivos (`SINPU`) e joystick

- `SINPU`: os grupos dos dois jogadores marcam o dispositivo gravado; Joystick *n* sem o
  *n*-ésimo joystick não se escolhe nem se navega (Ruling: sem o desenho cinza, bit 4 do
  controle, `0x44322c`); mesmo dispositivo nos dois → troca; CUSTOMIZE abre `SKEYB` ou
  `SJOYS` do jogador. Ruling: o jogador 2 só grava (não há dois jogadores).
- Ruling: joystick pela API de joystick da SDL3, na ordem de conexão (até 4): eixos 0/1
  `>> 8` (−128…127, como `joyGetPos`), o primeiro chapéu como direção cheia, botão *n* =
  índice SDL *n* − 1. `SJOYS`: espera um botão (o menor apertado); tecla ou clique cancela;
  com ≤ 2 botões esconde as armas; DONE grava `Joystick1/2`.

## Luz por célula

- Motor (`scene/light.rs`): estado (visível, sombra 0–8) só das células com entradas 1–2
  armadas; carga e tique como `0x4314e7`/`0x432d39`. Ruling: todas essas células andam a
  cada tique (o original só as da vista).
- Desenho: cenário e decalques de célula invisível não são desenhados; com sombra *s*, a
  tabela de sombra 18 + *s* do `DLS.TBL` (composta com a cor do objeto, `Dls::table`).
  Objetos (e o jogador) pelo centro da caixa na camada deles: invisível → não desenhados.
- Ruling: as camadas seguem todas desenhadas (a pintura por camadas, `0x405c7c`, fica em
  aberto).

## Testes

Fluxos das telas com dados reais (abertura completa e interrompida, laço, ociosidade,
`SINPU`/`SKEYB`/`SJOYS`); `Vcr` relido igual byte a byte; determinismo da cena; teclas
gravadas e relidas, troca nos conflitos, RESET; bits de joystick com um dispositivo falso
(limiar, andar, combinação, mapa); luz numa fase real (`H_J_006`: sala apagada, acende, sombra
e objetos escondidos).
