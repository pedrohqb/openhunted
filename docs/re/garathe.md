# Garathe Den, o Minotauro (tipo 2)

Segundo personagem jogável. Movimento e estados em
[`garathe-movimentos.md`](garathe-movimentos.md). O código de tiro, troca de arma, dano e
itens é o mesmo do Jake (`tiro-jake.md`, `armas.md`, `dano-vida.md`, `itens.md`); aqui só o
que muda. Convenção: **[confirmado]** lido no código ou nos dados; **[inferido]** dedução.

## Identidade e inicialização [confirmado]

- Tipo de objeto **2** (`+0x18 = 2`; Jake = 1). Inicialização `0x004279b3` (par da tabela
  de tipos em `0x47d490`); atualização `+0x1c0 = 0x00427b68`; desenho `+0x1c4 = 0x004282d3`;
  `+0x1cc = 0x00407796` (pisão, como o Jake); `+0x1c8 = 0`.
- Nas fases: `MINOTAUR.ANM` (ou `minotaur.ANM`) e `MN_DEFLT.BRD`; escala sempre L/A = **39/58**
  (todos os objetos tipo 2 dos dumps). O quadro 0 tem 66 × 97, logo escala 58/97.
- Primeira vez (`0x47d0f0`): controlador de esqueleto nº **2** com **164** entradas
  (`0x408683(2, 0xa4)`, guardado em `0x4888a0`), mesma tabela de juntas `0x471ab8`,
  `MINOTAUR.SKE`; caixa K = **28 × 50** (cabeçalho e caixa medida coincidem). O fator da
  compensação de desenho é **587/1000** (`juntas+0x18/+0x1c`, `0x427a3d`), contra 12/20 do Jake.
- Toda instância: nome "The Minotaur"; estado inicial 1 (`0x428302`); **massa 20** (Jake 10);
  `+0x1b4 = 0x79`; `+0x178 |= 0x14801`, `&= ~0x48`; par 1 forçado a `(1, 20)` (garras);
  `+0x254` = melhor arma (`0x43a8cc(obj, 1)`); `+0x288 = 4`; renascimento `+0x29c/+0x298`;
  `+0x280 = +0x27c`.
- Vida e armadura vêm da fase (`fields[14]`, `fields[15]`): 100/100 na maioria; `T_M_008`
  50/0. `0x004278cc` cria um Garathe solto (vida 100, armadura 50) mas não tem chamadas.
- Troca de estado: `0x00428302(obj, s)` com a tabela `0x478980 + 0x70·s` (ver
  `garathe-movimentos.md`).

## Quem controla [confirmado]

A entrada é a mesma do Jake (`0x407e2d` → `0x42ec1c` → `0x487a2c`; teclas em
`tiro-jake.md`). Sem jogador (`+0x19c = 0`), `0x42ec1c` devolve a entrada da IA: os
Garathe com `modos = 0xfe` nas fases do Jake (`H_J_*`, `D_J_*`) e os extras de `H_M_003`,
`D_M_027` são Minotauros inimigos com o mesmo código.

Jogador humano: `0x43cc17` procura, para cada jogador, objetos com bit 0 de `+0x1a8` (vaga),
`+0x1a8 & modo ≠ 0` e tipo = personagem `0x497318[i]`; no solo `0x497318 = 0` (`0x43cbdc`)
aceita tipo 1 **ou** 2 e sorteia entre os candidatos; grava `0x497318[i] = +0x18`,
`+0x19c = i+1`, `+0x198` = controle. **O personagem do solo é o tipo do objeto com
`modos & 3 = 3` na fase**; o `p=2` do `ENGLEVEL.TXT` só acompanha.

## Missões do Garathe [confirmado, `ohtool levels`]

24 com `p=2`: `T_M_002/004/006/008`, `I_M_003/102/109/008/106/011/014`,
`H_M_003/021/004/027`, `D_M_030/024/004/003/006/021/010/027/018`. Ele também está nas de
dois jogadores (`C_*`, `M_*`). Fim de missão: vozes 3786 (vitória) e 3785 (derrota)
(`fim-de-missao.md`).

## Inventário e armas [confirmado]

Mesmos 30 pares de `+0x28`. Lista de armas `0x47e978`: **1, 2, 5, 4, 6, 3**.

| Arma (par) | Item | Descritor | Pronto/guardar | Estados (cima, diag. cima, frente, baixo) | Projétil |
|---|---|---|---|---|---|
| 1 garras | — | `0x47e994` | 100/101 | 106/107, 106/107, 104/105, 108/109 | — |
| 2 clava | 15 `OCL00` | `0x47ea14` | 110/111 | 116, 116, 114, 118 | — |
| 3 chicote | 44 `OWH00` (+v); 45 `OWA` soma v | `0x47ea94` | 152/153 | 162, 160, 159, 161 | 108 |
| 4 facas | 25 (`okn`) | `0x47eb14` | 81/82 | 98, 96, 95, 97 | 25 |
| 5 granadas | 26 | `0x47ec14` | 140/141 | 150, 148, 147, 149 | 26 |
| 6 estrelas | 27 | `0x47eb94` | 128/129 | 138, 136, 135, 137 | 27 |

