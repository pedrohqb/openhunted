# Telas `.ITF` restantes e fluxo a partir do menu principal

Mecânica comum em [`interface-itf.md`](interface-itf.md); máquina de telas e `SMAIN` em
[`menu.md`](menu.md). Opções e gravação em [`opcoes-salvamento.md`](opcoes-salvamento.md).

## As 16 interfaces do CD 1.00 [confirmado pela leitura dos `.ITF`]

| ITF | Ids | Função | Tela | Documento |
|---|---|---|---|---|
| `SMAIN` | 11000–11013 | menu principal | `0x43f448` | `menu.md` |
| `SLEVL` | 22000–22026 | escolher missão / modo | `0x43fa7c` | `menu.md`, abaixo |
| `SLEVD` | 21500–21507 | briefing | `0x4414e0` | `briefing.md` |
| `SMISS` | 25001–25021 | resultado solo | `0x44432f` | `fim-de-missao.md` |
| `SOPTI` | 12000–12028 | opções | `0x43dfdc` | `opcoes-salvamento.md` |
| `SMULT` | 13000–13006 | dois jogadores: disputa ou cooperação | `0x43f0a8` | abaixo |
| `SINPU` | 29000–29014 | dispositivos de entrada | `0x43e79c` | abaixo |
| `SKEYB` | 17000–17026 | teclas (usa `Skeyb.anm`) | `0x4419e0` | abaixo |
| `SJOYS` | 17000–17028 | botões do joystick (mesma `Skeyb.anm`) | `0x442638` | abaixo |
| `SEGOC` | 15000–15017 | escolher personagem e cor (dois jogadores) | `0x4430a0` | abaixo |
| `SHVLD` | 32000–32011 | ajustes da disputa (tempo, mortes) | `0x4454e8` | abaixo |
| `SH2HS` | 30000–30033 | resultado de dois jogadores | `0x443c30` | abaixo |
| `SHERO` | 13000, 31001–31005 | lista de heróis | `0x444c84` | abaixo |
| `SHIGH` | 21000–21002 | nome do herói (diálogo) | `0x44530c` | abaixo |
| `S_CAR` | 38000–38007 | carro de fuga (peças) | `0x445ac0` | abaixo |
| `SABOUT` | 33000–33002 | "Sobre" (diálogo) | `0x445c44` | abaixo |

Não existem telas de senha, dificuldade separada, créditos ou carga/gravação próprias:
créditos são a sequência da abertura (`0x4399e0(2)`), dificuldade fica em `SOPTI`, e
carregar/gravar usa o diálogo de arquivo do Windows. `SHERO` usa o id de fundo 13000, o
mesmo de `SMULT` (não colidem: uma tela por vez).

## Fluxo [confirmado salvo indicação]

```
SMAIN ─ New Game ──────────────────────────────→ briefing (6) → jogo (7)
      ├ Restore → diálogo .hhg ─────────────────→ briefing (6)
      ├ Choose Level → SLEVL ─ solo ────────────→ briefing (6)
      │                       └ 2 jogadores ────→ SEGOC (15) → briefing / SHVLD
      ├ Multi Player → SMULT → SLEVL (5)
      ├ Options → SOPTI → SMAIN
      ├ Input Devices → SINPU ─ Customize → SKEYB / SJOYS → SINPU
      ├ Hero List → SHERO → SMAIN
      ├ Help → arquivo de ajuda (comando 2600)
      └ Exit → caixa 10005
jogo → [S_CAR] → [filme] → SMISS (solo) / SH2HS (dois) → briefing ou SMAIN
SMISS → [SHIGH] → [gravar .hhg]
```

## `SMULT` (`0x43f0bb`) [confirmado]

13001 `MMU_HEAD2HEAD` → jogadores 2, cooperação 0; 13002 `MMU_COOPERATIVE` → jogadores 2,
cooperação 1; ambos `0x4974b8 = 3` e estado **5** (`SLEVL`). 13004 `MMU_CANCEL`/Esc → 2.
13005 (fio) e 13006 (fogo) animam em laço com rótulos trocados (3 e 4).

## `SLEVL` (complemento de `menu.md`) [confirmado salvo indicação]

- Ao marcar uma missão (`0x44143d`): o 2º número do `p=` vai para `0x47f5bc` e é a tabela de
  cor (`DLS.TBL`) com que se desenha a animação 22010 `CLM_ANIMATION`; o 1º escolhe a lista
  de rótulos: 1 → Jake (`0x47f714`), outro → Garathe (`0x47f754`). Ao fim de cada ciclo a
  animação sorteia outro rótulo da lista (`0x43fad0`) [inferido: poses].
- Missões vencidas ganham marca (rótulo 5); nenhuma fica trancada (`opcoes-salvamento.md`).
- Fases de usuário (`*.hls`/`*.lev` soltos) entram depois das do `ENGLEVEL.TXT`; o `p=` delas
  vem do cabeçalho da fase (`0x4408f3`).

## `SINPU` (`0x43ea27`) [inferido em parte]

Dois grupos de opção, jogador 1 (29001–29005) e jogador 2 (29006–29010): teclado, joystick
1/2, gamepad 1/2. Lê e grava o dispositivo no `HUNTER.INI` (`Player1Input`…, valores
`Keyboard1/2`, `Joystick1–4`); joysticks ausentes ficam cinza (consulta ao `joyGetNumDevs`/
`joyGetPos`, `0x4706e5`/`0x4706df`). 29013/29014 `CUSTOMIZE` → `SKEYB` ou `SJOYS` conforme o
dispositivo (estados 9–14). 29011 `MID_DONE` → 2.

