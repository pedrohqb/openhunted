# Personagem: laço, controle, física e colisão do Jake (engenharia reversa, M3)

Este documento descreve como o `Hunter.exe` faz o Jake ficar parado, correr, andar em
modo furtivo, pular, cair e colidir com o cenário. Tudo foi lido da desmontagem com o
radare2; nada aqui é código do original. Cada afirmação traz o endereço onde foi vista.

Convenção: **[confirmado]** = lido diretamente no código ou nos dados; **[hipótese]** =
interpretação plausível, ainda não verificada no jogo rodando. Unidades: "px" = pixel da
fase; "tique" = um passo do laço (50 ms); "1/8 px" = unidade interna de velocidade.

## 1. Laço principal e ritmo

### 1.1 Ritmo fixo de 50 ms (20 Hz) [confirmado]

- O laço do jogo é `0x0040433c` (chamado por `0x0040107c`). Ao fim de cada volta ele chama
  a espera `0x0044a356` com o argumento −50 (a soma com 50 só acontece se a variável
  `0x00471988` for zero; ela vale 1 nos dados e ninguém a escreve).
- `0x0044a356`: com argumento negativo, o prazo passa a ser o prazo anterior + 50 ms; com
  argumento ≥ 0, agora + argumento. Enquanto `timeGetTime()` não alcança o prazo, a função
  esvazia a fila de mensagens do Windows (`PeekMessage`/`DispatchMessage`; se a janela
  está minimizada, bloqueia em `GetMessage`). **Ao sair, regrava o prazo com o instante
  atual** (`0x0044a4fe`). Resultado: cada volta dura no mínimo 50 ms contados do fim da
  espera anterior; se a volta demorar mais, o jogo simplesmente fica mais lento — não há
  passos de recuperação nem passo variável.
- Logo, o tique da física, da animação e da IA é 50 ms. As chamadas de `timeSetEvent`
  (50 ms, `0x0044b7ca`/`0x0044c7c2`) são do som, como suspeitado em `cenario.md`.
- Contador de tiques: `0x004879c4`, zerado ao entrar no laço e incrementado uma vez por
  volta (`0x00404541`). O mostrador "frame rate" do menu de depuração (`0x00405bc6`, F1)
  mede voltas por segundo com `0x00461195` a cada 100 tiques.
- Há outro limitador, `0x00404dac` (espera ativa até 1000/n ms), sem chamadas encontradas.

### 1.2 Ordem das etapas dentro de um tique [confirmado]

Em `0x0040433c`, quando o jogo não está pausado (`0x0047119c` = 0):

1. `0x00449b2f`, `0x00448366(−1)`: leitura dos dispositivos de entrada (teclado/joystick).
2. `0x00435e7f`: ativação por distância — objetos perto da vista recebem o bit `0x80000`
   em `+0x178`; só eles têm física e atualização. O tipo 1 (Jake) usa raio 99999, isto é,
   está sempre ativo.
3. `0x004045a5`: **física** de todos os objetos ativos (gravidade, colisão, posição seguinte).
4. `0x00404827`: rotinas pós-colisão de cada objeto (`+0x1cc`, `+0x1c8`).
5. `0x00404842`: **resposta de colisão** sobre a velocidade (impulso, deslizamento, atrito).
6. `0x00434bf5`: **atualização** de cada objeto ativo (`+0x1c0`; no Jake, a máquina de
   estados `0x0042293c`).
7. `0x00404bab` → `0x00434c64`: **efetivação** da posição (a posição seguinte vira a atual).
8. Câmera, desenho (`0x004365ee` → `0x00435315`), som, contador de tiques, espera.

Consequência importante: a máquina de estados roda **depois** da física do mesmo tique e
antes da efetivação; as velocidades que ela grava valem para a física do tique seguinte.

## 2. Campos do objeto usados pelo personagem

Deslocamentos dentro da estrutura de objeto (a mesma de `objetos.md`).

| Campo | Significado | Fonte |
|---|---|---|
| `+0x14c`, `+0x150` | posição atual (x, y), px inteiros | efetivada em `0x00434c64` |
| `+0x154`, `+0x158` | posição seguinte | calculada em `0x004045a5` |
| `+0x15c`, `+0x160` | deslocamento do tique em 1/8 px (durante a física); depois da efetivação, deslocamento real em px | `0x004054cc`, `0x00433999`, `0x00434c64` |
| `+0x164`, `+0x168` | velocidade (vx, vy) em 1/8 px por tique | estados; `0x004054cc` |
| `+0x16c`, `+0x170` | deslocamento de desenho (compensação do movimento embutido na animação, ver §5.4) | `0x00406c71` |
| `+0x174` | massa (Jake: 10) | `0x0042287d` |
| `+0x178` | bits de estado; bit `0x8` = **sem gravidade**; `0x80000` = ativo | vários |
| `+0x17c` | camada de profundidade (z da célula) | fase |
| `+0x180` | estado da máquina (0–196) | `0x00406b6c` |
| `+0x184` | quadro corrente dentro do rótulo; `+0x188` nº de quadros; `+0x190` quadro absoluto | `0x00406b6c` |
| `+0x194` | espelhamento do sprite (bit 1) | `0x00434815` |
| `+0x198` | índice do controle (jogador) | fase |
| `+0x1ec` | id do rótulo da ANM em execução | `0x00406b6c` |
| `+0x1f0`/`+0x1f4` | escala de desenho (numerador/denominador) | `0x00434647` |
| `+0x208` | lista de formas de colisão (BRD do estado) | `0x00434ec1` |
| `+0x20c..+0x218` | caixa envolvente das formas (x, y, l, a) relativa à posição | `0x00434fc3` |
| `+0x220..+0x23c` | resultado de colisão do tique (abaixo) | `0x00433999` |
| `+0x288` | lado para onde olha: 4 = esquerda, 8 = direita | `0x00422a62` |
| `+0x294` | recarga do ataque (tiques) | `0x00422aa2` |
| `+0x29c`/`+0x298` | ponto de renascimento (x | y<<16) e camada | `0x00422914` |
| `+0x2a8` | cópia do controlador de esqueleto (§6) | `0x0042284c` |

