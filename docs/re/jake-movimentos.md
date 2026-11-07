# Jake: estados de movimento (pulos, aterrissagens, agachar)

Complementa `personagem.md` (parado, correr, pulos lançados, queda) e
[`tiro-jake.md`](tiro-jake.md). Escalada, barra e portas em
[`jake-escalada.md`](jake-escalada.md); furtivo, esconder e golpes em
[`furtividade.md`](furtividade.md); deslocamento do esqueleto em [`ske.md`](ske.md).

## Tabela de estados `0x473300 + 0x70·s` [confirmado]

| Campo | Conteúdo |
|---|---|
| `+0x01` | BRD do estado (ASCIIZ): `JK_DEFLT`, `JK_EXTND` (76, 77, 80, 162), `JK_COLPS` (78, 81–84, 163, 169–179), `JK_CRAWL` (85–87), `JK_BARUP` (104–106) |
| `+0x10` | rótulo do `JAKE.ANM` (igual a `s`, salvo 2→149, 4→18, 6→146, 8→3, 13→12, 87→85, 102→100, 106→105, 166→164, 169/175→173, 177→77) |
| `+0x14` | 1 = anima pelo `.ANM`; 0 (estados 7 e 49) = pelo esqueleto (`+0x2a4 = 1`) |
| `+0x18` | atraso: anima a cada `atraso + 1` tiques (1 em 1, 15, 49, 73, 79, 107, 109, 112, 128, 131, 146, 147, 187, 188) |
| `+0x1c` | área passada por endereço, com o BRD, a `0x00434ec1` (carga da colisão) |

O nome da captura do estado `s` fica em `+0x20` da linha `s − 1` (ex.: 10 `JKRUN1LO`, 25/26
`CTRLFALL`, 27 `JMPAVOID`, 28/34 `JMPLONG`, 29/35 `JMPMED`, 30 `MNCLIMB1`, 31/80
`JUMPEXT0`, 32 `MNCLIMB2`, 16/17 `RTJUMP`, 3 `RT2BCK`, 76 `UNCTRLFA`, 77–79 `UNCTRLLA`,
85/86 `CRAWLSEQ`, 100/101 `PUSHSHLD`, 103–106 `SWINGBAR`, 107 `JKBRMOV0`, 162 `LANDOVRH`,
163 `KNEEL2ST`, 164/165 `JKHIDE`, 2 `JKH2H1`). `0x0042318e(obj, s)` troca de estado:
`+0x180 = s`, rótulo `+0x10` (`0x00406b6c`), BRD `+0x01`.

## Tique e despacho (`0x0042293c`) [confirmado]

Entrada (`0x00407e2d` → `0x487a2c`), lado `+0x288`, `+0x178 |= 0x4000`; depois, em ordem,
`0x0043b0cb`, `0x004231f9` (dano), `0x0042415c` (queda), `0x00426f6a`, `0x004235c4`; se
nenhum tratou, salta pelo estado (bytes `0x422b09`, alvos `0x422bce`):

| Estados | Rotina | | Estados | Rotina |
|---|---|---|---|---|
| 1, 6 | `0x00426ecc` (parado) | | 25 | `0x0042424d` (queda) |
| 2 | `0x004261d0` (andar furtivo) | | 26 | `0x00424385` |
| 3 | `0x00427089` | | 27 | `0x00424f49` |
| 4 | `0x00426f05` | | 28 / 29 / 80 | `0x004254df` / `0x0042568c` / `0x004253d3` |
| 7 / 8 | `0x004270c9` / `0x004270e5` | | 30, 32 | `0x0042710a` |
| 10 | `0x00426254` (correr) | | 31 | `0x00425382` |
| 11 / 12 / 13 | `0x004262c9` / `0x004263ee` / `0x00426820` | | 34 / 35 | `0x00425946` / `0x004259c6` |
| 15 | `0x0042705f` | | 76 / 77 / 78 / 79 | `0x004242f6` / `0x004243aa` / `0x00424421` / `0x00426f41` |
| 16 / 17 | `0x00424ca6` / `0x00424dd0` | | 81–84 | `0x00425a46`, `0x00425af4`, `0x00425b99`, `0x00425bdf` |
| 85 / 86 / 87 | `0x00426519` / `0x004266a4` / `0x004265c6` | | 100 / 101 / 102 | `0x00425c34` / `0x00425d04` / `0x00425ced` |
| 103 / 104 / 105 / 106 / 107 | `0x00424498` / `0x004246c4` / `0x00424ab4` / `0x00424bb8` / `0x004247f8` | | 162 / 163 | `0x00425839` / `0x004258b9` |
| 164 / 165 / 166 | `0x00425e82` / `0x00426020` / `0x00425f8a` | | 168, 169, 171, 173, 177, 179, 181 | dano e morte (`dano-vida.md`) |

