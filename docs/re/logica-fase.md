# Lógica de fase: gatilhos, interruptores, saída e fim de missão (M3)

Escopo: o que a `T_J_001` precisa para terminar — zonas de proximidade (tipo 73), a saída
(tipo 5), a lista de gatilhos dos objetos e o veredito da missão. Dicas do mago (tipo 30) em
[`dicas.md`](dicas.md); minas em [`minas.md`](minas.md); itens em [`itens.md`](itens.md);
vida, dano e morte em [`dano-vida.md`](dano-vida.md).

## Convenções

- `values[i]` da fase vão para `+0x260 + 4·i` do objeto (`values[0]` → `+0x260`,
  `values[1]` → `+0x264`…). Os campos `fields[13..15]` do JSON são `+0x254`, `+0x258`
  (**vida**) e `+0x25c` (**armadura**); `fields[16]` é `+0x1ac` (**objetivo**, abaixo);
  `fields[18]` é `+0x118` (**id de gatilho**). [confirmado pelo uso no código e pelos dados]
- `+0x18` é o tipo do objeto (o mesmo `kind` da fase; as inicializações às vezes o reescrevem).
- `0x488e7c` é a **cabeça da lista de objetos** (encadeada por `+0x1c`), não "o jogador":
  todas as varreduras abaixo partem dela. [confirmado]
- `0x4879c4` é o contador de tiques do jogo (incrementado em `0x404541`). [confirmado]

## Lista de gatilhos (`0x00435bd8`)

Cada objeto tem até 4 entradas de 12 bytes em `+0x11c`: `+0` id do alvo (o `+0x118` de outro
objeto), `+4` ponteiro para o alvo (resolvido na carga [inferido]) e `+8` **código**. Na fase,
cada gatilho é o par `(id, código)`: `[[1, 11], …]`.

`0x435bd8(&obj+0x11c, primeira, n)` (sempre chamada com 0, 4):

- O código é lido em grupos de dois dígitos decimais, do fim para o começo (`c % 100`,
  depois `c / 100`…). Num grupo `g`: dígito das unidades `u` = **número da entrada** (1–4)
  cujo alvo se testa; dígito das dezenas `t` = estado esperado (`t == 1` → ligado, senão
  desligado). Ex.: `11` = "o alvo da entrada 1 está ligado"; `1203` = "o alvo da entrada 3
  está desligado **e** o da entrada 2 está ligado". [confirmado]
- "Ligado" = bit `0x40` de `+0x178` do alvo; para alvos dos tipos 50–52, é `+0x180 > 2`.
- Os grupos de uma entrada combinam-se com **E**; as entradas, com **OU** (a varredura para
  na primeira entrada satisfeita). Entradas de código 0 não contam.
- Se nenhum grupo foi avaliado (todas as entradas com código 0), devolve −1 (verdadeiro).
  Uma entrada com alvo nulo vale verdadeiro. [confirmado]

## Tipo 73: zona de proximidade ("Proximity Switch", `0x0040e0ac`)

Inicialização: chama a genérica `0x40b2c8`, nome "Proximity Switch", liga `0x8` (sem
gravidade), `+0x1b4 = 20`; largura `values[0]` (0 → 32) e altura `values[1]` (0 → 16) viram a
caixa `+0x20c..+0x218 = (0, 0, L, A)`; timer `+0x27c = 0`; rotina `+0x1c8 = 0x40e14e`,
`+0x1d0` = vazia. Sem animação nem colisão; não é desenhada. [confirmado]

Rotina `0x40e14e` (por tique, via `0x4076ff`):

1. Se a lista de gatilhos do próprio objeto é falsa, nada acontece.
2. **Ocupada?** Com `values[5] == 2` está sempre ocupada (vira um temporizador). Senão,
   varre a lista de objetos e conta um objeto se: tipo ≠ 5, 50–52, 73, 74; mesma camada;
   `+0x178 & 0x80003 == 0x80001`; presente no modo de jogo; vida `+0x258 > 0`; com
   `values[5] == 0`, só jogadores (`+0x19c ≠ 0`); e a caixa dele cruza a da zona
   (desigualdades estritas). [confirmado]