Resultado de colisão (zerado no primeiro uso de cada tique por `0x004347cf`):
`+0x21c` objeto tocado por outro; `+0x220` bits (2 = cenário, 1 = objeto, 4 = contato
"fraco" ignorado pelos testes de chão/parede); `+0x224` o que foi tocado (célula ou objeto,
≠ 0 = houve contato); `+0x228`/`+0x22c` as duas formas envolvidas; `+0x230`/`+0x234` a
**normal de contato** (nx, ny), apontando do personagem para a superfície; `+0x238`/`+0x23c`
o deslocamento até o contato, em px.

## 3. Inicialização do Jake (`0x0042277e`) [confirmado]

Na primeira vez (bandeira `0x004788e0`):

- `0x00408683(1, 197)`: cria o **controlador de esqueleto** global nº 1 (guardado em
  `0x00488228[1]`) com 197 animações de 160 bytes. 197 = número de estados do Jake.
- `0x004085c6(0x00471ab8, 18, 3)`: copia uma tabela 18×3 de inteiros (uma linha por osso).
  Conteúdo: `2,5,8 / 3,11,15 / 4,11,15 / 0,0,0 / 2,1,6 / 7,0,0 / 0,0,2 / 1,9,10 / 0,0,0 /
  0,0,12 / 0,0,13 / 0,0,14 / 0,0,0 / 0,0,16 / 0,0,17 / 0,0,18 / 0,0,0 / 0,0,0`.
  [hipótese] ligações entre os 18 ossos (índices a partir de 1, 0 = nenhum), usadas para
  desenhar o esqueleto no modo de depuração (F4) e talvez em testes de acerto. Não é
  usada pelo movimento.
- `0x00409dff("JAKE.SKE")`: lê o esqueleto (§6). Em seguida fixa a animação 1, quadro 0,
  mede a caixa do esqueleto nesse quadro (`0x00408bb4`: 20 × 57 unidades) e a caixa do
  objeto na fase (`0x00408bf9`: 22 × 47 px, ver §7.3); guarda ainda o fator 12/20
  (`+0x18 = 12`, `+0x1c = 20`).

Em toda instância: copia o controlador para `+0x2a8`; nome "Jake Peril"; estado inicial 1
(`0x0042318e`); massa 10; `+0x1b4 = 120`; liga os bits `0x14801` e desliga `0x48` de `+0x178`
(gravidade ligada); rotinas `+0x1c0 = 0x0042293c` (atualização), `+0x1c4 = 0x0042315f`
(chamada no desenho), `+0x1cc = 0x00407796` (dano por pisão em outro objeto),
`+0x1d4 = 0x00407774` (filtro de contato: só aceita colisão com objetos de tipo 1–2 quando o
motivo é 1); arma inicial `0x0043a8cc(1)`; `+0x288 = 4` (olhando para a esquerda,
`0x004228fe`); guarda o ponto de renascimento.

Observação: o "stub de 77 bytes" citado antes não existe; `0x0042293c` é uma função de
~2 KB (vai até `0x0042315e`).

## 4. Entrada

### 4.1 Do dispositivo ao bitmask [confirmado]

Cada controle tem uma estrutura (`0x00499014[n]`, n = `+0x198`). Para o teclado
(`0x00448503`): eixo X = (DIREITA − ESQUERDA)·128 e eixo Y = (BAIXO − CIMA)·128, a partir
das teclas configuradas em `+0xb4..+0xb7` (CIMA, BAIXO, ESQUERDA, DIREITA); os botões
`+0x98..+0x9d` vêm das teclas seguintes. A ordem bate com a tela de configuração em
`SHELL.RES` (UP, DOWN, LEFT, RIGHT, SHOOT, JUMP, STEALTH, PREVIOUS WEAPON, PUNCH, NEXT
WEAPON). O estado anterior é copiado para `+0xa0..+0xb3` antes da leitura (bordas).

`0x00407e2d` converte para o bitmask usado pelo personagem (guardado em `0x00487a2c`):

| Bit | Significado | Regra |
|---|---|---|
| `0x1` / `0x2` | cima / baixo | Y < −50 / Y > 50 |
| `0x4` / `0x8` | esquerda / direita | X < −50 / X > 50 |
| `0x10`/`0x20`/`0x40`/`0x80` | cima/baixo/esquerda/direita **mudou** (apertou ou soltou) | comparação com o eixo anterior |
| `0x100` | ATIRAR | botão `+0x98` |
| `0x200` | PULAR | botão `+0x99` |
| `0x400` | FURTIVO | botão `+0x9a`, **ou** eixo inclinado mas não até o fim (|eixo| < 100) |
| `0x1000` / `0x800` | troca de arma (`+0x9b` / `+0x9c`) | [hipótese] anterior/próxima; `0x0043b0cb` as usa |
| `0x40000` | SOCO [hipótese] (`+0x9d`) | tratado junto com ATIRAR (`0x40100`) |
| `0x2000`, `0x4000`, `0x8000`, `0x20000`, `0x10000`, `0x80000` | botões acima mudaram | |

Detalhes: com joystick de menos de 4 botões, `+0x9b` = PULAR+direita+baixo e `+0x9c` =
PULAR+esquerda+baixo, com 20 tiques de bloqueio do PULAR depois disso (`0x00407e9e`). Com
`+0x19c` ≠ 0, saúde `+0x258` > 0 e o contador `+0x264` > 0, as direções são invertidas
(efeito de "confusão"), decrementando `+0x264`.

