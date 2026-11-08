# Abertura, demonstração, teclas, joystick e luz por célula

Complementa [`menu.md`](menu.md) (ordem da inicialização), [`menus-restantes.md`](menus-restantes.md)
(`SINPU`/`SKEYB`/`SJOYS`), [`opcoes-salvamento.md`](opcoes-salvamento.md) (`HUNTER.INI`) e
[`cores-sprites.md`](cores-sprites.md) (tabela de sombra). **[confirmado]** lido no código;
**[inferido]** dedução.

## Abertura `0x4399e0(modo)` [confirmado]

Modo 1: logotipo `0x439bdd(0)` → filme `hh*.avi` → título `0x439bdd(1)` → história
`0x43a18f(1)` → créditos `0x43a18f(0)`; cada etapa devolve 0 se interrompida e a primeira
interrupção encerra tudo. `0x493c0c` = "vista inteira". Modo 0: enquanto `0x493c0c`, repete a
sequência inteira até uma interrupção. Modo 2: só os créditos. A inicialização chama (1) e,
depois de carregar os recursos, (0); a inatividade do menu (estado 12345) para o som 10000 e
chama (1) e (0), e volta ao menu (2). Ou seja: **a sequência repete em laço até o jogador
interromper**. Interromper (`0x43a123`) = tecla (`0x499f34 ≠ 0`, salvo o bit 4 de
`0x499f38`), botão do mouse ou qualquer dos 8 botões de um dos 4 dispositivos `0x499020`.

### Logotipo e título (`0x439bdd`, tabela `0x47d6cc`, 24 bytes: rótulo, paleta, ms)

| Modo | Rótulo de fundo | Paleta | Duração | Sons |
|---|---|---|---|---|
| 0 | 1 "background" (`Sierra.BMP`) | `sierra.pal` | 30 000 ms (teto) | 3403 |
| 1 | 4 "Title Screen" (`stitle.bmp`) | `stitle.pal` | 1 500 ms | 3081, 3045, 3220, 3086 juntos |

- Desenha o primeiro quadro do rótulo em (0, 0) de uma área 640×480 centrada na janela; se o
  rótulo tem mais quadros, eles se sucedem em laço a partir do 2º, um por volta do laço.
- Modo 0: o rótulo 3 "tp" (193 quadros lógicos do `SIERRA.ANM`) só começa depois que o som
  3403 parou (`0x44e130(3403) ≤ 0`) e avança **um quadro por volta**; no último, o fim passa a
  ser agora + 500 ms. [inferido: uma volta por quadro de tela, 50 ms no porte]
- Sem interrupção: modo 0 limpa a tela, espera 1 000 ms e toca a voz 4100 esperando o fim;
  modo 1 toca a voz 4101 (o título ainda na tela) esperando o fim e espera 500 ms.

### História e créditos (`0x43a18f(modo)`) [confirmado]

| | História (1) | Créditos (0) |
|---|---|---|
| Animação, texto | `SIERRA.ANM`, `STORY.RES` | `SCREDIT.ANM`, `CREDITS.RES` |
| Fundo / animação | rótulos 2 / 6 | rótulos 1 / 2 |
| Coluna | alinhada à direita, `x = 640 − largura − 12` | centrada |
| Entre linhas | altura da fonte + 3 | altura da fonte + 10 |
| Velocidade | 30 px/s | 120 px/s |
| Animação | começa aos 3 000 ms, 50 ms por quadro; ao fim do ciclo, pausa 5 000 + `rand % 1000` ms | desde o início, 200 ms por quadro, em laço |

Fonte `credits.pft`, **cor 255**; textos na base 13001 (linha *n* = id 13000 + *n*). As linhas
começam em y = 480 e sobem `(t − início) · velocidade / 1000` px; acaba quando a subida passa
da altura total (a última linha saiu pelo topo). A música dos menus segue tocando (`0x402d48`
no laço, modo de janela 1).

## Demonstração e `PLAY.VCR` [confirmado]

A "demonstração" do menu ocioso é a **abertura acima** (estado 12345), não um replay de jogo.
O `PLAY.VCR` (`RESOURCE.001`, 514 995 bytes) **não é lido pelo executável**: nenhuma string
"VCR:", ".VCR" nem "PLAY.VCR" existe no `Hunter.exe`; só a ajuda de depuração
`0x4719f4` cita "F6:VCR", sem tratador achado. É uma gravação de **comandos de desenho**
(não de entrada) de ~634 quadros de uma missão do Minotauro; formato em
[`../formats/vcr.md`](../formats/vcr.md).

