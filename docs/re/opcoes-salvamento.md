# Opções, dificuldade, jogo salvo e recordes

Telas em [`menus-restantes.md`](menus-restantes.md); formatos dos arquivos em
[`../formats/salvamento.md`](../formats/salvamento.md).

## Arquivos de configuração [confirmado salvo indicação]

`0x44a661(seção, chave, padrão, arquivo)` = `GetPrivateProfileString`; `0x44a627` lê inteiro,
`0x44a6ec` grava inteiro. Seção ou arquivo nulos → padrão (`0x44a51c`):

- arquivo: o caminho do executável com `.INI` no lugar da extensão (**`HUNTER.INI`** ao lado do
  `Hunter.exe`); se ele não abre, procura `InstallPath` no `\SIERRA.INI` do Windows e usa
  `<InstallPath>\HUNTER.INI` [inferido: teste pelo `fopen`];
- seção: o nome do executável sem extensão (`[Hunter]`).

### Chaves do `HUNTER.INI`

| Chave | Padrão | Uso |
|---|---|---|
| `Difficulty` | 1 | 0 fácil, 1 médio, 2 difícil; fora de 0–2 vira 1 (`0x43dfef`) → `0x487854` |
| `MusicVolume` | 25 | 0–100; < 10 = música desligada (`0x49730c`) |
| `SfxVolume` / `SFXVolume` | 75 | 0–100; < 10 = efeitos desligados (`0x497310`, a "chave" de `som.md`) |
| `Brightness` | 1402 | 1402–1406 → paleta `WORLD_G1…G5.PAL` (`WORLD_G%d`, 1 a mais escura) e variante dos filmes |
| `NoMusicStart` | 0 | ver `videos-musica.md` |
| `NumSounds` | 1, depois 8 | ≠ 0 liga o DirectSound (`0x44da84(1)`); a 2ª leitura vai a `0x44e2d9` [inferido: canais] |
| `HideMouse` | — | esconde o cursor [inferido pelo nome] |
| `HunterX`, `HunterY`, `HunterWidth`, `HunterHeight`, `HunterShow`, `HunterShow2` | — | posição/estado da janela |
| `Player1Input` (e do jogador 2) | — | dispositivo: `Keyboard1`, `Keyboard2`, `Joystick1`…`Joystick4` (tabela `0x47f4c0`) |
| `InputKeys1`, `InputKeys2` | — | 10 inteiros `%d,…`: teclas virtuais (nomes no `KEYS.RES`, índice = código VK) |
| `Joystick1`, `Joystick2` | — | 6 inteiros: botões das ações |

`Sierra.ini` (explícito): `CDPath` (padrão `.`), `[Config] CDROM` (velocidade, padrão 2).

## Tela de opções (`SOPTI.ITF`, `0x43dfdc` + `0x43e26a`) [confirmado]

Tela cheia; teclado/joystick por `0x43e069`. Sem botões tipo 6 além de DONE: os outros são
imagens tipo 0 com rotina de clique própria (`0x4431d6`/`0x4432a8`), agrupadas em
**grupos de opção** (`0x443257(primeiro, último, marcado)` liga um e apaga os outros).

| Controles | Grupo | Valor |
|---|---|---|
| 12001 `MO_EASY`, 12002 `MO_MED`, 12003 `MO_HARD` | dificuldade | `Difficulty = id − 12001`; também `0x497490` |
| 12004–12008 `MO_LEVEL1…5` | brilho | `0x401624(1402 + id − 12004)`: troca a paleta já; grava `Brightness` |
| 12020 `MO_MUSIC_VOLUME` (12024/12025 setas −/+) | barra 0–100 | passo 10; grava `MusicVolume`, `0x44e040(2, v)`, `0x49730c = v` |
| 12018 `MO_SFX_VOLUME` (12021/12022 setas −/+) | barra 0–100 | passo 10; grava `SfxVolume`, `0x44e040(3, v)`, `0x497310 = v` |
| 12026 `MO_DONE`, Esc | — | estado 2 (menu principal) |
| 12028 `MO_SCREW_ANIM` | — | animação em laço |

As barras usam `0x43da74(id, 0, 100, valor)`; as setas chamam `0x43deb5(seta, barra, ±10)` e
reenviam o comando da barra. Tudo é gravado **na hora** no `HUNTER.INI`.

## Menu da janela durante o jogo [confirmado]

`0x401624(comando)` trata os mesmos ajustes; `0x43d33b` reaplica tudo e marca os itens:

| Comandos | Efeito |
|---|---|
| 1402–1406 | brilho (paleta `WORLD_G{c − 1401}.PAL`) |
| 2000–2099 | faixa de CD (`videos-musica.md`) |
| 2102–2104 | dificuldade `c − 2102` |
| 2200–2300 | volume da música `c − 2200` (itens de 10 em 10) |
| 2400–2500 | volume dos efeitos `c − 2400` |

## Dificuldade (`0x487854`) [confirmado]

Lida do `HUNTER.INI` em `0x43d33b`. **Não muda o dano que o jogador recebe**: nenhum uso
toca o jogador. Usos (todos):

