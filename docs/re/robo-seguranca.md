# Robô de segurança (tipo 8, `OSR.ANM`)

O inimigo mais espalhado da campanha do Jake: 109 objetos em 25 fases (`D_J`, `H_J`, `I_J`).
O executável não lhe dá nome; o som 3157 que ele toca chama-se "security hovering" no
`SOUND.RES`, daí o nome adotado aqui [inferido]. Os sprites mostram um disco voador com um
olho e dois canhões. Colisão `OSR.BRD`; sinalizadores nos dados `0x84801`/`0x84805`.

## Inicialização (`0x0041c610`) [confirmado]

- Animação/colisão padrão `OSR.ANM`/`OSR.BRD` se a fase não der outras; genérica `0x40b2c8`.
- `+0x178`: desliga `0x80` e `0x8` (**tem gravidade**), liga `0x4800` (personagem: o Jake
  o atravessa). `+0x174 = 15`, `+0x1b4 = 100`.
- `+0x278 = −1` (orientação, abaixo); alvo `+0x274 = 0`; atenção `+0x270 = 0`; rajada
  `+0x264 = 0`; última posição vista `+0x280/+0x284 = 0`; `+0x288 = 0`.
- `values[0]` (`+0x260`) = modo de patrulha, 0 → 3. Os canhões `+0x27c/+0x27e` (`i16`)
  começam em 2 (mira horizontal) — por isso `values[7] = 131074 = 0x20002` nos dados.
- **Variante** = `+0x180` (= `fields[5]` da fase: 0, 1, 2 ou 3; na
  campanha: 60/29/10/10). Patrulha: modo 3 em 54, 1 em 50, 2 em 5. Não há nome de variante.
- Rotinas: `+0x1c0 = 0x41c7fc` (atualização), `+0x1c4 = 0x41da5a` (pulso de cor e som).
- Vida: **sem padrão** (a fase dá 100 em 80 dos 109; 20–75 nos demais).

## Variantes (tabela `0x00472a18`, 4 registros de 21 `i16`) [confirmado]

| Desl. | Uso | 0 | 1 | 2 | 3 |
|---|---|---|---|---|---|
| `+0x00` | passos de giro por tique (volta = 32) | 1 | 1 | 2 | 4 |
| `+0x02` | velocidade de patrulha (px/tique) | 2 | 2 | 2 | 3 |
| `+0x04` | perda de atenção por tique com o alvo morto | 1 | 2 | 3 | 9999 |
| `+0x06` | multiplicador da atenção | 1 | 4 | 6 | 9 |
| `+0x08` | chance de mudar de rumo, por 100 000 tiques | 200 | 155 | 100 | 50 |
| `+0x0a` | período de tiro (tiques) | 8 | 7 | 6 | 4 |
| `+0x0c` | velocidade do raio (px/tique) | 9 | 10 | 14 | 17 |
| `+0x0e` | % de mirar exato no alvo | 10 | 20 | 40 | 50 |
| `+0x10` | tiros por rajada | 6 | 9 | 12 | 15 |
| `+0x12` | pausa após a rajada (tiques) | 40 | 30 | 20 | 10 |
| `+0x14` | distância mínima ao alvo (px, em x) | 80 | 80 | 80 | 80 |
| `+0x16` | distância máxima ao alvo (px, em x) | 200 | 180 | 180 | 180 |
| `+0x18` | velocidade de perseguição `s`: `8·v+4` (1/8 px) | 2 | 3 | 5 | 8 |
| `+0x1a` | altura-base da flutuação na célula (px) | 74 | 42 | 54 | 74 |
| `+0x1c` | amplitude da flutuação (px) | 14 | 32 | 28 | 12 |
| `+0x1e` | período da flutuação (tiques) | 100 | 90 | 80 | 70 |
| `+0x22` | % de morrer com vida < 0 | 100 | 100 | 100 | 100 |
| `+0x24` | valor posto em `+0x288` ao ser atingido | 35 | 20 | 15 | 5 |
| `+0x26` | período do pulso de cor (tiques) | 20 | 15 | 10 | 7 |
| `+0x28` | índice do pulso (`0x43becd`) | 40 | 41 | 39 | 38 |

`+0x20` vale 0 e não é lido. Perseguição: 2,5 / 3,5 / 5,5 / 8,5 px/tique.

## Animação e orientação (`0x0041c712`) [confirmado]

`OSR.ANM`: rótulo 2 `osr00` (17 quadros, o disco girando), 1 `osr01` (25 quadros = 5 × 5
posições dos dois canhões), 3 `osr02` (8 quadros, destroçado).

- `+0x278 = −1` (normal, de lado): rótulo 1, quadro `5·+0x27c + +0x27e`.
- `+0x278 = −2` (morrendo): primeiro quadro do rótulo 3.
- `+0x278 = 0…31` (girando): rótulo 2; `r ≤ 16` → quadro `r`, sem espelho; `r > 16` → quadro
  `32 − r`, espelhado (`+0x194 = 1`). 8 = olhando à esquerda, 24 = à direita, 0 e 16 =
  costas/frente (andando na vertical).