O teclado sempre dá eixo ±128, portanto o modo furtivo pelo teclado só vem da tecla
FURTIVO; no joystick analógico, inclinação parcial = furtivo.

### 4.2 Quem lê [confirmado]

`0x0042293c` chama `0x00407e2d` e depois `0x0042ec1c`, que devolve o mesmo valor para um
jogador humano e, para personagem controlado pelo computador (demonstração/IA), um bitmask
gerado pela IA (`0x0042d520`…) a cada `+0xdc` tiques. Os estados testam `0x00487a2c`.

Lado: se esquerda está apertada, `+0x288 = 4`; senão, se direita, `+0x288 = 8`
(`0x00422a62`); sem direção, mantém.

## 5. Máquina de estados

### 5.1 Atualização por tique (`0x0042293c`) [confirmado]

1. Sai se o jogo está congelado (`0x00471ba4`) ou o objeto tem o bit `0x2`.
2. Centro da caixa do Jake ÷ 128 dá a célula; se a célula **não existe** (vazia na fase,
   `0x004317db`): para o jogador (`+0x19c` ≠ 0) desfaz o último deslocamento; senão zera a
   saúde e remove o objeto (`0x00435b63`).
3. Se `tique mod (c+1) = 0` (c = campo da tabela de estados, §5.2), zera `+0x16c/+0x170`.
4. Lê a entrada (§4), liga o bit `0x4000`, atualiza o lado.
5. Se o bit `0x200000` (byte `+0x17a` & `0x20`) está desligado: decrementa a recarga
   `+0x294`; troca de arma (`0x0043b0cb`); depois tenta, nesta ordem, as transições
   globais — reações a dano (`0x004231f9`), **queda** (`0x0042415c`, §5.6), atordoamento
   por tempo `+0x274` (`0x00426f6a`) e temporizador `+0x284` que devolve ao estado 1
   (`0x004235c4`). A primeira que tratar encerra a etapa.
6. Senão, despacha o estado `+0x180` por uma tabela de bytes (`0x00422b09`, 197 entradas)
   que aponta para 62 casos (`0x00422bce`). A lista completa estado → rotina está em §5.3.
7. Para um grupo de estados "espelhados" (2, 6, 9, 18, 19, 22, 23, 49, 54–56, 62, 64,
   66–68, 73, 108, 109, 112, 122–125, 127, 128, 131, 141–144, 146, 147, 149–155, 186–188,
   193–196 — tiros, socos, arremessos, mira e o andar furtivo), inverte o sinal de
   `+0x16c` e de `+0x164`, e a regra de espelhamento do sprite é a oposta.
8. Ajusta o espelhamento (`0x00434815`): nos estados normais o sprite fica **espelhado
   quando olha para a direita** (os desenhos da ANM olham para a esquerda); nos espelhados,
   ao contrário. Ao espelhar, a posição é corrigida para que a caixa de colisão não salte.
9. `0x00408129`: piscar de invulnerabilidade (`+0x28c`).

### 5.2 Tabela de estados (`0x004732b0`, 197 × 112 bytes) [confirmado]

Cada registro: `+0x00` nome do grupo de esqueleto/sequência (ex.: `JKROOT`, `JKRUN1LO`,
`RTJUMP`), `+0x51` nome do BRD de colisão do estado, `+0x60` **id do rótulo da ANM**,
`+0x64` b (1 = animação dirigida pela ANM; 0 = dirigida pelo esqueleto, só nos estados 7
e 49), `+0x68` **c = tiques extras por quadro** (o quadro avança quando `tique mod (c+1) = 0`),
`+0x6c` cache do BRD (0). Ao mudar de estado (`0x0042318e` → `0x00406b6c`): grava o estado,
zera o quadro e a compensação de desenho, resolve o rótulo, sincroniza o esqueleto e troca
as formas de colisão pelo BRD do registro (`0x00434ec1`).

Avanço de animação: `0x00406c71(obj, modo)`, modo em bits — 1 = laço, 2 = para no último
quadro e devolve 1 ao terminar, 4 = de trás para frente, 8 = acompanhar o esqueleto.
Atalhos: `0x004238fa` (troca ou laço, modo 9), `0x00423835` (troca ou uma vez, modo 10, com
rotina de continuação ao terminar), `0x00423894` (uma vez de trás para frente, modo 14).

Estados de locomoção (os 197 estados completos estão no apêndice A):

