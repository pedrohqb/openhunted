# Barra de estado (HUD) durante o jogo

O original não escreve texto sobre a área de jogo: desenha uma **barra de inventário** de
bitmaps (`INVBAR01.BMP`, 211 quadros) embaixo da vista. Inventário e armas em
[`armas.md`](armas.md); vida e armadura em [`dano-vida.md`](dano-vida.md).

## Criação e lugar na tela (`0x0040a4ef(vista)`) [confirmado]

- Chamada ao criar cada vista da câmera (`0x436148`, `0x436302`). Carrega `INVBAR01.bmp`
  uma vez (`0x471cc8`). Com dois jogadores (`0x497300 ≥ 2`) as alturas caem pela metade
  (`0x471cd0`, `0x40a485`).
- Altura da barra = altura do quadro 0 (**75**). A **vista do jogo perde 75 px de altura** e a
  barra ocupa a faixa logo abaixo: estrutura de 24 bytes `{x, y, largura, altura, sinais,
  último cronômetro}` com `x = x_vista`, `y = y_vista + altura_vista` (já reduzida),
  `largura = largura_vista`. Em 640×480 solo: **jogo em 640×405, barra em y = 405…479**.
  [confirmado pela aritmética; os campos `+0x2c`/`+0x3c` da vista como alturas, inferido]
- Vista mais estreita que 640: as coordenadas x são escaladas por `largura / 640`
  (`0x471ccc`, `0x40a460`).

## Quando redesenha [confirmado]

O desenho do quadro (`0x4365ee` → `0x40a6d7` → `0x40a5b6`) só refaz as partes marcadas em
`+0x10` e depois zera: 1 = tudo, 2 = itens, 4 = vida, 8 = armadura, 0x20 = armas (força
tudo). `0x40a6fa` marca tudo (ao trocar dispositivos). O cronômetro tem teste próprio. Um port
pode redesenhar a barra inteira a cada quadro com o mesmo resultado.

Abaixo, `(x, y)` = canto da barra; todas as posições somam a ele.

## Fundo e itens (sinal 1 ou 2, `0x40a8dc`) [confirmado]

1. Quadro 0 (256×75) repetido de `x` até `x + largura`.
2. Quadro 1 (321×32, faixa de 6 casas) em `(0, 0)` e em `(largura − 321, 0)`.
3. Para `i = 0…5`, ícone centrado em y + 5:
   - par `13 + i` do inventário (itens dos tipos 62–67) → quadro `2 + i`, centro em
     `x = 35 + 51·i`;
   - par `7 + i` (tipos 80–85) → quadro `8 + i`, centro em `x = largura − 321 + 35 + 51·i`.

## Vida (sinal 4, `0x40aabe`) e armadura (sinal 8, `0x40abd0`) [confirmado]

| Peça | Vida | Armadura |
|---|---|---|
| Moldura: quadro 14 (97×46) | (0, 30) | (96, 30) |
| Ícone | rosto: quadro 15 (35×38), ou 93 (33×36) se `+0x270 ≠ 0`, em (6, 35) | quadro 16 (38×38) em (100, 32) |
| Número (3 dígitos) | `+0x258` em (43, 35) | `+0x25c` em (139, 35) |

Estilo do número pelo valor: ≥ 50 → 19, ≥ 20 → 18, senão 17 (três cores, [inferido]).

## Armas (sinal 0x20, `0x40acb2`) [confirmado]

Tabela por personagem, 3 bytes por arma `{quadro se tem, par, quadro se não tem}`; soma 1 ao
quadro se for a arma corrente (`+0x254`):

| Ordem | Jake (`0x471cd1`) | Garathe (`0x471ce9`) |
|---|---|---|
| 0 | punhos: 52 / par 1 / 196 | 66 / par 1 / 203 |
| 1 | granadas: 58 / par 6 / 199 | 68 / par 2 / 204 |
| 2 | estrelas: 56 / par 5 / 198 | 58 / par 5 / 199 |
| 3 | pistola: 60 / par 2 / 200 | 54 / par 4 / 197 |
| 4 | espingarda: 62 / par 3 / 201 | 56 / par 6 / 198 |
| 5 | foguetes: 64 / par 4 / 202 | 70 / par 3 / 205 |

Largura total `L` = soma das larguras dos 6 ícones + 6 × largura do quadro 50. Início
`X = largura/2 − L/2 + 63`, `y = 30`. Desenha o ícone 0; depois, para as armas 1–5: ícone, e
logo à direita a **caixa de munição** = número de 2 dígitos com a quantidade do par
(`+0x2c + 8·par`), estilo 50 se for a arma corrente, senão 51; avança a largura do quadro 50.
A caixa aparece mesmo sem a arma.

## Cronômetro (solo/cooperativo, `0x0040b159`) [confirmado]

Só quando `0x4972fc` mudou ou num redesenho total:

- Moldura: quadro 92 (93×44) em `(largura − 93, 30)`.
- 4 dígitos dos **segundos restantes** (`0x4972fc / 20`), da direita para a esquerda: o
  primeiro em `(largura − 93 + 68, 36)`, cada um 22 px à esquerda do anterior.
- Quadro do dígito `d` = `193 − 10·d − r`, faixa 94–193 (18×34): um **contador de rolo**. `r =
  (0x4972fc % 20) / 2` (0–9) rola o dígito das unidades e só passa ao seguinte enquanto o anterior
  é 9.
- Fases sem limite (campo 1004 = 0): mostra `0000` [inferido: o cronômetro fica em 0].

Com dois jogadores sem cooperação, outra rotina (`0x40b089`) desenha os pontos no lugar.

## Desenho de números (`0x0040a716(valor, estilo, x, y)`) [confirmado]

Desenha o quadro `estilo` como fundo em `(x, y)` e então os dígitos da direita para a
esquerda, cada um avançando a largura do glifo + 1 para a esquerda. **Zeros à esquerda são
desenhados**; o valor é limitado ao máximo de dígitos.

| Estilo | Dígitos | Glifos (0–9) | Primeiro (unidades) em |
|---|---|---|---|
| 17 / 18 / 19 (51×37) | 3 | 20–29 / 30–39 / 40–49 (13×29) | (x + 32, y + 4) |
| 50 / 51 (31×44) | 2 | 72–81 / 82–91 (11×29) | (x + 14, y + 8) |
| 195 (`0x40a836`; antes lido como 196) | 2 | 20–29 | (x + 20, y + 4) |

## Em aberto

- Quem liga cada sinal de `+0x10` (coleta, dano, troca de arma).
- O que é `+0x270` do jogador (rosto alternativo 93: veneno?).
- Os quadros 94+ além do cronômetro e dos pontos (196–210). `0x40b089` (disputa): moldura
  194 encostada à direita em `y + 30`, `max(0, F − S)` no estilo 195 em `+38, +5`
  (`multijogador.md`; metade disso na barra achatada).
- Conferir no Wine a faixa da vista (640×405) e as posições acima.
