# IA dos personagens: Minotauro inimigo e Jake inimigo

Complementa [`garathe.md`](garathe.md) e [`garathe-movimentos.md`](garathe-movimentos.md).
**[confirmado]** = lido no código/dados; **[inferido]** = dedução.

## Quem é controlado pela IA [confirmado]

`0x42ec1c(obj)` devolve os bits de entrada (`tiro-jake.md`) de todo personagem. Usa o teclado
(`0x487a2c`) se `+0x19c ≠ 0`, se o objeto não está presente (`+0x178 & 0x80003 ≠ 0x80001`, ou
`+0x1a8 & 0x4972f8 = 0` sem `+0x178 & 0x10`) ou se `+0x1a8 & 1` (vaga de jogador vazia). O
resto (bit 0 de `modos` desligado, ex. `0xfe`) é IA. O código **não olha o tipo**: vale para
Garathe (2) **e Jake (1)**; a diferença está em limiares (`0x4888b0` = tipo do pensante).

Contagem nos dumps (`modos & 1 = 0`, `& 2 ≠ 0`):

| Fases | Minotauros IA | Jakes IA |
|---|---|---|
| `H_J_*` | 19 em 10 (001:1, 006:3, 007:3, 012:1, 013:1, 018:1, 027:1, 039:1, 042:2, 045:5) | 10 (010:6, 012:3, 033:1) |
| `D_J_*` | 8 (001:2, 007:1, 009:2, 015:3) | 3 (001:1, 009:2) |
| `H_M_*` | 1 (003) | 15 (003:1, 004:3, 021:5, 027:6) |
| `D_M_*` | 7 (010:1, 027:5, 030:1) | 21 (004:3, 006:3, 010:3, 018:2, 021:4, 024:6) |

Também em `C_*`/`M_M_033`. `fields[16] = 4` ("destruir"): Minotauros de `H_J_001/006/007/012`,
Jakes de `H_J_010/033` e de `H_M_*`. Inventário pelo `points` (armas 2–6); vida/armadura da fase.

## Estrutura da IA (`+0x2a0`, 0xe0 bytes, `0x45be60`) [confirmado]

| Desl. | Papel |
|---|---|
| `+0x00` | alvo (objeto) |
| `+0x04`/`+0x08` | classe do estado do alvo / própria (`0x42c7b6`, abaixo) |
| `+0x0c` | lista de perigos a < 100 px (`0x42cad4`, encadeada por `+0x20`, distância em `+0x24`) |
| `+0x10…+0x20` | células própria, acima, abaixo, esquerda, direita (`0x42cbfd`) |
| `+0x28` | melhor arma (`0x43a8cc(obj, 1)`); `+0x30` vida |
| `+0x34/+0x38`, `+0x3c/+0x40` | centros próprio e do alvo |
| `+0x44`, `+0x48` | distância; ângulo 16 bits (0 = direita, 0x4000 = baixo) |
| `+0x4c/+0x50` | dx, dy até o alvo; `+0x54` = vê |
| `+0x58` | cópia de `+0x220…+0x23f` (contato), restaurada no fim |
| `+0x78` | última entrada; `+0x7c` tique da última visão |
| `+0x80/84/88` | célula-alvo do caminho (x, y, camada; −1 = nenhuma); `+0x8c` direção; `+0x90` entrada do passo; `+0x94` espera, em pensamentos (−1 por pensamento) |
| `+0x98` | modo: 1 caçar, 2 recuar, 3 vagar (0 vale 1) |
| `+0xdc` | **período de pensamento = 2**: nos tiques ímpares repete `+0x78` |

`+0x9c/+0xa0` (`0x42e938`, fuga por zonas) só são zerados: código morto.

## Percepção (`0x42d520`) [confirmado]

A entrada global `0x487a2c` começa cada pensamento em 0 (`0x407e2d` sem vaga de jogador,
`0x422a39`/`0x427c65`); as funções abaixo a escrevem e devolvem "conseguiu" (1/0).

