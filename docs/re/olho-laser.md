# Olho-laser (tipo 104, `OLE.ANM`) e raio laser (tipo 36)

Usado na `T_J_003` (um, no teto da sala inicial) e na `T_J_007` (dois: sobre o início do Jake
e no teto do corredor de baixo). Dica do mago nº 28 (`WIZARD.RES`): "To destroy the Laser Eyes
get underneath them and SHOOT + UP".

## Inicialização (`0x0041f288`) [confirmado]

- Genérica `0x40b2c8`; animação/colisão padrão `OLE.ANM`/`OLE00.BRD` se a fase não der outras.
- `+0x1c0 = 0x41f58f` (atualização). Nenhuma outra rotina (a genérica zera `+0x1c4..+0x1cc`).
- Variante `values[0]` (`+0x260`): negativa → 0; ≥ 5 → 4. Nome da variante (`0x45ea98`).
- Vida: 0 → 100 (a tabela nunca é consultada para isso; e qualquer dano mata, abaixo).
- `+0x174 = 5`; `+0x178`: desliga `0x80`, liga `0x801` e `0x8`, desliga `0x20` → **sólido**,
  sem gravidade; `+0x1b4 = 100`.
- Estado: `+0x274` (alvo) = 0, `+0x278` (atirando) = 0, `+0x26c` (posição da varredura) = 0,
  `+0x279` (sentido) = 1 (crescente); contadores da rajada `+0x268/+0x269/+0x26a` e período
  da varredura `+0x290` vêm da variante; velocidades zeradas; desenha o quadro (`0x41f45c`).
- **Alcance** `+0x264` = `values[1]`, salvo quando `values[15]` (`+0x29c`) ≠ 1: aí
  `+0x29c = 1` e o alcance vem da variante. Nos dados `values[15] = 1`: alcance 400 na
  `T_J_003` e 150 na `T_J_007`.
- `values[4]` (`+0x270`) é o interruptor "ativo": a atualização não faz nada se ≤ 0. Nos dados
  vale 5 (ativo).

## Variantes (tabela `0x00472d30`, 5 registros de 0x44 bytes) [confirmado]

| Desl. | Uso | Alpha 0 | Beta 1 | Gamma 2 | Delta 3 | All 4 |
|---|---|---|---|---|---|---|
| `+0x00` | nome ("… Laser Eye") | Alpha | Beta | Gamma | Delta | All |
| `+0x14` | alcance padrão (px) | 150 | 150 | 350 | 300 | 300 |
| `+0x18` | vida padrão (não usada) | 1 | 1 | 1 | 1 | 1 |
| `+0x1c` | desvio angular máx. do tiro (unid. 1/65536 volta) | 1024 | 1280 | 1536 | 1792 | 768 |
| `+0x20` | velocidade do raio (px/tique, ±3 sorteado) | 8 | 10 | 20 | 30 | 45 |
| `+0x24` | intervalo entre tiros (byte → `+0x268`) | 3 | 2 | 1 | 0 | 10 |
| `+0x28` | tiros por rajada (byte → `+0x269`) | 5 | 5 | 6 | 8 | 10 |
| `+0x2c` | pausa após a rajada (byte → `+0x26a`) | 10 | 15 | 20 | 25 | 30 |
| `+0x30` | tiques por passo da varredura (`+0x290`) | 5 | 4 | 3 | 2 | 1 |
| `+0x3c` | variante do raio (tipo 36) | 0 | 1 | 2 | 3 | sorteio 0–3 por raio |

`+0x34` (30/50/30/30/20), `+0x38` (40/41/39/38/42, os mesmos índices de pulso de cor das
minas) e `+0x40` (60/80/100/50/100) não são lidos por nenhum código encontrado. Todos os
olhos das fases tutoriais são **Alpha** (`values[0] = 0`).

## Atualização (`0x0041f58f`, por tique) [confirmado]

