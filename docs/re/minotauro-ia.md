# IA dos personagens: Minotauro inimigo e Jake inimigo

Complementa [`garathe.md`](garathe.md) e [`garathe-movimentos.md`](garathe-movimentos.md).
**[confirmado]** = lido no código/dados; **[inferido]** = dedução.

## Quem é controlado pela IA [confirmado]

`0x42ec1c(obj)` devolve os bits de entrada (`tiro-jake.md`) de todo personagem. Usa o teclado
(`0x487a2c`) se `+0x19c ≠ 0`, se o objeto não está presente (`+0x178 & 0x80003 ≠ 0x80001`, ou
`+0x1a8 & 0x4972f8 = 0` sem `+0x178 & 0x10`) ou se `+0x1a8 & 1` (vaga de jogador vazia). O
resto (bit 0 de `modos` desligado, ex. `0xfe`) é IA. O código **não olha o tipo**: vale para
Garathe (2) **e Jake (1)**; a diferença está em limiares (`0x4888b0` = tipo do pensante).

Contagem nos dumps (`modos & 1 = 0`, `& 2 ≠ 0`):

| Fases | Minotauros IA | Jakes IA |
|---|---|---|
| `H_J_*` | 19 em 10 (001:1, 006:3, 007:3, 012:1, 013:1, 018:1, 027:1, 039:1, 042:2, 045:5) | 10 (010:6, 012:3, 033:1) |
| `D_J_*` | 8 (001:2, 007:1, 009:2, 015:3) | 3 (001:1, 009:2) |
| `H_M_*` | 1 (003) | 15 (003:1, 004:3, 021:5, 027:6) |
| `D_M_*` | 7 (010:1, 027:5, 030:1) | 21 (004:3, 006:3, 010:3, 018:2, 021:4, 024:6) |

Também em `C_*`/`M_M_033`. `fields[16] = 4` ("destruir"): Minotauros de `H_J_001/006/007/012`,
Jakes de `H_J_010/033` e de `H_M_*`. Inventário pelo `points` (armas 2–6); vida/armadura da fase.

## Estrutura da IA (`+0x2a0`, 0xe0 bytes, `0x45be60`) [confirmado]

| Desl. | Papel |
|---|---|
| `+0x00` | alvo (objeto) |
| `+0x04`/`+0x08` | classe do estado do alvo / própria (`0x42c7b6`, abaixo) |
| `+0x0c` | lista de perigos a < 100 px (`0x42cad4`, encadeada por `+0x20`, distância em `+0x24`) |
| `+0x10…+0x20` | células própria, acima, abaixo, esquerda, direita (`0x42cbfd`) |
| `+0x28` | melhor arma (`0x43a8cc(obj, 1)`); `+0x30` vida |
| `+0x34/+0x38`, `+0x3c/+0x40` | centros próprio e do alvo |
| `+0x44`, `+0x48` | distância; ângulo 16 bits (0 = direita, 0x4000 = baixo) |
| `+0x4c/+0x50` | dx, dy até o alvo; `+0x54` = vê |
| `+0x58` | cópia de `+0x220…+0x23f` (contato), restaurada no fim |
| `+0x78` | última entrada; `+0x7c` tique da última visão |
| `+0x80/84/88` | célula-alvo do caminho (x, y, camada; −1 = nenhuma); `+0x8c` direção; `+0x90` entrada do passo; `+0x94` espera |
| `+0x98` | modo: 1 caçar, 2 recuar, 3 vagar (0 vale 1) |
| `+0xdc` | **período de pensamento = 2**: nos tiques ímpares repete `+0x78` |

`+0x9c/+0xa0` (`0x42e938`, fuga por zonas) só são zerados: código morto.

## Percepção (`0x42d520`) [confirmado]

- **Alvo** (`0x42c5cc`, se não há ou morreu, `+0x178 & 0x8000`): os dois primeiros jogadores
  presentes; fica o mais próximo se estiver na mesma célula (`0x435224`) ou se a busca de
  caminho (`0x42f88c`, profundidade 10) o alcançar; senão o outro. **Sem limite de distância nem
  esconderijo.**
- **Vê** (`+0x54`): distância < **400** px, mesma camada e linha livre centro a centro
  (`0x42d475`: `0x433aa3`/`0x43735d`), ou tocando o alvo. No modo 3, alvo escondido (classe 10) ou
  morrendo (14) = não vê. Ao ver: `+0x7c` = tique e apaga o caminho.