- **Alvo** (`0x42c5cc`, só quando não há ou ele morreu, `+0x178 & 0x8000`): os dois primeiros
  jogadores humanos (`+0x19c ≠ 0`) presentes e vivos; fica o mais próximo se estiver na mesma
  célula (`0x435224`) ou se a busca de caminho (`0x42f88c`, profundidade 10) o alcançar; senão o
  outro, nas mesmas condições; senão **nenhum**. **Sem limite de distância nem esconderijo**,
  e o alvo escolhido **fica** (não é revisto enquanto vive). Sem alvo o pensamento vai direto
  ao modo 3, que sem alvo só chama os perigos (`0x42f4f0` → `0x42f86e`): a IA fica parada.
  Sem alvo também não se refazem a lista de perigos nem as células (`+0x0c`, `+0x10…`; ficam
  as do último pensamento com alvo — o port usa as atuais).
- **Vê** (`+0x54`): mesma camada, distância < **400** px e linha livre centro a centro
  (`0x42d475`: `0x433aa3`/`0x43735d`) ou contato com o alvo (`+0x220 = 1`, `+0x224` = alvo).
  No modo 3, alvo escondido (classe 10) ou morrendo (14) **no pensamento anterior** (`+0x04`
  ainda não atualizado) = não vê. Ao ver: `+0x7c` = tique e apaga o passo e a espera
  (`0x42d6e3`).

Classes (`0x42c7b6`, estados Jake / Garathe): 1 parado (0,1,3,6–8,15,79 / 0–2,4,5,121); 2 pose
(4 / 76); 0xf especial (9 / 127); 3 andar/correr (2,10 / 3,10–12); 4 camada (11,12 / —); 5 subir
no pulo (16,17,27–29,31,80 / 19–22,24,25); 6 aterrissar/cair no chão (26,34,35,76–78,81–84,162,163
/ 23,26,28–33,35); 7 queda (25 / 27); 8 agachado (85–87 / 36–38); 9 empurrar (100–102 / 44–46);
**10 escondido (164–166 / 54–56)**; 11 barra (103–107 / 47,49–51); 12 escalar (30,32 / 52,53);
13 dano (168,169 / 58,59); 14 morrendo (171…181 / 61–69,80); 0x10 golpe/tiro; 0 outros.

## Decisão por modo (`0x42ec1c`) [confirmado]

Sem alvo → modo 3. **Modo 1** (`0x42ed64`):

1. Dano pendente > 10, ou dano com vida < 30 → modo 2 (`0x42f35b`).
2. Própria classe 2, 6 ou 0xf → entrada 0 (espera).
3. Vê: alvo escondido/morrendo → modo 3 (entrada 0). Por classe própria (tabela `0x42ee23`):
   5/7 → no ar (`0x42e0f9`: CIMA se o ângulo ≥ 0x8000 e `+0x168 > −16`); 11 → barra
   (`0x42e05d`: alvo logo abaixo, ângulo 0x3800–0x4800 → PULAR+BAIXO; célula própria fechada
   embaixo (ou vazia) e `|dy| ≥ 10` → BAIXO; Jake nos estados 104–106 → BAIXO; senão o lado do
   alvo); 12 → escada/corda (`0x42e129`: `dy > 20` com `|dx| < 10` ou `dy > 20` → ATIRAR
   (solta); `dy < 5` → CIMA; `|dx| > 20` → lado; nada → ATIRAR); 14 → 0; 16 → só o ataque;
   demais → ataque (`0x42e659`), senão aproximação (`0x42dd8f`). **Ataque e os casos no ar,
   barra e escada voltam sem os perigos**; a aproximação bem-sucedida passa por eles. Os três
   casos sempre conseguem (o desvio para o caminho quando falham nunca é tomado).
