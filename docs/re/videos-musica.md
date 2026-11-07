# Vídeos, música e vozes

Complementa [`menu.md`](menu.md) (abertura), [`fim-de-missao.md`](fim-de-missao.md) e
[`som.md`](som.md) (ids, canais, `SOUND.TBL`). Formato das tabelas em
[`../formats/tbl.md`](../formats/tbl.md).

## Vídeos

### Arquivos [confirmado pelos cabeçalhos e pelo ffprobe]

Nove `.AVI` na pasta `Movies\` do CD. **Não é Smacker**: é AVI RIFF com vídeo **Indeo 3.2**
(`IV32`, 320×240, 15 qps) e áudio **PCM sem sinal de 8 bits, estéreo, 22 050 Hz**.

| Arquivos | Quadros | Duração | Uso |
|---|---|---|---|
| `HHDARK`, `HHNORMAL`, `HHLIGHT` | 530 | 35,3 s | abertura (depois do logotipo) |
| `MIDDARK`, `MIDMED`, `MIDLITE` | 307 | 20,5 s | meio da campanha (7ª peça do carro) |
| `OUTDARK`, `OUTMED`, `OUTLITE` | 629 | 41,9 s | final (15ª peça do carro) |

As três variantes são o mesmo filme com brilho diferente. Decodificação no porte: o
**FFmpeg** (`libavcodec`, decodificador `indeo3`, e `pcm_u8`) lê os nove arquivos (conferido:
quadro 100 do `HHNORMAL` decodificado). Não há decodificador Indeo 3 em Rust puro conhecido;
alternativa é converter os AVI uma vez na instalação. libsmacker não se aplica.

### Escolha da variante [confirmado]

Chave `Brightness` do `HUNTER.INI` (padrão 1402; valores 1402–1406, ver
[`opcoes-salvamento.md`](opcoes-salvamento.md)):

| `Brightness` | Abertura (`0x4399e0`) | Meio/final (`0x444174`, tabela em `0x480488`) |
|---|---|---|
| 1402 | `hhdark.avi` | `midlite.avi` / `outlite.avi` |
| 1403, 1404 | `hhnormal.avi` | `midmed.avi` / `outmed.avi` |
| 1405, 1406 | `hhlight.avi` | `middark.avi` / `outdark.avi` |

A ordem está **invertida** entre a abertura e os outros filmes (tabela `0x480488`: lite,
med, dark). Como 1402 = `WORLD_G1.PAL`, a paleta mais escura, a abertura parece a regra
pretendida e o meio/final um erro do original [inferido]. Outro valor qualquer não escolhe
filme (ponteiro nulo).

### Tocar (`0x43a6f1(nome)` → `0x44d6d8`) [confirmado]

1. Caminho: `"%s\..\Movies\%s"` com `CDPath` do `Sierra.ini` (padrão `.`).
2. Limpa a tela, para a música (`0x4042c4`) e o som 10000, paleta `INTROBLK.PAL` (toda preta).
3. Tamanho: `Sierra.ini [Config] CDROM` (velocidade do drive, padrão 2). **≥ 3 → 640×480**
   (o MCI amplia ×2); senão **320×240 centrado** (sinal 1 de `0x44d6d8`: centra na janela).
4. MCI: `open AVIVideo`; `open <arq> alias mov style child parent <janela>`; janela e paleta
   do filme; `play mov window`.
5. Laço: bombeia mensagens e lê a entrada até `status mov mode` deixar de ser `playing` ou
   `seeking`, ou até **qualquer tecla, botão do mouse ou botão de joystick** (pular).
6. `stop mov`, `close mov`, restaura a paleta.

Devolve 0 se foi interrompido (`0x43a123`), o que encerra a sequência da abertura.

### Quando tocam [confirmado]

- **Abertura**: logotipo → `hh*.avi` → título → história → créditos (`menu.md`).
- **Meio**: depois de uma vitória que dá a **7ª** peça do carro (`0x47f858 = 7`), em
  `0x444174`: tela preta, música parada, espera 1 000 ms, voz `SX_4102.WAV` (2,5 s), o filme,
  tela preta e voz `SX_4103.WAV` (2,3 s). Cada espera pode ser pulada.
- **Final**: com a **15ª** peça: para o som, filme `out*.avi`, depois os **créditos**
  (`0x4399e0(2)`), e segue para o resultado da missão.
- O modo da janela passa a 2 durante os filmes (`0x403500(2)`: menus cinza, sem música de
  menu) e volta a 1 depois.

A tela `S_CAR.ITF` aparece antes, em toda vitória que dá peça (`menus-restantes.md`).

## Música

### Três fontes, por classe de id (`0x449978`) [confirmado]

| Id | Classe | Tocador |
|---|---|---|
| 1000–1999 | 1 | MCI `sequencer`, arquivo `SX_%04d.MID` (`0x44ee40`). **Sem uso**: não há `.MID` nos recursos |
| 2000–2999 | 2 | **faixa de CD de áudio** `id % 100` (MCI `cdaudio`, `0x44d117`) |
| 3000 | 3 | `SX_3000.WAV` — a **música dos menus** |

### Menus: `SX_3000.WAV` [confirmado]

PCM mono 8 bits, 11 025 Hz, 40,2 s, nos `RESOURCE.*`. `0x402d48` (uma vez por quadro do laço
de telas e do jogo): no modo de janela 1 (telas cheias `0x43f876`, história/créditos) para o
CD e toca o 3000 com **laço** (sinais 7, prioridade 5 000; não reinicia se já toca); no modo
2 (filmes) para o 3000. Exceção: na tela de opções, depois de mexer no volume da música
(`0x47f444 = 1`), toca o CD para o jogador ouvir.

### Jogo: faixas do CD [confirmado salvo indicação]

Não há música por missão: o jogo **percorre as faixas de áudio do CD em sequência**.

- Abertura do CD: `open cdaudio alias tbCDaudio wait`, formato `tmsf`; conta as faixas e
  guarda a **primeira faixa de áudio** (`0x49c544`, a primeira cujo `type` é `audio`) e a
  última (`0x49c548`); identidade do disco em `0x49c54c`.
- Faixa corrente `0x497308` (= 2000 + faixa; **2000 = "None"**). A cada 1 000 ms, no modo
  0, se o CD não está tocando, a fila do locutor está vazia e o som de classe 4 não toca:
  avança para a próxima faixa (passando da última → volta à primeira de áudio) e toca
  `play tbCDaudio from N` (até o fim do disco, `0x49c550 = 1`, prioridade 30 000).
- Começa na primeira faixa de áudio, salvo se o CD já estava tocando (então continua a
  faixa corrente). `NoMusicStart ≠ 0` no `HUNTER.INI` só pula a chamada de `0x402d48` logo
  depois da abertura (`0x401544`); como ela roda a cada quadro depois, o efeito é adiar o
  início [inferido].
- `MusicVolume < 10` → música desligada (`0x49730c = 0`, CD parado).
- O id com laço (sinal 2) repete a faixa (`0x44d495`, `play from N to N+1`); o jogo não usa
  laço no CD.

**Menu "Music" da janela** (comandos 2000–2099; o comando é a faixa). Com o CD do jogo
(`0x403ad7`: primeira faixa de áudio = 2 e existe `X:\hunter\hunter.exe` no drive do
`CDPath`), os nomes vêm do `SYSTEM.RES`: 2000 "None" (10011), faixa *t* → texto `10010 + t`:

| Faixa | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11–21 |
|---|---|---|---|---|---|---|---|---|---|---|
| Nome | Shock Wave | Oblivion | Titanium Tide | Rampant | Time | Altered Reality | Stone Soul | Revenge | Damage | "Song #10"… |

Com outro CD de áudio, os nomes vêm do `CDPLAYER.INI` do Windows (seção = identidade em
hexadecimal, chave = número da faixa, padrão "Track #%d"). Sem CD: item "CD audio not
available!" (10032), cinza.

As faixas de áudio **não estão nesta cópia** (`hh_iso` tem só a trilha de dados); que o CD
tenha 9 faixas de áudio a partir da 2 é [inferido] pelos nomes. O porte pode ler faixas
extraídas (`trackNN.ogg/.wav`) com a mesma regra de sequência.

## Vozes [confirmado salvo indicação]

- `SX_4100.WAV` e `SX_4101.WAV`: tocadas na abertura depois do logotipo e do título
  (`0x439bdd`, esperando o fim; puláveis) [inferido: conteúdo não ouvido].
- Locutor de dois jogadores (`0x444a53`): cooperativo vitória 3780+3788, tempo esgotado
  3776+3787, derrota 3777+3787; disputa: vence o jogador 1 → 3790, o 2 → 3792, empate →
  3773 "Death Match Tie" [inferido pelos ramos].
- **3714–3741** já têm nome no `SOUND.RES`; onde tocam:

| Id | Nome (`SOUND.RES`) | Onde |
|---|---|---|
| 3714 / 3732 | Jake "I need a key" / Monster "Need key" | porta trancada sem chave (`0x40f040`) |
| 3715 / 3730 | "It's locked" / "Locked" | idem |
| 3719 / 3741 | "It won't budge" / "Won't budge" | idem, porta que não abre |
| 3718 / 3731 | "Ah hah!" / "Uunngh?" | segredo achado (`0x40b3f5`, `0x40ebdc`) |
| 3717 / 3737 | "Oh no it's poison" / "Poison bad!" | veneno (`0x40b6ed`, `0x40f324`, `0x40f801`) |
| 3735 / 3736 | suspiro / respiração com veneno (Garathe) | `0x40b6ed`, `0x4284c5` |
| 3734 / 3740 | (sem nome) | pulo contra parede do Garathe (`garathe.md`) |

Sem nome e sem referência direta no código: 3733, 3739, 3742, 3743, 3770, 3771, 3778, 3779,
3783, 3784, 3789, 3791, 3793, 3794, 3796, 3797 (possivelmente tocados por `SOUND.TBL` ou
não usados).

## `SOUND.TBL`: grupos 0 e 1 [confirmado pelo número de quadros]

`som.md` cobre o grupo 2 (por tipo de objeto). Os outros dois são do **cenário**:

| Grupo | Lista | Animação | Quadros | Quem toca |
|---|---|---|---|---|
| 0 | id 10000 | `WORLD1.ANM` | 1 012 | células (`0x432808`: grupo = campo `+0` da célula, quadro corrente) |
| 0 | id 20000 | `WORLD2.ANM` | 913 | idem |
| 1 | id 5001 | `DECALW1.ANM` | 207 | decalques (`0x4305ac`: grupo = 5000 + `+0x14`) |
| 1 | id 5002 | `DECALW2.ANM` | 128 | idem |

As listas têm uma entrada por quadro da animação e o som toca quando o quadro muda, na
posição da célula (+64, +64) ou do decalque, como no grupo 2. Resolução em `0x43c2b6`: grupo
≥ 10000 → `0x496c20[grupo/10000]`; 5001–9999 → `0x491e08[grupo]`; 1–109 → `0x496c34[tipo]`.

## Correções

- `fim-de-missao.md`: o 7/15 que escolhe o filme é o **número de peças** (`0x445c07`
  devolve `0x480598`), não o retorno de `S_CAR.ITF`.
- `fim-de-missao.md`: o texto que pede o nome do jogo salvo é o id **10034** (`0x2732`,
  "PLEASE PICK A SAVE GAME FILENAME."), que existe; não 10050.

## Em aberto

- Conteúdo das vozes 4100–4103 e das sem nome (não ouvidas).
- Lista real de faixas do CD (falta a imagem com as faixas de áudio).
- Escala de volume do CD (`0x44e040(2, v)`: MCI não tem volume; talvez o mixer do Windows).
- Se a inversão lite/dark do meio/final é mesmo um erro.