Seletor `0x43ab05` (ramo `+0x18 = 2` em `0x43ab82`). Com SOCO (ou boca na parede,
`0x47d7b8 ≠ 0`) usa a **clava se tem o par 2**, senão as garras. Pose "pronto" na troca
(`0x43aa73`): 0/1→100, 2→110, 3→152, 4→81, 5→140, 6→128. Todos têm especial 127.
Diag. baixo usa o estado de baixo. Arma nova vira corrente se fica depois da melhor na lista
(`0x43af6c`, ramo `0x47e978`).

Registros (`tiro-jake.md`; `t` em passos de `+0x10`):

| Arma | passo | variante (`+0x00`) | golpe `+0x04..+0x08` | disparo | `+0x14/+0x18` | recuperação | v (px/tique) |
|---|---|---|---|---|---|---|---|
| garras | 1 | 0 | 3..4 | — | 8/8 | — | — |
| clava | 1 | 1 cima, 2 frente, 3 baixo | 5..7 | — | 11/12 | — | — |
| chicote | 2 | 4, 5, 6, 7 | 8..10 | 9..9 | 24/24 | 154 | (0,−20), (20,−20), (20,0), (20,20) |
| facas | 2 | — | — | 8..8 | 12/20 | 85 | idem |
| granadas | 2 | — | — | 8..8 | 12/20 | 142 | cima (3,−20), demais idem |
| estrelas | 2 | — | — | 8..8 | 12/20 (cima 20/24) | 130 | idem |

Bocas (dx esq./dir., dy): cima 13/19, −35 (chicote 13/21, −75); diag. cima −35/55, −23 (chicote
−60/90, −55); frente −35/65, 17 (granadas/estrelas 2; chicote −75/100, 20); baixo −20/50,
45 (chicote −65/90, 90).

Chicote: a linha centro→boca só é testada até a **metade** (`0x43bbe3`, ramo pronto = 152);
se algo bloqueia até **3/4** da linha (`0x43bcf2`), `t` e a animação avançam um passo a mais
por tique [inferido: o golpe recolhe mais rápido]. O projétil 108 (`0x40cb6c`) se divide:
depois de 6–8 tiques cria dois filhos a ±11,25° com a mesma velocidade, até a 4ª geração.
O 1º tique só arma (`+0x27c = 1`, `+0x278 = rand % 6 / 2 + 6`, `+0x274 = 10`); cada filho
sorteia a própria espera, tem geração `+0x27c` do pai + 1 (divide enquanto ≤ 3), tamanho
`+0x1f0` = `+0x274` do pai / 2 + 1 e `+0x274` = 3/4 do pai + 1 (tamanhos 10 → 6 → 5 → 4).
[confirmado em `0x40cc2c…0x40ce81`]
A faca 25 (`0x40ba54`) que bate em parede fica cravada (estado 3) e vira item coletável
(`0x40b3f5`); em objeto aplica `(25, …)` e some.

## Dano causado (`DAMAGE.TBL`, atacante 2) [confirmado]

| Variante | Vítima jogador (1) | Qualquer (0) | Outras linhas |
|---|---|---|---|
| 0 garras | 15, empurrão (2, 0), som 3038 | 15, empurrão, som 5062 | barril 28: natureza 2 (explode), 5; tipos 11, 4, 105, 89, 109: 8, empurrão, 3038 |
| 1 / 2 / 3 clava | 15, derrubada, (20, 30) / (20, 10) com efeito 11 raio 20 / (20, −10) | idem, 5062 | idem |
| 4 / 5 / 6 / 7 chicote | 18, derrubada, (20, 20) / (20, 20) / (20, 10) / (20, −20) | idem | idem |
| 8 / 9 especial | 8: 15, empurrão, módulo 20, efeito 1; 9: 15 | | 8 com barril: explode |

Projéteis: 108 chicote 20 e derrubada (módulo 20); 25 faca 12 e empurrão; 26/27 como no Jake
(`armas-2.md`). 3038 = "Club to Flesh".

Contra o Garathe há linhas próprias (vítima 2) antes das do jogador: espingarda (20) **10
com empurrão** (o Jake é derrubado); pistão 39: 35, derrubada, (54, −3); catapulta 41: 0,
derrubada, (48, −3).

## Especial (BAIXO+ATIRAR, estado 127 `MNVICT1`) [confirmado]

Mesma entrada do Jake (vida > 25), registro `0x47e228` (`8, 5, 20, …, +0x18 = 24`), efeito
`0x0042c348` em `5 ≤ t ≤ 20` (16 tiques; o estado dura até `t > 24`):

1. O Garathe perde **1** por tique (`+0x24c = 1`; ~16 de vida por uso).
2. Para cada objeto ativo na camada a menos de **140 px** (centros): jogadores (1, 2) a
   < 70 px: derrubada, `4·(19600 − d)/19600`, empurrão ±15/20 e crédito, como no Jake; a
   **70–139 px: `+0x274 = 30`** (atordoamento de terremoto, estado 121 nele ou o do Jake).
   Projéteis 19–21, 25–27, 29, 36, 99, 100, 106, 108 a < 70 px: variante 8 e destruídos.
   Demais a < 70 px: variante 9.

