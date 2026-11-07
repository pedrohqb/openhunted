# Minas flutuantes (tipo 102, `OHM00`, M3)

Inicialização `0x0041e6bc`, atualização `0x0041e7f5`. Dano e explosão em
[`dano-vida.md`](dano-vida.md).

## Variantes (tabela `0x00472bfc`, 4 registros de 72 bytes)

`values[0]` escolhe a variante (negativo → 0; ≥ 4 → 3). Campos do registro (deslocamento
dentro dele; todos `i32` salvo o nome) [confirmado na leitura; nomes de papel conforme o uso]:

| Desl. | Uso | Alpha (0) | Beta (1) | Gamma (2) | Delta (3) |
|---|---|---|---|---|---|
| `+0x00` | nome (`+0x45ea98`) | Alpha Mine | Beta Mine | Gamma Mine | Delta Mine |
| `+0x14` | período da oscilação vertical (tiques) | 32 | 64 | 32 | 32 |
| `+0x18` | velocidade (1/8 px por tique) | 25 | 15 | 45 | 20 |
| `+0x1c` | alcance de detecção (px, centro a centro) | 200 | 250 | 350 | 400 |
| `+0x20` | vida padrão (se a fase der 0) | 1 | 2 | 3 | 4 |
| `+0x24` | nº de explosões ao morrer | 1 | 1 | 1 | 1 |
| `+0x28` | valor de `+0x174` | 10 | 2 | 5 | 7 |
| `+0x2c` | amplitude da oscilação (px) | 24 | 32 | 28 | 12 |
| `+0x30` | % por tique de começar a "pairar" | 0 | 5 | 10 | 15 |
| `+0x34` | % por tique de parar de pairar | 100 | 20 | 20 | 10 |
| `+0x38` | espalhamento das explosões (px) | 46 | 30 | 50 | 60 |
| `+0x3c` | período do pulso de cor (tiques) | 30 | 50 | 30 | 20 |
| `+0x40` | índice do pulso de cor (`0x43becd`) | 40 | 41 | 39 | 38 |
| `+0x44` | % do dano recebido aplicado | 100 | 90 | 80 | 70 |

As 7 minas da `T_J_001` são **Alpha** (`values[0] = 0`) com vida 1 vinda da fase
(`fields[14] = 1`, `+0x258`). [confirmado]

## Inicialização (`0x41e6bc`) [confirmado]

- Genérica `0x40b2c8`; animação e colisão padrão `OHM00.ANM`/`OHM00.BRD` (se a fase não der
  outras); força o tipo `+0x18 = 102`.
- `+0x178`: desliga `0x80`, `0x8`, `0x20`, `0x10`; liga `0x801` → **sólida** e sem o bit
  "sem gravidade". A atualização reescreve a velocidade vertical pedida a cada tique; como
  isso convive com a gravidade da física ficou em aberto.
- Estado `+0x180 = 1`; `+0x1b4 = 145`; alvo `+0x27c = 0`; `+0x270` (morta) = 0;
  `+0x274` (pairando) = 0; velocidades `+0x15c..+0x168` = 0.
- Vida: se `+0x258 = 0`, usa a da variante.
- `+0x1c0 = 0x41e7f5` (atualização), `+0x1c4 = 0x41f22d` (pulso de cor).

`OHM00.ANM` tem um só rótulo (`ohm00`, 13 quadros) tocado em laço; o estado 1 usa
`0x407942(obj, 1, 0x41efa4)`, cujo retorno, ao fim do ciclo, procura alvo se não houver e
reinicia o rótulo 1. [confirmado]

## Atualização (`0x41e7f5`, por tique)

1. Morta (`+0x270`) → vida = 0 e sai.
2. `0x41eb64`: aplica dano pendente e trata contato (abaixo); se morrer, explode.
3. Morre (`+0x270 = 1`) se sair dos limites da fase (`x < 0`, `x ≥ 128·grade_x`, `y < 0`,
   `y ≥ 128·grade_y`), se a célula em `(x/128, y/128, camada)` não existir (`0x4317db`) ou
   se a vida ≤ 0; nesse caso é destruída (`0x435b63`) e sai. [confirmado]
4. Sem contato no tique (`+0x224 = 0`), zera `+0x164`/`+0x168` (velocidade pedida, 1/8 px
   [inferido]).
5. **Alvo** (`0x41ecf8`): mantém o atual se continuar válido; senão percorre a lista de
   objetos e escolhe o **jogador** (`+0x19c ≠ 0`) vivo **mais próximo** que seja válido.
   Válido (`0x41ed72`): mesma camada, vida > 0, distância entre centros (`0x467328`) ≤
   alcance, e linha de visão livre (`0x433aa3` e `0x43735d` dão 0 entre os centros
   [inferido: testes de parede do cenário]). [confirmado]
