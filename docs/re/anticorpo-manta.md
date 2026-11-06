# Anticorpo (tipo 105, `OAB.ANM`) e Manta (tipo 89, `OMR.ANM`)

Dois voadores com o **mesmo esqueleto de código** (a Manta é uma cópia do Anticorpo com
outro ataque e sons): nomes "Alpha/Beta/Gamma/Delta Antibody" (`0x4720c0`) e "… Manta"
(`0x473180`). Anticorpos: 121 objetos em 14 fases da campanha; Mantas: 36 em 4. Todos com vida
1 e sinalizadores `0x4001` nos dados (o leitor liga `0x80000`).

## Inicialização (`0x004110b8` / `0x0042127c`) [confirmado]

- Genérica; animação/colisão padrão `OAB.ANM`/`OAB.BRD` ou `OMR.ANM`/`OMR.BRD`;
  `+0x1c0 = 0x41129d` / `0x421475`. Variante `values[0]`: < 0 → 0; ≥ 4 → 3; nome dela.
- `+0x178 = 0x81 | 0x4000` (apaga tudo antes): sólido para colisão de objetos, personagem,
  rótulo inicial pelo `+0x1ec`; sem `0x8` (**com gravidade**, compensada pela velocidade
  pedida a cada tique [inferido]). `+0x174 = 1`.
- Vida 0 → `+0x18` da variante (1). "Casa" `(+0x268, +0x264)` = posição inicial; ângulo
  `+0x274` sorteado (16 bits); `+0x27a`, `+0x272` sorteados 0/1; modo `+0x270 = 0`;
  descanso `+0x279 = rand % r + r` (r = `+0x1c`).
- Anticorpo: estado 1, `+0x1c4 = 0x412204` (pulso de cor). Manta: estado/rótulo 3, ordem 123.
- `values[6]` cai nos bytes `+0x278…`: `+0x278 = 1` é "vivo"; com 0 o objeto é destruído no
  primeiro tique.

## Variantes (registros de 0x48 bytes) [confirmado nos números; papéis em parte inferidos]

| Desl. | Uso | Alpha | Beta | Gamma | Delta |
|---|---|---|---|---|---|
| `+0x14` | alcance de percepção (px, centros) | 200 | 150 | 200 | 250 |
| `+0x18` | vida padrão | 1 | 1 | 1 | 1 |
| `+0x1c` | descanso base (tiques: `rand%r + r`) | 1 | 5 | 1 | 10 |
| `+0x20` | % (+ `rand%5`) de voltar a descansar | 0 | 10 | 0 | 10 |
| `+0x24` | % por tique de atacar, com alvo | 2 | 3 | 2 | 2 |
| `+0x28` | ricochetes no passeio | 2 | 3 | 2 | 1 |
| `+0x2c` | fator de velocidade do passeio | 8 | 3 | 8 | 15 |
| `+0x30` | duração máxima de cada modo (tiques) | 50 | 40 | 50 | 30 |
| `+0x34` | velocidade do mergulho (1/8 px) | 60 | 40 | 60 | 80 |
| `+0x3c`/`+0x40` | cor fixa: `0x43becd(+0x3c, +0x40)` a cada tique (só o Anticorpo) | 44/−45 | 37/−54 | 43/−17 | 42/14 |
| `+0x44` | % do dano recebido: Anticorpo / Manta | 100 / 100 | 100 / 70 | 100 / 50 | 100 / 40 |

Anticorpo: tabela `0x4720c0`; Manta: `0x473180`. `+0x38` (50/30/50/20) não foi seguido.

## Comportamento (`0x0041129d`, `0x00421475`) [confirmado na estrutura]

1. Fora da fase ou sem célula → vida 0. Contato com parede (`+0x220 & 2`) → para.
2. Dano: Anticorpo vida −= dano·`+0x44`/100; Manta igual, depois da dificuldade (fácil ×2,
   difícil ÷2; `0x42231b`). Vida ≤ 0 → `0x43d629(obj, nulo, v)` (Anticorpo v = 4: efeito
   `oex` raio 15 sem dano; Manta v = 10: efeito 14, raio 12), destruído, liga `0x40`. Uma bala
   (2) basta.
3. Modos (`+0x270`):
   - **0, descanso**: velocidade zero; no quadro 0, 30 % de tocar um som (Anticorpo 3072/3073
     "Antibody", a cada ≥ 250 ms; Manta 3161 "manta on wall"/3163 "manta on wall2"). Passado
     o descanso: com `(+0x20 + rand%5)` % descansa de novo; senão, com alvo e `+0x24` % por
     tique, **ataca**: modo 2 (mergulho direto, `+0x272 = 1`) ou modo 3 (persegue pelo lado
     do alvo); sem alvo → recomeça o passeio (`0x42250b`: nova casa, ângulo sorteado).
   - **1, passeio**: anda na direção `+0x274` (`(cos, sen)·+0x2c`), até `+0x30` tiques ou
     `+0x28` ricochetes (`0x42221b`).
   - **2, mergulho**: `0x421fd6` leva-o ao alvo; ao chegar, modo 4.
   - **3, perseguição**: até `+0x30` tiques; a menos de **40 px** (centros, `0x4225ab`) do
     alvo → estado/rótulo 4 (bote).
   - **4/5, bote e retorno**: `0x4221b0` volta à casa; acabado, recomeça o passeio.
4. vx limitado a −32…37 (1/8 px).

Percepção (`0x421c69` na Manta): jogador vivo, mesma camada, distância ≤ `+0x14`, linha livre
(`0x433aa3`/`0x43735d`); fica o mais próximo. Sem teste de esconderijo.

## Ataque [confirmado]

No fim do rótulo 4 (bote):

- Anticorpo (`0x412249`): som **3071** "Antibody" posicional e `0x43d629(anticorpo, alvo,
  variante)`.
- Manta (`0x422416`): põe-se momentaneamente na posição do alvo e chama `0x43d629(manta,
  alvo, variante)`; depois estado 5 → 1.

Linhas do `DAMAGE.TBL`: `(105|89, 0, v)` com **natureza `0x10`**, dano **15 / 20 / 25 / 30**
(Alpha…Delta); o Anticorpo sem reação, a Manta com empurrão (reação 2) e efeito 11 (raio 12).
Soco do Jake: linhas `(1, 105)`/`(1, 89)` dano 8, som 3038.

## Rótulos

`OAB.ANM`: 1 `oab00` (5 quadros), 2 `oab01` (8). `OMR.ANM`: 1 `omr00` (10), 2 `omr01` (12),
3 `omr02` (12), 4 `omr03` (5, bote), 5 `omr04` (5). Manta `SOUND.TBL` (tipo 89): quadro 0 →
3161, 12 → 3162, 37 → 3160.

## Em aberto

- O que a natureza `0x10` faz na vítima (também usada pelo Camaleão, tipo 4); não aparece em
  `0x4231f9` como lido em `dano-vida.md`.
- Detalhes de `0x421fd6`, `0x42221b`, `0x4221b0` (trajetórias) e o campo `+0x38`.
- A rotina de criação logo após `0x4225ab` (`0x4226ae…`) cria um Jake (vida 100, armadura
  50): pertence ao renascimento, não à Manta.
