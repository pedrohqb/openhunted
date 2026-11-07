# Objetos hostis menores (2): Morte, Camaleão, formiga, Entidade, voadores

Continua [`inimigos-menores.md`](inimigos-menores.md) e [`anticorpo-manta.md`](anticorpo-manta.md).
**[confirmado]** = lido no código/dados; **[inferido]** = dedução.

## Morte (tipo 3, `ODE.ANM`) — máquina de estados [confirmado]

5 na campanha solo (`D_J_009`, `D_J_012`, `D_M_010`, `H_J_027`, `I_J_016`). Tique `0x415855`:

1. `+0x27c ≠ 0` (já morreu) → nada. `+0x278 = 0`: espera os gatilhos (`0x435bd8`); verdadeiros
   (ou sem gatilhos) → `+0x278 = 1` para sempre.
2. **Estado −1 (oculta**, `+0x178 |= 0x2`: sem colisão, sem radiação): sem nenhuma das falas
   3760–3763 tocando, com `rand%300 < 2` e `+0x264 > 30`, fala (40 % a 3763; senão uma sequência
   de 2–3 entre 3760/3761/3762, `0x43c868`). `+0x264 > 0` → −1 e sai. Zerado: **aparece**
   (`0x416251`); se não conseguir, rearma `+0x264` (`0x416753`).
3. Fora do −1: no estado 1 limita `vy ≥ −10`; dano (`0x416979`: 0 não faz nada; fácil ×2, difícil ÷2, só o negativo vira 1);
   vida ≤ 0 → estado 11 (`ode10`), liga `0x40`, fala 3764. Queda rápida (`+0x160 > 150` ou
   `+0x168 ≥ 150`) fora do 11 → estado −2 e som 3051. Depois a **radiação** (`0x416a48`) e a
   animação do estado (1–11: rótulo = estado, uma vez; −2: rótulo 1 ao contrário).

Temporizador `0x416753`: `+0x264 = (2 − dif)·75 + rand%100` (fácil 150–249, normal 75–174, difícil
0–99 tiques).

Aparição `0x416251`: sorteia um jogador vivo (até 9), passa à camada dele, `+0x268` = alvo; põe
a Morte colada ao lado da caixa do alvo (`0x416517`, pés alinhados, y −5) e tenta deslocá-la
10–19 px (lado sorteado) com a varredura `0x432e5d`; procura chão (`0x416646`: varredura de 15 px
para baixo) recuando de 10 em 10 px; até 2 tentativas, trocando de lado. Fora da fase/sem célula
(`0x4165d5`) → falha. Sucesso: vira-se para o alvo, desliga `0x2`, estado 1, som **3051** "Death
ascending".

Fim de animação (`0x415d63`):

| Estado | Rótulo (quadros) | No fim |
|---|---|---|
| −2 | 1 ao contrário | ½: −1 (oculta); ½: reaparece (`0x416251`) ou −1 |
| −1 | — | liga `0x2` |
| 1 | 1 (9) | com alvo: ⅓ −2 (som 3051), ⅓ 5, ⅓ 7; sem alvo: −2 |
| 5 | 5 (9) | **tiro** (`0x416696`) → 6 |
| 6 | 6 (6) | como o 1 |
| 7 | 7 (7) | **tiro** → 8 |
| 8 | 8 (4) | ⅓ −2, ⅓ 7, ⅓ 5; sem alvo: −2 |
| 2, 3, 4 | 2 (5), 3 (12), 4 (5) | 2: ½ 4, ½ 3; 3 → 4; 4: como o 1 |
| 11 | 11 (15) | `+0x27c = 1`, destruída |

Alvo válido (`0x41678d`): vivo, `|dy| ≤ 40`, mesma camada, `|dx| ≤ 1000`, linha livre; senão
esquecido. Os estados 2–4 só se entram pelo 4º caso de `rand % 3` (`0x41693b`: investida
`vx = ±20` px/tique e estado 2), que nunca sai: código morto do original.