Ociosidade (`0x43f5b0`): o relógio `0x4974a4` volta a zero ao abrir o menu, ao carregar um
jogo, quando o ponteiro mexe (`0x499f24/28`) ou quando os eixos de algum dispositivo mudam
(`+0x94/+0x96` contra `+0xa8/+0xaa`; o teclado também tem eixos); 30 000 ms → 12345.

## Dispositivos e `HUNTER.INI` [confirmado]

- `Player1Input`, `Player2Input` (o nome é `0x47f4bc` com o 7º byte trocado por '1'/'2'):
  `Keyboard1` (1), `Keyboard2` (2), `Joystick1`…`Joystick4` (3–6), tabela `0x47f4c0`.
  Padrão: jogador 1 `Keyboard1`; jogador 2 `Joystick1` se existe e o 1 não o usa, senão
  `Keyboard2`; iguais → o 2 fica com `Keyboard2` (`0x43edc1`). No jogador 1 `Keyboard2`
  vira `Keyboard1` e vice-versa no 2. Joystick ausente (`joyGetPos(i) ≠ 0`) → o teclado.
- `InputKeys1/2` = 10 códigos VK `%d,…` na ordem dos botões 17001–17010: cima, baixo,
  esquerda, direita, atirar, pular, furtivo, arma anterior, soco, próxima arma. Padrões
  (`0x480020`, `0x480070`): jogador 1 **38, 40, 37, 39, 17 (Ctrl), 16 (Shift), 88 (X),
  33 (PgUp), 90 (Z), 34 (PgDn)**; jogador 2 73 (I), 77 (M), 74 (J), 75 (K), 32 (espaço),
  13 (Enter), 186 (;), 45 (Ins), 191 (/), 46 (Del). Ausente → padrão dos dois (`0x44225a`).
- `Joystick1/2` = 6 números (botão 1-base) para atirar, pular, furtivo, arma anterior, soco,
  próxima; padrão 1…6. Valor maior que o número de botões fica negativo (desligado).

### Bits de entrada (`0x407e2d`) [confirmado]

Cada dispositivo (`0x499014[dev]`) tem eixos `+0x94` (x) e `+0x96` (y) de −128…127 e seis
botões `+0x98…+0x9d` = atirar, pular, furtivo, **próxima**, **anterior**, soco.

- Teclado (`0x448503`): eixo = 128·(tecla do lado + menos a do lado −); botões pelas teclas
  (ordem de cópia `0x43eed2`: atirar, pular, furtivo, `keys[9]` próxima, `keys[7]` anterior,
  `keys[8]` soco).
- Joystick (`0x4483c4`, `joyGetPos`): eixo = `(bruto − 32 767) >> 8`; botão `i` = bit
  `map[i] − 1` do estado, ou o bit `i` se o mapa está fora (`map[i] − 1 ≥ botões`).
- Direções: `x < −50` ESQUERDA, `x > 50` DIREITA, `y < −50` CIMA, `y > 50` BAIXO. Com
  ESQUERDA/DIREITA e `|x| ≤ 100` (ou CIMA/BAIXO e `|y| ≤ 100`) liga **furtivo** (0x400): a
  meia inclinação anda.
- Botões → 0x100, 0x200, 0x400, 0x1000, 0x800, 0x40000.
- Com **menos de 4 botões**: pular + BAIXO + DIREITA = próxima arma, + BAIXO + ESQUERDA =
  anterior; liga uma espera de 20 tiques (`0x487a30[jogador]`) durante a qual o pular fica
  desligado.
- Bordas: 0x10/0x20/0x40/0x80 e 0x2000…0x80000 = mudanças contra o tique anterior.

## `SINPU` (`0x43ea1f`) [confirmado]

29001 teclado e 29002–29005 Joystick1–4 do jogador 1; 29006 e 29007–29010 do jogador 2;
clique → `0x43e7af(jogador, dev)`: marca o grupo (`0x443257`), grava `PlayerNInput`; se o
outro jogador tinha o mesmo dispositivo, ele recebe o antigo. Joysticks ausentes ficam cinza
(`0x44322c`). CUSTOMIZE: teclado → `SKEYB` (estados 9 / 12), joystick → `SJOYS` (11 / 14).
DONE e Esc → estado de onde veio (`0x4974a0`).

## `SKEYB` (`0x4419e0`, procedimento `0x441bfb`) [confirmado]

