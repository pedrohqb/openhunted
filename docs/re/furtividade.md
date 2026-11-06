# Jake: furtivo, esconder-se, especial (BAIXO+ATIRAR) e soco

Entrada e mira em [`tiro-jake.md`](tiro-jake.md); estados em
[`jake-movimentos.md`](jake-movimentos.md); velocidades em [`ske.md`](ske.md).

## FURTIVO (bit 0x400, tecla X) = andar [confirmado]

O bit 0x400 também se liga quando o eixo do joystick passa de ±50 mas não de ±100
(`0x00407e2d`): é o modificador de **andar**. No Jake ele só é lido onde se escolhe entre
correr e andar (`0x004239d4` em pé, `0x00423d18` nos estados 3/7/8, `0x00423e53` no correr,
`0x0042401f` no andar; os do Garathe ficam em `0x00428b81…`). Fora da leitura do
teclado, só `0x0042f434` o liga (`0x0042f5bd`, `0x0042f68b`) [inferido: controle automático
do personagem, com sorteios]:

- ESQ/DIR + 0x400 → **estado 2** (`0x004261d0`; rótulo **149**, a pose de luta `JKH2H1`,
  9 quadros, atraso 0). Velocidade pelo esqueleto com k = 3, só horizontal (a vertical é
  preservada): 0, 3, 9, 15, 9, 9, 9, 9, 12 px/tique, ≈ 9,4 px/tique em média contra 12 do
  correr. O 2 é um estado de animação espelhada (`ske.md`).
- No 2 (`0x00423f85`): quatro direções → estado 4; parede à frente (`0x004070d1`) →
  empurrar; ATIRAR/SOCO → zera a velocidade e atira; soltar 0x400 → correr (com lado) ou
  parado; sem lado, CIMA/BAIXO tratam as zonas PR como em pé (portas 1/2, corda 4) e BAIXO
  agacha; PULAR (segurado) com CIMA → longo (28) ou para cima (16), sem CIMA → escolha por
  sonda (`0x00425144`).

Nenhum sinalizador é ligado: `+0x178` não muda no estado 2 (os únicos bits de `+0x178`
mexidos pelos manipuladores do Jake são 0x8, gravidade, e 0x4000, colisão entre
personagens). Andar não esconde o Jake dos inimigos [inferido: nenhum teste do estado 2 fora
do Jake foi achado].

## Esconder-se (CIMA sem zona; estados 164, 165, 166) [confirmado]

- **164** (`JKHIDE`, `0x00425e82`, 7 quadros): de pé, CIMA sem zona PR que sirva. Põe
  `+0x1b4 = 0x28` [inferido: ordem de desenho, atrás do cenário] e anima **dois quadros por
  tique** até o fim → 165. Interrompem: ATIRAR/SOCO (atira), PULAR (com lado → pulo
  lançado; com CIMA → 16; senão 27), ESQ/DIR (correr), BAIXO → 166.
- **165** (escondido, `0x00426020`, 1 quadro): liga 0x8 (sem gravidade), `+0x1b4 = 0x28`,
  velocidade zero. Com CIMA apertado fica; soltar CIMA → 166. ATIRAR/SOCO, PULAR e ESQ/DIR
  saem como no 164, devolvendo `+0x1b4 = 0x78` e a gravidade.
- **166** (`0x00425f8a`): o 164 ao contrário (`0x00423894`) → parado; CIMA de novo → 164.

É o "Press UP to hide against a wall" da dica 5 do tutorial (`missoes-tutorial.md`).

**Como os inimigos testam** (`0x0041db0f`, busca de alvo, chamada por `0x0041c93d`, nas
rotinas do tipo 8, `OSR.ANM`, iniciado em `0x0041c610`): ao percorrer os jogadores, pula o
Jake (personagem `+0x18 = 1`) quando `+0x180` ∈ {164, 165, 166} e o Garathe (`+0x18 = 2`)
quando `+0x180` ∈ {54, 55, 56}. O teste é pelo **estado**, não por um bit de `+0x178`. É a
única comparação com 164–166 fora do Jake no executável: o olho-laser (`0x0041f974`,
`olho-laser.md`) e os demais não a fazem.