Tiro (`0x416696`): bala 99 de `(x+30 | x−20, y+2)` (pelo espelhamento), `vx = ±20`, `vy =
rand%s − s/2`, `s = max(10, (2−dif)·15/2)` (fácil 15, demais 10). Linha `(99, 0)`: 15, empurrão,
efeito sorteado raio 28. Radiação: todo objeto com `0x80000` na camada com `|Δx|`, `|Δy|` < **45**
(campos `+0x154/+0x158`) recebe `0x43d629(morte, obj, 0)`; linha `(3, 1)`: **4** só no jogador,
empurrão (3, 0); no jogador toca 3156. Vale também nos estados −2 e 11.

## Camaleão (tipo 4, `OCH.ANM`) [estrutura confirmada; papéis de estados em parte inferidos]

19 na campanha solo (`H_J_006/009/013/024/027`, `D_M_010/027`, `I_M_014`, `T_M_006`).
Variantes (`0x47256c`, registros de 0x38: nome de 0x14 bytes e depois):

| Desl. | Uso | Alpha | Beta | Gamma | 3 | 4 |
|---|---|---|---|---|---|---|
| `+0x14` | alcance de percepção (`|dx|`, px) | 250 | 300 | 250 | 250 | 250 |
| `+0x18` | vida padrão | 80 | 50 | 80 | 80 | 80 |
| `+0x1c`/`+0x20` | cor normal `0x43becd(+0x1c, +0x20)` | 45/−10 | 42/0 | 37/−60 | 43/−30 | 38/−30 |
| `+0x24` / `+0x2c` | não seguidos | 70/130 | 100/130 | 70/130 | 70/130 | 70/130 |
| `+0x28` | → `+0x174` | 10 | 5 | 10 | 10 | 10 |
| `+0x30` | pode fugir por porta | 1 | 1 | 1 | 0 | 0 |
| `+0x34` | cor do clone | 37 | 45 | 42 | 44 | 39 |

Tique `0x418704`: fora da fase/sem célula → vida 0; sons ambientes 5120/5121/5122 (`rand%200 <
2`); clonagem (abaixo); alvo `0x41918b` = jogador mais próximo que passa em `0x419242` (vivo,
mesma camada, `|dx| ≤ +0x14`, linha livre). Dano pendente sem veneno/explosão (`0x419f89`): nos
estados 8, 9, 30–33 → 10; senão, fora do 12 e sem recarga (`+0x26a`) → 12 (recarga 40). Dano
`0x419fac` com dificuldade (×2/÷2). Vida ≤ 0 → estado **18**, som 5119, `0x40`, mata os clones
(`0x41b02c`). Queda com `vy ≥ 40` → 26 (ou 37); aterrissa → 27/38; parado 13 tiques no mesmo x
(`0x41a6cd`) → estado 3 e `vy += 10`.

Decisão com alvo (`0x41a79e`, chamada no fim dos estados 6, 11–13, 27, 34, 38):

1. 10 %: vida ≤ 30 e `+0x30` → zona PR 1/2 → estados 39/40 (troca de camada); senão 34.
2. `|dy|` entre centros ≤ 30 (`0x41a4ca`): de costas → 28 (virar); de frente: **85 % → estado
   13 e mordida** (`0x41a624`): se a distância entre centros ≤ **50**, põe-se no alvo (y −15) e
   `0x43d629(camaleão, alvo, 0)` → linha `(4, 1)`: **30** de dano, natureza `0x10`, som **3147**
   (só jogadores); 10 % → 7; senão 1.
3. Longe: de costas → 7 (30 %, com sonda para cima de 128 ou 256 px) ou 28; de frente e
   caminho livre (`0x41a22c`) → 4 (anda) e talvez inicia clonagem; senão 28.

