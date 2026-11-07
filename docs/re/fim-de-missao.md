# Fim de missão: anúncio, resultado e próxima missão

Continua [`logica-fase.md`](logica-fase.md) (veredito `0x43cdf0`, resultado `0x497328`:
1 = vitória, 2 = derrota). Controles e despacho em [`interface-itf.md`](interface-itf.md).

## Como a partida termina [confirmado]

- Laço do jogo (`0x40456e`): `0x43cdf0(1) ≠ 0` → `0x401624(101)`.
- Menu "Mission → End Mission" (comando 2100, `0x401858`): caixa genérica com o texto 10007
  (ver `pausa.md`); YES → `0x497328 = 2` e `0x401624(101)`.
- Comando 101 (`0x4017ca`): lê a entrada e chama **`0x444174`**.

## `0x00444174`: depois da partida [confirmado salvo indicação]

1. `0x444a53`: **anúncio do locutor** (fila `0x43c868(som1, som2, …)`). Solo:

   | Situação | Sons |
   |---|---|
   | vitória | 3780, depois 3782 (Jake) ou 3786 (Garathe) |
   | derrota com cronômetro esgotado (`0x4972fc ≤ 0` e campo 1004 > 0) | 3776, depois 3781 (Jake) ou 3785 (Garathe) |
   | outra derrota | 3777, depois 3781 ou 3785 |

   O personagem é `0x497318` (1 = Jake). Os nomes desses sons não estão no `SOUND.RES`
   (vozes do "MC"); o significado acima é [inferido] pelo uso.
2. Espera a fila do locutor esvaziar (`0x43c84a`), bombeando mensagens.
3. `0x445c07`: se uma peça do carro de fuga foi ganha nesta missão (`0x48059c`) e é vitória,
   guarda o progresso (`0x47f858 = 0x480598`) e mostra `S_CAR.ITF` (`0x445ac0`). Retorno 7 →
   filme `midlite/midmed/middark.avi`; 15 → `outlite/outmed/outdark.avi` e os créditos (o
   final do jogo). [inferido: a regra das peças não foi lida]
4. Dois jogadores → estado 16 (`SH2HS.ITF`); solo → **estado 17** (`SMISS.ITF`).

## Tela de resultado solo (`SMISS.ITF`, `0x44432f` + `0x4445e1`) [confirmado]

Tela cheia (`smiss.ANM`), fonte `slevd.pft`, **cor 231** para todo o texto, cada valor
**centrado** no retângulo do seu controle.

Ao entrar (vindo do jogo) guarda uma cópia das estatísticas e do jogo salvo (para restaurar
se o jogador for ao briefing e cancelar: estado anterior 6 ou 20) e calcula o **bônus**
(`0x444bf8`, `0x480480`), que só existe na vitória:

```
bônus = segundos restantes (0x4972fc / 20) + vida (+0x258) + armadura (+0x25c) + 500
        + munição: Jake = +0x3c + +0x44 + +0x4c + +0x54 + +0x5c (pistola, espingarda,
          foguetes, estrelas, granadas); Garathe = 25·(+0x38) + +0x44 + +0x4c + +0x54 + +0x5c
```

(do objeto do jogador 1, `0x444bd6(1)`).

### Valores mostrados (`0x4448ed`)

Estatísticas (zeradas no início da partida): achados `0x497344` segredos, `0x497348`
inimigos mortos, `0x497354` tesouros; totais da fase `0x49732c` segredos, `0x497330`
inimigos, `0x49733c` tesouros.

### Contagem de mortos (`0x43cfcf`) [confirmado]

No fim da partida, `0x43cfcf` zera os contadores e percorre as listas de objetos ativos e
mortos (`0x488e7c`, `0x488e80`…). Para os tipos **3, 4, 7, 8, 10, 11, 89, 102, 103, 104,
105 e 109** (`0x43d0f2`): total `0x497330 += 1`; se o crédito `+0x1b0 = 0` e o modo é solo
(`0x497300 = 1`), `+0x1b0 = 1`; com crédito e vida `+0x258 ≤ 0`, mortos do jogador `+= 1`.
No solo, portanto, **toda morte conta**, explosões, contato e cadeia incluídos. Objetos com
`0x10` são liberados ao serem destruídos (`0x435b63` → `0x433e2c`) e saem das listas: as
minas dos geradores, os clones do Camaleão e as formigas dos formigueiros (que ligam `0x10`)
destruídos não entram nem nos mortos nem no total; **vivos no fim, entram no total** (e o
percentual de mortos cai). Tipos
1–2 (jogadores) vão a `0x43d140`; 9 e 80–85 alimentam outros contadores (`0x49733c`).