| Endereço | Quem | Efeito (fácil / médio / difícil) |
|---|---|---|
| `0x40a36d` | coleta de munição (`armas.md`) | ×1,5 / ×1 / ×0,75 |
| `0x416979` | Morte (3), dano recebido | ×2 / ×1 / ÷2 |
| `0x416696` | Morte, espalhamento do tiro | `max(10, (2 − d)·15/2)` |
| `0x416753` | Morte, espera até o próximo ataque (`+0x264`) | `rand % 100 + (2 − d)·75` tiques |
| `0x41a002` | Camaleão (4), dano recebido | ×2 / ×1 / ÷2 |
| `0x417b31` | Bruiser (7) | ×2 / ×1 / ÷2 |
| `0x41ca37` | Robô de segurança (8) | ×2 / ×1 / ÷2 |
| `0x41dfaf` | Gerador de minas (103) | ×2 / ×1 / ÷2 |
| `0x42231b` | **só a Manta** (89) | ×2 / ×1 / ÷2 |

Ou seja: no fácil esses inimigos morrem com metade dos tiros e a munição rende mais; no
difícil, o dobro de tiros e menos munição. No médio (padrão) nada muda, o que confirma a
decisão da M7a. Nos seis pontos de dano: **dano 0 não faz nada** (salta tudo, inclusive o
tranco do Camaleão e do Bruiser); depois da escala **só um negativo vira 1** (`test ecx,ecx;
jge`) — não há mínimo 1: no difícil um dano de 1 vira 0. O Anticorpo (`0x412189`) **não lê a
dificuldade**: `vida −= dano·[0x472104 + 0x48·v]/100` (100 nas quatro variantes), também só
com dano ≠ 0. O gerador de dardos (`0x420dc5`) e a formiga (`0x42077e`) idem, sem dificuldade.

## Jogo salvo (`*.hhg`) [confirmado]

- **Carregar**: só pelo menu principal (11001 `MM_RESTORE_BUTTON`, `0x440bc6`): diálogo
  padrão do Windows (`GetOpenFileName`, título 10041 "Load Game:", extensão `hhg`). Lê 1 960
  bytes; se não começam pela assinatura `"Hunter hunted saved game"`, caixa 10009 "UNABLE TO
  LOAD. INVALID GAME FORMAT." e nada muda. Senão copia o bloco por cima do estado da
  campanha (`0x47f79c`) e guarda o caminho em `0x47f85c`.
- Depois de carregar (`0x43f769`): modo solo (0x5600); se a missão corrente já tem recorde,
  refaz a lista e avança (`0x440e95`, a próxima sem recorde); carrega a fase `0x47f974`;
  estado 6 (briefing).
- **Gravar** (`0x440ce4`): só na tela de resultado (`SMISS.ITF`), quando a contagem chega a
  100 e a pontuação acumulada `0x47f850` ≠ 0, e só se a missão corrente é da campanha
  (`0x47f848 < 0x47f840`, não fase de usuário):
  - sem nome ainda: caixa 10034 "PLEASE PICK A SAVE GAME FILENAME." (OKAY/CANCEL), depois
    `GetSaveFileName` (título 10042 "Save Game As:"); arquivo existente → 10040 "DO YOU WANT
    TO REPLACE AN EXISTING GAME?" (YES/NO);
  - **com nome**: grava direto no mesmo arquivo, sem perguntar (gravação automática);
  - falha ao abrir → 10044 "UNABLE TO SAVE GAME" com OKAY / 10045 "RETRY?".
- Não há menu de gravar no meio da missão: o progresso só é salvo entre missões.

### O que é salvo

O bloco de 1 960 bytes da memória, tal como está (layout em `../formats/salvamento.md`):
nome do jogador, índice da missão corrente, pontuação acumulada, id na lista de heróis,
**peças do carro**, caminho do arquivo, nome da fase corrente e o **recorde de cada missão**
(`0x47fa8c[300]`, 0 = não vencida).

### Progresso da campanha [confirmado salvo indicação]

- **Não há missões trancadas**: `SLEVL` lista todas as missões do modo; as que têm recorde
  ganham uma marca (rótulo 5 de `Slevl.anm`, `0x4409af`) [inferido pelo desenho].
- "Next" e o carregamento escolhem a próxima missão **sem recorde** (`fim-de-missao.md`).
- A peça do carro só vale na **primeira** vitória de cada missão (`0x445bd6`): número da
  peça = peças + 1; 7 → filme do meio, 15 → final.
- Pontuação acumulada = soma dos recordes (`0x4449da`).

## Lista de heróis (`hunter.fam`) [confirmado]

Arquivo `hunter.fam` na pasta corrente, até **14** linhas (nome, pontuação, id da
campanha), ordenadas por pontuação decrescente.

- `0x445036(p)`: entra se há menos de 14 linhas ou `p` supera a 14ª.
- Nesse caso `SHIGH.ITF` pede o nome (`0x44530c`, campo 21002 `HSM_NAME`, tipo 4) e
  `0x445098(nome, p)` grava: se já há linha com o id desta campanha (`0x47f854`), **atualiza**
  essa linha; senão cria uma (id = contador do arquivo + 1; com 14, substitui a última).
- Nome guardado também em `0x47f7bc` (vai para o jogo salvo).
- Tela `SHERO.ITF` em `menus-restantes.md`.

## Em aberto

- `0x47f844` e `0x47ff40` (no bloco salvo; zerados no New Game).
- O bloco salvo inclui o ponteiro da lista de missões (`0x47ff3c`): carregar põe um ponteiro
  velho ali [inferido: risco do original; o porte não deve gravar isso].
- `HideMouse` e a 2ª leitura de `NumSounds` não lidas por dentro.
- (Resolvido em [`casca-acabamento.md`](casca-acabamento.md): a chave do jogador 2 é `Player2Input`,
  e os padrões de `InputKeys1/2` e `Joystick1/2`.)