Fim do 28 (virar): 20 % → **invisível** (`+0x265 = 1`, som **3155**); senão 25 % → visível
(`+0x265 = 0`). Também voltam a 0: a contagem da clonagem e a entrada no 18; o fim do 18 põe 1
(o corpo esmaece). Desenho `0x419391` (só no original): o limiar `+0x268` desce 7 por quadro até
0x91 (com `+0x265 = 1`) ou sobe até 0xf7; os índices ≥ limiar da tabela `0x481048` são zerados
[inferido: tabela de transparência por cor].

**Clones**: só o original (sem `0x10`), < 3 clones: 25 % ao andar, `+0x26e = 50 + rand%50`,
`+0x26d = rand%5`; a cada 3 tiques a cor cicla (`0x47277c`) e `+0x26e` −1; zerado → `0x41aec0`
cria um tipo 4 com `0x10`, mesma variante, **metade da vida**, cor `+0x34`; som 3154
"chameleon spawns".

## Natureza `0x10` [confirmado]

Nenhum teste de `+0x252 & 0x10` no executável (todos os acessos: bits 1, 2, 4, 5, 6, 0x20).
É inerte: dano comum, absorvido pela armadura. Só impede, nesse tique, que o veneno marque o
bit 1 (`0x42334f`/`0x42851a` só o fazem com natureza 0).

## Formiga (tipo 109, `OAN.ANM`) e formigueiro (88, `OAH00`) [confirmado]

"Alpha…Delta Ant" (`0x472fb8`, registros 0x1c: alcance 60, vida 1). Nenhuma posta nas fases solo
(só em `C_*`, `M_M_027`); o formigueiro 88 (`0x4149a0`, em `D_M_030` ×2, `T_M_008`) cria
tipos 109 (`0x41fe70`). Vida 1: qualquer dano → `0x43d629(f, nulo, 4)` (fumaça raio 15) e some.
Estados (tique `0x41ffe4`, fins de rótulo `0x420313`): 2 anda (`0x4207c3` refaz vx = ±0x40
a cada tique, vy = −8); andando, **3 tiques no mesmo x** (`+0x274`/`+0x27a`) → 3 (vira,
parada); no fim de cada passo, `0x42081d` desloca a caixa 0x40 à frente e vira se não há chão
(`0x420979`: sonda vy 0x78) ou se toca um objeto que não é formigueiro nem jogador (outra
formiga: 50 %); o fim do 3 troca o lado (`+0x194 ^= 1`). Ao tocar um tipo 88 (fora de 1, −1
e 6) → 6; o fim do 6 desloca x ±0x14 e passa a −1 (oculta, `0x2`) por `0x55 + rand%50 − 25`
tiques; daí 1, com o lado trocado. 4/5 alternam na mordida (o fim do 4 morde). Criada pelo
formigueiro (`0x414bd4`) na posição dele, olha para o lado contrário ao bit 0 do `+0x194`
dele. Alvo = jogador a ≤ 60 px, mesma camada, linha livre → morde:
`0x43d629(f, alvo, 0)` → linha `(109, 0)`: **5**; sons 3151 (10 % ao andar) e 3153 (mordida).
Ao tocar um tipo 88 → estado 6. Derrubada: vx = ⅔ do empurrão. Contra ela: soco/garras 8.

Formigueiro, tique `0x414aed` [confirmado]: sem `0x40`, testa os gatilhos (`0x435bd8`) e
liga; ligado, zera a velocidade e, se `+0x264` (restantes: `values[1]`, 0 → `+0x18` da variante
`0x47234c`: 9/5/3; −1 = sem fim) ≠ 0, decresce `+0x26c`; ao zerar, `+0x264 −1` (salvo −1),
`+0x26c = +0x268` (`values[2]`, 0 → `+0x14`: 100/50/55) e `0x414bd4` cria uma formiga na
posição dele, na camada dele, variante `+0x180 − 1` (campo 5), `|= 0x11`, tamanho 40.

