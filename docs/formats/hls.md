# Arquivo de fases (`hunter.hls`)

Não é comprimido. É um índice no formato do [`RESOURCE.MAP`](volume.md) seguido, no mesmo
arquivo, do volume que ele descreve:

| Offset | Conteúdo |
|---|---|
| 0 | Cabeçalho de 4 bytes (`00 01 06 07`) |
| 4 | `u16` número de volumes (1) |
| 6 | Nome do volume, 13 bytes (`RESOURCE.001`, embora o volume esteja embutido) |
| 19 | `u16` número de entradas `N` |
| 21 | `N` × (`u32` hash, `u32` offset) |
| 21 + 8N | Volume embutido: em cada offset (contado a partir daqui), nome com 13 bytes, `u32` tamanho e dados |

Na versão 1.00: 201 entradas, volume a partir do byte 1629. Conteúdo: `ENGLEVEL.TXT` (lista
de missões, ver [`lev.md`](lev.md)) e as fases `.LEV`.

Abertura no executável: `0x004030ba` (caminho `%s\hunter.hls`) e `0x004406d3` (seleção de
fases, que lista `*.hls` e `*.lev`).