## Especial: BAIXO+ATIRAR (estado 9, efeito `0x00427643`) [confirmado]

Entra pela mira (`tiro-jake.md`: BAIXO com `desc+0x0c ≠ 0` e vida > 25), estado 9
(`0x0043b895`), registro `0x47d7bc` (janela 0..5). O efeito:

1. O próprio Jake: `+0x24c = 3` (dano pendente), tipo `+0x252 |= 0` (comum), sem empurrão
   (`+0x244 = +0x248 = 0`); `0x004081ae(jake, 0x27, 4)` [inferido: piscar].
2. Centro = centro da caixa do Jake (`0x004351b5`). Para cada objeto da lista `0x488e7c`
   (não o Jake, mesma camada, `+0x178 & 0x80003 = 0x80001`, presente no modo, vaga de
   jogador ocupada), mede a distância entre os centros (`0x00467328`): só age se for
   **< 70 px** (o laço pré-filtra < 140).
3. Por tipo `+0x18` do alvo:

| Tipo | Efeito |
|---|---|
| 1, 2 (jogadores) | `+0x250 = 4` (derrubar, `dano-vida.md`), `+0x24c = 4·(19600 − d)/19600` (= 3), `+0x244 = ±15` (para longe do Jake), `+0x248 = 20`; com os dois jogadores e alvo com vida, `alvo+0x1b0 = jake+0x19c` (crédito) |
| 19–21, 25–27, 29, 36, 99, 100, 106, 108 (entre eles os projéteis 19, 20, 21, 26, 27 de `tiro-jake.md` e o raio laser 36 de `olho-laser.md`) | `0x0043d629(jake, alvo, 8)` e **destrói** o alvo (`0x00435b63`) |
| demais | `0x0043d629(jake, alvo, 9)` (dano pela tabela, variante 9) |

## Soco: caixa e acerto (`0x0043accb`, `0x0043afe1`) [confirmado]

No estado de soco, enquanto `t` está na janela `+0x04..+0x08` do registro, `+0x1c8 =
0x0043afe1` (zerado no começo de cada tique, `0x00422a53`); o laço de objetos o chama.

Caixa do golpe (`0x0043accb(jake, &r)`): parte da caixa de colisão do Jake (`0x00434d75`):

1. `r.l = l/2`, `r.a = a/3`; olhando à direita, `r.x += l/2` (metade da frente).
2. Altura: estados "frente" do descritor (`+0x30/+0x34`, punhos 150/151) → `r.y += a/3`
   (terço do meio); "diag. baixo/baixo" (`+0x38…+0x44`, 154/155) → `r.y += 2a/3`; "cima"
   (152/153) fica no terço de cima.
3. Recorte pela parede: segmento horizontal na meia altura de `r`, do lado de trás (x − 5,
   ou x + l + 5) até a frente; se bate no cenário (`0x00433aa3`) ou em objeto
   (`0x0043735d`), a largura vai até o ponto de contato.

`0x0043afe1`: para cada objeto (não o Jake, mesma camada, `0x80001`, presente, sem 0x8000
em `+0x178`) cuja caixa (`0x00434d75`) cruza `r` (`0x00405698`), chama `0x0043d629(jake,
alvo, registro+0x00)` (variante de dano do registro do estado). Sem limite de um acerto por
golpe no próprio laço [inferido: cada tique da janela pode acertar de novo, salvo
proteção em `0x0043d629`].

## Em aberto

- Se algum inimigo reage à velocidade/ruído (andar × correr).
- Quem são os tipos 25, 29, 99, 100, 106 e 108 (também destruídos pelo especial).
- Os demais inimigos (fora o tipo 8) ignoram o esconderijo? Nenhum outro teste achado.
- `0x004081ae` (efeito visual) e a variante do registro de cada soco.
