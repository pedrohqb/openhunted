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

### Corpo

Leitores originais: `0x00434170` (objetos), `0x00432319` (células), `0x0042ff4c` (lista PR),
`0x00430278` (lista DC). Inteiros little-endian; "texto" = ASCIIZ.

**Lista de objetos** (global, e de novo dentro de cada célula): `s16` versão (a marca `"OB"`
seguida de 6 bytes vale versão 1), `s16` contagem `N`; com `N > 0` a versão tem de estar entre
1003 e 1013. Cada objeto:

| Campo | Condição |
|---|---|
| `u32` tipo, `i32` x, `i32` y | |
| 8 × `i32` (campos `15c`, `160`, `174`, `178`, `17c`, `180`, `184`, `18c` do objeto) | |
| `i32` (`1ec`): rótulo inicial da animação, válido se o bit 0x80 de `178` estiver ligado | versão ≥ 1007 |
| 7 × `i32` (`194`, `198`, `19c`, `1b8`, `254`, `258`, `25c`) | |
| `i32` descartado | versão < 1012 |
| 2 × `i32` (`1ac`, `1a8`): `1a8` = máscara de modos de jogo (padrão 0xfe; bit 0 = vaga de jogador, ligado nas versões < 1011 se `19c` ≠ 0) | versão ≥ 1010 |
| `s16` n + n × (`i32`, `i32`) pontos | versão ≥ 1006 |
| `s16` n (16 nas versões < 1004) + n × `i32` valores | |
| `i32` (`118`), `s16` n + n × (`i32` [+ `i32` se versão ≥ 1005]) gatilhos | versão ≥ 1004 |
| 2 textos | |
| 2 × `i32` | |

**Células**: `x × y × z` em sequência (x varia mais rápido). Cada uma começa por `s16` versão;
**versão 0 = célula vazia** (só esses 2 bytes). A marca `"CE"` seguida de 6 bytes vale versão 1.
Versões 1005 a 1010 (as anteriores guardam o cenário num recurso à parte, com bloco `CTG:`; não
aparecem nos dados):

| Campo | Condição |
|---|---|
| texto: animação do cenário (ex.: `WORLD1.ANM`) | |
| texto: colisão (ex.: `C1_07.BRD`) | |
| `i32` índice do cenário | |
| `s16` | versão ≥ 1006 |
| `s16`, `s16`, ponto (2 × `i32`), `s16`, retângulo (4 × `i32`), `s16`, `s16`, `s16` | |
| `s16` + `i32` (versão ≥ 1006) ou `i32` + `s16` | |
| `s16` | |
| `s16` n + n × `s16` | |
| `s16` n + n × (`i32` + `i32`) gatilhos | |
| lista de objetos, lista PR, lista DC | |

**Lista PR**: `s16` versão ≤ 1001 (marca `"PR"`), `s16` n, n × (`s16` tipo, 4 × `i32` retângulo).

**Lista DC** (decorações): `s16` versão ≤ 1004 (marca `"DC"`), `s16` n; cada item: texto
(animação), 5 × `s16`, `i32` x, `i32` y, `s16`, (versão > 1001) 2 × `s16`, (versão ≥ 1004) `s16`.

As 100 fases da versão 1.00 são lidas por inteiro, sem sobra. Exemplo: `T_J_001` tem 13 objetos
globais (7 do tipo 102, as minas flutuantes do tutorial) e 14 células, todas com `WORLD1.ANM`.