| Estado | Rótulo ANM | Quadros | c | BRD | Papel | Rotina |
|---|---|---|---|---|---|---|
| 1 | 1 (`J001`) | 34 | 1 | `JK_DEFLT` | parado (laço, 10 quadros/s) | `0x00426ecc` |
| 6 | 146 (`J146`) | 8 | 0 | `JK_DEFLT` | parado, variante [hipótese: arma em punho] (`0x0042dc43` decide) | `0x00426ecc` |
| 10 | 10 (`J010`) | 12 | 0 | `JK_DEFLT` | **correr** (laço) | `0x00426254` |
| 2 | 149 (`J149`) | 9 | 0 | `JK_DEFLT` | **andar furtivo** (laço, espelhado) | `0x004261d0` |
| 3 | 3 (`J003`) | 7 | 0 | `JK_DEFLT` | virar de costas (para escada/porta/saída) | `0x00427089` |
| 7 / 8 | 7 (inexistente) / 3 | – / 7 | 0 | `JK_DEFLT` | voltar a ficar de frente (8 toca `J003` ao contrário) | `0x004270c9` / `0x004270e5` |
| 164 | 164 (`J164`) | 7 | 0 | `JK_DEFLT` | "esconder" (cima sem zona) | `0x00425e82` |
| 85, 86, 87 | 85, 86, 85 | 5, 12, 5 | 0 | `JK_CRAWL` | agachar/rastejar (baixo) | `0x00426519`, `0x004266a4`, `0x004265c6` |
| 27 | 27 (`J027`) | 26 | 0 | `JK_DEFLT` | **pulo parado** (PULAR sem direção) | `0x00424f49` |
| 16 | 16 (`J016`) | 26 | 0 | `JK_DEFLT` | **pulo vertical** (PULAR+CIMA), agarra o teto | `0x00424ca6` |
| 17 | 17 (`J017`) | 12 | 0 | `JK_DEFLT` | impulso para cima a partir de escada/teto | `0x00424dd0` |
| 29 | 29 (`J029`) | 9 | 0 | `JK_DEFLT` | **pulo médio** (balístico) | `0x0042568c` |
| 28 | 28 (`J028`) | 8 | 0 | `JK_DEFLT` | **pulo longo** (balístico) | `0x004254df` |
| 31 → 80 | 31, 80 | 2, 8 | 0 | `JK_DEFLT`, `JK_EXTND` | pulo estendido (impulso, depois voo) | `0x00425382`, `0x004253d3` |
| 35 / 34 | 35 / 34 | 3 / 9 | 0 | `JK_DEFLT` | aterrissagem do pulo médio / longo | `0x004259c6` / `0x00425946` |
| 25 | 25 (`J025`) | 1 | 0 | `JK_DEFLT` | **queda controlada** | `0x0042424d` |
| 26 | 26 (`J026`) | 11 | 0 | `JK_DEFLT` | aterrissagem da queda | `0x00424385` |
| 76 / 77 | 76 / 77 | 18 / 13 | 0 | `JK_EXTND` | queda e aterrissagem "descontroladas" (fim do pulo estendido) | `0x004242f6` / `0x004243aa` |
| 100 | 100 (`J100`) | 6 | 0 | `JK_DEFLT` | empurrar (correndo contra parede) | `0x00425c34` |
| 11, 12, 13 | 11, 12, 12 | 12 | 0 | `JK_DEFLT` | passar para a camada da frente / de trás | `0x004262c9`, `0x004263ee`, `0x00426820` |
| 4 | 18 (`J018`) | 1 | 0 | `JK_DEFLT` | pose especial com as 4 direções juntas (bitmask = `0xF`) | `0x00426f05` |

As imagens conferem: `J010` é corrida para a esquerda, `J016` pulo com os braços para
cima, `J027` agachar-e-saltar no lugar, `J029` salto em extensão, `J026` agachar ao cair,
`J085` agachar até o chão, `J164` de costas, `J011` andar de frente para a câmera.

### 5.3 Controle no chão (`0x00423940`) e transições [confirmado]

É chamado pelo estado parado (1, 6, padrão) e por quem volta ao chão. Ordem dos testes:

1. Bitmask exatamente `0xF` → estado 4.
2. ATIRAR ou SOCO (`0x40100`) com recarga zerada → ataque (`0x0043b43e`, que escolhe o
   estado conforme a arma; fora do escopo).
3. Esquerda/direita: sonda 4 px na direção (`0x0040718d`, §8.4); se bate em parede →
   empurrar (100); senão, com FURTIVO → estado 2; senão → correr (10).
4. Sem PULAR, com cima ou baixo: consulta as zonas da célula (§8.6):
   cima + zona 1 + camada > 0 → estado 12 (entra na camada de trás);
   baixo + zona 2 + camada < última → estado 11 (vem para a da frente);
   cima + zona 4 (escada), ou zona `0x40` para o jogador → virar (3);
   sem zona aplicável: baixo → agachar (85); cima → esconder (164).
5. PULAR: com cima → pulo vertical (16); sem cima → pulo parado (27).
6. Nada disso: fica parado (estado 1 ou 6).

Correndo (`0x00426254` com os testes de `0x00423db9`): bitmask `0xF` → 4; contato de
parede no tique (`0x004070d1`) → empurrar; ataque → para (zera velocidade e congela a
posição) e ataca; FURTIVO + direção → 2; sem PULAR e sem direção com cima/baixo → zonas
como acima; PULAR + CIMA + direção → pulo longo (28); PULAR + CIMA sem direção → pulo
vertical (16); PULAR sem CIMA → **escolha automática do pulo** (§5.5); baixo → agachar.
Soltando a direção, a velocidade é zerada e volta ao parado — **sem desaceleração**.

O andar furtivo (`0x004261d0`, testes em `0x00423f85`) tem a mesma estrutura.

### 5.4 Movimento: por código ou pela animação

Há três mecanismos, todos confirmados:

**(a) Velocidade fixa por código.** Correr: a cada tique `vx = ±12 px/tique` (constante
`0x00478928` = 12, gravada como 96 em 1/8 px) conforme o lado, e `vy` recebe −1 px/tique
(`0x0047892c` = −1, isto é −8 em 1/8 px), o que com a gravidade dá +1 px/tique² líquido
enquanto corre. Pulos balísticos (§5.5): velocidade inicial fixa.

**(b) Movimento extraído do esqueleto ("root motion").** `0x00406fc0(obj, k, modo|0x10)`
pega a diferença entre a posição da raiz do esqueleto no quadro atual e no anterior
(`0x00406ea2`, tabela de deslocamentos da animação no `.SKE`), converte para px com a
escala do esqueleto (x·22/20, y·47/57, divisões inteiras), inverte x quando olha para a
direita, inverte y (o esqueleto tem y para cima) e multiplica por k:

| Estado | k | Eixos |
|---|---|---|
| 2 (furtivo) | 3 | x (vy preservado) |
| 16 (pulo vertical), 27 (pulo parado) | 3 | x **e** y |
| 31 (impulso do pulo estendido) | 2 | x (vy preservado) |
| 34, 35 (aterrissagens) | 1 | x (vy preservado) |

Exemplos calculados com os dados: furtivo, deltas da raiz 1, 3, 5, 3, 3, 3, 3, 4 →
vx = 3, 9, 15, 9, 9, 9, 9, 12 px/tique (um ciclo de 9 quadros avança ~75 px). Pulo
vertical, deltas de y nos quadros 10–18 = 7, 17, 9, 4, 1, −3, −8, −17, −10 → vy = −15, −42,
−21, −9, 0, +6, +18, +42, +24 px/tique (antes da gravidade). Esses números saem da regra
acima; ainda não foram medidos no original.

**(c) Compensação de desenho.** Ao avançar o quadro (`0x00406c71`) o jogo também calcula a
posição **acumulada** da raiz no quadro corrente × escala do esqueleto × 12/20 e guarda em
`+0x16c/+0x170` (y invertido). O desenho (`0x00435315`) usa posição − compensação. Isso
anula o deslocamento que já está pintado nos quadros da ANM: na corrida, os quadros de
`J010` andam ~15 px de ANM por quadro (−153 no quadro 11, ×47/88 = −82 px), e a raiz do
esqueleto vale −123 × 1,1 × 0,6 = −81 px — o sprite fica preso à posição do objeto, que
anda pela velocidade de código. [confirmado na conta; o efeito visual é hipótese]

### 5.5 Pulos [confirmado salvo indicação]

Constantes (`0x0047890c`), em px/tique:

| Pulo | vx | vy inicial | Estado | Endereço das constantes |
|---|---|---|---|---|
| médio | 10 | −11 | 29 (`J029`) | `0x0047890c`, `0x00478910` |
| longo | 10 | −15 | 28 (`J028`) | `0x00478914`, `0x00478918` |
| estendido | 17 | −15 | 80 (`J080`) | `0x0047891c`, `0x00478920` |

Entrada nos estados 28/29/80: se o Jake não está a até 4 px do chão (sonda para baixo de
4 px, `0x0040718d`), vai para a queda (25); senão grava vx = ±vx·8 e vy = vy·8 (1/8 px).
Depois, a cada tique, toca a animação uma vez; ao terminar: no chão → zera a velocidade e
aterrissa (35 para o médio, 34 para o longo, 77 para o estendido); contra parede → som
`0xe80` e queda (25); outro contato → parado (1); sem contato, segue no ar parado no
último quadro. O estendido, ao terminar a animação no ar, passa a 76.
**Controle no ar** (28 e 29): apertar o lado oposto ao movimento **inverte vx na hora**,
exceto se o contato do tique é uma parede daquele lado.

Escolha automática (`0x00425144`, PULAR sem CIMA durante a corrida, ou nos primeiros 10
quadros dos pulos 16/27): na primeira chamada calcula o alcance de cada pulo como
vx · (2·|vy| / g) = 110, 150 e 255 px; então procura chão (`0x00407ac7` no cenário,
`0x00407a04` em objetos) num retângulo de meia largura do Jake, 3 px de altura, logo abaixo
dos pés, deslocado de ¼ da largura + alcance para o lado em que olha. Chão a 110 px → pulo
médio; senão a 150 → longo; senão a 255 → estendido (estado 31, que dá o impulso e passa a
80); senão → médio.

Trajetória real com a gravidade de 2 px/tique² e a integração do §8.1: o pulo médio sobe
9, 7, 5, 3, 1 px (pico de 25 px) e volta à altura inicial em 10 tiques (100 px na
horizontal); o longo sobe 13, 11, …, 1 (pico de 49 px) em 7 tiques e volta em 14 tiques
(140 px). [cálculo a partir do código]

Pulo vertical (16): exige chão; nos primeiros 10 quadros, PULAR+direção desvia para o pulo
longo (com CIMA) ou para a escolha automática; contato com teto do cenário → agarra
(estado 103, `0x00424498`, barra/teto: "JUMP + UP to grab the ceiling"); zona de escada +
cima/baixo → escalar (`0x0042710a`, estados 30/32); ao fim da animação → parado. A subida
vem do esqueleto (k = 3) e há controle no ar (`0x00408205`).
Pulo parado (27): mesma estrutura; PULAR+CIMA nos 10 primeiros quadros troca para o estado
16 mantendo o quadro.
Estado 17: zera a velocidade e desliga a gravidade até o quadro 9, então vy = −20 px/tique
(`0x00424ee2`) e religa a gravidade; usado a partir de escalada (`0x0042710a`).

### 5.6 Queda e aterrissagem [confirmado]

- Detecção (`0x0042415c`, toda a vez antes do estado): se a gravidade está ligada e o
  deslocamento vertical do tique é ≥ 40 (1/8 px), isto é **≥ 5 px/tique para baixo**, vai
  para a queda (25). Ficam isentos os estados 16, 17, 25, 27–29, 76, 80–82; nos estados de
  mira/tiro (23, 54–56, 66–68, 122–125, 141–144, 150–155, 193–196) a queda só ocorre se o
  jogador não estiver atirando.
- Queda (25, `0x0042424d`): se toca chão (normal de chão, §8.5) zera a velocidade e vai para
  26, que toca `J026` uma vez e volta ao parado (pode ser interrompida pelo controle no
  chão). Ataque permitido; zona de escada + cima/baixo agarra. Liga a gravidade.
- Controle no ar (`0x00408205`, usado na queda e nos pulos 16/17/27): sem chão, esquerda
  com vx > −8 px/tique → vx = −8 px/tique, ou 0 se uma sonda de 8 px acusar parede; direita
  com vx < 8 → vx = +8 (a sonda da direita também é feita para a **esquerda**, −8 px — o
  que parece um erro do original).