Classes (`0x42c7b6`, estados Jake / Garathe): 1 parado (0,1,3,6–8,15,79 / 0–2,4,5,121); 2 pose
(4 / 76); 0xf especial (9 / 127); 3 andar/correr (2,10 / 3,10–12); 4 camada (11,12 / —); 5 subir
no pulo (16,17,27–29,31,80 / 19–22,24,25); 6 aterrissar/cair no chão (26,34,35,76–78,81–84,162,163
/ 23,26,28–33,35); 7 queda (25 / 27); 8 agachado (85–87 / 36–38); 9 empurrar (100–102 / 44–46);
**10 escondido (164–166 / 54–56)**; 11 barra (103–107 / 47,49–51); 12 escalar (30,32 / 52,53);
13 dano (168,169 / 58,59); 14 morrendo (171…181 / 61–69,80); 0x10 golpe/tiro; 0 outros.

## Decisão por modo (`0x42ec1c`) [confirmado]

Sem alvo → modo 3. **Modo 1** (`0x42ed64`):

1. Dano pendente > 10, ou dano com vida < 30 → modo 2 (`0x42f35b`).
2. Própria classe 2, 6 ou 0xf → entrada 0 (espera).
3. Vê: alvo escondido/morrendo → modo 3. Por classe própria: 5/7 → no ar, CIMA se o alvo
   está acima e `vy > −16` (`0x42e0f9`); 11 → barra (`0x42e05d`: alvo logo abaixo → PULAR+BAIXO;
   à altura → lado; senão BAIXO); 12 → escada/corda (`0x42e129`); 16 → só ataque; demais → ataque
   (`0x42e659`), senão aproximação (`0x42dd8f`).
4. Não vê, ou a aproximação falha → **caminho**: logo após perder de vista (2 pensamentos),
   50 % de pose (entrada 0xf, `0x42e8f3`); depois segue a busca de caminho até a célula do alvo;
   sem caminho espera `5·+0xdc` = 10 tiques, com 1/50 de pose e 1/1000 de passar ao modo 3.
5. Sempre por fim os perigos (`0x42e1a9`).

**Modo 2** (`0x42df9f`): corre para o lado contrário ao alvo; parede a 20 px ou buraco sem
pouso → volta ao modo 1. Sem ver → modo 1. **Modo 3** (`0x42f434`): com dano e alvo → modo 2;
alvo não escondido → modo 0 (= caçar; ver "Em aberto"). Senão vaga: direções de caminho
sorteadas 3/4 (esquerda/direita), sempre com **FURTIVO** (`0x400`, andar), 1/20 de pose;
na barra/escada solta (PULAR).

## Aproximação (`0x42dd8f`) [confirmado]

Só contra alvo de classe 1, 3–5, 7–9, 11–13, 15, 16. `|dy| < 64` (alvo pulando) vale 0.

- Mesma altura (`|dy| < 20`): perto (< 30 Jake, < 50 Garathe) e alvo andando (exceto
  empurrando) → segue o `vx` do alvo; parado → **recua andando** (FURTIVO, lado oposto) para ganhar
  espaço; parede a 20 px → inverte. Longe: `|dx| > 10` → corre para ele.
- Alvo bem acima (ângulo 0xbc00–0xc400) → PULAR+CIMA. Escalando: lado e CIMA/BAIXO pelo ângulo.
- Correndo para um lado, `0x42cf70` sonda o chão: chão à frente → segue; buraco com pouso a
  110/150/255 px (Jake, `0x478910…`) ou 156/350/225 px (Garathe, `0x47d128…`; `vx·(−2vy/g)`, g =
  2) → +PULAR (agachado: CIMA); sem pouso → PULAR+CIMA no lugar (não cai).

## Ataque (`0x42e659`) [confirmado]

Exige ver. `m` = 7/12 px (Jake/Garathe), `p` = 30/50 px.

| Distância | Ação |
|---|---|
| < `m` | nada |
| `m` … `(p−m)/2+m` (18/31) | repete a última entrada se tinha ATIRAR |
| até `p` | não bate em alvo que foge (sinal de dx = sinal do vx), salvo empurrando. Jake: arma 1 (soco). Garathe: **clava** se tem o par 2, senão garras |
| `p` … 200 | Garathe correndo (estado 10), d > 90, mesmo sentido do alvo, ângulo ±22,5° da horizontal e alvo correndo → **pulo longo** (lado+PULAR+CIMA, investida de 7). Senão melhor arma; Jake só com arma ≠ 1, Garathe só com arma ≥ 3 (facas, granadas, estrelas, chicote) |
| ≥ 200 | nada |

