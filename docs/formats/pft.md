# Fontes (`.PFT`)

Formato da biblioteca gráfica "Phoenix" da Dynamix (classe `GFXFont`; caminhos de compilação
`e:\librarys\phoenix\source\g_font…` no executável). Leitores originais: `GFXFont::read`
(`0x00455719`), `GFXBitmapArray::read` (`0x00453712`), `GFXBitmap::read` (`0x00454289`).

Não usa os blocos `TAG:`: aqui cada bloco tem um identificador de 4 letras sem `:` e um
`u32` tamanho (little-endian). **Os tamanhos dos contêineres não são confiáveis**: o tamanho
declarado de cada `PBMP` é 4 bytes menor que o conteúdo real, e o erro se acumula em `PBMA` e
`PFNT`. O leitor original percorre os blocos internos sem usar esses tamanhos; o OpenHunted
faz o mesmo.

## Estrutura

```
PFNT
  head   palavra de versão (byte alto = versão 0; 24 bits baixos = blocos seguintes)
         + 11 × u32: [69, 0, N, altura, altura+1, 255, 0, altura, 0x10000, 0x10000, 0]
  PBMA                     (GFXBitmapArray)
    head   u32 blocos seguintes, u32 número de glifos N
    user   12 bytes por glifo (zerados nos dados vistos)
    rmap   u32 por glifo: o código do caractere de cada glifo
    PBMP × N               (GFXBitmap)
      head   u32 versão (1), u32 largura, u32 altura, u32 bits (8), u32 atributos (1)
      data   altura × passo bytes, com passo = largura arredondada para múltiplo de 4
```

O terceiro campo do cabeçalho da fonte é N; os demais têm significado provável (altura,
espaçamento entre linhas, cor transparente 255?) a confirmar quando o texto for desenhado.

Os pixels dos glifos valem 0 (fundo) ou 255 (traço) nos dados vistos.

## Versão 1.00

5 fontes, todas lidas: `GARAM12.PFT` e `SHELLFNT.PFT` (156 glifos, caracteres 1 a 156),
`CREDIT2.PFT`, `CREDITS.PFT` e `SLEVD.PFT` (255 glifos). O `rmap` vale `[1, 2, 3, …]`: o
glifo 0 é o caractere 1. O `ohtool extract` gera uma folha de glifos em PNG (grade 16 × 16,
caractere `c` na linha `c / 16`, coluna `c % 16`).
