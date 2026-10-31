# Briefing da missão (`SLEVD.ITF`)

Ver [`menu.md`](menu.md) (como se chega aqui) e [`interface-itf.md`](interface-itf.md)
(controles e despacho).

## Quando aparece [confirmado]

- Estado 6 da máquina de telas (`0x4414e0`): depois de "New Game", de escolher uma missão, de
  "Replay" ou "Next" na tela de resultado. A fase **já foi carregada** e um quadro dela
  desenhado (`0x4365ee(0)`); o briefing é um diálogo por cima.
- Durante o jogo, pelo menu "Mission → View Goal" (comando 2106, `0x401846`).
- Solo (`0x497300 == 1`) ou cooperativo → `SLEVD.ITF` com o procedimento `0x4415ba`; disputa
  → `SHVLD.ITF` (`process_deathmatch_interface`, fora do escopo).

## Geometria [confirmado]

Diálogo modal (`0x4634d5`), centrado na tela: retângulo envolvente 338×240 → canto em
(151, 120) numa tela 640×480. Controles (coordenadas relativas ao diálogo):

| Id | Nome | Retângulo | Papel |
|---|---|---|---|
| 21500 | (fundo) | 0, 0, 338×240 | rótulo 2 de `slevd.ANM` |
| 21501 | `LDM_GO` | 36, 196, 133×32 | botão (rótulo 3) |
| 21502 | `LDM_CANCEL` | 175, 196, 133×32 | botão (rótulo 4) |
| 21503 | `LDM_TEXT_AREA` | 52, 59, 238×115 | área de texto (F = 0) |
| 21504 | `LDM_FAN_ANIM` | 45, 49, 253×127 | ventilador animado em laço (rótulo 5) |
| 21505 | `LDM_UP` | 311, 26, 22×26 | seta (rótulo 8) |
| 21506 | `LDM_DOWN` | 311, 184, 22×27 | seta (rótulo 9) |
| 21507 | `LDM_SLIDER` | 311, 52, 25×132 | barra de rolagem (rótulo 11) |

O ventilador fica atrás da área de texto (o texto é escrito por cima dele).

## Abertura (mensagem 0x110) [confirmado]

1. Instala a navegação por teclado (`0x44b60a`).
2. 21504: avanço de quadro em laço; sinal 1.
3. Fonte **`slevd.pft`** (`0x497c74`); botões 21501 e 21502 com sinais 0x81.
4. **Texto** (`0x4418b3`): `sprintf("%s\n\n%s", título, descrição)` — o título
   (`0x488a24`, o `t=`/campo título do cabeçalho), uma linha em branco e a descrição
   (`0x488a54`, o `d=`). Bloco de texto em `0x497d94`: fonte `slevd.pft`, retângulo da área
   21503, modo 2 (alinhamento/quebra, [inferido]: quebra de linha automática), **cor 162**,
   primeira linha visível = posição da barra / 3. Mede as linhas com altura 500 (`0x45f330`) e
   guarda o total em `0x497c78`; desenha com `0x45f1dc`.
5. Mais de **5 linhas** → liga as setas (avanço de quadro `0x43e011`) e a barra 21507 (faixa
   0…(linhas − 5) × 3); senão **remove** a barra e as setas.
6. Cursor do mouse no centro de **GO** (Enter = GO).
7. Para a música (`0x44ded8(10000)`) e tenta tocar a narração: nome da fase com a extensão
   trocada por `.wav` (`T_J_001.wav`) pelo id especial 4999. Não há esses arquivos no CD
   1.00 [confirmado pela listagem], então nada toca.

Não há "aperte uma tecla": o jogador clica GO (ou Enter, ou o botão 1 do joystick com o
cursor em GO).

## Comandos (0x2400) [confirmado]

| Id | Efeito |
|---|---|
| 21501 GO | fecha com **7** → a máquina de telas sai e o laço do jogo começa |
| 21502 CANCEL ou 2 (Esc) | fecha com o estado anterior (`0x47f53c`): volta à tela de onde veio |
| 21505 / 21506 | rola uma linha para cima / para baixo (`0x43deb5`, passo ±1 na barra de 3 em 3) |

Depois de fechar, `0x4414e0` para o som especial 4 (`0x44ded8(4)`) e, se o valor é 7 **e a
partida ainda não começou** (`0x4879c4 == 0`), toca o anúncio 3795 (solo/cooperativo) ou 3775
(disputa) pela fila do locutor (`0x43c868`; o 2000 passado deve ser um atraso, [inferido]). Pelo "View Goal" no meio do jogo não toca
nada.

## Texto de exemplo (`T_J_001`)

```
TUTORIAL: Jake Gets a Gun

Get to the exit.  Use the pistol to shoot the hovering mines.  Good luck Jake!
```

## Em aberto

- Semântica exata do modo 2 do bloco de texto e da quebra de linha (`0x45f330`/`0x45f1dc`
  não lidos por dentro); a altura da linha da `slevd.pft` dá ~23 px (5 linhas em 115).
- O conteúdo do anúncio 3795 (sem nome no `SOUND.RES`).
- Onde o id 4999 procura o `.wav` (recursos ou pasta) — irrelevante na 1.00.
