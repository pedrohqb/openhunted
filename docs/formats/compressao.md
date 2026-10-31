# Compressão

Blocos comprimidos aparecem nos quadros de bitmap. Engenharia reversa e endereços:
[`docs/re/compressao.md`](../re/compressao.md).

## Bloco

| Tamanho | Conteúdo |
|---|---|
| 1 | Tipo; método = `tipo & 0x7f` |
| 4 | `u32` tamanho armazenado = tamanho dos dados comprimidos + 4 |
| 4 | `u32` tamanho descomprimido |
| … | Dados comprimidos |

Métodos: 0 = cópia; 3 = LZ com flags; 1 e 2 existem no executável mas não aparecem nos
dados da versão 1.00 (o leitor os recusa).

## Método 3: LZ com flags

```
n            u8: quantos bytes extras tem a contagem (0 → 1 byte; 1 → 2; ≥2 → 4)
T            contagem de tokens, big-endian, 1, 2 ou 4 bytes
flags        ⌈T/8⌉ bytes, lidos do bit mais alto para o mais baixo
dados        literais (1 byte) e referências (2 bytes)
```

Para cada token, o próximo bit de flag decide:

- `0`: copia 1 byte dos dados para a saída;
- `1`: lê uma palavra big-endian `w` dos dados; copia `(w & 0x3f) + 3` bytes a partir de
  `(w >> 6) + 1` bytes atrás na saída (a cópia pode se sobrepor ao que está sendo escrito).

O OpenHunted exige que o resultado tenha exatamente o tamanho descomprimido declarado e
recusa referências que apontem antes do início da saída.

API: `decompress(método, dados, tamanho)` e `unpack_block(bloco)`.
