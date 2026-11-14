# Auditoria M17: projéteis, `DAMAGE.TBL` e especial

Continuação de [`armas.md`](armas.md) (placar comum lá). Fontes: `projeteis.md`,
`armas-2.md`, `garathe.md`, `dano-vida.md`; rotinas citadas por endereço. "orác." = oráculo
(`T_J_001` pistola, `H_J_030` foguete/espingarda/granada).

## Projéteis do jogador

| Comportamento | Original | Porte (medido) | Veredito |
|---|---|---|---|
| Tique de criação | criado no meio da atualização do Jake: fica na boca até o tique seguinte | orác. bala (1671, 71) no tique do disparo; porte andava 20 px; agora tiros entram depois de `tick_entities` | diferente → corrigido |
| Bala 19: rapidez, desvio, alcance | 20 px/tique, sem vida, some fora da fase | orác.: 25 balas, trajetórias idênticas tique a tique (43 e 59 tiques) | igual |
| Bala: rótulo pela direção | `0x40bff8` (1/2/3, espelho) | orác. rot 1/3 iguais | igual |
| Faísca `oex15` (raio 5) e 3046 | centro do **quadro** do atacante (`0x43d90c` → `0x43585c`); bit 2: x na vítima ∓ 4 | era o centro da caixa (12 px de erro); agora (1753, 70) contra (1754, 69) | diferente → corrigido |
| Duração dos efeitos | criado no meio da atualização: 8 tiques para 7 quadros | 7 tiques | diferente, em aberto (ordem de criação dos objetos, área do núcleo) |
| Efeito pela vítima | `oex11/12` no jogador, `oex14` no tipo 8 (linhas 84–86) | sempre `oex15` | diferente, em aberto |
| Cápsula 22 / cartucho 23 | boca e (±10, −10); some lenta (`0x40c9f9`) | orác.: trajetória igual até a parede; rebote −4 vs −5 px/tique e some antes (21 vs 26 tiques; cartucho 32 vs 50) | diferente, em aberto (resposta da física: atrito nos dois eixos, `personagem.md` §8.3) |
| Chumbo 20: 1º tique só arma | `0x40cfac`: geração 1, espera 2, tamanho 10 | orác.: divide no 3º tique depois do disparo (62 → 65); porte dividia no 2º; agora igual | diferente → corrigido |
| Chumbo: velocidade dos filhos | `cos·|v| >> 17` em px inteiros (para baixo), ×8 | orác.: (−160, 0) → (−160, −16) e (−160, 8); porte idêntico (era simétrico em 1/8 px) | diferente → corrigido |
| Chumbo: filhos encolhem | `+0x1f0 = +0x274/2 + 1`; `+0x274 = 3/4 + 1` (10 → 6 → 5) | não encolhia | não implementado → corrigido |
| Chumbo no alvo | linha `(20, 0)`: 10, explosão, derrubada módulo 20 **na direção centro→vítima** | era fixo (±20, 0); agora pela linha (`pellet_row_pushes_along_the_centres`) | diferente → corrigido |
| Foguete 21: criação e voo | boca (−25, 4), 30 px/tique, reto | orác. (35, 969) v −240; porte igual; explode no tique seguinte na parede, igual | igual |
| Foguete: linha `(21, 0)` | 20 direto, raio 64 (sorteio `oex01–04`), empurrão 30, área 50 (`R = 96`) | tabela igual; rótulo sorteado diverge (gerador) | igual |
| Dano de área no Jake | `0x40d499`: centro da caixa da explosão; vítima pelo **quadro** (`0x43585c`) | caixa do Jake: foguete 21 (orác. 28), empurrão (10, −3) (orác. (15, 7)); granada 14 (orác. 13) | diferente, em aberto (tentado o centro do quadro do sprite: piora com o espelho; falta a origem exata do quadro) |
| Granada 26: lançamento | t = 8, boca (−25, 2), (±20, 0), rótulo 2 sem espelho | orác. (97, 967) v −160, `esp` 0; porte espelhava; agora `esp` 0 | diferente → corrigido |
| Granada: quique | resposta comum da física: 20 → 8 px/tique na parede, +11 → −4 no chão; espelho invertido e 3042 | porte forçava 20 → 20 e zerava o quique do chão; agora a física comum (10 e −5) | diferente → corrigido em parte |
| Granada: animação | rótulo 2 de trás para a frente (q 11, 10, 9…), parada sem vx | para a frente | diferente, em aberto (visual) |
| Granada: pavio 35 + `rand % 10`, objetos que atravessa | `0x40d7ab`, `0x40d85a` | orác. 41 tiques; lista igual | igual |
| Estrela 27: onda, crava 100 tiques, devolve 1 | `0x40bc98` | testes do motor; sem sessão do oráculo | a confirmar |
| Faca 25 e chicote 108 (Garathe) | `0x40ba54`, `0x40cb6c` (gerações, encolhe 10 → 6 → 5 → 4) | testes do motor; chicote com o mesmo arredondamento px do chumbo | a confirmar |

## Balas inimigas e outros projéteis

| Comportamento | Original | Porte | Veredito |
|---|---|---|---|
| 99/100/106 (morte, Bruiser, sentinela), 29 dardo, 36 laser | linhas 90–108 do `DAMAGE.TBL` | as linhas valem (tabela conferida); comportamento nas áreas dos inimigos | a confirmar (inimigos) |

## `DAMAGE.TBL` e vida

| Comportamento | Original | Porte | Veredito |
|---|---|---|---|
| 149 registros | `0x43d514` | `real_damage`: idênticos ao arquivo | igual |
| Primeira linha que casa (vítima 0/1/tipo/< 0, variante) | `0x43d648` | `damage::row` | igual |
| Reação: derrubada (fixo com lado, ou vetor reescalado), empurrão, tranco | `0x43d6c5…0x43d8d2` | `Row::pending` | igual |
| Armadura absorve metade arredondada para cima, impede o letal | `0x407862` | orác.: 28 → 14 vida + 14 armadura; porte igual na fórmula | igual |
| Dificuldade só nos inimigos | `opcoes-salvamento.md` | fora desta área | decisão (fora) |

## Especial (BAIXO+ATIRAR)

| Comportamento | Original | Porte | Veredito |
|---|---|---|---|
| Jake: janela 1..5, 3 de vida/tique, pisca 0x27 | `0x47d7bc`, `0x427643` | igual (registro conferido) | igual |
| Alcance: centros < 140 e ação < 70 | `0x427791`, `0x4277f8` | < 70 | igual |
| Tipos destruídos (19–21, 25–27, 29, 36, 99, 100, 106, 108) | `0x42779d` | igual | igual |
| Projétil destruído: linha `(1, 0, 8)` | efeito `oex01` raio 5 no centro do atacante | sumia sem efeito; agora o efeito (`special_destroys_a_bullet_with_an_effect`) | diferente → corrigido |
| Demais: linha `(1, 0, 9)` 15 com empurrão (2, 0) | `0x4278a3` | igual | igual |
| Jogadores a < 70 px | derrubada, dano `4·(19600 − d)/19600` (3; 4 só em d = 0), empurrão (±15, 20) pelo centro, crédito | era (±10, −10) sem dano (Ruling); agora igual (`special_knockdown_hurts_three_…`) | diferente → corrigido |
| Garathe: 5..20, 1/tique; terremoto 70–139 (`+0x274 = 30`) | `0x47e228`, `0x42c348` | janela conferida; terremoto não medido | a confirmar |