1. **Dano** (`0x41f898`): qualquer dano pendente (`+0x24c ≠ 0`) → `+0x270 = 0`,
   `0x43d629(olho, nulo, 0)` (linha do atacante 104: efeito `oex` sorteado 1–4, raio 48,
   empurrão 15, dano de área 10 → `R = 72` px), destrói o objeto, vida 0, liga `0x40`. Um
   tiro de pistola (2 de dano) basta.
2. Inativo (`+0x270 ≤ 0`) → sai. Sem contato no tique → zera `+0x164/+0x168`.
3. **Alvo** (`0x41f8e0`): mantém o atual se ainda válido; senão percorre a lista e escolhe,
   entre os **jogadores** (`+0x19c ≠ 0`) vivos e válidos, o mais próximo (`0x41fafb`). Guarda
   também em `+0x288` (último alvo visto). Válido (`0x41f974`): mesma camada; vida > 0;
   distância entre centros (`0x467328`) ≤ alcance; linha de visão livre (`0x433aa3` e
   `0x43735d` dão 0, como nas minas); e o ângulo olho→alvo (`0x46718e(dx, dy)`) a no máximo
   **1500** unidades (≈ 8,2°) da mira atual. Ou seja, o olho só "vê" o Jake dentro do cone
   do feixe.
4. Sem atirar (`+0x278 = 0`): a cada `+0x290` tiques dá um passo na varredura (`0x41fdf0`):
   `+0x26c` ± 1 conforme `+0x279`; em 12 inverte e volta a 10; abaixo de 0 inverte e vai a 1
   (vaivém 0…11).
5. Com alvo: `+0x278 = 1`, `+0x27a = 1`, `+0x28c` = x do alvo. Sem alvo, se acabou de perdê-lo
   (`+0x27a`): escolhe o sentido da varredura para o lado em que o último alvo foi
   (`0x41fe3f`: x atual do `+0x288` < `+0x28c` → sentido decrescente) e dá um passo; repõe os
   contadores da rajada; `+0x278 = 0`.
6. **Rajada** (atirando e com alvo), por tique: se `+0x268 > 0`, decrementa; senão recarrega
   `+0x268` e: se restam tiros (`+0x269`), `+0x27c++`, **dispara** (`0x41fbe6`) e decrementa;
   senão, se resta pausa (`+0x26a`), decrementa; senão recarrega tiros e pausa. Alpha: um tiro
   a cada 4 tiques, 5 tiros (20 tiques), pausa de 10×4 = 40 tiques; ciclo ≈ 64 tiques.
7. **Seguir o alvo**: mira = `0x7a24 − 3000·posição` (16 bits). Se `|ângulo do alvo − mira|`
   ≥ 375: ângulo do alvo > mira e alvo andando para a esquerda (`+0x15c < 0`) → posição−−,
   sentido 0; ângulo ≤ mira e alvo andando para a direita → posição++, sentido 1. Limita a
   0…11. Parado, o Jake não é seguido.
8. Quadro (`0x41f45c`): atirando → rótulo 2 (`ole01`, 24 quadros), quadro
   `2·posição + (+0x27c & 1)` (alterna o clarão a cada tiro) e chama `0x43c774(centro, camada)`;
   senão rótulo 1 (`ole00`, 12 quadros), quadro = posição. Ao passar pelos quadros 2, 5 e 8 do
   rótulo 1 toca **3112** "lasereye move" posicional, no máximo a cada 250 ms (`0x461178`,
   relógio em ms; próximo instante em `0x488868`).

Convenção angular [inferido]: `0x46718e` é o atan2 de 16 bits e `0x467154`/`0x46717a` o
cosseno/seno (escala 2^14). Com y para baixo, a posição 0 mira ≈ 172° (esquerda), 5–6 quase
na vertical para baixo e 11 ≈ −9,5° (direita): o olho de teto varre o semicírculo de baixo.

## Disparo (`0x0041fbe6`) [confirmado]

