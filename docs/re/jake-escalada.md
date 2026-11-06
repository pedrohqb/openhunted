# Jake: cordas, escadas, barras e portas de camada

Estados e despacho em [`jake-movimentos.md`](jake-movimentos.md); velocidades do esqueleto
em [`ske.md`](ske.md); saída da fase (zona 0x40) em [`logica-fase.md`](logica-fase.md).
[`escada-camadas.md`](escada-camadas.md) cobre o mesmo terreno (zonas, escalada, camadas);
aqui ficam a barra, os números da escalada e as correções: a "tabela de pontos" que move a
escalada é o `JAKE.SKE`, não o `JAKE.ANM`, e `0x00424498` é pendurar na barra, não o topo
da escada.

## Zonas PR das células (`0x004075af`) [confirmado]

A lista PR de cada célula (`docs/formats/lev.md`) traz `(s16 tipo, retângulo)`; o tipo é uma
**máscara de bits**. `0x004075af(obj)` percorre os blocos de 128 px cobertos pela caixa do
objeto (`0x004351b5`, coordenadas `>> 7`; célula por `0x004317db(x, y, camada +0x17c)`),
desloca cada retângulo da lista `célula+0xd4` (itens: `+0x00` tipo, `+0x04` retângulo,
`+0x14` próximo) para a fase (`0x00405775`) e devolve o **OU dos tipos** das zonas que cruzam
a caixa (`0x00405698`).

| Bit | Uso no Jake | Nas 100 fases |
|---|---|---|
| 0x01 | porta de camada: CIMA → camada − 1 (estado 12) | 621 |
| 0x02 | porta de camada: BAIXO → camada + 1 (estado 11) | 616 |
| 0x04 | corda / escada: CIMA (em pé) ou CIMA/BAIXO (no ar) agarra | 2421 |
| 0x20 | empurrar (estado 101, `jake-movimentos.md`) | 158 |
| 0x40 | saída (CIMA no estado 3) | 306 |

Exemplos: `T_J_003` ("Use the up key while in front of the ropes to climb") tem 12 zonas 4,
retângulos de 32 px de largura e 73–95 de altura (ex.: célula 9, `(49, 50, 32, 78)`);
`T_J_007` tem o par porta 2 na célula 21 (camada 0) e porta 1 na célula 57 (camada 1, mesmo
x/y), ambos `(67, 66, 5, 32)`; `D_J_007` tem 19 zonas 4, 4 + 4 portas e uma zona 0x20
`(20, 92, 79, 22)`.

## Agarrar a corda [confirmado]

- **Em pé** (`0x00423940`): CIMA sobre zona 4 → estado 3 (`RT2BCK`, rótulo 3, 7 quadros,
  `0x00427089`; ATIRAR/SOCO o cancela). No fim chama `0x004270c9` → `0x00423b2b`: com CIMA
  ainda sobre zona 4 → escalar (`0x0042710a`); senão estado 8 (rótulo 3 ao contrário,
  `0x004270e5`) → parado. BAIXO em pé numa zona 4 **não** agarra (agacha).
- **No ar** (pulos 16, 17, 27, 28, 29, 80, quedas 25, 76, arremessado 81, 82): sem PULAR,
  CIMA **ou** BAIXO sobre zona 4 → escalar direto.

## Escalar (estados 30 e 32, `0x0042710a`) [confirmado]

A cada tique:

1. Fora de zona 4 → solta: desliga 0x8 (volta a gravidade), `+0x1b4 = 0x78`, estado 8 →
   parado. É assim que se sai pelo alto ou por baixo da corda.
2. ATIRAR/SOCO → solta (estado 8; o tiro não sai daqui).
3. PULAR → solta; com ESQ/DIR: + CIMA → pulo longo (28), senão escolha por sonda
   (`0x00425144`); só CIMA → pulo para cima 17 (`0x00424dd0`); nada → estado 8 (cai).
4. Senão monta a máscara de movimento com sondas de 4 px (`0x0040718d`): CIMA (0, −4), BAIXO
   (0, +4), ESQ (−4, 0), DIR (+4, 0); a direção conta se não houver contato ou se o contato
   não tiver o bit 2 de `+0x220`. **Subindo contra um contato com `+0x220 = 2` exatamente →
   pendura na barra (103)** [inferido: contato com caixa do BRD, tipo 2].