Mira (`0x42cca4`): perto (< `p`), setores largos sem "só BAIXO": direita, baixo-dir.
(0x800–0x4000), baixo-esq., esquerda, cima-esq., cima, cima-dir.; longe, só dispara alinhado a
±0x200 de 0, 0x2000, 0x6000, 0x8000, 0xa000, 0xc000 ou 0xe000. Logo **nunca usa o especial**.
Não atira nos estados (Jake) 9, 11, 12, 15, 25, 26, 76–78, 81–84, 87, 102, 162, 163, 166, 168,
169, 171, 173, 177, 179, 181; (Garathe) 11, 12, 26–33, 35, 38, 45, 55, 58, 59, 61, 63, 65, 67,
69, 80, 127.

## Perigos (`0x42e1a9`) [confirmado]

Célula própria com código `(c/10) % 100` ∈ {29, 30, 36, 37, 39} e bit 1 em `+0x40`: andando
para a borda → BAIXO [inferido: buraco/escada]. Depois, o objeto mais próximo da lista
`0x47d14c` a < 100 px (tipos 1–4, 7, 8, 10, 11, 39–42, 102–107, 109, 89, 19–21, 29, 36, 25–27,
99, 100, 108, 86, 68–72, 110):

| Tipo | Distância | Reação |
|---|---|---|
| 19–21, 25, 27, 29, 99, 100, 106, 108 (projéteis) | < 100, vindo para ele | BAIXO (agacha), se não rastejando |
| 26 granada, 42 fogo, 68–72 | < 100 | indo para ele → pula por cima; senão pula para um lado ao acaso |
| 39 pistão | < 95 | mantém PULAR/ATIRAR/SOCO; < 75 e abaixo dele → BAIXO |
| 40 cerca, 41 catapulta | < 50 | indo para ele → +PULAR+CIMA |
| 11 mina saltadora, 89 Manta, 109 formiga | < 50 | ATIRAR (+BAIXO se acima) virado para ele |

## Caminho (`0x42f88c`, `0x42f9a4`, `0x42d721`) [confirmado]

Busca em profundidade (≤ 10 passos, só se a distância Manhattan x+y+camada ≤ 10) a partir de
cada uma das 6 direções; devolve a primeira direção do menor caminho. Direção → célula:
1 y−1, 2 y+1, 3 x−1, 4 x+1, 5 camada−1, 6 camada+1. Permissões na **célula de origem**:

| Dir. | Teste | Bits (`célula+0x04`) |
|---|---|---|
| 1 subir | sem `0x40` e zona PR com bit 4 (corda/escada) | **`0x40` = fechada em cima** |
| 2 descer | sem `0x2` | **`0x2` = fechada embaixo** |
| 3 esquerda | sem `0x10`; destino sem portão (`célula+0x54` 1/2, `0x40f1f0`) | **`0x10` = parede à esquerda** |
| 4 direita | sem `0x8`; idem | **`0x8` = parede à direita** |
| 5/6 camada | zona PR bit 1 / bit 2 (portas) | — |

Passo (`0x42d721`): fixa a célula-alvo e `+0x94 = 10`; offset no centro da célula `o = c mod 128
− 64`. 1: centra (|ox| ≤ 15 Jake/25 Garathe) e CIMA; 2: centra e PULAR+BAIXO; 3/4: lado (+BAIXO
se `oy < −50/−40` fora da barra; +CIMA escalando) e a sonda de buraco; 5: CIMA; 6: BAIXO
(PULAR escalando). Passa da célula-alvo por mais de 1 → recomeça.

## Movimento, dano, morte e sons [confirmado]

A IA só gera bits; correr, pular, velocidades (Garathe 15 px/tique, pulos 13/−12 e 25/−14),
dano recebido (`0x4283b7`/`0x407862`, com armadura, **sem** fator de dificuldade), reações,
morte e cadáver são os do jogador. Não há sons próprios da IA; as falas de `0x40b9eb` só com
jogador.

## Em aberto

- Modo 3 com alvo visível põe `+0x98 = 0` (ramo sem desvio após `sub eax,4`, `0x42f479`) e
  devolve a entrada antiga nesse tique: erro do original? Efeito: volta a caçar.
- Significado dos códigos de célula 29/30/36/37/39 e de `célula+0x40`.
- `0x42f96a` lista as portas 50–52 (`0x4888cc`) para a busca, mas o uso não foi seguido.
