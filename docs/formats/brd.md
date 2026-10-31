# Cenários de colisão (`.BRD`)

"Climber Border File": caixas de colisão de um bloco de cenário ou de um objeto. Leitor
original: `0x00430afb`. Blocos `HED:` e `BDR:`; inteiros little-endian.

## `HED:` (24 bytes)

| Tamanho | Conteúdo |
|---|---|
| 20 | Assinatura ASCIIZ `"Climber Border File"` (o original compara a string) |
| 2 | `s16` versão (o original recusa valores > 0; todos são 0) |
| 2 | `s16` número de caixas `N` |

## `BDR:` (N × 10 bytes)

| Tamanho | Conteúdo |
|---|---|
| 2 | `u16` tipo (guardado num byte; 1 ou 2 nos dados vistos) |
| 2 + 2 | `s16` x, `s16` y |
| 2 + 2 | `u16` largura, `u16` altura |

O significado dos tipos (sólido, plataforma, escada…) fica para a M3.

## Versão 1.00

225 cenários, todos lidos. Nomes como `C1A38A0.BRD` (cenário de fase, montado pelo padrão
`C%d%c%02d%c0.BRD` no executável), `C1_00.BRD`, e os de objetos (`OBB.BRD`, `OGR2.BRD`…).
