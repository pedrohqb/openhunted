# `PLAY.VCR`: gravação de comandos de desenho

Único arquivo do tipo no CD 1.00 (`RESOURCE.001`, 514 995 bytes). **O executável não o lê**
(`../re/casca-acabamento.md`): o formato abaixo vem só dos dados. O leitor do OpenHunted é
`openhunted_formats::vcr` (`Vcr::parse`, `Vcr::to_bytes` reproduz o arquivo byte a byte).

## Blocos [confirmado]

Dois blocos no formato comum dos recursos (`TAG4` + `u32` tamanho + corpo):

| Bloco | Tamanho | Conteúdo |
|---|---|---|
| `VCR:` | 513 042 | fita: comandos de 1 byte com argumentos de tamanho fixo |
| `TAG:` | 1 937 | `u16` n (121) e n entradas `u16` id + nome terminado em 0 |

Os nomes do `TAG:` são o que um comando desenha ou toca: `WORLD1.ANM,11310` (animação,
quadro), `INVBAR01.bmp`, `SX_3061.WAV` e um `(null)`. Ids 1–121, em ordem.

## Comandos [confirmado o tamanho; significado inferido]

O tamanho de cada comando é fixo: com a tabela abaixo a fita inteira se lê sem sobras, e
termina em `FB` no último byte. Inteiros little-endian; `i16` com sinal.

| Op | Bytes | Campos | Significado [inferido] | Vezes |
|---|---|---|---|---|
| `F2` | 9 | `u32` 121, `u32` 181 | cabeçalho: número de marcas, ? | 1 |
| `F6` | 9 | 4 `i16` | recorte (x0, y0, x1, y1); (0, 0, 639, 405) = vista sem a barra | 1 520 |
| `F7` | 9 | `i32` tabela, `i32` fator | tabela de cor (`0x43becd`): (1, 0) identidade, (35, 0) Jake… | 4 346 |
| `FA` | 3 | `u16` | liga modos de desenho (1, 8, 0x100 = tabela de cor) | 20 232 |
| `F9` | 3 | `u16` | desliga modos (1, 0x100, 0) | 16 917 |
| `FD` | 18 | `u16` marca, `u16` quadro, `u16` peça, `i16` x, `i16` y, `i32` a, `i16` b, `u8` espelho | desenha a peça de um quadro ampliado; a/b = 192 nas células (128 × 1,5), tamanhos nos decalques e 15 000, 8 969… nos objetos | 14 884 |
| `F8` | 9 | 4 `i16` | retângulo (sempre entre `FA 8` e `F5`) | 3 285 |
| `F5` | 1 | — | fecha o `F8` | 3 285 |
| `F4` | 13 | `u16` som, 5 `i16` | toca um som (3061, 3794, 5054…) com posição | 156 |
| `EF` | 1 | — | começa a barra de estado (sem ampliação) | 634 |
| `F1` | 13 | `u16` marca, `u16` quadro, 4 `i16` | desenha um quadro de bitmap em (x, y) com (largura, altura): `INVBAR01.bmp` na barra | 3 441 |
| `FC` | 1 | — | fim do quadro de tela | 634 |
| `FB` | 1 | — | fim da fita | 1 |

São 634 quadros (31,7 s a 20 por segundo) de uma missão do Minotauro (`MINOTAUR.ANM`, `OSE`,
`OH1`, `OPP`, `ONS`) com o cenário ampliado 1,5×: uma ferramenta de depuração ("F6:VCR" na
ajuda `0x4719f4`) que grava as chamadas ao desenhista, não a entrada do jogador. Não dá para
"reproduzir" pelo motor: não há sementes nem entradas a repetir.

## Em aberto

- O 2º número do `F2` (181), os modos 1 e 8 de `FA`/`F9`, os campos a/b e a peça do `FD`,
  o retângulo `F8` (preenchimento?) e os cinco números do `F4`.
- Que missão foi gravada.
