# Cores dos sprites: tabelas do `DLS.TBL`, cor dos jogadores, pulsos

Como o jogo recolore os sprites que têm rotina de desenho (`+0x1c4`). Convenção:
**[confirmado]** lido no código ou nos dados; **[inferido]** dedução. Formato do `DLS.TBL` em
[`../formats/tbl.md`](../formats/tbl.md).

## A tabela de desenho (`0x43becd(a, b)`) [confirmado]

- `0x481048`: tabela de 256 bytes que o blitter aplica a **cada** pixel quando o modo `0x100`
  está ligado (`0x44f600`: `dst = T[src]`; versão transparente `0x44f641`: pula quando
  **`T[src] = 0`**, ou seja, a transparência é testada **depois** do remapeamento).
- `0x43becd(a, b)`: `a` = tabela do `DLS.TBL` (0–47; fora disso não muda nada), `b` = fator
  limitado a −127…127 (`b = 0x1e61` desempacota `a`, `b` e a sombra de um só `i32`). Chave
  `(sombra << 16) | (a << 8) | b` em `0x496c1c`; só recalcula quando muda (`0x496c20`).
- Composição, com `A` = tabela `a`, `S` = tabela de sombra `0x496c18`:
  - `a < 0x22` ou `b = 0`: `T[i] = S[A[i]]` para os 256 índices;
  - senão: `T[i] = S[A[i]]` para `i < 128`, e para `i = 128 + j`:
    `T[i] = S[A[128 + clamp(j − b, 0, 127)]]` — a metade alta é **deslocada** `b` posições
    (b > 0 repete `A[128]` no começo; b < 0 repete `A[255]` no fim).
- Sombra (`0x43be54(x)`): `x < 0` → tabela `0x12 − x`, `x > 0` → `x + 2`, 0 → tabela 0
  (identidade); devolve o `x` anterior. Usada no desenho dos objetos numa região com luz
  (`0x43574b`: `x = −|região+0x58|`, salvo o bit 1 de `0x49ccb1`) e em `0x405e9a`,
  `0x40615a`, `0x430546` (cenário).

## O desenho de um objeto (`0x435315`) [confirmado]

Se o objeto tem `+0x1c4`: chama a rotina (que escolhe a tabela), liga o modo `0x100`
(`0x45d384`), desenha (`0x446eb0`), volta à tabela `(1, 0)` (identidade) e restaura o modo.
Sem `+0x1c4`, desenha **sem** tabela. Só 7 tipos têm a rotina:

| Rotina | Objeto | Chamada |
|---|---|---|
| `0x42315f` | Jake | `(+0x27c ou 0x23, 0)` |
| `0x4282d3` | Garathe | `(+0x27c ou 0x24, 0)` |
| `0x419391` | Camaleão (tipo 4) | ver abaixo |
| `0x412204` | Anticorpo (105) | `(var+0x3c, var+0x40)`, tabela `0x4720c0` (registros de 72) |
| `0x41da5a` | robô de segurança (8) | `(var+0x28, p)`, `p = 32·sen`; morrendo (`+0x278 = −2`): −127 |
| `0x41e596` | gerador de minas (103) | `(var+0x30, 64·sen)`, tabela `0x472ad0` (registros de 0x38) |
| `0x41f22d` | mina flutuante (102) | `(var+0x40, 64·sen)`, tabela `0x472bfc` (registros de 72) |

## A arte desses sprites [confirmado nos dados]

As tabelas 35–47 têm a **mesma metade baixa** (0–127: cores fixas levadas à `WORLD.PAL`, ex.:
1 → `0xdd`) e uma **rampa de 128 tons** na metade alta, do claro (128) ao escuro (255), de
uma cor própria (35 marrom-acinzentado, 36 marrom, 37 cinza-claro, 38 vermelho, 39 dourado,
46 vermelho vivo, 47 verde…). A arte dos 7 tipos acima usa esses índices "de sprite" e não
os da `WORLD.PAL`: desenhada sem tabela, a metade alta sai como ruído de arco-íris
(`MINOTAUR.ANM`: 1,75 M pixels na metade alta contra 0,57 M na baixa; `JAKE.ANM` 0,30 M
contra 1,03 M; `OCH.ANM` quase toda alta). O fator `b` desloca a rampa: `b > 0` clareia,
`b < 0` escurece. Os `*.PAL` soltos (`BROWN.PAL`, `JAKEBRWN.PAL`…) não são citados pelo
executável [inferido: fontes das rampas].

## Jogadores [confirmado]

- `+0x280` = cor-base, `+0x27c` = cor atual; ambos 0 no solo (o Garathe copia `+0x27c` para
  `+0x280` na inicialização), então o desenho usa **35** (Jake) e **36** (Garathe).
