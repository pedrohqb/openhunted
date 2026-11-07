# Tabelas `.TBL`

Três tabelas binárias no arquivo de recursos (versão 1.00). Inteiros little-endian.

## `SOUND.TBL` (25 423 bytes; leitor `0x0043c0ce`) [confirmado]

| Campo | Conteúdo |
|---|---|
| texto | `"This file is a binary file and contains\r\nthe sound manager table for Hunter Hunted.\r\n"`, `0x00`, `0x1a` (o leitor pula até o `0x1a`) |
| `i32` | versão (1000) |
| 3 grupos | cada um: `i32` n, e n listas; cada lista: `i32` contagem + contagem × `u32` |

Valor (`u32`): bits 0–14 = id do som (0 = nada), bit 15 = laço, bits 16–31 = prioridade.
Cada lista é indexada pelo **quadro absoluto** de uma animação:

| Grupo | n | Listas |
|---|---|---|
| 0 | 2 | `WORLD1.ANM` (1 012 valores), `WORLD2.ANM` (913) — ids de grupo 10000 e 20000 |
| 1 | 2 | `DECALW1.ANM` (207), `DECALW2.ANM` (128) — ids 5001 e 5002 |
| 2 | 110 | uma por **tipo de objeto** (0–109); 26 não vazias (tipo 1 = `JAKE.ANM`, 933 valores) |

Uso: [`../re/som.md`](../re/som.md) e [`../re/videos-musica.md`](../re/videos-musica.md).

## `DLS.TBL` (12 288 bytes; leitor `0x0043be18`) [confirmado o formato]

Sem cabeçalho: **48 tabelas de 256 bytes** (`0x3000` bytes lidos para `0x493c14`), cada uma um
remapeamento de índice de cor (cor `i` → `tabela[i]`).

| Tabelas | Conteúdo observado |
|---|---|
| 0, 1, 34 | identidade |
| 2–17 | escurecimento progressivo (cada vez mais cores levadas a poucos índices escuros) |
| 18–33 | outra rampa, para tons próprios (`0xde`, `0xce`, `0xcc`, `0x9f`…) [inferido: clarão/cor] |
| 35–47 | recolorações (ex.: cores 1–2 → `0xdd`); 35–38 = cores dos personagens |

`0x43becd(tabela, fator)` compõe a tabela escolhida com uma tabela de sombra
(`0x496c18`) em `0x481048`, que o desenho usa; o resultado é recalculado só quando a
combinação muda. Tabelas ≥ 34 com fator ≠ 0 só remapeiam as 128 primeiras cores e parte do
resto [inferido]. Uso conhecido: `p=` do `ENGLEVEL.TXT` (35–38) e cor dos jogadores
(`../re/menus-restantes.md`).

## `DAMAGE.TBL`

Formato em [`../re/dano-vida.md`](../re/dano-vida.md) (149 registros de 15 `i32`).
