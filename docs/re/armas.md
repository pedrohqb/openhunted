# Armas, inventário e munição (M4)

Escopo: o que o Jake carrega, como ganha armas e munição, com que arma começa e como troca.
O tiro em si está em [`tiro-jake.md`](tiro-jake.md); os projéteis em
[`projeteis.md`](projeteis.md).

## Inventário no objeto [confirmado]

O inventário são **30 pares** `(tem, quantidade)` a partir de `+0x28`: o par do item `i` fica
em `+0x28 + 8·i` (tem) e `+0x2c + 8·i` (quantidade). A arma corrente é o número do par em
`+0x254`.

| Par | Campos | Conteúdo (Jake) |
|---|---|---|
| 0 | `+0x28/+0x2c` | não usado como arma (a lista de armas não o contém) |
| 1 | `+0x30/+0x34` | **punhos** (arma 1) |
| 2 | `+0x38/+0x3c` | **pistola** (arma 2) |
| 3 | `+0x40/+0x44` | espingarda (arma 3) |
| 4 | `+0x48/+0x4c` | lança-foguetes (arma 4) |
| 5 | `+0x50/+0x54` | estrelas ninja (arma 5) |
| 6 | `+0x58/+0x5c` | granadas (arma 6) |
| 7–12 | | itens dos tipos 80–85 (par = tipo − 73) |
| 13–18 | | itens dos tipos 62–67 (par = tipo − 49): chaves/bandeiras [inferido] |

Os nomes das armas vêm do projétil que cada uma cria (abaixo e em `projeteis.md`).

### Vem da fase

O leitor de objetos (`0x00434170`, a partir de `0x004343dc`, versão ≥ 1006) lê um `i16` n e n
pares `(tem, quantidade)` para `+0x28…`; no JSON das fases é o campo **`points`** de cada
objeto (30 pares). `fields[13]` vai para `+0x254`. [confirmado]

## Arma e munição iniciais do Jake

A inicialização do Jake (`0x0042277e`, em `0x004228d5`…) [confirmado]:

1. força o par 1 = `(1, 20)` (punhos "com 20");
2. `+0x254 = 1` e, logo depois, `+0x254 = 0x43a8cc(obj, 1)` = a **melhor** arma disponível;
3. `+0x288 = 4` (olhando para a esquerda).

Portanto `fields[13]` da fase é sobrescrito; o que manda são os `points`.

| Fase | `points` relevantes | Arma inicial |
|---|---|---|
| `T_J_001` | par 1 `(1, 20)`, par 2 `(1, 80)` | **pistola, 80 tiros** |
| `T_J_003` | par 2 `(0, 90)` | punhos (pistola sem o "tem") |
| `T_J_005` | par 2 `(1, 20)`, par 6 `(1, 2)` | pistola, 20 (granadas: 2) |
| `T_J_007` | par 2 `(1, 30)` | pistola, 30 |

[confirmado nos dados + regra acima]

## Lista de armas e "melhor arma" (`0x0043a8cc(obj, modo)`) [confirmado]

Cada personagem tem uma lista terminada em 0, da pior para a melhor:

- Jake (`0x0047df0c`): **1, 6, 5, 2, 3, 4** (punhos, granadas, estrelas, pistola, espingarda,
  foguetes).
- Garathe (`0x0047e978`): 1, 2, 5, 4, 6, 3.

Uma arma está **disponível** se `tem ≠ 0` **e** `quantidade > 0`.

| Modo | Resultado |
|---|---|
| 1 | a última disponível da lista (a melhor) |
| 2 | a primeira da lista |
| 4 | a disponível anterior à corrente, dando a volta (arma anterior) |
| 8 | a disponível seguinte à corrente, dando a volta (próxima arma) |

Devolve 0 se nada estiver disponível.

`0x0043af6c(obj, arma)` (ao ganhar uma arma): se a arma nova fica **depois** da melhor
disponível na lista, ela vira a corrente (`+0x254`). Sem som, sem evento. [confirmado]

