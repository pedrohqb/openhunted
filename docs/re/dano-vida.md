# Vida, armadura, dano e morte (M3)

Campos comuns a todos os objetos [confirmado pelo uso]:

| Campo | Papel |
|---|---|
| `+0x24c` | **dano pendente** do tique (somado por quem bate; zerado após aplicar) |
| `+0x244`/`+0x248` | empurrão pendente (vx, vy) |
| `+0x250` (`u16`) | reação pedida: `1` tranco, `2` empurrão, `4` derrubada, `8`/`0x10` derrubada para um lado/outro |
| `+0x252` (`u16`) | natureza do dano: `1` veneno, `2` explosão, `0x20` elétrico |
| `+0x258` | **vida** (da fase: `fields[14]`) |
| `+0x25c` | **armadura** (da fase: `fields[15]`) |
| `+0x270` | veneno (no Jake: quanto perde a cada 20 tiques) |
| `+0x1b0` | último jogador que causou dano (créditos) |

O Jake da `T_J_001` começa com **vida 50, armadura 0** (a inicialização `0x42277e` não mexe
nesses campos). Valores de outras fases: quase sempre 100/100. O Jake criado em
`0x4226xx` (renascimento nos modos de vários jogadores) recebe 100 e 50; a trapaça de
`0x404075` põe vida 100. [confirmado]

## Aplicação do dano (`0x00407862(obj)`, devolve a vida) [confirmado]

1. `+0x278 ≠ 0` (invulnerável) ou trapaça `0x49737c` ligada num jogador → devolve 1 e não
   aplica nada.
2. Sem dano pendente ou já morto → devolve a vida.
3. Se o dano **não** é veneno (`+0x252 & 1 = 0`) e há armadura, a armadura absorve — salvo
   quando o dano é 1, o tique é par e a vida ≠ 1:
   - `a = (dano + 1) / 2`; armadura −= a; dano −= a;
   - se dano ≥ vida: o excedente `dano − (vida − 1)` sai também da armadura e o dano vira
     `vida − 1` (a armadura **impede o golpe letal** enquanto durar);
   - armadura negativa: o que faltou volta para o dano; armadura = 0.
4. Vida −= dano (mínimo 0); `0x436358(obj, 0xc)` (aviso de mudança [inferido]: HUD).

## Reação do Jake (`0x004231f9`, a cada tique)

Chamada pela atualização do Jake (`0x422ab4`), antes da queda (`0x42415c`) e da máquina de
estados, salvo se `+0x178 & 0x200000` (morrendo/saindo) ou no estado 181. [confirmado]

1. Dano pendente com reação (`+0x250 & 7`) ou de explosão → `0x4081ae(obj, 0x2e, 4)`:
   pisca/tinge o sprite por 4 tiques [inferido; grava `+0x28c`/`+0x290`].
2. Reações (podem trocar de estado; qualquer uma "consome" o tique):
   - `2` → `0x423366`: em certos estados (1, 6, 79, 164–167) desliga `0x8` e vai ao
     estado 168 (`J168`, empurrado);
   - `1` → `0x4233e3`: tranco conforme o estado atual (tabela de saltos);
   - `4` → `0x4235fc`: derrubada; conforme o lado (`+0x288 & 8`) e o sinal de `+0x244`,
     marca `8` (estado 81, `J081`) ou `0x10` (estado 82, `J082`), com a velocidade
     `+0x164/+0x168 = 8·empurrão`;
   - elétrico (`+0x252 & 0x20`) → `0x423577`: fora dos estados 11/12, se `+0x284 = 0`, liga
     10 tiques de choque (estado 15, pisca com 0x25); `0x4235c4` conta e volta ao estado 1.
3. Veneno (`+0x270 ≠ 0`): pisca com 0x2f e, a cada 20 tiques (`tique % 20 = 0`), soma
   `+0x270` ao dano, marca `+0x252 |= 1` e às vezes toca a fala 3704 (`0xe78`).
4. `0x407862`: aplica. Se a vida chegou a 0 → `0x4236b9` (**morte**).
5. Zera empurrão, reação, natureza e dano pendente; vivo → `+0x1b0 = 0`.

O choque da saída recusada (`logica-fase.md`) entra por aqui: 20 de dano elétrico.

## Morte do Jake (`0x004236b9`) [confirmado]

Sem efeito se invulnerável/trapaça. Avisa `0x436358(obj, 4)` e escolhe a animação:

| Situação | Rotina | Estado (rótulo, quadros) |
|---|---|---|
| estado 26 | `0x426da2` | 173 (`J173`, 16) |
| estado 77 | `0x426ce8` | 177 (`J077`, 13) |
| estados 81/82 (em voo) | — | continua o voo [inferido: morre ao pousar] |
| estados 83/84 | `0x426d45` | 179 (`J179`, 12) |
| estados 30, 32 | `0x4270e5`, depois como "demais" | (sai do estado atual primeiro) |
| estados 3, 11, 12, 25, 76, 171, 173, 177, 179 | — | nada |
| demais, com `+0x250 & 0x10` | `0x426d45` | 179 |
| demais, com `+0x250 & 8` | `0x426ce8` | 177 |
| demais, com `+0x250 & 3` | `0x426da2` | 173 |
| demais, veneno | `0x426e36` | 171 (`J171`, 16) |

Todas: desligam `0x8`, ligam `0x200000`, põem `+0x1b4 = 120` (reordenando a lista de
objetos) e usam a colisão `JK_COLPS.BRD`. 173 e 171 também deixam o Jake cair
(`0x406fc0`). Rótulos e colisões vêm da tabela de estados do Jake (`0x473300 + 0x70·estado`:
`+0x01` BRD, `+0x10` rótulo; ver `personagem.md`).

