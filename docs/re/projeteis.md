# Projéteis: criação, bala da pistola, cápsula e impacto (M4)

Quem dispara e quando: [`tiro-jake.md`](tiro-jake.md). Tabela de dano, efeitos e reação da
vítima: [`dano-vida.md`](dano-vida.md). Mina: [`minas.md`](minas.md).

## Criação (`0x0040bff8(tipo, &pos, &vel, camada, lado, tamanho, dono)`) [confirmado]

Devolve 0 se o tipo, `pos` ou `vel` forem nulos (é assim que os punhos, com projétil 0, não
criam nada). Senão:

1. `0x433d80(1)` aloca; `0x40c900` faz a parte comum: nome "Bullet Object", `+0x1b4 = 160`
   (ordem de desenho), `+0x16c/+0x170 = 0`, sem rotinas `+0x1c4/+0x1c8/+0x1cc`, evento
   `+0x1d4 = 0x40ce8f`.
2. `+0x18 = tipo`; posição `+0x14c/+0x150` (e `+0x154/+0x158`) = `pos`.
3. Velocidade `+0x15c = +0x164 = 8·vx`, `+0x160 = +0x168 = 8·vy` (`vel` em px/tique; os
   campos em 1/8 px).
4. `+0x178 |= 0x2011` (presente; ativo em qualquer modo; `0x2000`: dois objetos com `0x2000`
   não colidem, logo balas não se chocam entre si); `+0x194 = lado`; `+0x17c = camada`;
   `+0x1b0 = dono` (jogador, para créditos); vida `+0x258 = 1`; `+0x260 = 1`.
5. Por tipo (animação, colisão, rotina `+0x1c0`, tamanho `+0x1f0`):

| Tipo | Nome | ANM / BRD | `+0x1c0` | `+0x1f0` |
|---|---|---|---|---|
| 19 | bullet (pistola) | `obu.anm` / por direção | `0x40c964` | 10 |
| 20 | bullet (espingarda) | `obk.anm` / por direção | `0x40ceec` | 10 |
| 21 | rocket | `oro.anm` / por direção | `0x40d678` | 25 |
| 22 | bullet shell | `obs00.anm` / `obs.brd` | `0x40c9f9` | 3 |
| 23 | buckshot shell | `osh00.anm` / `osh.brd` | `0x40c9f9` | 3 |
| 25 | knife | `okn.anm` / `okn01.brd` | `0x40ba54` | 3 |
| 26 | grenade | `ogr.anm` / `ogr2.brd` | `0x40d7ab` | 8 |
| 27 | ninja star | `ons.anm` / `ons2.brd` | `0x40bc98` | 10 |
| 99 | death bullet | `ods00.anm` / `ods.brd` | `0x40c964` | 20 |
| 100 | bruiser bullet | `obb00.anm` / `obb.brd` | `0x40c964` | 15 |
| 106 | sentinel bullet | `osb00.anm` / `osb00.brd` | `0x40caf6` | 5 |
| 108 | whip bullet | `owb.anm` / `owb.brd` | `0x40cb6c` | arg. ou 10 |

   Tipos 22/23 ligam `0x20` (só contato); 25 e 27 ligam `0x200`/`0x300`; a granada tem
   `+0x1b4 = 140` e `+0x260` sorteado em 35–44.
6. **Sem gravidade**: liga `0x8` em todos, exceto granada (`0x40d7ab`) e cápsulas
   (`0x40c9f9`).
7. `+0x1f4` = medida do quadro inicial (`0x44714b`); a escala é `+0x1f0 / +0x1f4`
   [inferido, como nos outros objetos].
8. **Direção** (tipos 19–21, 100, 106, 108): pela velocidade, `|vx| > 2|vy|` → horizontal;
   `|vy| > 2|vx|` → vertical; senão diagonal. `+0x194` é refeito como espelhamento
   (bit 1 horizontal, bit 2 vertical) e o rótulo é escolhido assim:

| Direção | Rótulo | `+0x194` | BRD (tipo 19) |
|---|---|---|---|
| direita / esquerda | 1 (`obu00`, 23×2) | 0 / 1 | `obu00.brd` (caixa 4×4 em x 16) |
| cima / baixo | 3 (`obu02`, 2×23) | 0 / 2 | `obu02.brd` (7×12) |
| cima-dir. / cima-esq. | 2 (`obu01`, 17×16) | 0 / 1 | `obu01.brd` (4×4 em x 11) |
| baixo-dir. / baixo-esq. | 2 | 2 / 3 | `obu01.brd` |

   (Tipo 20 usa `obk00/01/02.brd`; 21, `oro00/01.brd`.) O tipo 99 só escolhe o rótulo 1/4.
9. Liga na lista de objetos (`0x43470f`).

## Bala da pistola (tipo 19, atualização `0x0040c964`) [confirmado]

- **Sem tempo de vida**: voa em linha reta (sem gravidade) até tocar algo ou sair da fase.
- Velocidade: 20 px/tique na frente; (20, 20) nas diagonais; 20 para cima (registros em
  `tiro-jake.md`). A 20 tiques/s [inferido, `cenario.md`]: 400 px/s.
