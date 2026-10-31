# Abertura, máquina de telas e menu principal (solo)

Escopo: o caminho do executável até a primeira missão solo. A mecânica das telas `.ITF`
(controles, botões, despacho) está em [`interface-itf.md`](interface-itf.md); o briefing em
[`briefing.md`](briefing.md); o fim de missão em [`fim-de-missao.md`](fim-de-missao.md).

## Ordem da inicialização (`0x0040107c`, a partir de `0x0040151d`) [confirmado]

1. Textos: `system.res` em 10000, `graffiti.res` em 17000, `wizard.res` em 18000
   (`0x45e56f(base, arquivo, 0)`; a linha `n` do arquivo é o id `base + n`, contando de 1).
2. Configuração (`0x403ad7`, `Sierra.ini`) e `0x43d33b` (aplica as opções).
3. **Abertura** `0x4399e0(1)` (abaixo).
4. Tela de carga `SLOADING.BMP` (`0x402efd`).
5. Fonte `garam12.pft`, paleta `world.pal`, `DLS.TBL`; pré-carga das listas `ANIM.RES`,
   `BORDERS.RES`, `SOUND.RES` (`0x4028d2`, base 45000); `SOUND.TBL`; `DAMAGE.TBL`.
6. `0x4399e0(0)`: se a abertura do passo 3 foi vista **inteira**, repete-a em laço até o
   jogador apertar algo (modo de demonstração); se foi interrompida, segue direto.
7. `0x401624(100)` → `0x43f214(2)`: máquina de telas a partir do **menu principal**.
8. Ao sair das telas, o laço do jogo `0x40433c` roda até a janela fechar.

### Abertura (`0x4399e0(modo)`) [confirmado]

Cada etapa devolve 0 se o jogador interrompeu; a primeira interrupção encerra a sequência.
"Interromper" (`0x43a123`) = tecla (`0x499f34`), botão do mouse (`0x499f3c`/`0x499f40`) ou
botão de joystick.

| Etapa | Função | Recursos |
|---|---|---|
| Logotipo | `0x439bdd(0)` | `SIERRA.ANM`, rótulo 1 "background" (`Sierra.BMP`), paleta `sierra.pal`; rótulo 3 "tp" animado por cima, com o som 3403; acaba 500 ms depois do fim do "tp" (no máximo 30 000 ms) |
| Filme | `0x43a6f1(avi)` | `Movies\hhdark.avi`, `hhnormal.avi` ou `hhlight.avi` conforme `Brightness` do `Sierra.ini` (1402 → escuro; 1403–1404 → normal; 1405–1406 → claro), no caminho `CDPath` |
| Título | `0x439bdd(1)` | rótulo 4 "Title Screen" (`stitle.bmp`), paleta `stitle.pal`, 1 500 ms; sons 3081, 3045, 3220, 3086 |
| História | `0x43a18f(1)` | `SIERRA.ANM` rótulos 2 "story" (`sstory.BMP`) e 6 "story animation"; texto `STORY.RES` (base 13000) na fonte `credits.pft` |
| Créditos | `0x43a18f(0)` | `SCREDIT.ANM` rótulos 1 e 2; texto `CREDITS.RES` (base 13000); `credits.pft` |

A tabela de `0x439bdd` fica em `0x47d6cc` (24 bytes por entrada: rótulo, nome da paleta,
duração em ms). As imagens são centradas numa tela de 640×480. O modo 2 (`0x4399e0(2)`) mostra
só os créditos. [confirmado]

## Máquina de telas (`0x0043f214(estado)`) [confirmado]

Cada tela devolve o próximo estado; o anterior fica em `0x47f53c`. Laço até um estado de
saída.

| Estado | Tela | Função / ITF |
|---|---|---|
| 0, 7 | sai das telas (7 = "jogar": volta ao laço do jogo) | — |
| 1 | vai para 2 | — |
| 2 | **menu principal** | `0x43f448`, `SMAIN.ITF` |
| 3 | dois jogadores | `0x43f0a8`, `SMULT.ITF` |
| 4 | opções | `0x43dfdc`, `SOPTI.ITF` |
| 5 | escolher missão | `0x43fa7c`, `SLEVL.ITF` |
| 6 | **briefing** | `0x4414e0`, `SLEVD.ITF` (solo/coop) ou `SHVLD.ITF` |
| 8, 15 | escolher personagem | `0x4430a0`, `SEGOC.ITF` |
| 9, 12 / 10, 11, 13, 14 | teclado / joystick | `0x4419e0` `SKEYB.ITF` / `0x442638` `SJOYS.ITF` |
| 16 | resultado de dois jogadores | `0x443c30`, `SH2HS.ITF` |
| 17 | **resultado solo** | `0x44432f`, `SMISS.ITF` |
| 18 | heróis (recordes) | `0x444c84`, `SHERO.ITF` |
| 19 | dispositivos | `0x43e79c`, `SINPU.ITF` |
| 20 | disputa (dois jogadores) | `0x4454e8`, `SHVLD.ITF` |
| 12345 | inatividade no menu: abertura `0x4399e0(1)` e `(0)`, depois 2 | — |
| 12346 | créditos `0x4399e0(2)`, depois 2 | — |

Caso especial do estado 2: se o menu devolve 6, a máquina chama `0x440f25` (missão
corrente → nome em `0x47f974`), **carrega a fase** `0x43124b(0x47f974)`, desenha um quadro
(`0x4365ee(0)`) e segue para o estado 6 (briefing por cima da fase desenhada).

