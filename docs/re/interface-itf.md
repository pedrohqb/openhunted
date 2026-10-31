# Interfaces `.ITF`: controles, desenho e entrada

Complementa o formato de arquivo em [`../formats/itf.md`](../formats/itf.md). Telas que usam:
[`menu.md`](menu.md), [`briefing.md`](briefing.md), [`fim-de-missao.md`](fim-de-missao.md).

## Carga (`0x00461310(nome, sinais)`) [confirmado]

- Cada ITF carregada recebe um **id de janela**: `0x483a04` começa em 5000 e soma 100 a cada
  ITF; o valor fica em `+0x14` de cada controle e é devolvido. 5000 significa "nenhuma tela
  aberta" (é a condição para a pausa por tecla, ver `pausa.md`).
- Sinal 1: cria uma janela (diálogo modal); sinal 4: desloca todos os controles para que o
  canto do retângulo envolvente fique em (0, 0). Telas cheias usam 4; diálogos usam 1 (que
  liga também o 4).
- Os controles entram na lista global `0x49a3c0`, ordenada por janela e id [inferido pela
  inserção em `0x44a778`]. O fundo tem sempre o id-base da tela (11000, 21500, 25000…), o
  menor: é desenhado primeiro.

### Estrutura do controle (memória) [confirmado]

| Campo | Conteúdo |
|---|---|
| `+0x0c` | id (nome no `TAG:`) |
| `+0x10` | tipo (0–6) |
| `+0x14` | id da janela |
| `+0x18` | campo A do arquivo = forma do cursor do mouse sobre o controle (0 = padrão) |
| `+0x1c`, `+0x20`, `+0x24`, `+0x28` | x, y, largura, altura (retângulo de toque) |
| `+0x2c`, `+0x30` | rotina de desenho e de tratamento (da tabela de tipos) |
| `+0x34` | sinalizadores (abaixo) |
| `+0x38` | "sujo" (redesenhar; `0x44ad7c` desenha os sujos) |
| `+0x40` | animação (`ANM`) ou 0 |
| `+0x44` | primeiro quadro do rótulo |
| `+0x48` | número de quadros do rótulo |
| `+0x4c` | quadro corrente (0…n−1) |
| `+0x50`, `+0x54` | origem x, y |
| `+0x5c`, `+0x60` | argumento e rotina própria de atualização (ver `0x461c00`) |

**Campo F** do arquivo: se 0, o rótulo **não** é resolvido e o controle não tem animação
(`+0x40 = 0`): é uma **área de texto** que o código preenche (títulos, números, descrições).
Se 1, o rótulo vira (primeiro quadro, contagem de quadros até o fim do rótulo). [confirmado]

### Tabela de tipos (`0x483a08`, 5 palavras por tipo) [confirmado]

`{inicialização, ?, desenho, tratamento, bytes extras}`:

| Tipo | Inicialização | Desenho | Tratamento | Uso nos dados |
|---|---|---|---|---|
| 0 | — | `0x461c7e` | `0x461c34` (animação em laço) | imagens, animações e áreas de texto |
| 1, 2, 5, 6 | `0x461ca0` (tipos 2 e 6 ligam o sinal 1) | `0x461b8a` | `0x461cb9` (botões) | só o 6 aparece |
| 3 | — | `0x461b8a` | `0x461f9c` | não aparece |
| 4 | `0x462060` (lê os 18 bytes extras) | `0x46229d` | `0x4621a2` | campo de edição (`SHIGH.ITF`, nome do recorde) |

### Sinalizadores `+0x34` [confirmado salvo indicação]

`0x44a9db(id, s)` liga `s` (menos os bits 2 e 4, que são internos).

| Bit | Efeito |
|---|---|
| 1 | o tratamento roda **a cada quadro**, mesmo sem o mouse em cima (botões e animações) |
| 2 | interno: marca a primeira ITF da janela |
| 8 | oculto: não desenha nem recebe entrada |
| 0x40 | apaga a área antes de redesenhar [inferido] |
| 0x80 | parada da navegação por teclado/joystick |

`0x44a89e(id, 0)` **remove** o controle (usado para esconder, por exemplo, `SMM_FAILURE` numa
vitória).

## Desenho [confirmado]

`0x461b8a`: se há animação e não está oculto, desenha o quadro `+0x44 + +0x4c` da `ANM` na
posição `(x + origem_x, y + origem_y)`; o quadro da `ANM` tem seu próprio deslocamento. Nos
dados, origem = `(−x, −y)` quase sempre: os quadros da `ANM` já vêm com a posição absoluta na
tela 640×480. O tipo 0 chama antes a rotina própria `+0x5c`, se houver.