## Munição [confirmado]

- `0x0040a432(obj, arma)` lê a quantidade; `0x0040a449(obj, arma)` lê o "tem".
- `0x0040a401(obj, arma, n)`: se quantidade > 0, subtrai n (mínimo 0) e devolve o resto;
  senão devolve 0 sem mexer. Cada tiro chama com n = 1.
- `0x0040a36d(obj, arma, valor)`:
  - `valor = 0` → só liga o "tem" (`= 1`); a quantidade não muda;
  - `valor ≠ 0` → ajusta pela dificuldade `0x487854`: 0 → ×1,5; 2 → ×0,75; outra → ×1
    (conversão por `0x46d138`; mínimo 1), soma e **limita a 99**. Não liga o "tem".

Os punhos também têm "quantidade" (20); como o soco nunca gasta munição (`tiro-jake.md`), ela
só serve para os punhos contarem como disponíveis.

## Coleta de armas e munição (`0x00409fe8(quem, item)`) [confirmado]

Chamada pela coleta genérica (`itens.md`, passo 9). `v` = `values[0]` do item (`+0x260`).
"Dá a arma" = `0x43af6c` + `0x40a36d(·, arma, 0)`.

| Quem | Tipo do item | Efeito |
|---|---|---|
| Jake | 12 | dá a pistola e soma **3·v** de munição |
| Jake | 16 | soma 3·v à pistola |
| Jake | 13 / 17 | dá a espingarda + v / soma v |
| Jake | 14 / 18 | dá o lança-foguetes + v / soma v |
| Jake | 27 | dá as estrelas (arma 5) + v |
| Jake | 26 | dá as granadas (arma 6) + v |
| Garathe | 15, 25, 26, 27, 44 | dá as armas 2, 4, 5, 6, 3 + v |
| Garathe | 45 | soma v à arma 3 |
| ambos | 62–67 | par `tipo − 49`: liga o "tem" e soma 1 |
| ambos | 80–85 | par `tipo − 73`: idem |
| ambos | 9 | chama `0x445c37` (não lido) |

A aceitação (recusa com munição ≥ 99) está em `itens.md` (`0x40b6ed`).

## Troca de arma (`0x0043b0cb`, todo tique) [confirmado]

Chamada pela atualização do Jake (`0x00422aa9`) antes da máquina de estados (salvo com
`+0x178 & 0x200000`). Com a entrada global `0x487a2c` (bits em `tiro-jake.md`):

- `bits & 0x10800 = 0x10800` (ANTERIOR pressionado **neste** tique) → `0x43a8cc(obj, 4)`;
- senão `bits & 0x21000 = 0x21000` (PRÓXIMA pressionada neste tique) → `0x43a8cc(obj, 8)`.

Se a arma muda: `+0x26c = 7`, som **3090** "Changed weapon" (`0x43c93d(0xc12, 0, 0, 0, −1,
0)`, não posicional), `+0x254` = nova, aviso `0x436358(obj, 0x20)` (HUD [inferido]).

Durante esses 7 tiques, `0x0043b15c` (chamada em `0x00422ce4`) decrementa `+0x26c` e mostra o
rótulo da pose "pronto" da arma nova (`0x0043aa00`: arma 0/2 → 18, 1 → 146, 3 → 108,
4 → 62, 5 → 127, 6 → 186); no último tique volta ao rótulo do estado corrente.

## Sem munição

Ver `tiro-jake.md` (`0x0043b1d0`): ao tentar atirar com 0, toca 3090, troca para a melhor
arma disponível e volta à pose "pronto" dela.

## Em aberto

- Papel do par 0 e dos pares 19–29; o que são os itens 62–67 e 80–85 no jogo (chaves,
  bandeiras, peças).
- `0x445c37` (item tipo 9).
- Arredondamento exato de `0x46d138` (provável `fistp` com o modo do FPU).
- Se o jogo carrega inventário entre missões da campanha (aqui só se viu a carga pela fase).