4. Não vê, ou a aproximação falha → **caminho** (`0x42ef48`):
   - no primeiro pensamento após perder de vista (`tique − 2·+0xdc < +0x7c < tique`), pose
     (`0x42e8f3`: parado ou andando, 501/1000; nas outras classes sempre) — e volta sem perigos;
   - sem passo e sem espera: busca até a célula do alvo e passo (`0x42d721`); sem caminho
     espera `+0x94 = 5·+0xdc = 10` **pensamentos (20 tiques)**: a cada um, `+0x94 −1`; na
     classe 1, 1/50 de pose; senão entrada 0 e 1/1000 de passar ao modo 3;
   - na célula-alvo do passo: centrado (`0x42db71`: `|ox| < 15/25`, `|oy| < 50/40` com
     `o = c mod 128 − 64`) → nova busca e passo; se a entrada nova difere da última, pose
     (`0x42e8f3`, como acima) e volta sem perigos. Sem centrar: espera no fim → nova busca;
     senão repete a direção do passo e `+0x94 −1`;
   - fora da célula-alvo: a mais de 1 célula (Manhattan) → apaga o passo e **entrada 0** neste
     pensamento; a 1, como "sem centrar".
5. Por fim os perigos (`0x42e1a9`), salvo nos retornos acima.

**Modo 2** (`0x42f35b`): sem ver → modo 1 (entrada 0). Vendo: classe própria 6 → 0; senão
`0x42df9f`: corre para o lado contrário ao alvo; parede a 20 px ou buraco sem pouso → volta
ao modo 1 sem perigos (a entrada de lado fica neste pensamento); escalando com o alvo bem
acima, +BAIXO; conseguiu → perigos. (`0x42e938`, fuga por zonas, só com `+0x9c ≠ 0`: morto.)

**Modo 3** (`0x42f434`): com dano e alvo → modo 2; alvo não escondido → modo 0 (= caçar),
entrada 0. Por classe própria: 2, 4–7, 15, 16 → 0; 8, 9, 13 → modo 1, 0; 11, 12 → PULAR
(solta). Senão: sem alvo, só os perigos. Com alvo: sem passo e sem espera, passo para o lado
sorteado (`rand & 1` → esquerda, senão direita), sem consultar a grade; com espera, `−1` e,
na classe 1, 1/20 de pose; na célula do passo e centrado, segue a mesma direção (parede da
célula própria → inverte) e 1/20 de pose sem perigos; sem centrar ou ao lado, segue o passo.
Passo com PULAR (buraco) é apagado e fica só **FURTIVO**; toda entrada de passo leva FURTIVO.

## Aproximação (`0x42dd8f`) [confirmado]

Só contra alvo de classe 1, 3–5, 7–9, 11–13, 15, 16. `|dy| < 64` com o alvo na classe 5
(pulando) vale 0.

- Mesma altura (`−20 < dy < 20`): perto (**`d`** < 30 Jake, < 50 Garathe): alvo com `vx ≠ 0`
  (qualquer classe, salvo empurrando) → segue o `vx`; senão, fora da barra/escada, **recua
  andando** (FURTIVO, lado oposto); parede a 20 px do lado escolhido → inverte (nos dois
  casos). Longe: `|dx| > 10` → corre para ele; senão nenhuma entrada. Depois `0x42cf70` e
  **consegue sempre**.
- Alvo bem acima (ângulo 0xbc00–0xc400) → PULAR+CIMA. Escalando: esquerda (0x4800–0xbc00)
  ou direita (fora de 0x3800–0xc400); CIMA em 0x9000–0xf000, senão sempre BAIXO (o teste de
  `0x42df7a` aceita todo ângulo). Senão falha.
- `0x42cf70` (só com a entrada exatamente ESQ ou DIR) sonda o chão: chão à frente → segue;
  buraco com pouso a 110/150/255 px (Jake, `0x478910…`) ou 156/350/225 px (Garathe,
  `0x47d128…`; `vx·(−2vy/g)`, g = 2) → +PULAR (agachado: a entrada vira CIMA, `0x42e8d0`);
  sem pouso → falha (quem chama põe PULAR+CIMA no lugar).

