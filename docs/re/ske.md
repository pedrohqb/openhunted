# Esqueleto (`.SKE`): "animação com deslocamento"

Formato em [`docs/formats/ske.md`](../formats/ske.md). Aqui, como o motor usa o esqueleto
para mover o personagem. Estados do Jake em [`jake-movimentos.md`](jake-movimentos.md).

## Carga [confirmado]

Na inicialização do Jake (`0x0042277e`, só na primeira vez: `0x4788e0`):

1. `0x00408683(1, 197)` cria o descritor global `0x48888c` (6 `i32`): `+0x00` identificador
   esperado, `+0x04` capacidade, `+0x08` rótulo corrente, `+0x0c` quadro corrente do
   esqueleto, `+0x10` estrutura de juntas, `+0x14` tabela de entradas (0xa0 bytes cada).
2. `0x004085c6(0x471ab8, 18, 3)` cria a estrutura de juntas (`+0x00` 18, `+0x04` 3, `+0x20`
   cópia da tabela fixa). `0x00409dff("JAKE.SKE", desc)` lê o arquivo; a largura/altura do
   cabeçalho vão para `juntas+0x08/+0x0c`.
3. Rótulo 1, quadro = primeiro do rótulo 1. `0x00408bf9(jake, desc)` grava em
   `juntas+0x10/+0x14` a largura e a altura do **quadro de referência do sprite já
   escalado** (`0x0043585c`: quadro 0 do `JAKE.ANM`, 42 × 88, vezes `+0x1f0/+0x1f4`).
   `0x00408bb4(desc)` grava em `juntas+0x08/+0x0c` a largura e a altura da caixa das 18
   juntas projetadas no quadro 0 do rótulo 1 (`0x00409a4d`).
4. `juntas+0x18 = 12`, `juntas+0x1c = 20`. Cada Jake recebe uma cópia do descritor em
   `+0x2a8` (`rep movs` de 6 `i32`).

Projeção de uma junta (`0x004094b2` → `0x004094ef`): x' = (x·cos a − z·sen a) >> 14, y' = y,
com `a` = ângulo da entrada (u16) e cosseno em Q14 da tabela `0x483f74` (`0x00467154`; seno
em `0x0046717a` = cos(a − 90°)); somam-se a raiz do quadro e dx/dy da entrada. Com os dados,
a caixa do rótulo 1 dá **K = 20 × 57** (igual ao cabeçalho). Todo Jake das fases tem os
campos de escala 22/47 (escala 47/88), logo **S = 22 × 47** px.

Garathe: `0x004279b3` faz o mesmo com `MINOTAUR.SKE` (identificador 2, 164 entradas).

## Quadro do esqueleto acompanha o da animação [confirmado]

- `0x00406b6c(obj, estado, rótulo)` (início de rótulo) põe `desc+0x08` = rótulo e
  `desc+0x0c` = primeiro quadro da entrada; `0x00406c1a` (início ao contrário) põe o último.
- `0x00406c71(obj, modo)` anima: bit 0x1 = laço, 0x2 = parar no fim (devolve 1 ao
  acabar), 0x4 = de trás para a frente, **0x8 = sincronizar o esqueleto**: `desc+0x0c =
  início + passo·(+0x184)`, limitado ao intervalo da entrada. Com `+0x2a4 ≠ 0` (modo
  "Use Skeleton", tecla F4 de depuração `0x471984` [inferido pela string], ou estado cuja
  coluna `+0x14` da tabela vale 0: 7 e 49) é o esqueleto que avança por si e o sprite não é desenhado
  (`0x00435339`).
- Os estados do Jake animam por `0x004238fa` (modo 9, laço), `0x00423835` (modo 0xa, para
  no fim e chama uma rotina) e `0x00423894` (modo 0xe, ao contrário); todos só animam a cada
  `atraso + 1` tiques globais (`0x4879c4`), `atraso` = coluna `+0x18` do estado.

## Deslocamento: `0x00406ea2(obj, modo, &saída)` [confirmado]

Com `f` = `desc+0x0c` e `p` = raízes da entrada do rótulo corrente (sem entrada ou sem
raízes, saída = 0):

1. Sem o bit 0x10: `v = p[f]` (posição acumulada). Com 0x10: `v = p[f] − p[f−passo]`
   (ao contrário, `p[f] − p[f+passo]`); no primeiro quadro, `v = 0`.
2. Se o lado `+0x288` tem o bit 8 (direita), `v.x = −v.x`.
3. `v.x = v.x·S.l / K.l`, `v.y = v.y·S.a / K.a` (divisões truncadas).
4. Sem o bit 0x10, ainda `v = v·12 / 20` (`juntas+0x18/+0x1c`).

Dois usos:

**a) Deslocamento do desenho** (`+0x16c`, `+0x170`). Toda chamada de `0x00406c71` termina
com `0x00406ea2(obj, modo sem 0x10, +0x16c)` e `+0x170 = −+0x170`. O sprite é desenhado em
`(+0x14c − +0x16c, +0x150 − +0x170)` (`0x00435364`) e a caixa de colisão do BRD é posta no
mesmo ponto (`0x00434e99`): o corpo se afasta da posição lógica conforme a raiz da captura
**sem mover o objeto**. O tique do Jake zera os dois quando `tique % (atraso+1) = 0`
(`0x00422a0a`); as aterrissagens 34/35 e a entrada no pulo estendido também.

**b) Velocidade pedida** (`0x00406fc0(obj, k, modo)`): chama `0x00406ea2` com 0x10 e faz
`+0x164 = 8·k·v.x`, `+0x168 = −8·k·v.y` (1/8 px por tique). Muitos estados preservam a
vertical (guardam `+0x168` antes e repõem depois). A física (`0x004054cc`, no laço
`0x004045a5`) soma a gravidade, 2 px/tique (`0x47197c`), salvo com o sinalizador 0x8, e
copia para a velocidade real `+0x15c/+0x160`.

Em resumo, por tique: **px = k · trunc(Δraiz · S/K)**, com S/K = 22/20 na horizontal e
47/57 na vertical para o Jake padrão (Δy positivo = para cima).

Estados de animação espelhada (2, 6, 9, 18, 19, 22, 23, 49, 54–56, 62, 64, 66–68, 73, 108,
109, 112, 122–125, 127, 128, 131, 141–144, 146, 147, 149–155, 186–188, 193–196: as poses de
arma e de luta, desenhadas olhando para o outro lado): no fim do tique do Jake
(`0x00422fce`) `+0x16c` e `+0x164` são negados e o espelhamento do sprite (`+0x194` bit 1,
`0x00434815`) segue o lado ao contrário.

## Fator k e rótulos que usam o esqueleto [confirmado]

| Estado (rótulo) | Rotina | k | Uso |
|---|---|---|---|
| 2 (149) | `0x004261d0` | 3 | andar furtivo, só x |
| 16, 17 (16, 17) | `0x00424ca6`, `0x00424dd0` | 3 | pulo para cima |
| 27 (27) | `0x00424f49` | 3 | pulo parado |
| 31 (31) | `0x00425382` | 2 | impulso do pulo estendido, só x |
| 34, 35 (34, 35) | `0x00425946`, `0x004259c6` | 1 | aterrissagens, só x |
| 30, 32 (30, 32) | `0x0042710a` | 2 | escalar corda/escada |
| 86 (86) | `0x004266a4` | 2 | engatinhar, só x |
| 107 (107) | `0x004247f8` | 4 | mão ante mão na barra, só x |
| 162, 163 | `0x00425839`, `0x004258b9` | 2 | equilíbrio na beirada, levantar |

Correr (10), os pulos lançados (28, 29, 80) e empurrar (100/101) usam velocidades fixas;
o esqueleto só desloca o desenho.

Valores por tique (olhando à direita, + = para a frente / para baixo), quadro 0 → n−1:

- **27**: vy = 0 ×9, −18, −18, −12, −3, +6, +12, +27, +6, 0 ×10 (mais a gravidade).
- **16/17**: vy = 0 ×10, −15, −42, −21, −9, 0, +6, +18, +42, +24, 0 ×7.
- **31**: vx = 0, 16 (dois tiques; depois o pulo 80).
- **34**: 0, 4, 3, 2, 2, 3, 2, 2, 2 (mas anima dois quadros por tique, ver
  `jake-movimentos.md`); **35**: 0, 5, 5.
- **149** (estado 2): 0, 3, 9, 15, 9, 9, 9, 9, 12 (média ≈ 9,4 px/tique).
- **30** (subir): vy = 0, −2, −10, −6, 0, 0, 0, −2, −14, −8, 0, 0, 0 (42 px por ciclo).
- **32** (lado): 0, 16, 14, 4, 10, 6, 0 (50 px por ciclo).
- **86**: 0, 10, 16, 8, 6, 10, 8, 14, 8, 8, 8, 10 (106 px por ciclo).
- **107**: 0, 8, 8, 4, 0, 0, 0, −4, 0, 0, 0, 4, 4, 8, 8, 8 (48 px por ciclo).

## Em aberto

- Conferir no jogo a magnitude (k × S/K) [inferido: a fórmula está confirmada, os px não
  foram medidos]: dependem da escala do Jake (22/47 nos 172 objetos de tipo 1 das fases;
  Garathe 39/58) e do arredondamento da caixa K.
- Topologia das juntas (`0x471ab8`) e o desenho do esqueleto em modo de depuração.
- O que o Garathe faz com o `MINOTAUR.SKE` (mesmo mecanismo, outros estados).