- **Velocidade terminal: não existe** para o Jake. Nenhum código limita `vy`; só a resposta
  de colisão (`0x0040550a`) troca para uma escala menor quando a velocidade relativa passa
  de 75, para evitar estouro.
- Sair da grade da fase pela física (`0x004045a5`): objeto comum é removido; o jogador
  volta ao ponto de renascimento com velocidade zero.

## 6. `JAKE.SKE` (esqueleto)

Formato lido por `0x00409dff` [confirmado]; inteiros little-endian `s16`:

| Campo | Conteúdo |
|---|---|
| texto | `"Hunter Hunted Skeleton File.\r\n\0"`, lido até o byte `0x1A` |
| id | tem de ser igual ao do controlador (Jake 1, `MINOTAUR.SKE` 2) |
| n | número de animações (Jake 197, Minotauro 164) |
| largura, altura | caixa de referência (Jake 20 × 57, Minotauro 28 × 50) |
| n × animação | ver abaixo |

Animação: `s16` quadros q; `s16` ângulo (rotação em torno do eixo vertical, 65536 = volta
inteira); `s16` ox, `s16` oy; q × (`s16` x, `s16` y) **posição da raiz por quadro**;
`s16` ossos m (18 no Jake); q × m × (`s16` x, y, z) pontos dos ossos. O arquivo é lido sem
sobra. A animação de índice i corresponde ao **estado i** (não ao rótulo da ANM), com o
mesmo número de quadros na maioria dos estados (há exceções: estado 1 tem 68 quadros de
esqueleto para 34 de ANM).

Projeção (`0x004094b2`/`0x004094ef`): gira (x, z) pelo ângulo com tabela de cossenos Q14
(`0x00483f74`, 2049 valores, `0x00467154`/`0x0046717a`) e usa o x girado e o y. A caixa de
um quadro (`0x00409a4d`) soma a raiz do quadro e (ox, oy).

O controlador precisa do esqueleto para o movimento? **Em parte, sim.** A corrida e os
pulos balísticos não dependem dele; mas o andar furtivo, os pulos vertical e parado, o
impulso do pulo estendido e o deslize das aterrissagens tiram a velocidade das posições da
raiz (§5.4 b), e a compensação de desenho também (§5.4 c). Os pontos dos ossos (x, y, z)
só servem, até onde foi visto, para a caixa de escala e o desenho de depuração. Uma
reimplementação pode pré-calcular, por estado, a tabela de deslocamentos da raiz já em px
e dispensar os ossos.

## 7. Posição, escala e caixa

### 7.1 Escala do objeto [confirmado]

O leitor de objetos (`0x00434647`) começa com escala 1/1, lê os dois últimos `i32` do
registro do objeto (largura L e altura A desejadas) e, se L ≥ A, usa escala L / largura do
quadro de referência; senão A / altura. No Jake das fases, L = 22 e A = 47, e o quadro 0 de
`J001` tem 42 × 88: escala **47/88 ≈ 0,534** (sprite de 22 × 47 px). Só depois disso chama
a inicialização do tipo.

### 7.2 Origem [confirmado]

A posição (`+0x14c/+0x150`) é o **canto superior esquerdo** do retângulo de referência do
quadro (o retângulo do quadro × escala + posição, `0x0043585c`). As formas do BRD são
relativas a esse ponto, já em px da fase (não são escaladas). Os pés do Jake ficam em
y + 47 (fundo da caixa de `JK_DEFLT.BRD`).

### 7.3 Formas de colisão do Jake [confirmado]

`JK_DEFLT.BRD` é um octógono (vértices (2,6) (8,4) (14,4) (20,6) (20,43) (14,46) (8,46)
(2,43), cantos chanfrados como arestas do tipo 3) mais três caixas: corpo (2,6) 18×37,
topo (8,3) 6×3, base (8,43) 6×4. Caixa envolvente: x 2–20, y 3–47. Os chanfros fazem o
personagem escorregar por quinas. Outros estados trocam para `JK_EXTND` (2,30 32×17),
`JK_COLPS` (2,30 18×17), `JK_CRAWL` (2,24 18×23) e `JK_BARUP` (2,3 18×16).

## 8. Física e colisão

### 8.1 Integração [confirmado]

Por tique, para cada objeto ativo (`0x004045a5`):

1. Se o bit `0x8` de `+0x178` está desligado, soma a gravidade (`0x00471978/0x0047197c` =
   (0, 2) px/tique², isto é +16 em 1/8 px) à velocidade (`0x00404bc7`, `0x004054cc`).
2. Deslocamento do tique = velocidade (1/8 px).
3. Colisão com objetos próximos (`0x00437614`, `0x00436caf`) e com o cenário (`0x00432e5d`),
   que podem encurtar o deslocamento.
4. Posição seguinte = posição + deslocamento **deslocado 3 bits à direita** (divisão por 8
   arredondando para baixo). **Não há acumulação de frações**: a parte fracionária se perde
   a cada tique (vx = +7 não anda; vx = −7 anda −1 px).

### 8.2 Varredura contra o cenário (`0x00432e5d`) [confirmado; matemática fina não detalhada]

- Toma a caixa do personagem estendida pelo deslocamento e percorre as células (128 px)
  que ela cobre, na camada do personagem. Para cada célula (`0x004317db`) cujo retângulo
  deslocado toca a varredura (`0x004338df`), testa cada forma do personagem contra cada
  forma do BRD da célula (`0x0043339c`), com as formas da célula espelhadas conforme a
  célula (`+0x40`, em torno de 0,0–128,128) e as do personagem conforme `+0x194`.