## Ataque (`0x42e659`) [confirmado]

Exige alvo e ver. `m` = 7/12 px (Jake/Garathe), `p` = 30/50 px.

| Distância | Ação |
|---|---|
| < `m` | nada |
| `m` … `(p−m)/2+m` (18/31) | repete a última entrada se tinha ATIRAR |
| até `p` | não bate em alvo que foge (sinal de dx = sinal do vx), salvo empurrando. Arma 1 (soco; Garathe: **clava** se tem o par 2). `0x42cdef` dá a mira **ou 0**, e o ataque **conta como feito mesmo com 0** |
| `p` … 200 | Jake: só com arma ≠ 1. Garathe: correndo (estado 10), d > 90, `vx` dele e do alvo com o mesmo sinal (0 = +), ângulo em 0xf000–0x1000 ou 0x7000–0x9000 e **alvo no estado 10** → **pulo longo** (lado+PULAR+CIMA, investida de 7); senão só com arma ∉ {1, 2}. Atira se `0x42cdef ≠ 0` |
| ≥ 200 | nada |

`0x42cdef(alvo, ângulo, d)`: mira `0x42cca4` e, pelo **tipo e estado do alvo**, 0 nos estados
(Jake) 9, 11, 12, 15, 25, 26, 76–78, 81–84, 87, 102, 162, 163, 166, 168, 169, 171, 173,
177, 179, 181; (Garathe) 11, 12, 26–33, 35, 38, 45, 55, 58, 59, 61, 63, 65, 67, 69, 80, 127 —
alvo em especial, troca de camada, choque (Jake), queda, pouso duro, derrubada, levantando,
saindo do esconderijo, empurrado/tranco ou morrendo não é atacado. Senão mira | ATIRAR.

Mira (`0x42cca4`): perto (< `p`), setores direita [0xf800, 0x800), baixo-dir. [0x800,
0x4000), baixo-esq. [0x4000, 0x7800), esquerda [0x7800, 0x9000), cima-esq. [0x9000, 0xb000),
cima [0xb000, 0xd000), cima-dir. [0xd000, 0xf800) — sem "só BAIXO"; longe, só alinhado em
`[c − 0x200, c + 0x200)` de 0, 0x2000, 0x6000, 0x8000, 0xa000, 0xc000 ou 0xe000. Logo **nunca
usa o especial**.

## Perigos (`0x42e1a9`) [confirmado]

Célula própria vazia → nada. Código `k = (c/10) % 100` da célula, `x = centro mod 128`,
`m = +0x40 & 1`:

1. `k = 30`, ou `k` ∈ {29, 36, 37, 39} com (`m`, entrada **exatamente** DIR, `x > 64`) ou
   (não `m`, exatamente ESQ, `x < 64`) → BAIXO (agachado: +BAIXO).
2. `k` ∈ {29, 36, 37, 39} com (não `m`, exatamente DIR, `x < 64`) ou (`m`, exatamente ESQ,
   `x > 64`) → +BAIXO. [inferido: buraco/escada na borda da célula]
3. A lista `0x47d14c` a < 100 px (sem o próprio nem o alvo, mesma camada; tipos 1–4, 7, 8,
   10, 11, 39–42, 102–107, 109, 89, 19–21, 29, 36, 25–27, 99, 100, 108, 86, 68–72, 110),
   **ordenada pela distância**: o primeiro objeto cuja condição vale decide.

