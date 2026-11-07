# Tiro do Jake: controles, estados, mira e cadência (M4)

Inventário e troca de arma em [`armas.md`](armas.md); o que sai do cano em
[`projeteis.md`](projeteis.md). Estados: tabela `0x473300 + 0x70·estado` (`+0x10` rótulo,
`+0x18` atraso de quadro); o rótulo do `JAKE.ANM` é igual ao número do estado nos estados
de tiro. `+0x180` é o **estado corrente** do Jake.

## Entrada (`0x00407e2d`, por tique) [confirmado]

Lê a estrutura do dispositivo do jogador (`0x499014[jogador]`) e monta os bits em
`0x487a2c`:

| Campo | Bit | Ação |
|---|---|---|
| eixo X `+0x94` < −50 / > 50 | `0x4` / `0x8` | ESQUERDA / DIREITA |
| eixo Y `+0x96` < −50 / > 50 | `0x1` / `0x2` | CIMA / BAIXO |
| `+0x98` | `0x100` | ATIRAR |
| `+0x99` | `0x200` | PULAR |
| `+0x9a` | `0x400` | FURTIVO (também ligado se o eixo pressionado não passa de ±100) |
| `+0x9b` | `0x1000` | PRÓXIMA ARMA |
| `+0x9c` | `0x800` | ARMA ANTERIOR |
| `+0x9d` | `0x40000` | SOCO |
| mudou desde o tique anterior | `0x2000`, `0x4000`, `0x8000`, `0x20000`, `0x10000`, `0x80000` | ATIRAR, PULAR, FURTIVO, PRÓXIMA, ANTERIOR, SOCO |
| 2º eixo `+0xa8/+0xaa` | `0x10…0x80` | não usados no tiro |

Com `+0x264 > 0` (confusão), os pares de direção se invertem e `+0x264` decresce. Em
dispositivos com menos de 4 botões, PULAR+BAIXO+DIREITA/ESQUERDA vira próxima/anterior arma.

**Teclado** (`0x00448530`): CIMA, BAIXO, ESQUERDA, DIREITA são as teclas em `+0xb4…+0xb7`
(eixo ±128); o botão `i` é a tecla em `+0xb8 + i`. `0x0043eed2` copia as teclas de
`0x480048` (jogador 1) ou `0x480098` (jogador 2), na ordem do `SHELL.RES` (CIMA, BAIXO,
ESQUERDA, DIREITA, ATIRAR, PULAR, FURTIVO, ANTERIOR, SOCO, PRÓXIMA):

| Ação | Jogador 1 | Jogador 2 |
|---|---|---|
| direções | setas | I, M, J, K |
| ATIRAR | **Ctrl** | Espaço |
| PULAR | Shift | Enter |
| FURTIVO | X | `;` |
| ANTERIOR / PRÓXIMA | PgUp / PgDn | Insert / Delete |
| SOCO | Z | `/` |

(Códigos VK; nomes no `KEYS.RES`, índice = VK − 1.) Se essas tabelas são o padrão ou a
configuração salva é [inferido: padrão].

**Lado**: todo tique, ESQUERDA põe `+0x288 = 4` e DIREITA `+0x288 = 8` (`0x00422a70`), em
qualquer estado — inclusive atirando. `+0x294` (trava de tiro) decresce todo tique.

## Descritor da arma (`0x0043ab05`, 32 `i32`) [confirmado]

Jake: arma 0/1 → `0x47df28` (punhos), 2 → `0x47dfa8`, 3 → `0x47e028`, 4 → `0x47e0a8`,
5 → `0x47e128`, 6 → `0x47e1a8`. Com o modo soco `0x47d7b8 ≠ 0`, sempre os punhos.

| Campo | Uso | Punhos | Pistola |
|---|---|---|---|
| `+0x00` | estado "pronto" | 146 | 18 |
| `+0x04` | estado "guardar" | 147 | 19 |
| `+0x0c` | estado do especial (BAIXO) | 9 | 9 |
| `+0x10` | passo do contador por tique | 1 | 1 |
| `+0x14` | tiques em "pronto" antes de guardar | 10 | 10 |
| `+0x18/+0x1c` | projétil / cápsula | 0 / 0 | 19 / 22 |
| `+0x20…+0x44` | estados de tiro: cima, cima, diag. cima ×2, frente ×2, diag. baixo ×2, baixo ×2 | 152,153,152,153,150,151,154,155,154,155 | 56,56,54,54,23,23,55,55,55,55 |
| `+0x48…+0x6c` | registro de tempo/boca de cada estado acima | | |
| `+0x70` | registro do estado especial | `0x47d7bc` | `0x47d7bc` |
| `+0x74` | rotina de disparo própria | — | `0x43bab7` |
| `+0x78` | efeito do especial | `0x427643` | `0x427643` |
| `+0x7c` | volta ao repouso | `0x426ecc` | `0x426ecc` |

