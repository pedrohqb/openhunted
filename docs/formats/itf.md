# Interfaces (`.ITF`)

Telas de menu: uma animação de base e uma lista de controles. Leitor original: `0x00461310`.
Blocos `ITF:` e `TAG:`; inteiros little-endian.

## `ITF:`

| Tamanho | Conteúdo |
|---|---|
| 2 | Assinatura `"TB"` |
| 2 | `u16` versão (1103 nos dados; o original aceita até 1103) |
| … | Nome ASCIIZ da animação (até 80 bytes; ex.: `sabout.ANM`) |
| 2 | `u16` número de controles `N` |
| 2 × N | Tipos dos controles (0 a 6), para o cálculo da memória |

Cada controle:

| Tamanho | Conteúdo |
|---|---|
| 2 | `u16` tipo (0 a 6; tabela de 7 entradas em `0x00483a08`) |
| 2 | `u16` id (nome no `TAG:`) |
| 2 | `s16` campo A (0 nos dados vistos) |
| 2 + 2 | `s16` x, `s16` y |
| 2 + 2 | `u16` largura, `u16` altura |
| 2 | (versão ≥ 1103) `u16` campo F; nas anteriores vale 1 |
| 2 | `u16` rótulo da animação (0 ou `0xFFFF` = nenhum) |
| 2 + 2 | (versão ≥ 1102) `s16` origem x, `s16` origem y; nas anteriores, `(-x, -y)` |
| 18 | Só no tipo 4 (campo de edição): bytes extras, que o leitor original pula |

Os tipos 1, 2, 5 e 6 compartilham o mesmo tratamento (2 e 6 ligam um sinalizador extra).

## `TAG:`

`u16` contagem e pares (`u16` id, nome ASCIIZ), como `AB_DONE`, `ECM_OK`, `MH2H_REPLAY`.

## Versão 1.00

16 interfaces, todas lidas; a animação de cada uma existe nos dados. Exemplo:
`SABOUT.ITF` tem 3 controles: `AB_VER` (tipo 0), `AB_DONE` (tipo 6, rótulo 2
"donebutn") e `AB_BACKGROUND` (tipo 0, rótulo 1 "about box", 320×256).