## Anticorpo × Manta: o byte `+0x278` [confirmado]

O tique do Anticorpo (`0x41129d`) sai logo se `+0x278 ≤ 0` (o byte baixo de `values[6]`): o
objeto fica parado e intocável; só a saída da fase o zera depois. Nada mais o põe em 1 (as
duas escritas de 1 no executável são da formiga e da mina). A Manta (`0x421475`) testa a vida
no lugar desse byte — nos dados solo todos os Anticorpos têm 1 e todas as Mantas 0.

## Entidade (107): zona do alvo [confirmado]

`0x41299d`: `0x436635(alvo)` acha na lista `0x493b70` (por `+0x58`) o registro de vista com
`+0x50 = alvo`; o retângulo `+0x30` passa por `0x43695c` (camada `+0x08`) e se testa contra a
caixa da Entidade (`0x405698`). Logo a confusão só vale **com a Entidade dentro da vista
(tela) do jogador-alvo** [inferido: registro = câmera do jogador].

## Anticorpo e Manta: trajetórias [confirmado]

- Passeio `0x42221b`: `v = (cos, sen)(+0x274)·(+0x2c) >> 11`, limitado só por cima a `+0x34`;
  a cada tique o ângulo gira **±0x1000 (22,5°)** (sentido `+0x27a`). O "ricochete" contado
  contra `+0x28` é uma **volta completa** (16 tiques): o passeio são laços.
- `0x421e9b(obj, ponto)` (Anticorpo: `0x411cff`): `v = (ponto − centro)/2`, limitado só por
  cima a `+0x34`; depois cada componente **< 10 dobra**: o código chama `abs()` (`0x40d1fd`)
  sobre o booleano `v < 10`, logo toda componente negativa também dobra (o limite −32…37 é
  só do vx, depois). A velocidade sai sempre; devolve "chegou" quando a distância < largura + 15.
- Etapa inicial (`+0x273`, `0x411600`/`0x421866`): posição x do alvo < a do voador → 4,
  senão 1.
- Manta, perseguição `0x422031`: ponto-base = **centro x** da caixa do alvo e topo −
  `rand%10` (sorteado a cada tique). Etapas: 1 base − 60 → 2 base → 3 base + 75 (alvo à
  direita); 4 base + 75 → 5 base → 6 base − 60 (alvo à esquerda). Ao chegar ao fim (3 ou 6),
  `0x4223e2` espelha a casa em x pela posição atual (y −`rand%10`) e modo 5 (volta); a
  velocidade do tique ainda é a do ponto. Com o alvo a < 40 px (`0x4225ab`) e fora dos
  estados 4/5 → estado 4 (bote), sem sair da perseguição; a mordida sai no fim do 4.
- Anticorpo, perseguição `0x411e95`: ponto-base = **centro** da caixa do alvo; etapas 1 −75 →
  2 centro → 3 +75 / 4 +75 → 5 centro → 6 −75; ao chegar ao centro (2 e 5) **morde**
  (`0x412249`) e segue; no fim, `0x41222b` espelha a casa só em x e modo 5.
- Tempo da perseguição (`+0x276`): só sobe com `rand & 1` (metade dos tiques) [não portado:
  a porta conta todo tique].

## Bits de célula `0x2/0x8/0x10/0x40` [confirmado pela busca de caminho]

`0x2` fechada embaixo, `0x8` parede à direita, `0x10` parede à esquerda, `0x40` fechada em
cima (ver [`minotauro-ia.md`](minotauro-ia.md)); o robô (`robo-seguranca.md`) usa os mesmos.

## Em aberto

- Camaleão: papel de cada um dos ~38 rótulos (7, 34, 35, 39, 40), `0x41ac2d` (2ª mordida) e
  `+0x24/+0x2c` das variantes.
- Morte: a fórmula exata da aparição (`0x432e5d`) e as unidades de `+0x15c`.
- Formiga: estados 1–6.