Os de tiro estão em `tiro-jake.md`; os demais (0, 5, 14, 20, 21, 24, 33, 36–53…) caem em
`0x00426ecc`.

## Decisão em pé (`0x00423940`) [confirmado]

Chamada pelos estados parados e pelo fim de vários outros; na ordem:

1. Entrada = exatamente 0xf (as quatro direções) → estado 4 (rótulo 18, pose de arma).
2. ATIRAR/SOCO e `+0x294 = 0` → tiro (`0x0043b43e`).
3. ESQUERDA/DIREITA: sonda `0x0040718d(obj, ±4, 0)`; bloqueado (`0x004070d1`) → empurrar
   (100); com FURTIVO (0x400) → estado 2; senão correr (10).
4. Sem PULAR, com CIMA ou BAIXO: lê as zonas PR sob o Jake (`0x004075af`, ver
   `jake-escalada.md`): CIMA + zona 1 + camada ≠ 0 → estado 12; BAIXO + zona 2 + camada <
   máximo → estado 11; CIMA + zona 4 (ou zona 0x40 com jogador) → estado 3. Sem zona que
   sirva: **BAIXO → agachar (85)**; **CIMA → esconder (164)**.
5. PULAR: com CIMA → pulo para cima (16); senão pulo parado (27).

## Pulo parado (27) e para cima (16, 17) [confirmado]

- Entrando fora do chão (`0x00407031` falso) → cancela: parado.
- ATIRAR/SOCO: zera `+0x168` e atira.
- Antes do quadro 10 (`+0x184 < 10`), com PULAR + ESQ/DIR → vira pulo lançado: com CIMA o
  longo (28), senão a escolha por sonda (`0x00425144`, 110/150/255 px). No 27, PULAR + CIMA
  sem lado troca para o 16 no mesmo quadro.
- Sem PULAR, CIMA/BAIXO numa zona PR 4 → agarra a corda (30).
- No 16/17: contato com teto (`0x0040712f`: normal para baixo, |ny| > 2|nx|) e `+0x220 = 2`
  → pendura na barra (103).
- Velocidade: esqueleto com k = 3 (`ske.md`); **controle no ar** `0x00408205`: ESQ/DIR põe
  vx = ∓8 px/tique se a sonda de 8 px estiver livre (senão 0).
- Fim da animação (26 quadros) → pose 1, velocidade zero, parado. O 17 (`0x00424dd0`) é o
  pulo que sai da corda com PULAR+CIMA; usa a entrada 17 do esqueleto, igual à 16.

## Impulso (31) e pulo estendido (80) [confirmado]

`0x00425382`: anima o 31 (2 quadros, atraso 0, para no fim) e pede vx pelo esqueleto com
k = 2, mantendo a vertical: tique 1 → 0, tique 2 → 16 px/tique; no 3º tique a animação
acaba e chama `0x004253d3`, que zera `+0x16c/+0x170`, entra no 80 e lança (17, −15).
No ar (28, 29, 80): sem PULAR, CIMA/BAIXO numa zona PR 4 → corda. **28/29**: só no fim da
animação (8/9 quadros) olham o chão: no chão → aterrissagem; contra parede (`0x0040707c`:
|ny| < 2|nx|) → som 0xe80 e queda 25; outro contato (sem o bit 4 de `+0x220`) → pose 1.
**80**: chão a qualquer momento → 77; contato durante a animação → pose 1; fim no ar →
queda descontrolada 76. `+0x178 &= ~0x4000` nos pulos (passa a colidir com personagens).