- Ao colidir, o evento `0x40ce8f(bala, outro, código)` grava o contato (`+0x220 = código`,
  `+0x224 = outro`), zera `+0x228…+0x23c` e a velocidade `+0x15c/+0x160` (a bala para).
- No tique seguinte, se há contato e ele **não** é só-de-toque (`+0x220 & 4 = 0`), ou se a
  bala saiu da fase (`x < 0`, `y < 0`, `x ≥ 128·grade_x`, `y ≥ 128·grade_y`):
  `0x43d629(bala, 0x4076d0(bala), 0)` e destrói (`0x435b63`). `0x4076d0` devolve o objeto
  tocado (`+0x21c`, ou `+0x224` se `+0x220 & 1`); parede dá vítima nula.
- Contato só-de-toque (código com `4`, como os itens `OGF` e a cápsula): ignorado, segue.
- Senão anima o rótulo em laço (`0x4079d9`).

Códigos de contato vistos: `1` objeto, `4` só toque (`5` = objeto só-de-toque); parede =
contato sem o bit 1 [inferido: `2`].

Não se viu exclusão do próprio atirador na colisão (`0x436caf`); como a bala nasce a 25 px e
se afasta a 20 px/tique, na prática não acerta o Jake. [inferido]

## Linhas do `DAMAGE.TBL` para o tipo 19 [confirmado]

Primeira linha que casa (ordem do arquivo; campos em `dano-vida.md`):

| Vítima | Natureza | Reação | Dano | Efeito (`oex`) | Raio | Bits | Som |
|---|---|---|---|---|---|---|---|
| 1 (jogador) | — | 2 empurrão | **2** | 11 (11 ou 12) | 5 | 3 | — |
| tipo 8 | — | 2 | 2 | 14 | 5 | 3 | 5056 "Bullet hits metal" |
| 0 (qualquer, inclusive nenhuma/parede) | 2 | 2 | 2 | **15** | 5 | 3 | — |

Empurrão fixo e módulo 0 nas três; sem dano de área. Bits 3 = herda a velocidade (já zerada)
e desloca 4 px para a vítima. O efeito é um objeto Explosion (tipo 86) de `2·5 = 10` px:
`oex15` (7 quadros) na parede/objetos, `oex11/12` (4–5 quadros) no jogador, `oex14` no tipo 8.
O som do impacto na parede vem do quadro: o primeiro quadro de `oex15` (quadro 148 do
`OEX.ANM`) toca **3046** "Bullet Hitting Wall" (ver `som.md`).

Não há faísca de boca como objeto: o clarão está nos quadros do próprio Jake (`J023gn`,
`J054gn`…) [inferido pelos nomes].

## Mina atingida [confirmado pelas duas rotinas]

A bala colide com a mina (sólida, `0x80801`); a mina vê no contato um objeto de tipo 19, que
não está na lista de exceções de `0x41eb64`, e vai a vida 0; a linha `(19, 0)` também lhe
põe `+0x24c = 2` (Alpha: 2·100 % ≥ vida 1). Ela explode como em `minas.md` (raio 28, dano de
área 10), o que pode atingir o Jake se ele estiver a menos de 42 px.

## Cápsula (tipos 22/23, `0x0040c9f9`) [confirmado]

Sai com a 1ª bala da rajada, com gravidade, só-de-toque. Ao tocar algo sólido: tipo 22 toca
3020 ou 3021 (sorteio) "Bullet shell clink", posicional; tipo 23 nenhum (um tipo 17 tocaria
3022). É destruída quando a velocidade fica abaixo de 1 px/tique nos dois eixos, quando
alguém a toca (`+0x21c`), quando pousa num objeto em movimento, ou ao sair da fase; senão
anima o rótulo 1 em laço.

## Linhas com o Jake como vítima (inimigos) [confirmado]

| Atacante | Dano | Reação | Outros |
|---|---|---|---|
| 19 bala | 2 | empurrão | efeito 11 |
| 20 espingarda | 10 | derrubada, módulo 20 | |
| 25 faca | 12 | empurrão | |
| 27 estrela | 5 | empurrão | área 20, raio 30 |
| 29 | 1 | empurrão | |
| 108 chicote | 20 | derrubada, módulo 20 | |
| 3 | 4 | — | empurrão fixo 3 |
| 4 | 30 | — | natureza `0x10`, som 3147 |
| 36 (variante 0–3) | 5 / 7 / 10 / 5 | empurrão | natureza 10 |
| 99 / 100 / 106 (qualquer) | 15 | empurrão / — / empurrão | efeito sorteado 1–4, raio 28 / 28 / 18; só o 100 tem área (empurrão 30, dano 10) |

## Em aberto

- Rotinas dos outros projéteis (`0x40ceec`, `0x40d678`, `0x40d7ab`, `0x40bc98`, `0x40caf6`,
  `0x40cb6c`) e as linhas 20/21/26.
- Código de contato exato da parede e se a BRD espelhada (`+0x194`) é usada na colisão.
- Se `+0x164/+0x168` (onde entra o desvio da rajada) ou `+0x15c/+0x160` movem a bala.
