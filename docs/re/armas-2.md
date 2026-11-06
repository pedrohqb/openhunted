# Armas do Jake 3–6: espingarda, foguetes, estrelas ninja e granadas

Continuação de [`tiro-jake.md`](tiro-jake.md) (descritor, registro de estado, fluxo do tiro e
pistola), [`armas.md`](armas.md) (inventário, munição, coleta) e
[`projeteis.md`](projeteis.md) (criação `0x40bff8`, tabela de tipos). Cada disparo gasta **1**
de munição (`0x40a401`), inclusive o da espingarda.

## Descritores (32 `i32`) [confirmado]

| Campo | Espingarda `0x47e028` | Foguetes `0x47e0a8` | Estrelas `0x47e128` | Granadas `0x47e1a8` |
|---|---|---|---|---|
| pronto / guardar | 108 / 109 | 62 / 64 | 127 / 128 | 186 / 187 |
| especial / passo | 9 / 1 | 9 / 1 | 9 / **2** | 9 / **2** |
| tiques em "pronto" | 10 | 10 | 10 | 10 |
| projétil / cápsula | 20 / 23 | 21 / — | 27 / — | 26 / — |
| cima, diag. cima | 125, 123 | **67, 67** | 144, 142 | 196, 194 |
| frente, diag. baixo, baixo | 122, 124, 124 | 66, 68, 68 | 141, 143, 143 | 193, 195, 195 |
| rotina própria `+0x74` | — | — | — | — |

Os dois estados de cada par são iguais. Os foguetes não sobem na vertical: "cima" usa o
estado diagonal 67. Nenhuma das quatro tem rotina própria (a pistola tem `0x43bab7`).

## Registros de estado (18 `i32`) [confirmado]

Campos como em `tiro-jake.md`: janela de disparo `+0x0c..+0x10`, novo disparo depois de
`t > +0x14`, reavaliação `t > +0x18`, recuperação `+0x1c`, boca dx (esq./dir.) `+0x20/+0x24`,
dy `+0x28`, velocidade `+0x2c` (|vx|, sinal pelo lado) e `+0x30` (vy) em **px/tique**,
cápsula `+0x34..+0x44`.

| Arma | Estado | Janela | `+0x14` | `+0x18` | Recup. | dx esq./dir. | dy | v (px/tique) |
|---|---|---|---|---|---|---|---|---|
| Espingarda | 125 cima | 1..1 | 8 | 12 | 112 | 12 / 7 | −35 | (0, −20) |
| | 123 diag. cima | | | | | −20 / 20 | −23 | (20, −20) |
| | 122 frente | | | | | −25 / 40 | 6 | (20, 0) |
| | 124 diag./baixo | | | | | −15 / 25 | 35 | (20, 20) |
| Foguetes | 67 cima/diag. | 1..1 | 24 | 32 | 73 | −20 / 18 | −23 | (30, −30) |
| | 66 frente | | | | | −25 / 25 | 4 | (30, 0) |
| | 68 baixo | | | | | −25 / 23 | 35 | (30, 30) |
| Estrelas | 144 cima | 7..7 | 11 | 12 | 131 | 8 / 8 | −15 | (0, −10) |
| | 142 diag. cima | | | | | −5 / 18 | −10 | (10, −10) |
| | 141 frente | | | | | −25 / 30 | 7 | (18, 0) |
| | 143 baixo | | | | | −15 / 25 | 35 | (10, 10) |
| Granadas | 196 cima | 7..7 | 11 | 12 | 188 | 8 / 8 | −15 | (3, −20) |
| | 194 diag. cima | | | | | −5 / 18 | −10 | (20, −20) |
| | 193 frente | | | | | −25 / 30 | 2 | (20, 0) |
| | 195 baixo | | | | | −15 / 25 | 35 | (20, 20) |

Cápsula da espingarda (tipo 23): dx 15/0, dy −5, v (−10, 0) em 125; 35/−5, 0, (−10, −10) em
123; 30/20, 0, (−10, −10) em 122; −20/20, 35, (10, −10) em 124.

**Cadência** (regra de `tiro-jake.md`, segurando ATIRAR): espingarda 1 tiro a cada ~8 tiques;
foguetes 1 a cada ~24; estrelas e granadas (passo 2, `t` = 2, 4, …; dispara em `t = 8`, recomeça
em `t = 12`) 1 a cada ~6 tiques. Uma arremessada por acionamento. [inferido da regra]

## Espingarda: chumbo que se divide (tipo 20, `0x0040ceec`) [confirmado]

- Sem célula sob si, contato não só-de-toque ou fora da fase → avança pela velocidade medida
  (`+0x238/+0x23c`) e trata o impacto como a bala da pistola (`0x40c964`).
