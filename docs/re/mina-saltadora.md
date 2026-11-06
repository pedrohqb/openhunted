# Mina saltadora (tipo 11, `ohs.anm`)

Nome no executável: **"Mine"** (`0x472990`). 157 objetos em 13 fases da campanha (`D_J`,
`H_J`, `I_J`), todos com vida 10, `values[5] = 4`, `values[7] = 5`, colisão `ohs0.BRD`,
sinalizadores `0x80801` (sólida; 13 com `0x180801`). Bichinho verde de 20 × 22 px que pula pelo
chão e explode ao encostar em algo. Não confundir com a mina flutuante (tipo 102,
`minas.md`).

## Inicialização (`0x0041bdb8`) [confirmado]

- Genérica `0x40b2c8`; `+0x1c0 = 0x41bed9`; nome "Mine"; animação `ohs.anm`; colisão
  `ohs0.BRD` (ponteiro `0x472964`, cache `0x48882c`).
- `+0x178`: desliga `0x80`, `0x20`, `0x10`; liga `0x801` → **sólida, com gravidade**.
- Estado `+0x180 = 2`; `+0x1b4 = 100`; vida 0 → 10; `+0x174 = 10`.
- Zera alvo `+0x268`, `+0x260`, morta `+0x278`, sentido `+0x264` (0 = esquerda, 1 = direita) e
  "no ar" `+0x26c`; `+0x27c = 5` (pulinhos até o pulo grande).

## Estados e rótulos [confirmado]

`0x41be8a(obj, estado, rótulo)` como no tipo 10 (guarda o estado anterior em `+0x260`;
estado −1 toca ao contrário; colisão sempre `ohs0.BRD`).

| Estado | Rótulo | Papel |
|---|---|---|
| 1 | 1 `ohs00` (10 quadros) | pulo grande |
| 2 | 2 `ohs01` (8) | parada |
| 3 | 3 `ohs02` (11) | pulinho |
| 4 / −1 | 4 `ohs03` (7) / ao contrário | virar-se |

## Atualização (`0x0041bed9`, por tique) [confirmado]

1. Morta (`+0x278`) → nada. Fora da fase ou sem célula em `(x/128, y/128, camada)` →
   `+0x278 = 1` e destruída.
2. **Contato e dano** (`0x41c49a`): o objeto tocado (`0x4076d0`) zera a vida se o tipo dele
   **não** for 6, 9, 11, 22–25, 30, 46, 50–52 ou 56 — logo o Jake, o Garathe, balas,
   inimigos, barris etc. a detonam ao tocar; outra mina saltadora, grafite, cápsulas,
   dicas, teletransporte, portas `OH` e plataformas `OPL` não. Depois subtrai o dano
   pendente. Vida ≤ 0 → `0x43d629(mina, tocado, 0)`, destruída, liga `0x40`.
3. No ar (`+0x26c`) → só repete a verificação de aterrissagem (`0x41c093`).
4. Senão toca o rótulo do estado com a rotina de fim `0x41c093`.
5. Tocando um barril (tipo 28) → zera velocidade e velocidade pedida.

## Fim de animação / aterrissagem (`0x0041c093`) [confirmado]

- Fora do estado 2, zera vx. Se `vy > 7` (1/8 px: ainda caindo), marca `+0x26c = 1` e espera.
- Senão `+0x26c = 0`, procura alvo (`0x41c233`). Depois de virar (estado −1 ou 4) inverte o
  espelho (`0x434815(obj, 1)`).
- **Com alvo válido**: se `+0x27c > 0`, `+0x27c−−` e **pulinho** (estado 3, `0x41c5aa`:
  `vx = ±(4 + rand%4 − 2)` px/tique, `vy = −8` px/tique); senão `+0x27c = 5` e **pulo
  grande** (estado 1, `0x41c543`: `vx = ±(9 + rand%6 − 3)`, `vy = −10 + rand%4 − 2`).
  O sinal de vx é o sentido `+0x264` (1 → direita).
- **Sem alvo**, sorteio `rand % 100`:
  - < 33 → **vira** (estado 4, ou −1 se `+0x264 = 1`), pulinho vertical `vy = −7`
    (`0x41c5f9`) e `+0x264 = 1 − +0x264`;
  - 33–65 → parada (estado 2);
  - ≥ 66 → anda como com alvo (5 pulinhos de 2–5 px/tique, depois um pulo de 6–11 px/tique).

A mina não persegue: com alvo, segue pulando no sentido em que está — e só vê para a frente.

## Percepção [confirmado]

`0x41c233`: entre os jogadores vivos, o válido de menor `|dx|`. Válido (`0x41c2a2`): vida
> 0, `|dy| ≤ 50`, mesma camada, `|dx|` **≤ 450**, **do lado do sentido** (`+0x264 = 1` →
alvo à direita ou na mesma x; 0 → à esquerda) e linha livre entre os centros. Sem teste de
esconderijo.

## Explosão e dano [confirmado]

Linha `(11, 0)` do `DAMAGE.TBL`: **20 de dano direto** em quem a tocou (sem reação), efeito
sorteado com raio 12, empurrão 20 e dano de área 20 → `R = 18` px. O Jake que encosta leva
20 + até 20 de área. Atirando, nada atinge o Jake se ele está a mais de 18 px.

- Bala da pistola: o contato com um tipo 19 já zera a vida (passo 2).
- Soco do Jake: linha `(1, 11)` dano 8, empurrão 2, som 3038 "Club to Flesh" — mas o contato
  do Jake também zera a vida; o efeito é a explosão.

## Sons [confirmado no `SOUND.TBL`]

Tipo 11: quadro 1 (pulo grande) 3014; quadro 10 (parada) 3010, prioridade 5; quadro 20
(pulinho) 3011; quadro 29 (virar) 3012. Nomes fora do `SOUND.RES`.

## Em aberto

- Nomes dos sons 3010–3014; `values[5] = 4` e `values[7] = 5` (não lidos aqui; `+0x27c` é
  reposto a 5 pela inicialização).
- O que significa o bit `0x100000` de 13 objetos.
- Se a gravidade/atrito da física zera vx no chão entre os pulos (o código só zera fora do
  estado 2).