## Menu principal (`SMAIN.ITF`, `0x43f448` + procedimento `0x43f4c3`) [confirmado]

Entrada: `0x497300 = 1` (um jogador), `0x497304 = 0` (sem cooperação). Fundo: controle
11000 `MM_BACKGROUND` (rótulo 20 de `Smain.anm`, 640×480).

Na abertura da tela: as animações 11013 `MM_FIRE_ANIMATION`, 11011 `MM_SPARKS` e 11012
`MM_BLINKER` recebem o avanço de quadro em laço (`0x43f46c`); os botões 11001–11009 recebem os
sinalizadores 0x81 (ver `interface-itf.md`); o cursor do mouse vai para o centro do último
botão escolhido (`0x47f540`, padrão 11002 "New Game").

| Controle | Ação |
|---|---|
| 11002 `MM_NEW_GAME_BUTTON` | `0x440fe0` (zera a campanha), solo, `0x4972fc = 0`, estado **6** |
| 11001 `MM_RESTORE_BUTTON` | diálogo de carga `0x440bc6`; se ok, modo 0x5600, carrega a fase salva, estado 6 |
| 11003 `MM_CHOSE_LEVEL` | estado 5 |
| 11009 `MM_MULTI_PLAYER` | estado 3 |
| 11004 `MM_OPTIONS` | estado 4 |
| 11005 `MM_INPUT_DEVICES` | estado 19 |
| 11007 `MM_HERO_LIST` | estado 18 |
| 11006 `MM_HELP` | comando 2600 (ajuda) |
| 11008 `MM_EXIT_GAME` | `0x43f826`: diálogo 10005 "ARE YOU SURE YOU WANT TO QUIT HUNTER HUNTED?" (YES 10003 / NO 10004); YES → espera 1 500 ms e encerra |
| 11013 (clique no fogo com o bit 4 de `0x499f38`) | estado 12346 (créditos) |
| Esc | id 2: ignorado no menu |

Sem mexer o mouse por **30 000 ms** (`0x4974a4`), o menu devolve 12345 (demonstração).

## Lista de missões e "New Game" [confirmado]

- `0x440fe0`: modo `0x4974b4 = 0x5600` (solo), índice corrente `0x47f848 = 0`, zera contadores,
  o nome da fase `0x47f974`, os recordes por missão `0x47fa8c[300]` e o nome do jogo salvo.
- `0x4406d3` monta a lista (`0x47ff3c`, 344 bytes por entrada: `+0` nome
  `"HUNTER.HLS|T_J_001.lev"`, `+0x118` título, `+0x148`/`+0x14c` os dois números do `p=`,
  `+0x150` ponteiro da descrição). Fonte: `0x4403e6` lê o `ENGLEVEL.TXT` de dentro do
  `HUNTER.HLS` como lista de textos na base 5000 — a missão `i` ocupa as linhas `5i+1`
  (arquivo), `5i+2` (`p=`), `5i+3` (`t=`), `5i+4` (`d=`). Depois acrescenta outros `*.hls` e
  `*.lev` soltos da pasta (fases de usuário).
- **Filtro por modo**: 0x5600 (solo) pula as missões com `p=3` ou `p=4`; 0x5601 só `p=3`
  (disputa); 0x5602 só `p=4` (cooperativo). No solo a lista é a do `ENGLEVEL.TXT` sem as de dois
  jogadores, **na mesma ordem**: Jake e Garathe misturados. O total vai para `0x47f840`.
- `0x440f25` copia o título para `0x488a24`, a descrição para `0x488a54` e o nome para
  `0x47f974`. Com índice 0, a primeira missão é `T_J_001.lev`.

## Carregar a fase e começar a partida [confirmado]

`0x43124b(nome)` mostra `SLOADING.BMP`, lê o `LEV:` (nome em `0x48890c`, título `0x488a24`,
descrição `0x488a54`, notas `0x488c54`, campo 1004 em `0x488e54`) e o corpo, e no fim chama
`0x43c95c` (início da partida):

- modo `0x4972f8`: com um jogador sempre **2**; com dois, tabela por personagens e cooperação
  (0x8…0x80);
- zera as estatísticas `0x49732c…0x497373` (`fim-de-missao.md`);
- cronômetro `0x4972fc = campo1004 × 20` (tiques; decrementado a cada tique em `0x43ca8e`).

`0x497300` (jogadores) e `0x497304` (cooperação) já vêm do menu: 1 e 0 no solo.

## Escolher missão (`SLEVL.ITF`, `0x43fa7c` + `0x43fc68`) [inferido em parte]

Os botões 22016 `CLM_1PLAYER`, 22017 `CLM_2PL_HEAD2HEAD` e 22018 `CLM_2PL_COOP` trocam o modo
(`0x4974b4` = 0x5600, 0x5601, 0x5602: os próprios ids) e refazem a lista; a descrição da
missão marcada aparece nas sete linhas 22019–22025 (`slevd.pft`); 22007/22008/22009 rolam;
22005 `CLM_LOAD` escolhe (estado 6, ou 15 para escolher personagens nos modos de dois);
22006 `CLM_CANCEL`/Esc volta.

## Em aberto

- Regras de missões liberadas no `SLEVL` (`0x44143d`, `0x441402`, comparações com
  `0x47f848`).
- O que `0x4365ee(0)` desenha exatamente antes do briefing (tudo indica o primeiro quadro da
  fase).
- Formato do diálogo de carga/gravação (`0x440bc6`, `0x440ce4`) e do `hunter.fam` (recordes).
- Bit 4 de `0x499f38` (tecla modificadora?) no atalho dos créditos.