- Cada contato encontrado **encurta** o deslocamento até o ponto de contato menos 1/8 px
  (`0x00433999`, que também grava o resultado de colisão, §2); a busca continua com o
  deslocamento reduzido e para se ele zerar. Fica o contato mais próximo. Não há
  deslizamento dentro do mesmo tique: o deslizamento vem da resposta (§8.3) no tique seguinte.
- O teste forma × forma é um teste varrido (tempo de impacto) entre círculo (tipo 1 na
  memória, não usado pelos dados), caixa (2) e aresta (3), com auxiliares `0x00405820`,
  `0x00405775`, `0x004057c4`, `0x00405863` e `0x00438d55`. A matemática exata não foi
  transcrita (ver pendências).

### 8.3 Resposta de colisão (`0x00404842`) [confirmado]

Para objetos com contato no tique (não "fraco", gravidade ligada):

1. Impulso (`0x0040550a`) entre o personagem (massa `+0x174`, Jake 10) e o que foi tocado
   (cenário = massa 8000 e velocidade zero; objeto = massa e velocidade dele), ao longo da
   normal, só se estão se aproximando: é praticamente uma reflexão da componente normal.
2. Retira a componente normal da velocidade (projeção). Se a componente normal for maior
   que 32 (4 px/tique), retira só metade — sobra um quique de ~metade da velocidade.
   Quando a normal é oblíqua, só faz isso se |v| ≥ 50 (`0x00471994`).
3. Atrito: em cada eixo, se |v| > 7 subtrai 7 na direção de v; senão zera (em 1/8 px).

Na prática, parado ou correndo no chão a vy volta a 0 a cada tique; os estados zeram
explicitamente a velocidade (`0x00407009`) ao parar, ao aterrissar e ao atacar.

### 8.4 Sondas [confirmado]

`0x0040718d(obj, dx, dy)` simula um deslocamento de (dx, dy) px com a colisão de objetos e
de cenário, restaura a velocidade e o deslocamento, e devolve 1 se houve contato não
"fraco". O resultado do contato fica em `+0x220..` para os testes abaixo.

### 8.5 Classificação do contato [confirmado]

Com a normal (nx, ny) apontando do personagem para a superfície:

| Teste | Condição | Função |
|---|---|---|
| chão | ny > 0 e 2·|nx| < ny (inclinação < ~26,6°) | `0x00407031` |
| teto | ny < 0 e |ny| > 2·|nx| | `0x0040712f` |
| parede | |ny| < 2·|nx| | `0x0040707c`; sem contato "fraco": `0x004070d1` |

### 8.6 Tipos nos dados: BRD e zonas

- **BRD** (formas sólidas): nos 225 arquivos da versão 1.00 os tipos são **2 = caixa**
  (378), **3 = aresta** (12) e **4 = vértice** (12), mais três registros 0 vazios
  (`OCH0..2.BRD`). Todas as caixas das células são tipo 2. A aresta tipo 3 vai do ponto do
  registro anterior até o seu; os campos de largura/altura dela são recalculados como a
  normal (y2−y1, x1−x2) (`0x00430c4a`/`0x004053c2`). O tipo 4 marca o vértice e é ignorado
  na colisão. Não há distinção entre piso, parede e plataforma: tudo é sólido por todos os
  lados, e a classificação vem só da normal. Não foram vistas plataformas de mão única.
  (`docs/formats/brd.md` diz "1 ou 2 nos dados"; a contagem acima corrige isso.)
- Os **tipos 1 e 2** que o controle consulta são **zonas** da lista PR de cada célula
  (`+0xd4`, leitor `0x0042ff4c`), testadas por sobreposição de retângulos em `0x004075af`
  (OR dos tipos de todas as zonas que tocam a caixa do Jake, espelhadas com a célula).
  Nas 100 fases: 4 (2421), 1 (621), 2 (616), 64 (306), 32 (158).

| Zona | Uso | Endereço |
|---|---|---|
| 1 | porta para a camada de trás (CIMA) | `0x00423a1a` |
| 2 | passagem para a camada da frente (BAIXO) | `0x00423a4a` |
| 4 | escada/escalada (CIMA/BAIXO) | `0x00423a7c`, `0x004256a8` |
| 64 | com o jogador, virar de costas e chamar `0x0040e4dd` [hipótese: saída/porta especial] | `0x00423ba4` |
| 32 | não visto no controle | — |

## 9. Constantes (resumo)

| Grandeza | Valor | Onde |
|---|---|---|
| Tique | 50 ms (20 Hz), sem recuperação | `0x00404580`, `0x0044a356` |
| Unidade de velocidade | 1/8 px por tique | `0x004054cc`, `0x004046a0` |
| Gravidade | 2 px/tique² (16/8) | `0x0047197c` |
| Correr | 12 px/tique, sem aceleração | `0x00478928` |
| Ajuste vertical ao correr | −1 px/tique por tique | `0x0047892c` |
| Pulo médio | (10, −11) px/tique | `0x0047890c` |
| Pulo longo | (10, −15) | `0x00478914` |
| Pulo estendido | (17, −15) | `0x0047891c` |
| Impulso vertical (estado 17) | −20 px/tique | `0x00424ee2` |
| Controle no ar | ±8 px/tique | `0x00408205` |
| Limiar de queda | 5 px/tique para baixo | `0x0042416f` |
| Sonda de chão / parede | 4 px | `0x004256d1`, `0x004239b7` |
| Atrito de contato | 7/8 px/tique² por eixo | `0x00404b2b` |
| Massa do Jake / do cenário | 10 / 8000 | `0x0042287d`, `0x0040497c` |
| Escala do Jake | 47/88 | dados da fase |
| Escala esqueleto → px | x 22/20, y 47/57; compensação × 12/20 | `0x00406ea2` |
| Velocidade terminal | nenhuma | — |

