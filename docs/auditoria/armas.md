# Auditoria M17: armas, tiro e reações a dano

Área "Armas, projéteis e reações a dano" (método em `README.md`). Projéteis, `DAMAGE.TBL` e
especial em [`armas-projeteis.md`](armas-projeteis.md). Fontes: `tiro-jake.md`, `armas.md`,
`armas-2.md`, `garathe.md`, `dano-vida.md` e o executável (endereços abaixo). Medição:
`tests/real_arsenal.rs` (tabelas lidas do `Hunter.exe` dos dados), testes do motor e o
**oráculo** (`tools/oraculo/`, sessões `T_J_001` pistola e `H_J_030` foguete, espingarda e
granada; entradas no fim). "orác." = medido no original no Wine.

Placar das duas páginas (linhas das tabelas): **igual 21**, **diferente → corrigido 27**,
**não implementado → corrigido 5**, **diferente em aberto 7**, **a confirmar 6**,
**decisão 2**.

## Tabelas e fluxo do tiro (`0x43b43e`, `0x43b50b`, `0x43b1d0`)

| Comportamento | Original | Porte (medido) | Veredito |
|---|---|---|---|
| 12 descritores (pronto, guardar, passo, projétil, cápsula, 10 estados, rotina própria) | `0x47df28…0x47e1a8`, `0x47e994…0x47ec14` | `real_arsenal`: idênticos | igual |
| 47 registros (variante, golpe, disparo, `+0x14/+0x18`, recuperação, bocas, velocidades, cápsulas) | `0x47d804…0x47e930` | `real_arsenal`: idênticos | igual |
| Listas de armas e registros do especial | `0x47df0c`, `0x47e978`, `0x47d7bc`, `0x47e228` | `real_arsenal` | igual |
| 1º tique do estado de tiro no tique do aperto | `0x43b43e` → `0x43b50b` (t = 1 já) | era 1 tique depois; orác.: bala no tique do aperto; agora igual | diferente → corrigido |
| Cadência segurando ATIRAR | recomeça e dispara no mesmo tique com `t > +0x14` | orác.: pistola a cada 6 (10,16,22…), granada t=8 no 4º tique; porte igual (espingarda 8, foguetes 24, estrelas/granadas 5) | diferente → corrigido |
| Desvio da rajada | `0x43bab7`: `(−vy, vx)·8/|v|` em `+0x164` (1/8 px), 1ª e 3ª | orác. v (−160, 8), (−160, 0), (−160, −8): porte idêntico; diagonal 5/8 (era 8/8) | diferente → corrigido |
| Soltar ATIRAR com direção, pulo ou troca | `0x43b591`: sai na hora, trava `+0x294 = 8` | orác.: `rec` 8→0 nos tiques 37–45, porte igual | não implementado → corrigido |
| Saída pelo repouso `+0x7c` decide já | `0x426ecc` → `0x423940` | orác.: corre (est 10) no tique da saída; porte igual | diferente → corrigido |
| Mira refeita a cada tique | `0x43b66f`: troca o estado sem zerar `t`, quadro já andado | orác. tiques 66–69 (56 → 23, `t` 4, q 3): porte igual | não implementado → corrigido |
| Animação de tiro | `0x423835` modo 0xa: uma vez, para no fim, `passo` quadros/tique | orác.: q 6,7,7,7 (pistola), 0,2,4… (granada); porte igual | diferente → corrigido |
| Sem munição | no tiro vazio da janela: 3090, melhor arma, "pronto" (`0x43b3b9`) | antes só no aperto seguinte; agora no tique vazio | diferente → corrigido |
| Modo soco e boca bloqueada | `0x47d7b8` por tique; linha centro→boca para **toda** arma, inclusive corpo a corpo | era só no começo e só armas de tiro; agora por tique | diferente → corrigido |
| Linha do disparo do chicote | `0x43b1d0` testa a linha inteira (a metade só no modo soco) | usava a metade | diferente → corrigido |
| Chicote: algo a 3/4 da linha | `0x43bcf2`: `t` e a animação andam um passo a mais | ausente | não implementado → corrigido |
| Janela do golpe | `0x43b7a7`: `+0x04 < t ≤ +0x08`, a cada tique (clava 2 acertos) | era `t = +0x04`, uma vez | diferente → corrigido |
| BAIXO + SOCO | `0x43ae4c` não distingue: especial | exigia ATIRAR | diferente → corrigido |
| Sons de arma | `SOUND.TBL` por quadro novo: estrela no 7º quadro; Garathe faca 3034, estrela 3110, chicote 3097/3098 (passo 2 pula quadros ímpares) | estrela no 1º quadro; Garathe sem som; agora por quadro | diferente → corrigido |
| Prioridade do som por quadro | 3 em `J141`, 5 em `G137` | 9 em todos | diferente, em aberto (menor) |
| Recuperação e guardar | atraso de quadro 1 (73, 112, 131, 188, 109, 128, 147, 187, 146); guardar de trás para a frente (`0x423894`) | orác.: 112 em q 0,0,1,1…; 109 entra no q 13; porte igual | diferente → corrigido |
| "Pronto" | `+0x14` = 10 tiques e guardar | orác. `c260` 10…1; porte 10 | igual |
| Interromper pronto/recuperação/guardar | `0x43b94e`: bits `0x1e0f` seguros (direções, pulo, furtivo, troca) → repouso | só direções e borda do pulo; agora `0x1e0f` | diferente → corrigido |
| Troca: borda, ordem, som 3090, melhor arma | `0x43b0cb`, `0x43a8cc` | orác. N 93/99: arma 2 → 6 | igual |
| Troca em qualquer estado | `0x422aa9` antes da máquina (salvo morrendo) | orác.: troca no 84; porte só controlável → agora sempre | diferente → corrigido |
| Pose da troca | `+0x26c = 7`, conta e aparece só no repouso; fora dele espera | orác.: 6 tiques (99–104; 54–59 depois do 84); porte igual | diferente → corrigido |
| Escolha direta W1–W6 | não existe | extensão do porte | decisão |
| Gasto de munição | 1 por disparo, da arma corrente (`0x40a401`) | orác. 80→79→78… idêntico | igual |
| Granada para cima (3, −20) | vx com o sinal do lado (`0x43b22b`) | idem | igual (era Ruling) |
| `desc+0x08` (149, 103, 113) | não lido | — | a confirmar |
| Tiro no ar e na barra | `0x42415c`, `0x4292d8` | área dos movimentos | a confirmar |