3. Desligada (sem `0x40`):
   - fora e (`values[4] & 1 == 0` ou timer > 38 888 888) → timer = 0;
   - ocupada e timer ≤ 0 → timer = `values[2]` (**atraso** em tiques);
   - ocupada → timer−−; ao chegar a 0: se `+0x1ac & 1`, toca o som 3093 ("Secret
     discovered"); liga `0x40`; timer = `values[3]` (**duração**; 0 → 77 777 777).
4. Ligada: ocupada e `values[4] & 2` → timer = duração (rearma); timer−−; desliga (limpa `0x40`,
   timer = 77 777 777) quando a duração ≠ 0 e o timer chega a 0, ou, com duração 0, quando
   fica vazia. [confirmado]

**99999999** em `values[3]` é uma duração na prática infinita: a zona liga uma vez e fica
ligada. 77 777 777 e 38 888 888 são sentinelas do próprio código.

Na `T_J_001`: zona id 1 em (1391, 92)–(1455, 108), perto do início do Jake (x = 1696), e id 2
em (76, 100)–(140, 116), perto da saída; ambas com atraso 1 e só para jogadores. Cada uma
aciona uma dica (gatilhos `(1, 11)` e `(2, 11)`). [confirmado nos dados]

## Tipos 5 e 74: interruptor e saída ("Toggle Switch", `0x0040e393`)

Inicialização [confirmado]:

- Tipo 5 (**saída**): força o rótulo `+0x1ec = 8` (`osw08`), liga `+0x1ac |= 2` (objetivo
  "acionar"), carrega a colisão padrão (`0x434ec1(obj, 0, 0)`) e põe a escala `+0x1f0 = 100`.
  Tipo 74 (interruptor comum): escala mínima 25.
- Nome "Toggle Switch"; liga `0x8` e `0x20` (sem gravidade, só contato); `+0x1b4 = 20`;
  caixa `+0x20c` = retângulo do quadro (`0x43585c`); desliga `0x80`; `+0x278 = +0x27c = 0`;
  quadro inicial = primeiro do rótulo; `+0x1c0 = 0x40e4b4`.
- `values[0] ≠ 0` → começa ligado (`0x40`) e anima com modo 5.

`0x40e4b4` (por tique): ligado → `0x406c71(obj, 2)` (toca o rótulo uma vez, para a frente);
desligado → modo 6 (uma vez, ao contrário). A alavanca anda e para no extremo.

**Acionar** (`0x0040e4dd(ator)`, chamado pelo Jake em `0x423bbb` e pelo Garathe em
`0x428d4c`): pega o retângulo do ator (`0x4351b5`) e procura objetos dos tipos 74 e 5 na
mesma camada cuja caixa (posição + `+0x214`/`+0x218`) o toque (desigualdades não estritas).
Para cada um:

- Tipo 5, em solo (`0x497300 == 1`) ou cooperativo (`0x497304 ≠ 0`): pergunta
  `0x43cdf0(0)` (abaixo). Se 0 → devolve **0** (recusado) sem mexer na saída; senão o
  resultado final será **−1**.
- Se `+0x278 < tique`, inverte `0x40`. Depois `+0x278 = tique + 1` se `values[1] ≠ 0`
  (pode voltar), senão `tique + 77 777 777` (aciona uma vez só).
- Sem saída aceita, devolve 1. [confirmado]

**No Jake** (`0x423b8e`): a tecla CIMA só chega aqui no estado `+0x180 == 3` e com o
retângulo dele sobre uma zona PR de tipo `0x40` (bitmask de `0x4075af`). Na `T_J_001` a zona
PR 64 da célula 0 fica em (90, 69, 35×32), colada à direita da saída em x = 51. [confirmado]
O retorno decide:

- **−1** → `0x426820`: o Jake entra na porta — estado 13 (rótulo `J012`), liga
  `0x200008`, zera a colisão `+0x208`; espera 10 tiques (`+0x298`) e depois encolhe a escala
  `+0x1f0` em 1 por tique até 30. [confirmado]
- **0** (objetivos incompletos) → choque: `+0x252 |= 0x20` (elétrico), `+0x24c = 20` de
  dano, som 3772 ("MC Objectives not satisfied", prioridade 200) e som posicionado 3135
  ("Electric Fence Zap #1") no centro do Jake. [confirmado]
- **1** → estado 8 (rótulo `J003`, apertar o botão), retorno `0x426ecc`.

## Veredito da missão (`0x0043cdf0(fim)`)

Resultado em `0x497328`: **0** = continua, **1** = vitória, **2** = derrota/fim. Começa em 1
e percorre a lista de objetivos `0x497314` (encadeada por `+0x2ac`; [inferido] objetos com
`+0x1ac ≠ 0`). Bits de `+0x1ac` [confirmado]:

| Bit | Significado | Falha quando |
|---|---|---|
| 1 | **segredo** (zonas e itens tocam 3093 "Secret discovered") | não lido aqui |
| 2 | **acionar** | `0x40` desligado (com `fim = 0`, a própria saída é ignorada) |
| 4 | **destruir** | vida `+0x258 > 0` |
| 8 | **sobreviver** | vida ≤ 0: jogador → 2 quando todos os jogadores morreram; não jogador → 2 na hora |

Nas 100 fases: exatamente uma saída (tipo 5, `+0x1ac = 2`) e o jogador com `+0x1ac = 8`;
inimigos a matar têm 4. Na `T_J_001`, só a saída e o Jake. [confirmado nos dados]

Depois:

- Vários jogadores sem cooperação: limites de pontos (`0x497378`) e de tempo (`0x497374`
  minutos × 60 000 ms) → 2.
- Resultado 0 com `0x488e54 ≠ 0` (campo 1004 do cabeçalho [inferido]) e `0x4972fc = 0` → 2.
- Com `fim ≠ 0` e resultado ≠ 2: se a saída existe e está desligada → 0; senão, em solo ou
  cooperativo, resultado 0 vira 2.
- Com `fim ≠ 0`, um resultado ≠ 0 só "pega" depois de **40 tiques seguidos**
  (`0x497478`, recarregado a 40 sempre que dá 0); aí `0x43cfcf` fecha as estatísticas.

Uso: o laço principal chama `0x43cdf0(1)` a cada tique (`0x40456e`); resultado ≠ 0 →
`0x401624(0x65)` encerra a partida. As telas seguintes testam `0x497328 == 1`: vitória usa o
recurso 30032 (`0x7550`, `0x443d31`) e o 25002 (`0x4444a6`), a derrota o 25001 — tela/texto
e música [inferido].
"Desistir" (diálogo 10007 do `SYSTEM.RES`, `0x401872`) grava 2 direto. [confirmado]

**Na `T_J_001`**: o Jake chega à esquerda, aperta CIMA diante da saída, `0x43cdf0(0)` dá 1
(está vivo), a saída liga, e 40 tiques depois o veredito do laço dá **vitória**. Se a vida
chega a 0, dá **derrota** após 40 tiques.

## Texto do objetivo

O objetivo mostrado ao jogador é a descrição da fase (`d=` do `ENGLEVEL.TXT`, igual ao campo
"descrição" do cabeçalho): "Get to the exit. Use the pistol to shoot the hovering mines.
Good luck Jake!". As notas do autor ("Simplest of the simple…") não são exibidas [inferido].

## Em aberto

- Onde a lista `+0x11c` é resolvida (id → ponteiro) e onde `0x497314` é montada.
- `values[3] = 1558` e `values[5] = 4` da saída (não lidos pelo código do tipo 5 visto).
- O papel exato do estado 3 do Jake na tecla CIMA e das teclas em `0x487a2c`.
- Onde a descrição da missão é desenhada (briefing).
- Duração real do tique (40 tiques de espera do veredito).
