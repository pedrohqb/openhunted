# Dicas do tutorial: o "mago" (tipo 30, M3)

Ver também [`logica-fase.md`](logica-fase.md) (lista de gatilhos e zonas de proximidade).

## Recursos

Na primeira chamada (com objeto nulo), `0x004152d5` carrega uma vez `SWIZARD.BMP`
(`0x488738`, moldura) e a fonte `slevd.pft` (`0x48873c`). [confirmado]

Os textos vêm do `WIZARD.RES`, carregado na inicialização (`0x40113a`) com
`0x45e56f(18000, "wizard.res", 0)`. A busca de texto `0x45e980(id)` devolve a linha
`(id % 1000)` **contando de 1** (`tabela[id % 1000 − 1]`). [confirmado] O mesmo vale para os
outros `.RES` (`system.res` em 10000, `graffiti.res` em 17000 [inferido pelo padrão de
`0x401104`/`0x40111f`]).

## Objeto

Inicialização (objeto não nulo) [confirmado]:

- genérica `0x40b2c8`; nome "wizard"; velocidades `+0x15c..+0x168` zeradas;
- `+0x178`: liga `0x28` e `0x81`, desliga `0xc0` (sem gravidade, só contato, nunca "visto");
- `+0x1b4 = 10`; `+0x1d0 = 0x4152d0` (vazia); animação e colisão nulas (não é desenhado);
- caixa `+0x20c..+0x218 = (0, 0, 32, 32)`; `+0x1c0 = 0x4153d8`.

## Quando aparece (`0x004153d8`, por tique)

1. Se já mostrou (`0x40` de `+0x178` ligado), nada.
2. Se a lista de gatilhos (`+0x11c`) é falsa, nada. Sem gatilhos, é verdadeira na hora.
3. Se `values[0] ≤ 1`, nada (a linha 1 do arquivo, "OK", nunca é dica).
4. Liga `0x40` (**mostra uma vez só**) e pega o texto `18000 + values[0]`, isto é, a linha
   `values[0]` do `WIZARD.RES` (contando de 1). Texto vazio → nada. [confirmado]

`values[1]` (10 em quase todos) e `values[3]` (1) não são lidos por esta rotina. [confirmado]

Na `T_J_001`:

| Mago | Posição | Gatilho | `values[0]` | Texto |
|---|---|---|---|---|
| 1 | (1514, 31) | zona id 1 ligada | 2 | "Hovering mines explode on contact. SHOOT them before they get you." |
| 2 | (164, 63) | zona id 2 ligada | 24 | "To exit press UP." |

Conferido também em outras fases: `T_M_004` usa 7 → "Find anti-venom…", `T_J_003` usa 16 →
"Press UP to climb the ladder.", coerentes com as descrições das missões.

## Como é mostrada [confirmado, geometria aproximada]

- Mede o texto na fonte `slevd.pft` (largura × altura do bloco, `0x45f6d3`/`0x45f6fe`):
  `s = (área · 8 / 5) · 125 / 100 / 4`, lado `h = isqrt(s)` (`0x46724c`); a caixa de texto
  tem `4h × h` (proporção 4:1).
- Vertical: centrada na tela, `(altura_tela − h) / 2` (`0x49cccc` = altura).
- Horizontal: pega o retângulo do objeto seguido pela câmera (`[0x493b70]+0x50`, o Jake) em
  coordenadas de tela. Se o mago está **à esquerda** do Jake (`x` menor), a caixa vai à
  esquerda dele: `esq_jake − 4h − 20`, no mínimo 16; senão à direita: `dir_jake + 20`, no
  máximo `largura_tela − 4h − 16` (`0x49ccc8` = largura).
- Desenha a moldura `SWIZARD.BMP` com margem de 8 px (`0x45ca44`, cor 211), o texto por
  cima (`0x45f1dc`) e manda a área para a tela (`0x464d52`).
- Toca o som 3774 ("MC Player Hint", `0x44dc20`).

## Pausa

A dica é **modal**: um laço próprio redesenha e lê a entrada (`0x402d48`) sem avançar o
jogo. Sai quando:

- passaram ≥ 500 ms e o jogador **soltou** uma tecla/botão que estava apertado (transição de
  "algo apertado" para "nada apertado" em `0x499f34`/`0x499f3c`/`0x499f40` ou nos joysticks
  de `0x499018`); ou
- passaram mais de 15 000 ms; ou
- `0x44a356(−50)` pede saída (fechamento da janela [inferido]).

Depois para o som (`0x448366(−1)`) e o jogo segue. [confirmado]

## Em aberto

- Valores exatos de `0x45ca44` (cor 211 = moldura? fundo?) e se o texto quebra em linhas
  (o código de `0x45f1dc` não foi lido).
- Significado de `values[1]` (10) e `values[3]` (1).
