# Compressão dos recursos (engenharia reversa)

## Descompressor genérico — `0x00466d1c`

Recebe um fluxo posicionado no início de um bloco comprimido e um destino. Lê, nesta ordem:

1. `u8` tipo. O método é `tipo & 0x7f`; só 0 a 3 são aceitos (outro valor: recua um
   byte no fluxo e falha). O bit `0x80` não muda nada no despacho: os bitmaps usam `0x83`.
2. `u32` tamanho armazenado. O tamanho dos dados comprimidos é esse valor menos 4.
3. `u32` tamanho descomprimido.
4. Os dados comprimidos.

Depois despacha:

| Método | Função | Comportamento |
|---|---|---|
| 0 | `0x004673e8` | cópia direta (`rep movsd` + `rep movsb`) |
| 1 | fluxo criado em `0x00459cce` sobre um fluxo de memória (`0x00459a50`) | não visto nos dados |
| 2 | `LZHRStream` (`0x00458f22`) sobre um fluxo de memória | não visto nos dados |
| 3 | `0x00467050` | LZ com fluxo de flags separado (abaixo) |

Devolve o tamanho descomprimido.

## Método 3 — `0x00467050`

Entrada: os dados comprimidos. Saída: bytes escritos.

**Contagem de tokens.** O primeiro byte `n` diz quantos bytes vêm a seguir para o número de
tokens `T`, em big-endian:

- se `n > 1`: dois bytes, deslocados de 24 e 16 bits;
- se `n > 0`: um byte, deslocado de 8 bits;
- sempre: um byte final, sem deslocamento.

Ou seja: `n = 0` → `T` em 1 byte; `n = 1` → 2 bytes; `n ≥ 2` → 4 bytes.

**Fluxos.** Logo depois da contagem começa o fluxo de flags, com `⌈T / 8⌉` bytes; o fluxo
de dados começa imediatamente depois dele.

**Tokens.** Lê-se o primeiro byte de flags. Para cada token `i` de 0 a `T-1`:

- se o bit mais alto do byte de flags atual estiver ligado: **referência**. Lê dois bytes
  do fluxo de dados formando uma palavra big-endian `w`. Comprimento = `(w & 0x3f) + 3`;
  distância = `(w >> 6) + 1`. Copia, byte a byte (as cópias podem se sobrepor), a partir de
  `saída - distância`;
- senão: **literal**. Copia um byte do fluxo de dados para a saída.

Depois de cada token: se `i % 8 == 7`, carrega o próximo byte de flags; senão, desloca o
byte de flags um bit para a esquerda.

A rotina original não confere limites: o OpenHunted confere e falha em vez de ler ou
escrever fora dos buffers.

## Uso nos dados da versão 1.00

- Quadros de bitmap (`BMP:`): tipo `0x00` (cru, 1 bitmap) ou `0x83` (método 3, os outros
  1653). Exemplo: `SABOUT.BMP`, quadro de 320×256, tamanho armazenado `0xF813`, tamanho
  descomprimido `0x14000` (= 320 × 256).
- O `hunter.hls` **não** é comprimido.
- Não se encontrou uso dos métodos 1 e 2; o leitor os recusa com `Unsupported` até que
  apareçam.