Demais (passo, projétil, estados): espingarda 1, 20/23, 125,123,122,124,124; foguetes 1,
21/0, 67,66,68; estrelas **2**, 27/0, 144,142,141,143; granadas **2**, 26/0, 196,194,193,195.

## Registro de estado (`0x43ac05(obj, desc, estado)`, 18 `i32`) [confirmado]

`t` = contador `+0x260` (zerado ao entrar no estado, soma o passo a cada tique).

| Campo | Uso | Pistola (23) |
|---|---|---|
| `+0x04..+0x08` | janela do golpe corpo a corpo (`+0x1c8 = 0x43afe1`; desliga `0x4000`) | 0..0 |
| `+0x0c..+0x10` | **janela de disparo**: dispara se `+0x0c ≤ t < +0x10 + passo` | 1..3 |
| `+0x14` | após `t > +0x14`: segurando ATIRAR → novo disparo; movendo/pulando → sai | 6 |
| `+0x18` | após `t > +0x18`: reavalia (sem ATIRAR → recuperação) | 12 |
| `+0x1c` | estado de recuperação | 22 |
| `+0x20/+0x24` | dx da boca olhando à esquerda / à direita (px) | −25 / 25 |
| `+0x28` | dy da boca | 2 |
| `+0x2c`, `+0x30` | velocidade do projétil: \|vx\| (sinal pelo lado) e vy, **px/tique** | 20, 0 |
| `+0x34/+0x38`, `+0x3c` | dx (esq./dir.) e dy da cápsula | 22 / −4, 0 |
| `+0x40`, `+0x44` | vx (sinal pelo lado) e vy da cápsula | −10, −10 |

Pistola, demais: 56 (cima) dx 9/12, dy −35, v (0, −20); 54 (diag. cima) dx −16/24, dy −23,
v (20, −20); 55 (diag. baixo **e** baixo) dx −23/27, dy 33, v (20, 20). A pistola não atira
reto para baixo. Os dx/dy somam-se a `(+0x14c, +0x150)`. Cápsulas (registros `0x47d9b4`,
`0x47d9fc`, `0x47da44`, `0x47da8c`): 56 dx 9/12, dy −10, v (−10, 0); 54 dx 40/−30, dy 0,
v (−10, −10); 23 dx 22/−4, dy 0, v (−10, −10); 55 dx −7/25, dy 20, v (10, −10).

## Fluxo do tiro [confirmado]

Despacho por estado (`0x00422b02`): 18, 62, 108, 127, 146, 186 → `0x43b99b` (pronto);
19, 64, 109, 128, 147, 187 → `0x43ba0b` (guardar); 22, 73, 112, 131, 188 → `0x43ba54`
(recuperação); estados de tiro → `0x43b50b`; 149 → `0x43b43e`; 9 → `0x43b895`.

1. **Entrada**: num estado que aceita tiro, com `bits & 0x40100` (ATIRAR ou SOCO) e
   `+0x294 = 0`, chama `0x0043b43e`.
2. `0x43b43e`: com ATIRAR/SOCO, põe `+0x180 = 0` e escolhe o estado pela direção
   (`0x43ae4c`, abaixo); um dos dois estados do par, por sorteio (`rand & 1`), vai a
   `0x43b50b`. Sem ATIRAR: recuperação (`+0x1c`) ou "pronto".
3. `0x0043b50b(obj, estado)`: com SOCO liga `0x47d7b8 = 1`, com ATIRAR o zera; se a linha do
   centro da caixa do Jake até a boca bate em parede (`0x43bbe3`, testes `0x433aa3`/
   `0x43735d`), `0x47d7b8 = 2` (dá um soco). Estado novo → define estado/rótulo e `t = 0`;
   mesmo estado → anima. `t += passo`; aplica as janelas acima. A cada novo disparo o estado
   foi zerado no passo 2, então a animação recomeça.
4. **Disparo** (`0x0043b1d0`): sem munição → passo 6. Linha do centro da caixa até a boca
   bloqueada → não atira. Senão gasta 1 (`0x40a401`), chama `+0x74` se houver; senão cria
   o projétil e a cápsula (`0x40bff8`, `projeteis.md`); aviso `0x436358(obj, 0x20)`.