`0x41d9c8(obj, r_desejado)`: com `−1` parte de 8 (sem espelho) ou 24 (espelhado); se já está
em `r_desejado` devolve 1; senão gira `+0x00` passos pelo lado mais curto (empate a 16:
sorteio) e devolve 0. O robô só se desloca alinhado.

## Atualização (`0x0041c7fc`, por tique) [confirmado]

1. Sai se `+0x178 & 0xa` (o `0x8` é ligado ao virar sucata). Contato `+0x220` diferente de
   1/2 é descartado; sem contato e fora do estado −2, zera a velocidade pedida.
2. Alvo morto → atenção −= `+0x04`. Alvo em outra camada: diferença > 1 → esquece; senão
   guarda uma vez o centro dele em `+0x280/+0x284` (última posição vista).
3. Atenção −1 por tique; ≤ 0 → esquece o alvo.
4. **Busca** sem alvo: `0x41db0f(robô, lista, 10, 256)` junta até 9 jogadores e escolhe um
   **ao acaso** (`rand % n`). Atenção = `(20 + rand%20) · +0x06` tiques.
5. Rajada: `+0x264 < 0` (pausa) sobe 1 por tique; ao chegar a 0, recarrega `+0x10` tiros.
6. **Dano** pendente: reação 4 (derrubada) → soma à velocidade 15 px/tique na direção do
   empurrão (`(cos,sen)·15 >> 11`). **Vida acima de 45 cai para 45**; dificuldade
   (`0x487854`): 0 → dano ×2, 2 → dano ÷2; dano negativo → 1. Vida −= dano; vida **< 0**
   → estado −2. `+0x288 = +0x24`. (Vida 0 ainda está viva.)
7. Com alvo e vida > 0 → perseguição (`0x41d08c`); sem alvo e fora do −2 → patrulha
   (`0x41d426`).
8. Estado −1 com alvo → os dois canhões (`0x41ce0d`).
9. **Flutuação** (estado −1, sem contato, sem velocidade pedida): `alvo_y = +0x1a +
   +0x1c·sen(2π·((tique + endereço) mod +0x1e)/+0x1e)`; `vy = alvo_y − (y mod 128)` limitado a
   ±64 (1/8 px), zerado se levaria abaixo de 0. Como na mina (`minas.md`), corrige 1/8 da
   diferença por tique.
10. **Queda e explosão** (estado −2, vida ≤ 0, sem `0x8`, contato de parede `+0x220 = 2`,
    `|vx|`,`|vy|` < 8): som 3103 (não posicional), `+0x178 |= 0x48`, `0x407009`, sem colisão,
    ordem de desenho 39, e **10 explosões** `0x43d629(robô, nulo, 2)` em pontos sorteados em
    ±30 px do canto da caixa. Depois o quadro e a compensação de plataforma (`0x404bc7`).

## Percepção (`0x0041db0f`) [confirmado]

Percorre a lista e aceita objetos com jogador (`+0x19c ≠ 0`), **exceto o Jake nos estados
164–166 e o Garathe nos 54–56** (esconder-se, `furtividade.md`), na mesma camada, vida > 0,
`+0x178 & 1`, distância entre centros **≤ 256 px** e linha livre (`0x433aa3`/`0x43735d`
dão 0). O teste de esconderijo vale só na aquisição: com o alvo já escolhido, o robô só o
perde por atenção esgotada, morte, troca de camada > 1 ou linha bloqueada.

## Perseguição (`0x0041d08c`) [confirmado]

- Ponto de mira: centro do alvo, ou `+0x280/+0x284` se houver. Linha bloqueada → esquece alvo
  e posição.
- Gira para o lado do alvo (24 se o alvo está à direita, senão 8); só anda alinhado e sem
  contato; ao alinhar, `+0x278 = −1`.
- `s = 8·+0x18 + 4`. Em outra linha de células: se a célula do robô deixa passar (bit `0x2`
  desligado) e o alvo está abaixo, `vy += s`; se deixa (bit `0x40` desligado) e o alvo está
  acima, `vy −= s` [inferido: bits de passagem da célula]. Nesse caso `|dx|` ganha 64.
- Indo à última posição vista: anda `s` até ela; ao chegar, usa as zonas PR (`0x4075af`): bit
  1 e alvo numa camada menor → camada −1; bit 2 e alvo numa maior → centra na célula
  (y = 64) e camada +1; senão esquece.
- Alvo à vista: `|dx| > +0x16` → aproxima-se a `s`; `|dx| < +0x14` → afasta-se. Mantém-se
  entre 80 e 180–200 px.

## Canhões (`0x0041ce0d`) [confirmado]

