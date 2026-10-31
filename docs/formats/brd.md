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
| 2 | `u16` tipo (guardado num byte): 2 = caixa (378 nos dados), 3 = aresta de polígono (12), 4 = vértice (12), 0 = vazio (3) |
| 2 + 2 | `s16` x, `s16` y |
| 2 + 2 | `u16` largura, `u16` altura |

Todas as formas são sólidas dos quatro lados (não há plataforma de mão única); ver
[`docs/re/personagem.md`](../re/personagem.md). Os tipos 1, 2, 4, 32 e 64 que aparecem na
lista PR das células são outra coisa (zonas: portas, passagens, escadas, saída).

## Versão 1.00

225 cenários, todos lidos. Nomes como `C1A38A0.BRD` (cenário de fase, montado pelo padrão
`C%d%c%02d%c0.BRD` no executável), `C1_00.BRD`, e os de objetos (`OBB.BRD`, `OGR2.BRD`…).