## `SKEYB` / `SJOYS` (`0x4419e0` / `0x442638`) [inferido em parte]

Mesma arte (`Skeyb.anm`). Cada botão (17001–17010: cima, baixo, esquerda, direita, atirar,
pular, furtivo, arma anterior, soco, próxima arma) espera uma tecla; o nome aparece na área
`*_STR` ao lado (texto do `KEYS.RES`, linha = código VK; no joystick, `JOYSTICK.RES` "Button
n"/`GAMEPAD.RES`). A ajuda vai em 17021 `CKM_DESCRIPTION_STR`: linhas do `SHELL.RES` (teclado),
`JOYSTICK.RES` ou `GAMEPAD.RES`, fonte `slevd.pft`. 17023 mostra o personagem; 17025
`CKM_RESET` volta ao padrão; 17024 `CKM_DONE` grava `InputKeys1/2` (10 números) ou
`Joystick1/2` (6 números) e volta. Em `SJOYS` as direções vêm do eixo, por isso 17028
`CKM_GREYOUT` cobre os quatro botões de direção. "Your joystick does not have enough
buttons." quando o joystick tem poucos botões.

## `SEGOC` (`0x4430a0`) [inferido em parte]

Para cada jogador, um de cada vez: personagem (15016 `ECM_JAKE_PICK` / 15017
`ECM_GARTHE_PICK`, com os retratos 15010/15011) e cor (15012 marrom, 15013 vermelho, 15014
preto, 15015 branco) → `0x497318`/`0x49731c` (personagem; padrão 1 e 2) e
`0x497320`/`0x497324` (tabela de cor, padrão 35 e 36). 15009 `ECM_PLAYER_INPUT` leva aos
dispositivos. 15003 OK → próximo jogador; depois do 2º, briefing (`SLEVD`) ou `SHVLD` na
disputa; 15004 CANCEL volta.

## `SHVLD` (`process_deathmatch_interface`, `0x445501`) [inferido em parte]

Diálogo da disputa: nome da fase em 32011; contadores de **tempo** (32007, setas 32003/32004)
e de **mortes** (32008, setas 32005/32006) em `0x480564` e `0x480560`; o tempo anda de 20
em 20; ambos com teto 1 980; 32009 GO / 32010 CANCEL. Fora do escopo do solo.

## `SH2HS` (`0x443c30`) [inferido em parte]

Resultado de dois jogadores, igual em espírito ao `SMISS`: por jogador, número/pontos/% de
mortos, tesouros e segredos (`MH2H_N*`, `S*`, `P*`), mortes dadas e sofridas
(`KILLS`/`KILLO`) e total; 30031/30032 sucesso/falha (`fim-de-missao.md`); 30001 REPLAY,
30030 NEXT, 30003 MAIN MENU.

## `SHERO` (`0x444c97`/`0x444ced`) [confirmado]

Tela cheia, fonte `slevd.pft`, **cor 209**. 31002 `MHL_HEROES` (179, 53, 421×296) mostra
`hunter.fam` (`opcoes-salvamento.md`): uma linha nova **por quadro** (`0x480508` até 14), cada
uma com o som 3093; nome à esquerda e pontuação à direita. 31005 pistão em laço. 31001
`MHL_OK` ou Esc → 2.

## `SHIGH` (`0x44530c`) [confirmado em parte]

Diálogo centrado (396×227): campo de edição 21002 `HSM_NAME` (tipo 4) e 21001 `SHM_DONE`.
Devolve o nome (máx. 128 bytes, `0x47f7bc`).

## `S_CAR` (`0x445ac0` + `0x445b08`) [confirmado]

Tela cheia mostrada **depois de toda vitória que dá peça** do carro. 38002 `MC_CAR_ANIM`
recebe o rótulo **peças + 3** ("part 01"…"part 15" do `s_car.ANM`, rótulos 4–18) e toca
**uma vez**, parando no último quadro (`0x445ad3`); 38003–38007 monitores em laço; cursor em
38001 `MC_DONE`, que fecha (estado 0). Depois vêm o filme (7ª/15ª peça) e o resultado.

## `SABOUT` (`0x445c44` + `0x445c62`) [confirmado em parte]

Diálogo 320×256 aberto por um comando do menu da janela (`0x4017f9`, Help → About
[inferido]). 33002 `AB_VER` recebe "1.0.0.0" em `slevd.pft`, cor 255; 33001 `AB_DONE` fecha.

## `ENGLEVEL.TXT` e cabeçalho da fase (complemento de `../formats/lev.md`)

- `p=a,b`: `a` = personagem/modo (1 Jake, 2 Garathe, 3 disputa, 4 cooperativo; filtra a
  lista). `b` (35–38) = **tabela de cor** do `DLS.TBL` usada no retrato da `SLEVL`
  [confirmado o uso; inferido que são as quatro cores da `SEGOC`, 35 = padrão do jogador 1].
- Os dois números no cabeçalho `LEV:` (versão ≥ 1006) são **lidos e descartados** pelo leitor
  da fase (`0x4313ff`); só a lista os usa.
- Campo 1004 do cabeçalho = **tempo limite em segundos** (0 = sem limite; `menu.md`).

## Em aberto

- Máquina exata de `SINPU`/`SKEYB`/`SJOYS`/`SEGOC` (estados 8–15) e o comando do "About".
- Tabelas de rótulos do retrato (`0x47f714`, `0x47f754`).
- Regras completas da disputa (`SHVLD`, `SH2HS`).