Texto num controle (`0x44214c(controle, texto)`): recorta no retângulo, redesenha o controle
de fundo (id arredondado para a centena: 25005 → 25000) para apagar, e escreve o texto
**centrado** na horizontal e na vertical, com a fonte e a cor correntes (`0x45d32c(cor)`).

## Comportamento dos tipos [confirmado]

**Tipo 0**: sem rotina própria e com mais de um quadro, avança `+0x4c` um quadro por
atualização, em laço (fogo, faíscas, pistões). `0x43f46c` faz o mesmo e é instalada pelas
telas nos controles animados.

**Tipo 6 (botão)**. Quadro 0 = normal, quadro 1 = mouse em cima, quadros 1…n−1 = afundando.
Usa `0x49cca4` (controle "capturado") e `0x499f3c` (botões do mouse; bit 2 = acabou de
apertar):

1. Fora do retângulo: quadro 0; se estava capturado e o botão foi solto, solta a captura.
2. Dentro, não capturado: botão solto → quadro 1; botão acabou de apertar → captura.
3. Dentro e capturado: avança um quadro por atualização até o último; quando está no último
   e o botão foi solto, volta ao quadro 0, solta a captura e devolve **3** = ativado.

O clique só "pega" quando a animação de afundar chegou ao fim **e** o botão foi solto em
cima do controle.

## Laço e despacho [confirmado]

Por quadro, `0x44aa91` percorre a lista: o controle capturado primeiro; depois cada controle
visível com o mouse em cima e botão ativo, ou com o sinal 1; chama o tratamento. Resultado com
bit 2 → `0x460a14` manda à tela a mensagem **0x2400** com `wParam = id do controle`.

As telas são procedimentos no estilo Windows `(janela, msg, wParam, lParam)`:

| Mensagem | Quando |
|---|---|
| 0x110 | abertura (configurar controles, fontes, foco) |
| 0x121 | a cada quadro (ocioso) |
| 0x2400 | comando: `wParam` = id do controle, ou **2** = cancelar |
| 2 | fechamento (liberar fontes) |

**Tela cheia** (`0x43f876(itf, procedimento)`): carrega com sinal 4, manda 0x110 e entra no
laço: bombeia mensagens (`0x44a356`), atualiza os controles, manda 0x121, lê a entrada
(`0x402d48`); **Esc** (`0x499f34 == 0x1b`) manda 0x2400 com id 2. Sai quando o procedimento
grava o próximo estado em `0x4974b0` (sentinela −7777); manda 2, libera a ITF e devolve o
estado.

**Diálogo modal** (`0x4634d5(itf, procedimento, …)`): carrega com sinal 1 e **centra** o
retângulo envolvente na tela (`(640 − l) / 2`, `(480 − a) / 2`); o procedimento fecha com
`0x47052f(janela, valor)`, e o valor é devolvido.

### Teclado e joystick (`0x44b60a`, instalada com `0x44aa7e`) [confirmado]

`0x44b488` lê: setas (e eixos do joystick além de ±100) → direção; Enter, espaço ou botão 1
do joystick → **clique simulado** na posição do cursor (liga `0x499f3c`); botão 2 do joystick
→ cancelar. Com uma direção, `0x44b1c0` procura o controle com sinal 0x80 mais próximo naquela
direção e `0x44b42a(id)` **leva o cursor do mouse** ao centro dele. Cancelar manda 0x2400
com id 2. Ou seja: o teclado move o ponteiro entre os botões e Enter clica.

## Caixa genérica (`0x00445dc8(texto, botão1, botão2, ?)`) [inferido em parte]

Montada em tempo de execução, sem ITF: moldura `SGENERIC.BMP`, texto em `slevd.pft`, tamanho
calculado pelo texto (como a dica do mago, `dicas.md`), centrada; o prefixo `~x`/`~l` do texto
controla a formatação; um ou dois botões com os textos dados. Devolve ≠ 0 para o primeiro
botão (YES/OKAY).

## Em aberto

- Tipos 1, 3 e 5 (não usados nos dados) e o tipo 4 em detalhe.
- O que o bit 1 de `+0x34` faz além de forçar a atualização; bit 0x40 confirmado só no
  desenho.
- Cursores por `+0x18` (todos 0 nos dados).
- Geometria exata da caixa genérica e significado de `~x`/`~l`.
