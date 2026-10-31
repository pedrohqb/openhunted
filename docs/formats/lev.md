# Fases (`.LEV`) e lista de missões (`ENGLEVEL.TXT`)

Ficam dentro do [`hunter.hls`](hls.md). Leitor de fase: `0x0043124b`.

## Lista de missões (`ENGLEVEL.TXT`)

Blocos separados por linha em branco, em Latin-1 com CRLF:

```
T_J_001.lev
p=1,36
t=TUTORIAL: Jake Gets a Gun
d=Get to the exit.  Use the pistol to shoot the hovering mines.  Good luck Jake!
```

- linha 1: arquivo da fase;
- `p=`: dois números. O primeiro é o personagem (1 = Jake, 2 = Garathe, 3 e 4 = modos
  para dois jogadores); o segundo tem significado ainda desconhecido (35 a 38). Os dois
  coincidem com os dois últimos campos do cabeçalho da fase;
- `t=`: título; `d=`: descrição, com `\n` literal para quebra de linha.

São 100 missões na versão 1.00, na ordem em que o jogo as apresenta: tutoriais (`T_`),
introduções (`I_`), caçadas (`H_`), fases `D_`, e as de dois jogadores (`C_` cooperativo,
`M_` disputa). A **primeira missão do Jake** é `T_J_001.lev`, "TUTORIAL: Jake Gets a Gun"
(grade 14×1×1); a primeira fora dos tutoriais é `I_J_101.LEV`, "INTRO: Jake and Heavy Metal".

`LEVELS.RES` (no arquivo de recursos) é uma lista mais antiga: 82 nomes, 5 dos quais não
existem no `hunter.hls` (`T_J_002`, `T_J_004`, `T_J_008`, `T_J_010`, `T_M_009`).

## Fase (`.LEV`)

Um bloco `LEV:` com o cabeçalho, seguido do corpo binário (fora de qualquer bloco).

### Cabeçalho (`LEV:`)

| Campo | Condição | Conteúdo |
|---|---|---|
| nome | | ASCIIZ (ex.: `T_J_001.LEV`) |
| paleta | | ASCIIZ (`WORLD.PAL`) |
| versão | | `s16`; o original recusa > 1006; todas as fases são 1006 |
| título, descrição | versão ≥ 1002 | dois ASCIIZ (descartados se o nome contém `|`) |
| notas | versão ≥ 1003 | ASCIIZ (notas do autor, ex.: "RichT 9/19") |
| campo 1004 | versão ≥ 1004 | `s16` (desconhecido) |
| grade x | versão ≥ 1000 | `s16` (nas anteriores, o próprio número da versão) |
| grade y, grade z | | `s16`, `s16` |
| jogador, extra | versão ≥ 1006 | `s16`, `s16` (os dois números do `p=`) |

### Corpo (a decifrar na M2)

O carregador lê, em seguida:

1. uma lista de objetos (`0x00434170`): marca opcional `"OB"` (`0x424f`), versão até 1013
   e registros com muitos campos dependentes da versão;
2. `x × y × z` células (`0x00432319`), cada uma com marca `"CE"` (`0x4543`), versão até
   1010, e referências a blocos `CTG:` em outros recursos.

O OpenHunted guarda esse corpo cru em `Level::body` até a M2.
