# Bitmaps (`.BMP`)

Não é o BMP do Windows: é um recurso da Dynamix com um bloco `BMP:` (ver
[`blocos.md`](blocos.md)) contendo um ou mais quadros de 8 bits indexados. Leitor original:
`0x0045b914`.

## Conteúdo do bloco `BMP:`

| Tamanho | Conteúdo |
|---|---|
| 2 | `u16` versão (1000 na versão 1.00; o original recusa valores maiores) |
| 2 | `u16` número de quadros `n` |
| 4 × n | Tabela: o primeiro `u32` é o offset do quadro 0 contado desde a tag `BMP:`; os demais, os offsets dos quadros seguintes. O leitor original **ignora** a tabela e lê os quadros em sequência |

Cada quadro:

| Tamanho | Conteúdo |
|---|---|
| 2 | `u16` largura |
| 2 | `u16` altura |
| 1 | Tipo: `0` = cru; outro valor = bloco comprimido (ver [`compressao.md`](compressao.md)) |

- **Cru:** depois do tipo, `u32` tamanho e `largura × altura` bytes.
- **Comprimido:** o tipo é o primeiro byte do bloco comprimido (`0x83` = método 3).

Os pixels são índices de paleta, linha a linha, sem preenchimento. O bitmap não indica a
paleta: ela vem do contexto (animação, interface, fase).

## Versão 1.00

1.654 bitmaps, todos lidos. Exemplos: `SEGOCBAK.BMP` (1 quadro, 640×480),
`SABOTBUT.BMP` (6 quadros, o primeiro 36×32), `SEGOCBUT.BMP` (63 quadros),
`ITF.BMP` (7 quadros, o primeiro 124×25 e cru, o único quadro cru de toda a versão).