## Reações do Jake (`0x4231f9`)

| Comportamento | Original | Porte (medido) | Veredito |
|---|---|---|---|
| Derrubada: 81 ou 82 | `0x423657`: 81 se empurrado para o lado em que olha, senão 82 | orác.: olhando à esquerda, empurrão à direita → 82; porte dava 81 | diferente → corrigido |
| Voo da derrubada | rótulo uma vez (modo 0xa), decide só no fim (`0x425a70`, `0x425b1e`) | aterrissava ao tocar o chão (26) | diferente → corrigido |
| Levantar de trás | 82 → 83 (14) → 84 (18) → repouso, com decisão em pé no fim de cada um | orác. tiques 12–53: estados e tempos idênticos | não implementado → corrigido |
| 81 no fim | no chão aterrissa (`0x4243aa`); no ar cai | idem (estático) | diferente → corrigido |
| Morte derrubado | `+0x250 & 8` → 177 (`J077`), `0x10` → 179; 83/84 → 179 | idem | igual (lado agora certo) |
| Velocidade e dano da explosão no Jake | foguete na parede: 28 (14 vida + 14 armadura), empurrão (15, 7) | 21 (10 + 11), (10, −3) | diferente, em aberto (ver `armas-projeteis.md`) |

## Correções (commits em `audit/armas`)

- `90c5ed9` tabelas contra o `Hunter.exe` (`tests/real_arsenal.rs`).
- `cba4b96` fluxo do tiro: 1º tique imediato, trava, mira/arma/modo soco por tique, sem
  munição no tiro vazio, janela do golpe, chicote 3/4, desvio em 1/8 px, animação e sons
  por quadro, pose da troca (`jake/weapons/fire.rs`, 8 testes).
- `813683a` tiros entram depois da atualização (oráculo), efeitos no centro do quadro,
  saída decide em pé, mira nova no quadro andado.
- `9eabcec` derrubada 81/82 e levantar 83/84, poses com atraso e guardar invertido, troca
  em qualquer estado, chumbo e granada, linha da boca pixel a pixel.
- Último commit: especial (dano 3 e empurrão (±15, 20) nos jogadores, `oex01` nos
  projéteis destruídos) e chumbo pela linha `(20, …)`.

Testes de outras áreas ajustados à cadência fiel: `real_missions` (o piloto da `I_J_002`
segura ATIRAR, não entra no vão da escada e luta até o último robô), `real_ai` (a IA colada
repete o ataque anterior, ramo de perto de `attack`), `real_garathe` (garras sob o teto →
clava, abaixo), `scene/ai/tests.rs` (pistola recarregada).

## Em aberto

- **Garras na barra (`I_M_109`)**: encostado no gerador, a linha centro→origem cruza o
  gerador; pelo código (`0x43b556`) o golpe sai com a clava, que chega tarde. Confirmar no
  oráculo com o Garathe (o teste `claws_hit_…` agora espera a clava).
- Ordem da IA de alvo indefeso em `real_ai` (área da IA).

Entradas do oráculo: `T_J_001` `10*,20*S,6*,S,R,12*,12*S,4*,3*US,8*S,20*` (100 tiques);
`H_J_030` `10*,S,40*,N,10*,S,30*,N,5*,N,10*,S,40*,US,50*` (200 tiques). Depois das
correções, na `T_J_001` só `x` difere (atraso de um tique da corrida, área do núcleo); na
`H_J_030` estado, rótulo, arma e inventário iguais salvo os tiques da explosão.
