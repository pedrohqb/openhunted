# Gerador de minas (tipo 103, `OHG.ANM`)

Nome por variante: "Alpha/Beta/Gamma/Delta/All Generator" (`0x472ad0`). 65 objetos em 22
fases da campanha; colisão `OHG00.BRD`; sinalizadores `0x80809`. É uma caixa fixa que se abre e
solta **minas flutuantes** (tipo 102, [`minas.md`](minas.md)) quando vê um jogador.

## Inicialização (`0x0041dcc8`) [confirmado]

- Genérica `0x40b2c8`; `OHG.ANM`/`OHG00.BRD` se a fase não der outros; `+0x1c0 = 0x41de45`,
  `+0x1c4 = 0x41e596` (pulso de cor).
- Variante `values[0]` (`+0x260`): negativa → 0; ≥ 5 → 4. Nome da variante (`0x45ea98`).
- `+0x174 = 1`; `+0x178`: desliga `0x80`, `0x20`; liga `0x801` e `0x8` → **sólido, sem
  gravidade**. Estado `+0x180 = 1`; `+0x1b4 = 100`; velocidades zeradas.
- Vida: 0 → `+0x18` da variante. Alvo `+0x27c = 0`; morto `+0x26c = 0`.
- `+0x274` (contagem até a próxima leva) = `+0x20`; `+0x270` (estoque) = `+0x24`. Os
  `values[4..6]` da fase caem nesses campos e são **sobrescritos** aqui.
- Quatro vagas de mina `+0x280/+0x284/+0x288/+0x28c` = nulas.

## Variantes (tabela `0x00472ad0`, 5 registros de 0x38 bytes) [confirmado]

| Desl. | Uso | Alpha 0 | Beta 1 | Gamma 2 | Delta 3 | All 4 |
|---|---|---|---|---|---|---|
| `+0x14` | alcance (px, centro a centro) | 150 | 200 | 250 | 300 | 200 |
| `+0x18` | vida padrão | 15 | 20 | 25 | 20 | 20 |
| `+0x1c` | exige linha de visão | 0 | 0 | 0 | 1 | 0 |
| `+0x20` | intervalo entre levas (tiques) | 20 | 30 | 40 | 50 | 60 |
| `+0x24` | estoque de minas | 50 | 40 | 100 | 100 | 100 |
| `+0x28` | minas por leva | 1 | 3 | 4 | 2 | 5 |
| `+0x2c` | período do pulso de cor | 30 | 50 | 30 | 20 | 10 |
| `+0x30` | índice do pulso (`0x43becd`) | 40 | 41 | 39 | 38 | 42 |
| `+0x34` | % do dano recebido aplicado | 100 | 70 | 60 | 40 | 50 |

Nos dados: variantes 0/1/2/3/4 em 11/20/20/13/1 objetos; vida 35 em 50 deles.

## Atualização (`0x0041de45`, por tique) [confirmado]

1. Morto (`+0x26c`) → nada. **Dano** (`0x41dfaf`): dificuldade (fácil ×2, difícil ÷2,
   mínimo 1), vezes `+0x34`/100, subtrai da vida. Vida ≤ 0 → `0x43d629(gerador, nulo, 0)` e
   destrói (não liga `0x40`; o objetivo "destruir" olha a vida).
2. Zera velocidades; procura/valida alvo (`0x41e073`); esquece vagas cujas minas morreram
   (`0x41e62d`).
3. Fora do estado 2 (abrindo), com alvo, estoque > 0 e alguma vaga livre (`0x41e5f7`):
   `+0x274−−`; ao chegar a 0: estoque −1, `+0x274 = iv + (rand % iv)/2 − iv/4` (iv =
   `+0x20`: entre 0,75·iv e 1,25·iv) e **abre** (estado 2, rótulo 2).
4. Toca o rótulo do estado: 1 → rótulo 1 `ohg00` (1 quadro, fechado); 2 → rótulo 2 `ohg01`
   (8 quadros, abrindo); −1 → rótulo 2 ao contrário (fechando). Rotina de fim `0x41e33b`.

## Fim de animação (`0x0041e33b`) [confirmado]

- Sem alvo → procura.
- Estado 2 (aberto): repete `+0x28` vezes "pôr uma mina na primeira vaga livre" (no máximo 4
  vivas por gerador); depois estado −1 (fecha). Outros estados → estado 1.

Criação da mina (`0x41e497(variante, &pos, &vel, camada)`): objeto novo (`0x433d80`);
variante da mina = a do gerador (0–3) ou, no "All", sorteada 0–3; inicialização da mina
(`0x41e6bc`); liga `0x10`; tipo 102; posição = `(x + 15, y + 8)` do gerador; velocidade
zero; camada do gerador; entra na lista; **escala 25** (`+0x1f0 = 25`, `+0x1f4` = medida do
quadro). Vida da mina: a padrão da variante (1–4), pois a fase não lhe deu vida.

Note que o estoque cai **um por abertura**, não por mina: um Beta pode soltar 3 minas por leva
e 40 levas.

## Percepção (`0x0041e073`, `0x0041e0ed`) [confirmado]

Mantém o alvo se ainda válido; senão percorre os jogadores vivos e fica com o válido **mais
próximo** (distância entre centros, `0x41e250`). Válido: mesma camada, vida > 0, distância
≤ `+0x14`, e linha livre entre os centros **só no Delta** (`+0x1c = 1`). Sem teste de
esconderijo. As outras variantes "veem" através das paredes.

## Pulso e sons

`0x41e596`: `0x43becd(+0x30, 64·sen(2π·((tique + endereço) mod +0x2c)/+0x2c))`, valor em
±64, como o das minas. `SOUND.TBL` tipo 103: quadro 2 (abrindo) → 3133,
prioridade 7 (nome fora do `SOUND.RES`).

## Destruição

Linha `(103, 0)`: efeito sorteado, raio 48, empurrão 15, dano de área 10 → `R = 72` px.
Bala da pistola: dano 2 (linha `(19, 0)`); a conta é inteira (`2 · % / 100`): um Alpha com
vida 35 cai com 18 tiros no normal; Beta, Gamma e All (70/60/50 %) tiram 1 por tiro (35
tiros); o **Delta (40 %) é imune à pistola** (`2 · 40 / 100 = 0`) e só cai com armas mais
fortes ou explosões. [inferido do cálculo]

## Em aberto

- `values[4..6]` (100/40/50, 40/30/20…, 5/10) variam nos dados mas são sobrescritos pela
  inicialização: resto do editor?
- Se a mina criada tem gatilhos/objetivos (não recebe nenhum).
- Nome do som 3133.