- Entrada: jogador (1 → `0x480048`, 2 → `0x480098`); tabela de cor do retrato 17023 =
  `0x497320[jogador]`; placa 17026 = rótulo 47/48; rótulos do retrato por ação: Jake
  `0x4800c4` = 12, 14, 13, 15, 16, 17, 18, 19, 21, 19 e 35 parado; Garathe `0x4800f0` = 22…29,
  31, 29 e 36. Textos `SHELL.RES` (base 2001), `KEYS.RES` (base 3001: nome da tecla VK *v* =
  linha *v*), fonte `slevd.pft`; os nomes nas áreas 17011–17020 com cor 209 (`0x442417`).
  Ponteiro em DONE; descrição 17021 = 2001.
- Clique numa ação (`0x442291`): passa a **esperar a tecla** (`0x4800c0`): RESET (17025) mostra
  o rótulo 34 "reset" (fora da espera, 38 "reset all"), o retrato faz a ação, a descrição é a
  da ação (2002–2011; **o original troca soco e próxima arma**: 17009 → 2011, 17010 → 2010) e
  o nome pisca (troca a cada 6 quadros, `0x480120`). Na espera a navegação por teclado e o
  realce ficam desligados.
- Na espera (`0x441a5c`, a cada quadro): Esc ou botão do mouse → sai sem mudar. Outra tecla
  VK → **troca**: em cada um dos dois jogadores, a ação que já usava a tecla recebe a antiga
  desta ação (`0x442537`); grava e escreve o nome; descrição volta a 2001.
- RESET: na espera, só esta ação volta ao padrão (com a troca); fora, os dois jogadores
  voltam ao padrão. DONE e Esc gravam `InputKeys1` e `InputKeys2` e voltam ao `SINPU` (19).

`SJOYS` (`0x442638`) é igual sobre os 6 botões 17005–17010 (17028 cobre as direções):
espera um botão do joystick (o menor bit ligado → "Button *n*", `JOYSTICK.RES` 2009 + *n*),
qualquer tecla ou clique cancela; com ≤ 2 botões esconde as armas (17008/17018, 17010/17020);
DONE grava `Joystick1`/`Joystick2`. Placa 39–46 pelo joystick e jogador (`0x480290`).

## Luz por célula [confirmado]

As entradas 1–2 da lista de gatilhos da célula (`+0xa0`; as 3–4 são da porta,
`itens-campanha.md`) decidem se ela está **acesa**: `0x433a81` = `0x435bd8(+0xa0, 0, 2)`
(sem entradas = verdadeiro). Estado da célula: bit 0 de `+0x38` (visível) e sombra `+0x58`.

- Carga (`0x4314e7`): acesa → visível, sombra 0; apagada → invisível, sombra 8.
- Tique da célula (`0x432d39`, para as células na vista `0x432a7a`): acesa → se invisível,
  sombra 8 e visível; depois sombra − 1 até 0. Apagada → sombra + 1 até 8; com 8, invisível.
- Desenho: célula invisível = vazia (`0x4060f1`; aparece a camada de trás); visível com
  sombra *s* > 0 → cenário (`0x40615a`), decalques (`0x430546`) e objetos com a tabela de
  sombra **18 + s** do `DLS.TBL` (`0x43be54(−s)`). Objeto (`0x435745`): célula do centro da
  caixa (`/128`, camada `+0x17c`); célula invisível → **não é desenhado**. O bit 0 de
  `0x49ccb1` (depuração) desliga a luz.
- 49 das 65 missões solo têm células assim (código 11 = "o alvo da entrada 1 ligado", em
  geral uma zona de proximidade): salas que aparecem quando o jogador entra.

### Camadas no desenho (`0x405c7c(rect, camada, prof.)`) [confirmado em parte]

A vista desenha a camada dela; nos buracos, a camada − 1 com profundidade + 1 (salvo
`0x4879e0 = 2`), escurecida pela sombra 26 + prof. (prof. ≥ 7 → 33). As camadas > a da vista
não são desenhadas. O OpenHunted ainda desenha todas as camadas sem escurecer (ver Em aberto).

## Em aberto

- A pintura completa por camadas (`0x405c7c`, passes de decalques e objetos com profundidade
  > 0) e a transição da vista (`+0x44/+0x48`): 60 das 65 missões solo têm mais de uma camada.
- Significado de vários comandos do `PLAY.VCR` (`../formats/vcr.md`).
- O texto "Your joystick does not have enough buttons." (2008): quando aparece.
- O que o bit 4 de `0x499f38` é (não conta como interrupção).
