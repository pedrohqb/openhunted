# Pausa, Esc e o menu do Windows durante o jogo

Ver [`menu.md`](menu.md) (máquina de telas), [`interface-itf.md`](interface-itf.md) (caixa
genérica `0x445dc8`) e [`fim-de-missao.md`](fim-de-missao.md).

## Não há menu de pausa próprio [confirmado salvo indicação]

O laço do jogo (`0x40433c`) não trata Esc: as únicas teclas lidas ali são **F1** (`0x70` →
`0x4041a8`, ajuda) e `S` (passo a passo, só com a depuração `0x471980 = 0`). Nenhuma
comparação com 0x1b (Esc) existe fora das telas `.ITF` e da configuração de teclas.
[confirmado pela busca no executável; o efeito do Esc no Windows, inferido: nenhum]

As ações de partida ficam no **menu da janela** (recurso `PROGRAMMENU` do executável), tratado
por `0x401624(comando)`:

| Menu | Item | Comando | Efeito |
|---|---|---|---|
| File | Exit | 1000 | `0x43f826`: 10005 "ARE YOU SURE YOU WANT TO QUIT HUNTER HUNTED?" YES/NO |
| Music | Theme #1… | 2000–2099 | faixas do CD |
| Mission | Choose Mission | 2107 | caixa 10007 YES/NO; YES → máquina de telas no estado 5 (`SLEVL`) |
| Mission | End Mission | 2100 | caixa **10007** YES/NO; YES → `0x497328 = 2`, comando 101 (fim de missão como derrota) |
| Mission | View Goal | 2106 | briefing `0x4414e0` (`briefing.md`) |
| Mission | Pause Game | 2105 | caixa 10006 "GAME PAUSED" com OKAY |
| Options | Input Devices | 2101 | `SINPU.ITF` e telas de teclas |
| Options | Sound Volume / Music Volume | 2400–2500 / 2200–2300 | `SfxVolume`/`MusicVolume` = comando − base (`0x497310`/`0x49730c`) |
| Options | Difficulty Level | 2102–2104 | `Difficulty` = comando − 2102 |
| Options | Brightness | 1402–1406 | paleta `WORLD_G%d.PAL` com `d = comando − 1401` |
| Help | Online Documentation / About | 2600 / 2602 | ajuda / `SABOUT.ITF` |

Os textos 10005–10008 do `SYSTEM.RES` (linha `n` = id `10000 + n`):

| Id | Texto |
|---|---|
| 10005 | ~xARE YOU SURE YOU WANT TO QUIT HUNTER HUNTED? |
| 10006 | GAME PAUSED |
| 10007 | ~xARE YOU SURE YOU WANT TO STOP PLAYING THE CURRENT MISSION AND CHOOSE A DIFFERENT ONE? |
| 10008 | ~xARE YOU SURE YOU WANT TO END THE CURRENT MISSION? |

"End Mission" usa o **10007**, não o 10008 (que fica sem uso): defeito do original.
[confirmado: `0x401872` e `0x40181d` empurram ambos 0x2717]

### Barra de menu em tela cheia (`0x402a94`) [inferido em parte]

Com a janela do tamanho da tela, a barra de menu fica escondida e aparece quando o mouse fica
mais de 500 ms na faixa do topo (altura do menu + legenda, `GetSystemMetrics(15)` e `(4)`);
some quando ele sai. `0x47119c` = menu visível.

## Pausa por tecla (`0x402a94(0)`, chamada a cada tique do jogo) [confirmado]

Se nenhuma tela ITF está aberta (`0x483a04 == 5000`) e a tecla lida é **`P`** (0x50) ou
**Pause** (0x13), e essa tecla **não** está entre as 12 teclas configuradas dos jogadores 1 e
2 (`+0xb4` dos dispositivos `0x499018`/`0x49901c`), manda o comando 2105: caixa genérica
"GAME PAUSED" / OKAY. A caixa é modal: o jogo não avança até o OK (Enter, clique ou botão do
joystick).

## O que um port mínimo precisa

- Pausa: `P`/Pause → caixa "GAME PAUSED" com OKAY, jogo parado.
- Sair da missão: uma ação equivalente a "End Mission" (pergunta 10007, YES → derrota →
  tela de resultado) ou a "Choose Mission".
- Esc no jogo: o original não faz nada; mapear Esc para o menu é uma escolha do port, não
  fidelidade.

## Em aberto

- Formato visual exato da caixa genérica (`SGENERIC.BMP`) e dos códigos `~x`.
- `0x4041a8` (ajuda: provavelmente `WinHelp` com `HUNTHELP.HLP`).
- Comportamento do menu quando a janela não está em tela cheia.