## 10. Pendências

- Matemática exata do teste varrido forma × forma (`0x0043339c`, `0x00438d55`,
  `0x00405820`) e da colisão entre objetos (`0x00436caf`). Para a primeira fase, uma
  varredura caixa × caixa com a regra "encurtar até o contato − 1/8 px" deve reproduzir o
  comportamento contra células, que só têm caixas.
- Conferir no original (Wine, com o menu de depuração F2 "Physics") os números derivados
  do esqueleto (furtivo, pulo vertical) — o multiplicador 3 parece alto.
- Significado de `0x0042dc43` (escolha entre os estados 1 e 6), dos estados de escalada
  (30, 32, `0x0042710a`), barra (103–107), empurrar (100–102), mudança de camada (11–13),
  esconder (164–166) e da zona 32.
- Papel exato da tabela de ossos `0x00471ab8` e do esqueleto no desenho (`0x004082cc`,
  modo "Use Skeleton" `0x00471984`) e nos estados 7 e 49 (animação dirigida pelo esqueleto).
- `+0x19c`: [hipótese] número do jogador humano (1, 2…), atribuído em tempo de execução;
  nas fases vale 0.
- Ataques (`0x0043b43e`, `0x0043ba54`…) e reações a dano (`0x004231f9`) ficaram de fora.

## Apêndice A: os 197 estados

Formato: estado: rótulo ANM (BRD, c) — rotina. Estados sem rótulo (0, 5, 14, 20, 21, 24,
33, 36–48, 50–53, 57–61, 63, 65, 69–72, 74, 75, 88–99, 110, 111, 113–121, 126, 129, 130,
132–140, 145, 148, 156–161, 167, 170, 172, 174, 176, 178, 180, 182–185, 189–192) não
são usados e caem na rotina padrão `0x00426ecc`.

1: 1 (DEFLT, 1) `0x00426ecc` · 2: 149 (DEFLT) `0x004261d0` · 3: 3 `0x00427089` ·
4: 18 `0x00426f05` · 6: 146 `0x00426ecc` · 7: 7 (b = 0) `0x004270c9` · 8: 3 `0x004270e5` ·
9: 9 `0x0043b895` · 10: 10 `0x00426254` · 11: 11 `0x004262c9` · 12: 12 `0x004263ee` ·
13: 12 `0x00426820` · 15: 15 (c 1) `0x0042705f` · 16: 16 `0x00424ca6` · 17: 17 `0x00424dd0` ·
18: 18 `0x0043b99b` · 19: 19 `0x0043ba0b` · 22: 22 `0x0043ba54` · 23: 23 `0x0043b50b` ·
25: 25 `0x0042424d` · 26: 26 `0x00424385` · 27: 27 `0x00424f49` · 28: 28 `0x004254df` ·
29: 29 `0x0042568c` · 30: 30 `0x0042710a` · 31: 31 `0x00425382` · 32: 32 `0x0042710a` ·
34: 34 `0x00425946` · 35: 35 `0x004259c6` · 49: 49 (b = 0, c 1) `0x00426ecc` ·
54–56: 54–56 `0x0043b50b` · 62: 62 `0x0043b99b` · 64: 64 `0x0043ba0b` ·
66–68: 66–68 `0x0043b50b` · 73: 73 (c 1) `0x0043ba54` · 76: 76 (EXTND) `0x004242f6` ·
77: 77 (EXTND) `0x004243aa` · 78: 78 (COLPS) `0x00424421` · 79: 79 (c 1) `0x00426f41` ·
80: 80 (EXTND) `0x004253d3` · 81–84: 81–84 (COLPS) `0x00425a46`, `0x00425af4`,
`0x00425b99`, `0x00425bdf` · 85: 85 (CRAWL) `0x00426519` · 86: 86 (CRAWL) `0x004266a4` ·
87: 85 (CRAWL) `0x004265c6` · 100: 100 `0x00425c34` · 101: 101 `0x00425d04` ·
102: 100 `0x00425ced` · 103: 103 `0x00424498` · 104: 104 (BARUP) `0x004246c4` ·
105: 105 (BARUP) `0x00424ab4` · 106: 105 (BARUP) `0x00424bb8` · 107: 107 (c 1) `0x004247f8` ·
108: 108 `0x0043b99b` · 109: 109 (c 1) `0x0043ba0b` · 112: 112 (c 1) `0x0043ba54` ·
122–125: 122–125 `0x0043b50b` · 127: 127 `0x0043b99b` · 128: 128 (c 1) `0x0043ba0b` ·
131: 131 (c 1) `0x0043ba54` · 141–144: 141–144 `0x0043b50b` · 146: 146 (c 1) `0x0043b99b` ·
147: 147 (c 1) `0x0043ba0b` · 149: 149 `0x0043b43e` · 150–155: 150–155 `0x0043b50b` ·
162: 162 (EXTND) `0x00425839` · 163: 163 (COLPS) `0x004258b9` · 164: 164 `0x00425e82` ·
165: 165 (sem BRD) `0x00426020` · 166: 164 `0x00425f8a` · 168: 168 `0x00426be9` ·
169: 173 (COLPS) `0x00426c5e` · 171: 171 (COLPS) `0x00426e36` · 173: 173 (COLPS)
`0x00426da2` · 175: 173 (COLPS) padrão · 177: 77 (COLPS) `0x00426ce8` · 179: 179 (COLPS)
`0x00426d45` · 181: 181 `0x00426b09` · 186: 186 `0x0043b99b` · 187: 187 (c 1) `0x0043ba0b` ·
188: 188 (c 1) `0x0043ba54` · 193–196: 193–196 `0x0043b50b`.

BRD omitido = `JK_DEFLT.BRD`; c omitido = 0.
