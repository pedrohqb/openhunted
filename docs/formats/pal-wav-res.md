# Paletas, sons e textos

Três formatos comuns, sem nada próprio da Dynamix.

## Paleta (`.PAL`)

RIFF padrão do Windows: `RIFF <tam> "PAL "`, com um sub-bloco `data` contendo `u16`
versão `0x0300`, `u16` número de cores (256) e as entradas de 4 bytes `R G B flags`.
São 26 paletas na versão 1.00 (`WORLD.PAL`, `SMAIN.PAL`, `INTRFACE.PAL`, `JAKE*.PAL`…).

## Som (`.WAV`)

RIFF `WAVE` PCM padrão. Os 280 sons da versão 1.00 passam no leitor; o primeiro,
`SX_3000.WAV`, é mono, 11025 Hz, 8 bits. Outros formatos que não PCM são recusados.

## Textos (`.RES`)

Texto puro em Latin-1, uma entrada por linha (CRLF). Linhas vazias têm significado
(por exemplo, nos créditos), então são preservadas. Alguns arquivos começam com uma
linha de comentário ("This file contains…"), que fica a cargo de quem consome a lista.