**Depois** (`0x4268ac`, retorno ao fim da animação, só com o Jake parado no chão:
`vx = 0`, `0 ≤ vy ≤ 16`, contato `+0x224`):

- Solo ou cooperativo: vira **cadáver** — `+0x178 |= 0x8028`, colisão `+0x208 = 0`,
  `+0x198 = 0`, rotinas `+0x1c0/+0x1c8/+0x1cc/+0x1d0/+0x1d4` zeradas; fica no último quadro.
  Não há renascimento: o veredito (`0x43cdf0`, objetivo "sobreviver") dá **derrota** e,
  40 tiques depois, a partida acaba.
- Vários jogadores (disputa): o corpo é clonado (`0x433d80`), contam-se os pontos em
  `0x497334`/`0x497338` e o jogador renasce (`0x42696a…`). [confirmado em parte]

Não se viu dano por queda nas rotinas lidas (a queda `0x42415c` só troca de estado). [em
aberto]

## `DAMAGE.TBL` (`0x0043d514`)

Texto ASCIIZ de cabeçalho ("This file contains the binary formatted damage table…"),
`i32` n (149) e n registros de 15 `i32` (60 bytes), carregados em `0x49747c` com um registro
nulo no fim. [confirmado]

| Desl. | Campo |
|---|---|
| `+0x00` | tipo do atacante |
| `+0x04` | vítima: 0 qualquer; 1 só jogador (tipo ≤ 2); > 1 tipo exato; < 0 só sem vítima |
| `+0x08` | variante (0 = qualquer; senão igual ao 3º argumento) |
| `+0x0c` | natureza → `+0x252 |=` |
| `+0x10` | reação: 1 tranco, 2 empurrão, 4 derrubada (`+0x250`) |
| `+0x14` | dano direto → `+0x24c =` |
| `+0x18`/`+0x1c` | empurrão fixo (vx, vy; vx com sinal pelo lado) |
| `+0x20` | módulo do empurrão (reescala pela direção) |
| `+0x24` | efeito: −1 nenhum; 0–15 rótulo do `oex.anm` (0 = sorteio 1–4; 11 = 11 ou 12) |
| `+0x28` | raio do efeito |
| `+0x2c` | empurrão de área |
| `+0x30` | dano de área |
| `+0x34` | bits: 1 herda velocidade, 2 desloca 4 px para a vítima, 4 centro da caixa |
| `+0x38` | som posicional (0 = nenhum) |

`0x0043d629(atacante, vítima, variante)` acha a primeira linha que casa, aplica o dano
direto/reação/empurrão à vítima (se houver), cria o efeito e toca o som. [confirmado]

Detalhe da vítima (`0x43d6c5…0x43d8d2`) [confirmado]: `+0x24c =` dano (atribui, não soma);
`+0x252 |=` natureza; reação:

- 4 derrubada: `+0x250 = 4`; empurrão fixo ≠ (0, 0) → copiado, com vx negado se a vx do
  atacante (`+0x15c`) é < 0 ou, parado, se o centro do atacante está à direita do da vítima;
  fixo (0, 0) → centro da vítima − centro do atacante. Com módulo ≠ 0, o vetor é reescalado
  (`atan2` `0x46718e`, `cos/sen · módulo >> 14`).
- 2 empurrão (só se `+0x250 ≠ 4`): `+0x250 = 2`; fixo ≠ (0, 0) → copiado, lado só pelos
  centros; fixo (0, 0) → a velocidade do atacante (`+0x15c/+0x160`; zerada no contato dos
  projéteis). O módulo não é usado.
- 1 tranco: `+0x250 = 1` se ainda 0; sem empurrão.

O port transcreve a tabela em `damage/table.rs` (o teste `real_damage` confere com o
arquivo) e consulta `damage::row` nos golpes do jogador (soco, garras, clava, chicote) e nos
projéteis 108 e 25.

## Explosão (`0x0040d20c` e `0x0040d499`) [confirmado]

`0x40d20c(rótulo, &centro, vel, camada, dano_área, empurrão, raio, dono)` cria um objeto
tipo 86 "Explosion" (`oex.anm`; rótulos `oex01`–`oex05`, `oex11`–`oex15`), sinalizadores
`0x99`, `+0x260 = max(raio, 2)`, escala tal que o sprite meça `2·raio` px, centrado no
ponto; espelhado conforme a direção. Por tique (`0x40d45a`) avança um quadro e se destrói no
fim do rótulo. Com dano de área ≠ 0, roda **uma vez** `0x40d499`:

- `R = 1,5 · raio`; para cada objeto presente, `+0x178 & 0x80003 = 0x80001`, na mesma
  camada, com centro a distância `d < R` do centro da explosão:
- `+0x252 |= 2`; `+0x24c += dano_área · (R − d) / R`;
- empurrão `(+0x244, +0x248) = empurrão · (R − d)/R` na direção centro → vítima;
- se `empurrão · (R − d) / R > 7` → `+0x250 |= 4` (derrubada).

Mina (raio 28, empurrão 15, dano 10): `R = 42`; a 0 px dá 10 de dano e derruba (15 > 7);
encostada (d ≈ 22–27) dá 3–4 e não derruba. Com armadura, metade (arredondada para cima)
sai da armadura.

## Em aberto

- Tique em que `0x4231f9` vê o dano da explosão (ordem de atualização dos objetos).
- Estado do Jake ao morrer em voo (81/82) e no estado 3.
- Dificuldade: `0x487854` dobra/reduz o dano recebido por alguns inimigos (`0x42231b`);
  não se viu efeito no Jake.
- `0x4081ae` (efeito visual exato) e `0x436358` (eventos 4, 0xc, 0x20).