## Aterrissagens [confirmado]

| Vem de | Estado (quadros) | Comportamento |
|---|---|---|
| 29 (médio) | 35 (3) | zera o deslocamento; anima **dois quadros por tique**; vx do esqueleto k = 1: 5 px no tique 1; parado no 2º (`0x4259c6`: se um dos dois `0x423835` passa do fim → parado) |
| 28 (longo) | 34 (9) | idem; vx 4, 2, 3, 2 px nos tiques 1–4; parado no 5º |
| 25 (queda) | 26 (12) | `0x00424385`: primeiro a decisão em pé (interrompível), senão anima até o fim → parado; sem velocidade própria (só desenho) |
| 80 / 76 (descontrolada) | 77 → 78 → 79 | 77: com 25 px livres acima (`0x0040718d(0, −25)` = 0), primeiro a decisão em pé; na entrada, beirada (`0x00407d09`: a caixa cruza a divisa de dois blocos de 128 px e o chão não a cobre toda) → 162; fim → teto baixo: engatinhar (86), senão 78. 78: teto baixo ou BAIXO → 86; decisão em pé; fim → com `rand & 3 = 0` toca o 79 e fica parado, senão parado |
| 77 na beirada | 162 (43) → 163 (15) | equilíbrio e levantar, esqueleto k = 2 só em x; teto baixo → 86 |

Os dois quadros por tique vêm de chamar `0x00423835` duas vezes; a velocidade usa só o Δ do
último quadro.

## Agachar e engatinhar (85, 86, 87) [confirmado]

- **85** (`CRAWLSEQ`, 5 quadros, BRD `JK_CRAWL`): BAIXO em pé sem zona; anima a dobro e no
  fim passa a 86. CIMA → 87. ATIRAR e PULAR funcionam como em pé.
- **86** (engatinhar, 12 quadros): tiro e pulo só se houver 30 px livres acima
  (`0x0040718d(0, −30)` livre). Soltar BAIXO com 30 px livres → 87. ESQ/DIR com 4 px livres
  → anima em laço e vx pelo esqueleto (k = 2: 106 px por ciclo); parado → vx = 0 e só o
  deslocamento do desenho.
- **87** (levantar = 85 ao contrário, `0x00423894`): ATIRAR, PULAR, ESQ/DIR (correr) o
  interrompem; BAIXO de novo → zona 2 → estado 11, senão 85; no fim → parado.

O tiro agachado é o mesmo de pé (`0x0043b43e`); BAIXO+ATIRAR é o especial
(`furtividade.md`).

**Ruling (OpenHunted)**: a caixa do estado vale já na física do tique em que o estado muda
(o original troca o BRD com o estado). Num estado controlável no chão que pede a caixa
padrão (em pé, tiro, pulo, esconder) sem que ela caiba sob um teto baixo, o Jake fica no
86 com a caixa `JK_CRAWL`, como o original faz com teto baixo no 77/78/86. Só o 86 tem a
porta dos 30 px; o 85, o 87 e o esconder (164–166) atiram e pulam como em pé. No ar e nos
estados sem controle (derrubada etc.) a caixa menor fica até a padrão caber.

## Empurrar (100, 101) [confirmado]

100 (`PUSHSHLD`): ESQ/DIR contra parede; anima 100 até o fim → 101 (`0x00425d04`). Em 101:
BAIXO → agachar; sem bloqueio a 32 px à frente ou sem chão a 4 px → 102 (`0x00425ced`) →
parado; numa zona PR **0x20** (só na primeira entrada no 101) chama `0x0040f040(célula,
máscara das chaves +0x94…+0xbc, 1)`: a porta da célula (`itens-campanha.md`); vx =
±4 px/tique (32), vy = 0; anima 101 em laço. Som 0x13b3 ao começar.

## Em aberto

- `0x0043b0cb`, `0x00426f6a`, `0x004235c4` (pré-tratamentos) e o estado 4.
- (resolvido: `0x0040f040` é a porta de célula, `itens-campanha.md`)
- Se a pose 1 no ar (contato durante o pulo) é só visual até a queda.