## Vida, reações e morte [confirmado]

Por tique (`0x004283b7`, ignorado no estado 80): pisca com reação ou explosão; `2` →
`0x428531`, `1` → `0x42859e`, `4` → `0x4287a6`, elétrico → `0x428713`, veneno → `0x4284c5`
(a cada 20 tiques soma `+0x270`; com jogador às vezes fala 3736); aplica `0x407862`; vida 0
→ `0x428863`.

| Reação | Estados em que vale | Vai para |
|---|---|---|
| empurrão (2) | 1, 54–56, 58 | 58 `MNDAMAGE` (gravidade ligada; volta ao 1) |
| tranco (1) | 1, 10, 28, 31, 35–38, 44–46, 58, 76, 81, 82, 85, 95–98, 100, 101, 103–111, 113, 114, 116, 118, 128–130, 135–138, 140–142, 147–150, 152–154, 159–162 | 59 (rótulo 63, desliza k=2 em x), depois 35; nos 52, 53 → queda 27 (a barra 47, 49–51 não reage: `0x428609…0x42861e`) |
| derrubada (4) | fora de 11/12 | 32 `BLASTFWD` (marca 8) ou 33 `BLASTBK0` (0x10) pelo lado × sinal de `+0x244`; v = 8·empurrão |
| elétrico | fora de 11/12, `+0x284 = 0` | 121 no quadro +17, 10 tiques parado, pisca 0x25; depois 1 |

Morte (`0x428863`; nada se `+0x278` ou trapaça): 28 → 63; 30 → 67; 35 → 69; 52/53 →
gravidade e volta de frente, depois a regra geral; 4, 11, 12, 25, 29, 32, 33, 61, 63, 67, 69:
nada. Geral: `+0x250 & 0x10` → 69 `MNDIE3`; `& 8` → 67; `& 3` → 63 `MNDIE2B`; veneno → 61
`MNDIE1`; senão 63. Todos ligam `0x200000`, `+0x1b4 = 0x78`, BRD `MN_COLPS`; 61/63/65
deslizam pelo esqueleto (k = 2). Fim `0x42b604` (parado no chão, `0 ≤ vy ≤ 16`): solo,
cooperativo ou sem jogador → **cadáver** (`0x8028`, sem colisão nem rotinas); disputa →
corpo clonado, pontos (`0x497334/0x497338`), y −20 e estado 80 (renascer: zera os 30 pares,
veneno e temporizadores, vida 100).

## Sons e falas [confirmado]

`0x40b9eb(obj, fala Jake, fala outro, prio, prio)` toca a 2ª quando o tipo ≠ 1:

| Situação | Jake | Garathe |
|---|---|---|
| vida/antídoto | 3700 | 3735 ou 3738 |
| veneno pego / respiração | 3717 / 3704 | 3737 / 3736 |
| segredo | 3718 | 3731 |
| porta trancada | 3715 + 3714/3719 | 3730 + 3732/3741 |
| pulo contra parede | 3712 (0xe80) | 3734 (médio), 3740 (longo), posicionais |

Também posicionais: 5053 ao aterrissar do pulo longo, 5043 ao empurrar, 3135 + 3772 no
choque da saída. O `MINOTAUR.ANM` não tem instrução de som (só a 1 no rótulo 162)
[confirmado]; os golpes soam pelas linhas do `DAMAGE.TBL`. Nenhum id 4000–4999 nas rotinas
do Garathe.

## HUD e interfaces [confirmado]

Mesma barra (`hud.md`): tabela de armas `0x471ce9` (garras 66, clava 68, granadas 58,
facas 54, estrelas 56, chicote 70); rosto, vida e armadura iguais (quadro 15, 93 envenenado).
Bônus do `SMISS`: `25·(+0x38) + +0x44 + +0x4c + +0x54 + +0x5c`. Não há ITF própria; as telas
de teclado/joystick trocam um ponteiro de dados quando `0x497318 = 2` (`0x441a11`,
`0x44267f`) [inferido: imagem ou texto do personagem].

## Objetos da fase [confirmado nos dumps]

Bits de `+0x178` (`fields[3]`): `0x100` Jake pega, `0x200` Garathe pega.

| Tipos | Quem (contagem) |
|---|---|
| 15, 25, 44, 45 | só `0x200` (44, 56, 67, 150) |
| 12, 13, 14, 16, 17, 18 | só `0x100` |
| 26, 27, 32–35, 37, 76–79 | quase todos os dois |

26 dá ao Garathe a arma 5 (Jake: 6) e 27 a 6 (Jake: 5). O inimigo tipo 8 o ignora nos
estados 54–56 (`furtividade.md`).

## Em aberto

- IA do Minotauro: ver [`minotauro-ia.md`](minotauro-ia.md).
- Quanto a faca cravada devolve; geração final e fim do chicote 108.
- `desc+0x08` (103, 113) e `+0x00` do registro do especial (8): não lidos.
- Conferir no jogo o efeito visual de `0x4282d3` (`0x43becd`).