5. Liga 0x8 (sem gravidade), `+0x1b4 = 0x28` [inferido: ordem de desenho, a corda na frente].
6. Conforme a máscara:

| Máscara | Estado | Animação | Velocidade (esqueleto, k = 2) |
|---|---|---|---|
| nenhuma (ou CIMA+BAIXO) | fica (30 se vier de fora) | parada | zero (`0x00407009`); só o deslocamento do desenho |
| CIMA | 30 | laço para a frente (modo 9) | vertical do rótulo 30 (sobe 42 px por ciclo de 13 quadros) |
| BAIXO | 30 | laço ao contrário (modo 0xd) | Δ ao contrário (desce) |
| CIMA+ESQ / CIMA+DIR | 30 | para a frente | vertical, e `vx = vy` / `vx = −vy` (diagonal a 45°) |
| BAIXO+ESQ / BAIXO+DIR | 30 | ao contrário | vertical, e `vx = −vy` / `vx = vy` |
| ESQ ou DIR | 32 (`MNCLIMB2`) | laço para a frente | horizontal do rótulo 32 (50 px por ciclo de 7) |

**Ruling (OpenHunted)**: CIMA+BAIXO com um lado (máscara 7, que a tabela não cobre) conta
como só o lado: 32.

O lado `+0x288` segue ESQ/DIR (todo tique), por isso o 32 anda para o lado pressionado. Não
há tiro nem troca de arma na corda.

## Barra / teto (estados 103–107) [confirmado]

Entra-se: no pulo para cima (16/17) ao bater num teto (`0x0040712f`: contato com normal
para baixo, |ny| > 2|nx|) com `+0x220 = 2`, ou subindo na corda contra ele (acima). Não há
outro "agarrar beirada": nenhum estado do Jake testa quinas de plataforma.

- **103** (`SWINGBAR`, pendurado, `0x00424498`): na entrada liga 0x8, `+0x1b4 = 0x6e`.
  PULAR+BAIXO exatamente (0x202) → solta (`+0x1b4 = 0x79`, queda 25). Sem contato a 4 px
  acima → solta (queda). ATIRAR/SOCO → solta e atira. ESQ/DIR → 107. CIMA → 105. BAIXO, ou
  PULAR acabado de apertar (0x4000) → solta (queda). Senão laço do 103, velocidade zero.
- **107** (`JKBRMOV0`, mão ante mão, `0x004247f8`): sem teto a 4 px → queda; ATIRAR → solta
  e atira; PULAR ou BAIXO (0x202) → solta. ESQ/DIR: sonda lateral de 4 px; testa a barra
  adiante com um segmento vertical de 4 px acima da quina dianteira da caixa
  (`0x00433aa3`), e de novo 3 px mais à frente; sem barra nos dois → queda; lado bloqueado
  → volta a 103; senão laço do 107 e vx do esqueleto com **k = 4** (48 px por ciclo de 16
  quadros, ≈ 3 px/tique). CIMA → 105.
- **105** (`0x00424ab4`, BRD `JK_BARUP`): puxar o corpo; anima o 105 até o fim → 104
  (`0x004246c4`): segura em cima (laço do 104) enquanto CIMA estiver apertado. No 105,
  PULAR/BAIXO/ESQ/DIR sem CIMA → 106; no 104, soltar CIMA → 106 (`0x00424bb8`, 105 ao
  contrário, ao acabar → 103); CIMA no 106 → volta ao 105. Em todos: sem teto → queda; ATIRAR → solta e atira;
  PULAR acabado de apertar → solta.

O 105/104 só muda a pose e a caixa (`JK_BARUP.BRD`): o Jake não sobe para cima da barra
[inferido: nenhum estado o põe sobre ela].

## Portas de camada (estados 11 e 12)

Descritas em [`escada-camadas.md`](escada-camadas.md) (contador `+0x268`, `+0x17c ± 1` nos
contadores 3 e 4, meia-volta, vista `0x00436635`), que confere com esta leitura. Acréscimos
[confirmado]: o estado 11 também se alcança do levantar (87, BAIXO + zona 2) e do andar
furtivo (2); a porta de saída (estado 13) usa o mesmo rótulo 12. O limite é a variável
`0x488e74` (`escada-camadas.md` a dá como `grid_z`).

## Em aberto

- Valores de `+0x220` além de 2 (e o bit 4 que faz `0x0040718d` ignorar o contato).
- `0x0040f040` (zona 0x20) e o estado 102.