| Controle | Retângulo | Valor |
|---|---|---|
| 25021 `SMM_MISSION_TITLE` | 186, 56, 405×22 | título da missão |
| 25005 / 25006 / 25007 `SMM_NUM_*` | x 341/341/340, y 130/154/180 | mortos / tesouros / segredos |
| 25008 / 25009 / 25010 `SMM_SCORE_*` | x 421 | mortos × **30** / tesouros × **65** / segredos × **215** |
| 25011 / 25012 / 25013 `SMM_PERCENT_*` | x 506 | 100 × achados / total (total 0 → 100) |
| 25019 `SMM_BONUS` | 381, 231, 176×29 | bônus (só na vitória) |
| 25014 `SMM_SCORE` | 381, 255, 176×26 | pontuação = 30·mortos + 65·tesouros + 215·segredos + bônus |
| 25015 `SMM_CUMUL_SCORE` | 381, 278, 177×30 | soma dos recordes de todas as missões (só na vitória) |
| 25020 `SMM_IMPROVE_TEXT` | 188, 309, 403×32 | "MISSION SCORE IMPROVED BY %d POINTS." (10043) |

Os números são escritos com `itoa` (sem `%` nem separador; os rótulos e o `%` estão na arte de
fundo [inferido]).

**Contagem**: todos os números aparecem multiplicados por `p / 100`, com `p` (`0x48047c`)
começando em 0 e subindo **5 por quadro** (rotina do controle de sucesso/falha, `0x4443ff`);
qualquer tecla salta para 100. Enquanto `p ≤ 100`, cada quadro redesenha os números e toca o
som 3093 (o mesmo de "Secret discovered"). São ~21 quadros.

**Imagem de resultado**: vitória → remove 25017 `SMM_FAILURE` e mostra 25016 `SMM_SUCCESS`
(188, 81, 402×219, rótulo 7); derrota → o contrário (25017, rótulo 8). O quadro fica parado
(a rotina própria substitui o avanço de quadro). 25018 `SMM_PISTONS` anima em laço.

**Recorde da missão** (só vitória, na abertura): se a pontuação supera `0x47fa8c[índice]`,
grava a nova e, se havia uma anterior ≠ 0, escreve em 25020 a diferença.

**Quando a contagem chega a 100** (uma vez, `0x480484`): se a soma dos recordes entra na lista
de heróis (`0x445036`), abre `SHIGH.ITF` para o nome e grava `hunter.fam`; depois, com soma
≠ 0, oferece gravar o jogo (`0x440ce4`; sem nome de jogo salvo, antes pergunta numa caixa
genérica com o texto 10050, que **não existe** no `SYSTEM.RES` 1.00, de 45 linhas).

### Botões

Cursor inicial: **NEXT** na vitória, **REPLAY** na derrota. Botões com sinal 0x80.

| Controle | Retângulo | Efeito |
|---|---|---|
| 25001 `SMM_REPLAY` | 34, 397, 189×33 | recarrega a mesma fase (`0x43124b(0x47f974)`), estado 6 (briefing) |
| 25002 `SMM_NEXT` | 226, 398, 188×33 | `0x440e95` (próxima missão), carrega, estado 6 — também na derrota |
| 25004 `SMM_MAIN` | 426, 421, 190×33 | estado 2 (menu principal) |
| Esc (id 2) | — | estado 2, **exceto** se o estado anterior é 7 (o caso normal ao sair do jogo): ignorado |

### Próxima missão (`0x440e95`) [confirmado]

Índice `0x47f848 + 1`, voltando a 0 no fim da lista. Se ainda há missões sem recorde, pula as
que já têm (`0x47fa8c[i] ≠ 0`) até achar uma sem. Depois `0x440f25` (nome, título,
descrição). Da `T_J_001` vencida, a próxima é a segunda entrada solo do `ENGLEVEL.TXT`.

## Correção de `logica-fase.md`

Os números 30032 (`0x443d31`) e 25002 (`0x4444a6`) não são recursos de texto: são **ids de
controles**. 30032 é `MH2H_FAILURE` do `SH2HS.ITF` (removido na vitória de dois jogadores;
na derrota remove-se 30031 `MH2H_SUCCESS`); 25002 é `SMM_NEXT`, para onde o cursor vai na
vitória (na derrota, 25001 `SMM_REPLAY`). E o diálogo 10007 é "parar a missão e escolher
outra", não "sair" (ver `pausa.md`).

## Em aberto

- Regras das peças do carro (`0x48059c`, `0x480598`, `0x47f858`) e a tela `S_CAR.ITF`.
- Como os contadores de tesouros e segredos são incrementados (os mortos estão acima).
- Texto pretendido para o id 10050 e o que a caixa genérica faz com texto nulo.
- `0x445036`/`0x44530c`/`0x445098` (recordes, `hunter.fam`) e a gravação `0x440ce4`.
