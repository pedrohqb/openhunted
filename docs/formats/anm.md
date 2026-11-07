# Animações (`.ANM`)

Recurso com os blocos `ANM:`, `TAG:`, `LAB:` e `PAL:` (ver [`blocos.md`](blocos.md)).
Engenharia reversa: [`docs/re/anm.md`](../re/anm.md). Inteiros little-endian.

## `ANM:`

| Tamanho | Conteúdo |
|---|---|
| 2 | Assinatura `"TB"` |
| 2 | `u16` versão (1004 em todos os ANM da 1.00; o original aceita até 1004) |
| 2 | `u16` `A`: entradas da tabela A |
| 2 | `u16` `B`: número de quadros |
| 2 | `u16` `C`: número de rótulos |
| 2 | `u16` `D`: tamanho do roteiro em bytes |
| 2 + 2 | `u16` largura e `u16` altura de referência |
| 6 | (versão ≥ 1002) três `s16` que o original descarta |
| 2 × A | Tabela A (`u16`): quadro lógico → quadro físico (ver abaixo) |
| B × 12 | Quadros: `s16` x, `s16` y, `s16` largura, `s16` altura, `u16` início do roteiro (em palavras), `s16` extra (versão ≥ 1002) |
| C × 4 | Rótulos: `u16` quadro inicial, `u16` id |
| D | Roteiro: instruções `s16` opcode + argumentos `s16` |

**Tabela A** [inferido pelos dados]: vazia na maioria; não vazia em `SEGOC`, `SHERO`,
`SLEVL`, `S_CAR`, `SIERRA` e `OCH`, onde os rótulos passam do número de quadros (ex.:
`S_CAR` tem 364 quadros, 976 entradas e rótulos até 847). Os rótulos contam quadros
**lógicos**, e a entrada `i` dá o quadro físico do lógico `i` (repetições seguram um
quadro). As telas (`Screen`) seguem isso desde a M9; os objetos de jogo (`OCH`) ainda não.

Tamanho de cada instrução em palavras, opcode incluído:

| Opcode | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Palavras | 1 | 1 | 6 | 7 | 6 | 2 | 2 | 6 | 4 | 1 | 1 |

Significado provisório: 0 = fim do quadro; 1 = fim da animação; 2 = desenhar
(id do bitmap, quadro, x, y, opções); 8 = som (número, ?, ?). Os demais, a decifrar.
Opcodes encontrados nos 87 ANM da 1.00: 0, 1, 2, 3, 4 e 8.

## `TAG:` e `LAB:`

`u16` contagem e, para cada entrada, `u16` id e nome ASCIIZ. `TAG:` lista os bitmaps
(ids usados pelo opcode 2); `LAB:` dá nome aos rótulos.

## `PAL:`

Caminho ASCIIZ da paleta no ambiente de autoria (ex.: `J:\hunter\DLS\world1.pal`). O nome
do arquivo **não** corresponde diretamente aos recursos (`world1.pal` não existe; há
`WORLD.PAL`). Qual paleta o jogo usa de fato fica para a M2.

## Versão 1.00

87 animações, todas lidas. Cinco referências a bitmaps que não existem nos dados originais:
`WORLD1.ANM` → `CDUMMY.bmp`; `WORLD2.ANM` → `C2C03D.bmp`, `C2C03E.bmp`, `C2C03F.bmp`,
`CDUMMY.bmp`.

O `ohtool info` e o `ohtool extract` exportam animações em JSON.