| Tipo | Distância | Reação |
|---|---|---|
| 19–21, 25, 27, 29, 99, 100, 106, 108 (projéteis) | < 100 | vindo para ele (só o sinal do `vx` do objeto em relação ao x dele) e não rastejando (86/36) → BAIXO |
| 26 granada, 42 fogo, 68–72 | < 100 | a entrada aponta para o objeto → +PULAR+CIMA; senão lado ao acaso + PULAR+CIMA |
| 39 pistão | < 95 | entrada com PULAR ou ATIRAR/SOCO → fica; < 75 e abaixo dele, não rastejando → BAIXO; senão a entrada (agachado: +BAIXO) |
| 40 cerca, 41 catapulta | < 50 | a entrada aponta para o objeto → +PULAR+CIMA |
| 11 mina saltadora, 89 Manta, 105, 109 formiga | < 50 | ATIRAR (+BAIXO se acima) virado para ele |

4. Nenhum: agachado (classe 8) → CIMA (`0x42e8d0`).

O port compara os centros (o original, `+0x14c/+0x150` com as distâncias de `0x435951`).

## Caminho (`0x42f88c`, `0x42f9a4`, `0x42d721`) [confirmado]

Busca em profundidade (≤ 10 passos, só se a distância Manhattan x+y+camada ≤ 10, sem voltar
pela direção oposta, `0x47d1ec`) a partir de cada uma das 6 direções; devolve a primeira
direção do menor caminho. Direção → célula: 1 y−1, 2 y+1, 3 x−1, 4 x+1, 5 camada−1,
6 camada+1. Permissões na **célula de origem**, que precisa existir (`0x4317db` é NULL fora da
grade **ou na célula vazia**; o destino pode ser vazio, mas não se sai dele):

| Dir. | Teste | Bits (`célula+0x04`) |
|---|---|---|
| 1 subir | sem `0x40` e zona PR com bit 4 (corda/escada) | **`0x40` = fechada em cima** |
| 2 descer | sem `0x2` | **`0x2` = fechada embaixo** |
| 3 esquerda | sem `0x10`; destino sem portão (`célula+0x54` 1/2, `0x40f1f0`); na célula do pensante com portão e o centro em `x mod 128 > 64` → não (`0x42fc70`) | **`0x10` = parede à esquerda** |
| 4 direita | sem `0x8`; idem, com `x mod 128 < 64` (`0x42fd0a`) | **`0x8` = parede à direita** |
| 5/6 camada | zona PR bit 1 / bit 2 (portas) | — |

As portas `OH` (tipos 50–52) ficam de fora: `0x42f96a` as lista em `0x4888cc`, que nunca é
lida (código morto).

Passo (`0x42d721`): direção 0 → apaga e espera 10 pensamentos. Senão fixa a célula-alvo e
`+0x94 = 5·+0xdc = 10` (pensamentos) e a entrada, com `o = c mod 128 − 64` e as folgas
`cx` 15/25, `cy` 50/40: 1 → centra (`ox < −cx` DIR, `> cx` ESQ) senão CIMA; 2 → centra senão
PULAR+BAIXO; 3/4 → lado, +BAIXO se `oy < −cy` fora da barra, senão +CIMA se `oy > 0`
escalando ou subindo no pulo, e `0x42cf70` (falha → PULAR+CIMA); 5 → CIMA (0 se `oy < −cy`),
centrando por cima; 6 → BAIXO (PULAR escalando; 0 se `oy < −cy`), centrando por cima.

## Movimento, dano, morte e sons [confirmado]

A IA só gera bits; correr, pular, velocidades (Garathe 15 px/tique, pulos 13/−12 e 25/−14),
dano recebido (`0x4283b7`/`0x407862`, com armadura, **sem** fator de dificuldade), reações,
morte e cadáver são os do jogador. Não há sons próprios da IA; as falas de `0x40b9eb` só com
jogador.

## Em aberto

- Modo 3 com alvo visível põe `+0x98 = 0` (ramo sem desvio após `sub eax,4`, `0x42f479`) e
  volta com a entrada 0: erro do original? Efeito: volta a caçar no pensamento seguinte.
- Significado dos códigos de célula 29/30/36/37/39 e de `célula+0x40`.