6. Com alvo:
   - não pairando: sorteio `rand % 100 < %começar` → passa a pairar (Alpha: nunca);
   - senão, se a mina está **toda acima** do alvo (base < topo do alvo, `0x41efcb`) e pode
     descer (`0x41f057` tenta um passo de física `0x432e5d` com vy = +velocidade e desfaz se
     bater) → vy = +velocidade;
   - senão, se está **toda abaixo** (`0x41f011`) e pode subir (`0x41f140`) → vy = −velocidade;
   - senão **oscila**: `fase = ((tique + x) mod período) · 65536 / período`;
     `alvo_y = 64 + amplitude · sen(fase)` (seno de `0x46717a`, escala 2^14) e
     `vy = alvo_y − (y mod 128)`, limitado a ±64; se `(y mod 128) + vy < 0`, vy = 0. Isto é, a
     mina persegue uma altura que oscila em torno do **meio da célula** (y = 64), corrigindo
     1/8 da diferença por tique; [confirmado]
     e na horizontal: centro do alvo à esquerda do da mina → vx = −velocidade, senão
     +velocidade (Alpha: 25/8 ≈ 3,1 px/tique, sempre **em direção ao jogador**);
   - pairando: não mexe na velocidade; sorteio `rand % 100 < %parar` → para de pairar.
   Sem alvo: nada é recalculado (com o passo 4, fica parada se nada a empurra).
7. Estado 1 → anima (acima).
8. `0x404bc7(obj, &d)`: subtrai `8·d` de `+0x164`/`+0x168` (compensação de plataforma/
   correnteza [inferido]).

`rand` (`0x460a74`) é um gerador aditivo atrasado (24, 55) de 16 bits.

## Contato e explosão (`0x41eb64`) [confirmado]

- Dano pendente `+0x24c ≠ 0` → `vida −= dano · %aplicado / 100`; zera `+0x252` e `+0x24c`.
  Qualquer tiro que registre dano mata a Alpha (vida 1).
- Objeto em contato: `+0x21c` (quem me tocou) se não for outra mina; senão `+0x224` se
  `+0x220 & 1`. Se houver um e o tipo dele **não** for 6, 22–25, 27, 46, 50–53, 65–69, 99 ou
  100, a vida vai a 0: a mina explode ao tocar no Jake (tipo 1), no Garathe, em projéteis etc.
  (Encostar em itens `OGF`, plataformas `OPO` e afins não conta.)
- Vida ≤ 0: `+0x270 = 1`, liga `0x40`; para cada uma das `+0x24` explosões: põe a mina num
  ponto sorteado em ±`espalhamento/2` em torno do próprio centro e chama
  `0x43d629(mina, nulo, variante)` (tabela de dano, linha do tipo 102); depois destrói a mina.
  Para a Alpha: **uma** explosão perto do centro.

Linha do `DAMAGE.TBL` usada (atacante 102, qualquer vítima, variante 0–3): sem vítima direta,
gera um objeto **Explosion** (`oex.anm`, rótulo sorteado 1–4) com raio 28, empurrão 15 e
dano de área 10 — iguais para as quatro variantes. O dano chega ao Jake pela área, com queda
linear até 42 px do centro: ver [`dano-vida.md`](dano-vida.md). Nenhum som próprio na linha
(campo de som 0). [confirmado]

Na prática, numa colisão lateral (centros a ~22–27 px), o Jake leva cerca de 3–4 de dano
e um empurrão abaixo do limiar de derrubada. Minas a menos de 42 px umas das outras também
levam dano — explodem em cadeia (vida 1). [inferido do cálculo]

## Pulso de cor (`0x41f22d`, rotina `+0x1c4`)

A cada tique chama `0x43becd(índice, 64·sen(2π·((tique + endereço) mod período)/período))`
com o índice e o período da variante. `0x43becd` aceita índices 0–47 e valores −127…127 e
monta a tabela de cor do desenho (`cores-sprites.md`): a rampa do sprite clareia e escurece. O som
3016 "Hover Mine Floating" consta do `SOUND.RES`, mas a chamada não foi localizada.

## Em aberto

- O papel de `+0x174`.
- Onde e como o som 3016 é tocado.
- Semântica exata de `+0x164`/`+0x168` frente a `+0x15c`/`+0x160` e de `0x404bc7`; por que
  a mina sem alvo não cai (o bit `0x8` fica desligado).
- Funções de linha de visão `0x433aa3`/`0x43735d` (assumidas).