- Duas bocas: `boca1 = posição do objeto + tabela 0x00472e84[pos]`,
  `boca2 = posição + tabela 0x00472f14[pos]` (registros de 12 bytes: dx, dy, ângulo extra 0).
  Ex. (dx, dy) da boca 1: pos 0 (1, 4), 5 (18, 22), 11 (45, 4); boca 2: pos 0 (2, 4),
  5 (31, 22), 11 (45, 4). São coordenadas do sprite em escala 1.
- Ângulo = atan2(boca1 → centro do alvo) + sorteio uniforme em [−desvio, +desvio).
- Velocidade = `+0x20` da variante + (`rand % 6` − 3); `v = (cos·s, sen·s) >> 14`.
- Cria **dois** raios (um por boca) com `0x410a18(&boca, &v, camada, variante_do_raio)`.

## Raio laser (tipo 36, criado em `0x00410a18`) [confirmado]

- Objeto novo (`0x433d80`), `+0x18 = 36`, `+0x1b4` sorteado 90 ou 170 (frente/trás), na
  lista; `+0x180` = variante; posição = boca; velocidade `+0x15c/+0x164 = 8·vx`,
  `+0x160/+0x168 = 8·vy` (1/8 px); sinalizadores `|= 0x3031` (`0x20` só contato, `0x2000`
  raios não colidem entre si, `0x10`, `0x1000`); camada do olho; `+0x174 = 1`; colisão
  `ola.brd`; escala 1/1. A caixa é deslocada para trás conforme o sinal da velocidade.
- Som posicional na criação: 3037 (variantes 0 e 2) ou 3036 (1 e 3), "Laser" (`0x47209c`).
- Rotinas: `+0x1c0 = 0x410bda`, `+0x1d0 = 0x410dae`, `+0x1d4 = 0x411043` (evento de colisão).

Atualização `0x410bda`:

- Fora da fase → destruído.
- Contato (`+0x224`) não só-de-toque (`+0x220 & 4 = 0`): se o código tem o bit 2 (parede
  [inferido]) só para; se é outro objeto que não um raio, `0x43d629(raio, tocado, variante)`.
  Em ambos, `+0x274 = 0`.
- `+0x274` começa em −1; quando chega a 0 o raio "morre": `+0x270 = 5`, liga `0x8`, para, e
  é destruído 5 tiques depois. Há ainda um caso (`+0x280`) que reaproveita a velocidade
  medida `+0x238/+0x23c` quando > 2 [em aberto].

Linhas do `DAMAGE.TBL` (atacante 36) [confirmado]:

| Vítima | Variante | Natureza | Reação | Dano | Efeito |
|---|---|---|---|---|---|
| 1 (jogador) | 0 / 1 / 2 / 3 | 10 (`0x2|0x8`) | 2 empurrão | **5** / 7 / 10 / 5 | `oex01`, raio 5 |
| 0 (outro) | 0 / 1 / 2 / 3 | 10 | — | 5 / 7 / 10 / 5 | nenhum |

A natureza inclui `0x2` (explosão): um raio que acerte um barril o faz explodir
([`barril-fogo-cerca.md`](barril-fogo-cerca.md)).

## Na prática

- O Jake perde 5 por raio (Alpha), até 10 por disparo (dois raios), com tranco.
- Matar: qualquer tiro que acerte (o olho é sólido para a bala). A explosão do olho tem
  `R = 72` px: atirando de baixo, a 100+ px, não fere o Jake [inferido do cálculo].
- O olho dorme se o Jake fica fora do cone de ≈ 8° do feixe e só o segue enquanto ele anda.

## Em aberto

- `0x43c774` (chamado ao atirar e ao apagar fogo): provavelmente para um som em laço naquele
  ponto ou acende uma luz.
- Caso `+0x280` do raio (reaproveitar a velocidade medida) e o código exato da parede.
- Campos `+0x34`, `+0x38`, `+0x40` da variante.
- Convenção exata do atan2 (`0x46718e`) e da caixa do raio (`ola.brd`).