- 1º tique: geração `+0x27c = 1`, espera `+0x278 = 2`, tamanho `+0x274 = 10`, guarda a
  velocidade (`+0x270/+0x26c`).
- Enquanto a geração ≤ 2: a espera cai 1 por tique; a 0, cria **dois** chumbos tipo 20 no
  mesmo ponto, com a mesma rapidez e ângulos **±0x400** (±5,6°) em relação ao seu
  (`0x46718e`, `0x467154`/`0x46717a`, `>> 17`), tamanho `t/2 + 1`, geração +1, espera 2,
  `+0x274 = 3t/4 + 1`; e se destrói.
- Resultado: 1 chumbo nos ~2 primeiros tiques, 2 depois, **4** a partir do ~4º tique, em
  −11,25°, 0°, 0°, +11,25°. A 20 px/tique: divisões a ~40 e ~80 px [inferido].

Linhas `(20, …)`: no Jake (vítima 1) **10**, derrubada, módulo 20, efeito 11 (raio 12); no
Garathe (2) 10 com empurrão; nos demais natureza 2, derrubada, 10, efeito 15. Um alvo
atingido pelos 4 leva 40. Som 3045 "ShotGun" pelo quadro (`J122`–`J125`).

## Foguete (tipo 21, `0x0040d678`) [confirmado]

Reto, sem gravidade, rótulo pela direção (1 ou 2). Ao tocar algo não só-de-toque ou sair da
fase: avança pela velocidade medida, `0x43d629(foguete, tocado, 0)` e some. Linha `(21, 0)`:
**20** de dano direto em quem foi tocado, explosão sorteada de raio 64, empurrão 30, dano de
área **50** → `R = 96` px. **O Jake perto do impacto se fere** (a área não exclui o dono).
Som 3035 "Rocket" pelo quadro (`J066`–`J068`). O Bruiser usa o mesmo projétil (`bruiser.md`).

## Estrela ninja (tipo 27, `0x0040bc98`) [confirmado]

Estados 1–3: voo (`0x40bd79`); 4–5: item no chão (`0x40bcd7`, coleta genérica `0x40b3f5`).

- 1º tique: avança uma fração sorteada (0–39/8 px) na direção do voo; guarda a velocidade
  base e `|v|`; fase 0 (espelhada: +0x8000).
- Por tique: velocidade = base + componente **perpendicular** `(sen(φₖ₊₁) − sen(φₖ))·256/|v|`,
  com `φₖ = (k mod 32)·2048 + fase`: voo em **onda** de ±32 px e 32 tiques de período.
- Contato de parede com `0x40707c` → **crava**: recua 4 px, estado 5, `+0x178 |= 0x28`, vida
  `+0x264 = 100` tiques como item recolhível (devolve 1 estrela, `armas.md`); acabado o prazo,
  ou atingida por explosão, `0x43d629(estrela, nulo, 0)` e some. Quadro 48 do `ONS` → 3030
  "Knife stick in wall".
- Contato com objeto → `0x43d629(estrela, tocado, 0)` e some.

Linhas `(27, …)`: no Jake 5 + empurrão; nos demais natureza 2, 5 + empurrão; ambas com
explosão sorteada de raio 30, empurrão 30 e **dano de área 20** (`R = 45`). Som 3110
"NinjaStar" no 7º quadro de cada rótulo de arremesso (`J141`–`J144`).

## Granada (tipo 26, `0x0040d7ab`) [confirmado]

- Estado 0 (recém-lançada) → estado/rótulo 2 e **pavio** `+0x260 = 35 + rand%10` tiques.
  Estado 1: item no chão (coleta genérica).
- Estado 2 (`0x40d85a`): **com gravidade**. Quicando em parede (`0x40707c` e `+0x220 & 2`):
  inverte o espelho e toca 3042 "Grenade bouncing" posicional na base da caixa. Tocando um
  objeto que não seja dos tipos 5, 6, 9, 22–24, 30, 38, 39, 41, 46, 50–52, 56, 68–74 (nem um
  Bruiser já morto) → **explode na hora**. Velocidade x ≠ 0 → anima.
- `0x40d819`: dano de explosão pendente (natureza 2) ou pavio esgotado → explode.

Explosão: `0x43d629(granada, nulo, 0)` → linha `(26, 0)`: sem dano direto, raio 55, empurrão
30, área **30** → `R = 82` px. Fere o Jake também.

## Em aberto

- `0x40707c` (teste de "quique"/parede sólida) e a física de quique da granada.
- Velocidade exata do lançamento da granada para cima (3, −20): o 3 não leva o sinal do lado?
- Se o especial (BAIXO+ATIRAR) muda com a arma: as quatro usam o mesmo registro `0x47d7bc`.
- Nome do som 5103 nos rótulos `J193`/`J195`.