- Piscar `0x4081ae(obj, cor, n)`: só se não houver um em curso (`+0x28c = 0`); se `cor` =
  `+0x280`, troca 35↔37 e 36↔38; grava `+0x28c = n`, `+0x290 = cor`.
- A cada tique (`0x408129`, fim da atualização): com `+0x28c > 0`, decrementa e põe
  `+0x27c` = `+0x290` quando (envenenado: `t mod 4 < 2`; senão: `t` ímpar), senão `+0x280`.
- Chamadas: dano com reação (`+0x250 & 7`) ou explosão (`+0x252 & 2`) e `+0x24c ≠ 0` →
  **0x2e** por 4 (`0x42322d`, Garathe `0x4283e8`); envenenado (`+0x270 ≠ 0`) → **0x2f** por 4
  a cada tique (`0x42328e`, `0x428449`); choque (`+0x252 & 0x20`, 1ª vez) → **0x25** por 10
  (`0x4235b2`, `0x42874e`); efeito do especial BAIXO+ATIRAR → **0x27** por 4 (`0x427671`).
  Antídoto: `+0x27c = +0x280` (`0x40b6e4`).
- Truques (`0x40407f`…, globais `0x497394`…`0x4973a4`) põem 0x2b, 0x29, 0x2c, 0x2f ou 0x2d
  em `+0x27c` e `+0x280` de todos os jogadores.
- A cor escolhida na `SEGOC` (`0x497320`/`0x497324`, padrão 35/36) vai para `+0x280` do
  jogador **só na disputa**, na atribuição das vagas (`0x43cd2f`; `multijogador.md`).

## Camaleão (`0x419391`) [confirmado]

Variantes em `0x47256c` (registros de 0x38): `+0x1c` cor normal, `+0x20` fator, `+0x34` cor
de clone.

| Variante | `+0x1c` | `+0x20` | `+0x34` |
|---|---|---|---|
| 0 Alpha | 45 | −10 | 37 |
| 1 Beta | 42 | 0 | 45 |
| 2 Gamma | 37 | −60 | 42 |
| 3 Gamma | 43 | −30 | 44 |
| 4 Gamma | 38 | −30 | 39 |

- Tabela: clone (`+0x178 & 0x10`) → `var[+0x26d].+0x34` (o clone nasce com `+0x26d` = variante
  do original, `0x41b021`); original com `+0x26d < 0` → `+0x1c`; senão `0x47277c[+0x26d]` =
  **38, 39, 40, 41, 40, 39**. Fator sempre `var[+0x260].+0x20`.
- Ciclo da clonagem: ao começar, `+0x26d = rand % 5` (`0x41aa40`); a cada 3 tiques
  (`+0x26c`: 2, 1, 0) `+0x26d++`, e passado de 5 volta a −1 (cor normal) e segue; ao criar o
  clone, `+0x26d = −1`, `+0x26c = −2` (para).
- Esmaecer (só o original): depois de montar a tabela, **zera `T[i]` para `i ≥ +0x268`**
  (pixels de origem ≥ limiar ficam transparentes) e move o limiar 7 por desenho: para baixo
  até **0x91** se invisível (`+0x265`), para cima até **0xf7** se visível (o sentido do
  próximo passo vem de `+0x265`, com um quadro de atraso). Invisível, sobram os índices
  < 0x91: a metade baixa e os tons mais claros da rampa (o "brilho" do corpo); visível, os
  9 tons mais escuros (0xf7–0xff) continuam transparentes. O clone nasce com `+0x268 = 0xff`.

## Pulsos [confirmado]

`p = A · sen(((tique + endereço) mod P) · (0x10000 / P)) >> 14` (seno de 14 bits `0x46717a`,
tique global `0x4879c4`, divisão inteira de 0x10000 por P, deslocamento aritmético).

| Tipo | A | P / tabela por variante |
|---|---|---|
| robô (8), `0x472a18` (21 `i16`; `+0x26`, `+0x28`) | 32 | 20/40, 15/41, 10/39, 7/38 |
| gerador (103), `0x472ad0` (`+0x2c`, `+0x30`) | 64 | 30/40, 50/41, 30/39, 20/38, 10/42 |
| mina (102), `0x472bfc` (`+0x3c`, `+0x40`) | 64 | 30/40, 50/41, 30/39, 20/38 |

Anticorpo (sem pulso): `(44, −45)`, `(37, −54)`, `(43, −17)`, `(42, 14)`.

## Em aberto

- Luz por região (`+0x58` da região, sombra 18 + |x|) no desenho dos objetos e do cenário.
- Quem chama os truques que gravam `0x497394`…`0x4973a4`.
