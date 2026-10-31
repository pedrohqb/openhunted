# Volume de recursos (`RESOURCE.MAP`, `RESOURCE.00n`)

Formato de arquivo de recursos da Dynamix, o mesmo do Johnny Castaway. Todos os inteiros
são little-endian.

## Mapa (`RESOURCE.MAP`)

| Offset | Tamanho | Conteúdo |
|---|---|---|
| 0 | 4 | Cabeçalho (`01 02 03 05` na versão 1.00; não validado) |
| 4 | 2 | Número de volumes `V` |

Em seguida, para cada um dos `V` volumes:

| Tamanho | Conteúdo |
|---|---|
| 13 | Nome do volume, ASCIIZ (ex.: `RESOURCE.001`) |
| 2 | Número de entradas `N` |
| 8 × N | Entradas: `u32` hash do nome, `u32` offset no volume |

## Volume (`RESOURCE.001`, `RESOURCE.002`…)

Em cada offset indicado pelo mapa:

| Tamanho | Conteúdo |
|---|---|
| 13 | Nome do recurso, ASCIIZ (ex.: `SX_3000.WAV`) |
| 4 | Tamanho `S` dos dados |
| S | Dados |

## Versão 1.00

2.312 entradas em 2 volumes. Por extensão: BMP 1654, WAV 280, BRD 225, ANM 87, PAL 26,
ITF 16, RES 13, PFT 5, TBL 3, SKE 2, VCR 1.

## Questões abertas

- O algoritmo do hash do nome (os nomes nos volumes bastam para montar o índice, então o
  hash não é usado). A confirmar na engenharia reversa do executável.
- O `hunter.hls` usa um cabeçalho parecido, mas com o índice e os dados no mesmo arquivo:
  ver `hls.md`.
