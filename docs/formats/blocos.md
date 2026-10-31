# Blocos `TAG:`

Os recursos próprios da Dynamix (BMP, ANM, BRD, ITF, VCR) são sequências de blocos:

| Tamanho | Conteúdo |
|---|---|
| 4 | Tag: três caracteres e `:` (ex.: `BMP:`, `ANM:`, `TAG:`, `LAB:`, `PAL:`, `HED:`, `BDR:`, `ITF:`, `VCR:`) |
| 4 | `u32` tamanho `S` dos dados (little-endian) |
| S | Dados |

Nos jogos DGDS, o bit 31 do tamanho marca um bloco contêiner, cujos dados são outros
blocos. Nenhum bloco com esse bit foi visto no Hunter Hunted 1.00; o leitor recusa esse
caso com `Unsupported`, para que apareça se existir.

Exemplos na versão 1.00:

- `SABOUT.ANM`: `ANM:`(216) `TAG:`(30) `LAB:`(25) `PAL:`(25)
- `C1A38A0.BRD`: `HED:`(24) `BDR:`(30)
- `SABOUT.ITF`: `ITF:`(89) `TAG:`(37)
- `PLAY.VCR`: `VCR:`(513042) `TAG:`(1937)