Duas bocas, tabelas de 5 registros (dx, dy, ângulo `u16`) relativas a `(+0x14c, +0x150)`:

| Pos. | Boca A (`0x4729a0`) | Boca B (`0x4729dc`) | Ângulo |
|---|---|---|---|
| 0 | (2, −5) | (6, −4) | 38912 (≈ 34° acima da horizontal, à esquerda) |
| 1 | (−2, 3) | (3, 2) | 35840 (≈ 17° acima) |
| 2 | (−3, 11) | (1, 8) | 32768 (horizontal) |
| 3 | (−2, 20) | (3, 15) | 29696 (≈ 17° abaixo) |
| 4 | (2, 27) | (6, 21) | 26624 (≈ 34° abaixo) |

- Altura visada: A mira `topo + a/8` do alvo, B mira `base − a/8`. Posição: 0 se `y + dy₀ >
  h + 40`; 1 se `y + dy₁ > h + 10`; 4 se `y + dy₄ < h − 40`; 3 se `y + dy₃ < h − 10`; senão 2.
- Disparo só com `+0x264 > 0` e no tique certo: A quando `(tique + +0x0a/2) mod +0x0a = 0`,
  B quando `tique mod +0x0a = 0` — os canhões alternam a cada meio período.
- Boca espelhada pela caixa (`0x405820`) conforme `+0x194`. Ângulo = o da tabela + sorteio em
  ±0x600 (±8,4°); com chance `+0x0e` %, se o ângulo até o centro do alvo estiver a menos de
  0x600 do sorteado, usa-o (tiro certeiro). `v = (cos, sen)·+0x0c >> 14`, vx negado se
  espelhado.
- Cria um **raio laser** (tipo 36, `0x410a18`, `olho-laser.md`) com variante = a do robô:
  dano no Jake 5 / 7 / 10 / 5 com empurrão. Cada tiro gasta 1 da rajada; zerada,
  `+0x264 = −+0x12`.

Variante 0: um raio a cada 4 tiques alternando os canhões, 6 por rajada, pausa de 40.

## Patrulha sem alvo (`0x0041d426`) [confirmado na estrutura; bits inferidos]

- Rumo `(+0x26c, +0x26d)` (−1/0/1 em x e y); só um eixo por vez. Anda `+0x02` px/tique,
  girando para 8/24 (horizontal) ou 0/16 (vertical) antes.
- "Preferência" de virar: com contato, ou sorteio `rand < +0x08·65536/100000`, vale o rumo
  atual (logo vira). Gatilhos (`+0x11c`): para cada entrada com alvo, se o código for 0,
  `+0x278 = −1` e o robô não anda neste tique; senão, se o centro do robô estiver a mais
  de `código` px do alvo, a preferência aponta de volta (**coleira** em torno do objeto).
  23 robôs da campanha têm gatilhos.
- Fora da fase, o rumo se inverte.
- Ao passar o centro da célula (x mod 128 ≥ (128 − largura)/2, ou y mod 128 ≥ 32), vira se
  o lado à frente está fechado (bits `0x8`/`0x10` em x, `0x2`/`0x40` em y), se há
  preferência, ou se o modo não permite o eixo (`values[0]` bit 1 = andar em x, bit 2 =
  andar em y). Uma virada por célula (`+0x268`); `+0x26e` conta viradas e escolhe o
  sentido do novo eixo (até 4 tentativas).

## Pulso, som e dano recebido

- `0x41da5a`: `p = 32·sen(fase)` com período `+0x26`; `0x43becd(+0x28, p)` (morrendo: −127).
  Com `p > 30`, toca **3157** "security hovering" posicional, no máximo a cada 250 ms (global
  `0x488834`). `SOUND.TBL` tipo 8: quadro 42 (destroço) → 3100.
- Bala da pistola: linha `(19, 8)` do `DAMAGE.TBL`: dano 2, efeito `oex14`, som 5056 "Bullet
  hits metal". Com o teto de 45, morre com 46 de dano (23 tiros no normal, 12 no fácil).
- Explosões da queda, linha `(8, 0, 2)`: efeito sorteado, raio 48, empurrão 15, dano de área
  10 → `R = 72` px. São 10: até ~100 de dano a quem estiver embaixo [inferido do cálculo].
- Objetivo: 7 robôs com `+0x1ac = 4` (destruir) na campanha; o veredito só olha a vida.

## Em aberto

- Significado exato dos bits de célula `0x2/0x8/0x10/0x40` usados na patrulha e na
  perseguição.
- O ramo de `0x41d08c` para o estado −2 com vida > 0 (recupera 1 de vida por tique abaixo de
  30 após `+0x288` tiques) parece inalcançável: o −2 só nasce com vida < 0.
- `0x407009` e o papel de `+0x174`.
- Nomes dos sons 3100 e 3103 (fora do `SOUND.RES`).
