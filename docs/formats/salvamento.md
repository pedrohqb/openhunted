# Jogo salvo (`.hhg`) e lista de heróis (`hunter.fam`)

Comportamento em [`../re/opcoes-salvamento.md`](../re/opcoes-salvamento.md). Inteiros
little-endian; `i32` com sinal.

## Jogo salvo (`*.hhg`) [confirmado]

Cópia crua de **1 960 bytes** (`0x7a8`) da memória a partir de `0x47f79c`, gravada com
`fwrite` e lida com `fread` (`0x440ce4`, `0x440bc6`). Sem cabeçalho além da assinatura, sem
compressão nem soma de verificação. Extensão padrão `hhg`, local escolhido no diálogo.

| Desl. | Tamanho | Endereço | Conteúdo |
|---|---|---|---|
| `0x000` | 32 | `0x47f79c` | assinatura ASCIIZ `"Hunter hunted saved game"` (conferida na carga) |
| `0x020` | 128 | `0x47f7bc` | nome do jogador (o da lista de heróis) |
| `0x0a0` | 4 | `0x47f83c` | número de entradas da lista de missões [efêmero; gravar 0] |
| `0x0a4` | 4 | `0x47f840` | número de missões da campanha no modo solo |
| `0x0a8` | 4 | `0x47f844` | ? (zerado no New Game) |
| `0x0ac` | 4 | `0x47f848` | índice da missão corrente |
| `0x0b0` | 4 | `0x47f84c` | índice da última missão carregada |
| `0x0b4` | 4 | `0x47f850` | pontuação acumulada (soma dos recordes) |
| `0x0b8` | 4 | `0x47f854` | id desta campanha na lista de heróis (0 = nenhum) |
| `0x0bc` | 4 | `0x47f858` | peças do carro de fuga (0–15) |
| `0x0c0` | 280 | `0x47f85c` | caminho do próprio arquivo salvo |
| `0x1d8` | 280 | `0x47f974` | fase corrente, ex. `HUNTER.HLS|T_J_001.lev` |
| `0x2f0` | 1 200 | `0x47fa8c` | `i32[300]`: recorde de cada missão (índice da lista solo; 0 = não vencida) |
| `0x7a0` | 4 | `0x47ff3c` | ponteiro da lista de missões [lixo; não usar] |
| `0x7a4` | 4 | `0x47ff40` | capacidade da lista de missões [efêmero] |

Strings ASCIIZ com o resto do campo indefinido (o New Game zera os campos). Um leitor
compatível deve aceitar arquivos de 1 960 bytes que comecem pela assinatura e ignorar os
campos marcados efêmeros.

Um escritor compatível grava `0x0a0` **zero** junto com o ponteiro e a capacidade
[confirmado]: depois da carga, o NEXT (`0x440e95`) só remonta a lista (`0x4406d3`) quando
`0x47f83c = 0`; com a contagem e o ponteiro nulo, ele segue por `0x440f25`, que lê
`[0x47ff3c]` (acesso a nulo), e `0x440388` percorre `contagem` entradas.

## Lista de heróis (`hunter.fam`) [confirmado]

Escrita por `0x445098`, lida por `0x444e3b`; pasta corrente.

| Tamanho | Conteúdo |
|---|---|
| 4 | `u32` contador: último id de campanha dado (claro) |
| 1 | chave `k` (sorteada 0–255 a cada gravação, em claro) |
| … | entradas, **cada byte com XOR `k`** |

Cada entrada (até 14, em ordem decrescente de pontuação):

| Tamanho | Conteúdo |
|---|---|
| n + 1 | nome ASCIIZ |
| 4 | `i32` pontuação |
| 4 | `i32` id da campanha |

Leitura original: lê o contador, lê o resto do arquivo e aplica XOR com o **primeiro byte**
a tudo (a chave vira 0), depois percorre as entradas a partir do byte 1 até o fim do arquivo.
A "cifra" é só ofuscação.
