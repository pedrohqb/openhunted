# Esqueletos (`.SKE`)

Captura de movimento dos personagens: para cada rótulo da animação, a posição da raiz do
corpo quadro a quadro e as juntas em 3D. Há dois na versão 1.00, ambos no `RESOURCE.001`:
`JAKE.SKE` (109 194 bytes) e `MINOTAUR.SKE` (95 760 bytes). Leitor original: `0x00409dff`;
gravador (do editor embutido): `0x00409bb8`. Engenharia reversa do uso:
[`docs/re/ske.md`](../re/ske.md). Inteiros little-endian; **não** usa blocos (`blocos.md`).

## Cabeçalho

| Tamanho | Conteúdo |
|---|---|
| variável | texto `"Hunter Hunted Skeleton File.\r\n"`, um byte 0 e o byte `0x1a`; o leitor descarta tudo até o primeiro `0x1a`, inclusive |
| 2 | `s16` identificador: 1 no `JAKE.SKE`, 2 no `MINOTAUR.SKE`; o leitor recusa o arquivo se não for o valor que quem o chama espera |
| 2 | `s16` `N`: número de entradas (197 e 164); o leitor usa `min(N, capacidade)`, com capacidade 197 (Jake, `0x0042277e`) e 164 (Garathe, `0x004279b3`) |
| 2 + 2 | `s16` largura e `s16` altura de referência do esqueleto (20 × 57 e 28 × 50); o jogo as recalcula ao carregar (ver `docs/re/ske.md`) |

## Entradas (`N` vezes)

A entrada `i` corresponde ao **rótulo `i`** do `.ANM` do personagem (`JAKE.ANM`,
`MINOTAUR.ANM`). Entradas vazias têm só os 10 bytes iniciais com zeros.

| Tamanho | Conteúdo |
|---|---|
| 2 | `s16` `n`: número de quadros |
| 2 | `u16` ângulo de giro do corpo em torno do eixo vertical (0x10000 = 360°) |
| 2 + 2 | `s16` dx, `s16` dy: deslocamento fixo da raiz |
| n × 4 | por quadro: `s16` x, `s16` y da raiz (**posição acumulada** desde o início do rótulo, em unidades do esqueleto; y cresce para cima; o "para a frente" do personagem é x negativo) |
| 2 | `s16` `m`: número de juntas por quadro (0 = sem juntas; 18 em todas as entradas cheias) |
| n × m × 6 | quadro a quadro, junta a junta: `s16` x, `s16` y, `s16` z |

Os 197 + 164 registros são lidos sem sobra. No `JAKE.SKE`, 85 entradas têm quadros; o número
de quadros quase sempre é igual ao do rótulo no `JAKE.ANM` (exceções: 1, 10, 12, 17, 19, 23,
26, 54–56, 66–68, 78, 79, 83, 84, 100, 101, 128, 131, 141–144, 162: o esqueleto tem mais
quadros, ou menos, do que a animação; a entrada 2 não tem rótulo no `JAKE.ANM` e é igual à
149). Os rótulos 9, 181, 186–188 e 193–196 do `JAKE.ANM` não têm esqueleto.

Alguns últimos quadros têm lixo (rótulo 10: x salta de −123 para 192; rótulo 12: x = 25 272).

## Na memória

Cada entrada ocupa 0xa0 bytes. O leitor preenche `+0x54` = `n`, `+0x58` = ângulo, `+0x5c` =
passo (1), `+0x60` = primeiro quadro (0), `+0x64` = último (`n − 1`), `+0x68`/`+0x6c` = dx/dy,
`+0x70` = tabela de raízes (`n` pares de `i32`) e `+0x9c` = tabela de juntas (`n·m` triplas
de `i32`). O gravador salva só os quadros de `+0x60` a `+0x64` com o passo `+0x5c`, o que
permite ao editor gravar um trecho.

A topologia das 18 juntas (pai/filhos) não está no arquivo: vem de uma tabela fixa de 18 × 3
inteiros no executável (`0x00471ab8`), a mesma para os dois personagens [inferido: ossos; não
decifrada, o jogo não a usa no movimento].

## Versão 1.00

`ohtool extract --type SKE` grava os arquivos como estão; `ohtool info` ainda não decodifica
o conteúdo.