5. **Fim**: `0x43ba54` → estado `+0x1c` (pistola 22, `J022`, 4 quadros) e depois "pronto"
   (`0x43b99b`: 18, `J018`) por `+0x14` = 10 tiques; aí `0x43ba0b` → "guardar" (19, `J019`,
   9 quadros) → repouso. Direção/pulo/furtivo/troca (`0x1e0f`) interrompem e chamam `+0x7c`.
6. **Sem munição**: som 3090, `+0x254 = 0x43a8cc(obj, 1)`, aviso 0x20, pose "pronto" da arma
   nova (`0x42318e`).

## Cadência da pistola [confirmado pela regra; tempo em tiques]

Janela 1..3 com passo 1 → **3 balas por acionamento**, uma por tique (t = 1, 2, 3), cada
uma gastando 1 tiro. A rotina `0x43bab7` desvia a 1ª e a 3ª: soma ao vetor pedido
(`+0x164/+0x168`) 8/8 px (1 px/tique) perpendicular: 1ª `(−vy, vx)·8/|v|`, 3ª o oposto,
ambos negados se `+0x288 & 4` (na mira de frente: 1ª desce, 3ª sobe, nos dois lados);
`|v|` por `0x467328`. A cápsula só sai com a 1ª. Segurando ATIRAR, o ciclo recomeça quando `t` chega a 7: rajadas
de 3 a cada **6 tiques**. Soltando, sem mexer, o Jake fica no quadro até `t > 12`.

## Mira (`0x43ae4c(obj, desc, bits, &a, &b, &lado)`) [confirmado]

| `bits & 0xf` | Estados | Lado |
|---|---|---|
| nenhum, CIMA+BAIXO, ESQ+DIR | frente (`+0x30/+0x34`) | mantém |
| CIMA (com ou sem ESQ+DIR) | cima (`+0x20/+0x24`) | mantém |
| ESQ/DIR (+CIMA+BAIXO) | frente | esq./dir. |
| CIMA+ESQ/DIR | diag. cima (`+0x28/+0x2c`) | esq./dir. |
| BAIXO+ESQ/DIR | diag. baixo (`+0x38/+0x3c`) | esq./dir. |
| BAIXO (com ou sem ESQ+DIR) | **especial** se `+0x0c ≠ 0` e vida > 25; senão baixo (`+0x40/+0x44`) | mantém |

**Especial** (estado 9, `j009`, 8 quadros, `0x43b895`; efeito `0x427643` na janela 0..5 do
registro `0x47d7bc`): custa **3 de vida por tique** ao Jake (5 tiques por uso, `furtividade.md`), pisca, e atinge objetos a até 140 px com
as linhas do atacante 1, variantes 8/9 do `DAMAGE.TBL`. Atenção: BAIXO+ATIRAR com vida > 25
faz isto em vez de mirar para baixo.

## Onde se pode começar a atirar [confirmado]

Chamam `0x43b43e` com ATIRAR: o tratamento comum de "em pé" `0x423940` (estados 1, 4, 6, 26,
77–79, 83, 84, 162, 163, 168); 7 e 8 (`0x423b2b`, zera a velocidade antes); **10**
(`0x423db9`, andar/correr: **zera a velocidade**, o Jake para para atirar); 2 (`0x423f85`,
idem); 3, 16, 17, 25, 27, 85–87, 103–107, 164–166 (alguns zeram só `+0x168`). Os estados de
tiro são de pé: não há tiro correndo. No ar (16/27 [inferido: pulo]) o tiro zera a
velocidade vertical pedida; a queda nos estados de tiro é de `0x42415c`.

## Soco

SOCO (Z), ou ATIRAR com a arma 1, ou a boca dentro da parede, usa os punhos: estados 150/151
(frente), 152/153 (cima), 154/155 (baixo); golpe na janela `+0x04..+0x08` (150: 2..3) via
`0x43afe1`; nunca dispara projétil (janela de disparo vazia). Dano: linhas do atacante 1.

## Em aberto

- `0x43afe1` (acerto do soco) e as linhas exatas que usa.
- Significado dos estados 2, 3, 16, 17, 25, 27, 85–87, 103–107, 164–166 (ver
  `personagem.md`).
- Se o tiro no ar mantém a queda normal; fricção nos estados de tiro.
- Origem exata de `(+0x14c, +0x150)` em relação ao sprite (dy = 2 na frente sugere que não é
  o topo).
