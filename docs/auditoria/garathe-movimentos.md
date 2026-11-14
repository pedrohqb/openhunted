# Auditoria: movimentos do Garathe (M17)

Tabela `0x478980 + 0x70·s`, rotinas em `docs/re/garathe-movimentos.md`. Porte medido nos
testes `jake/garathe/auditoria.rs` (sintéticos), `real_garathe.rs` e no `ohtool render
--trace-json`; oráculo em `T_M_002` (sessões "10*,…", start (86, 60)). **Núcleo** = diferença
do núcleo comum (máquina antes da física: o porte age um tique antes; velocidade guardada;
quadro de entrada nos rótulos; atrito), fora desta área (`audit/nucleo`).

Placar (49 linhas): **24 igual**, **18 corrigidas** (13 diferente + 5 não implementado),
**2 decisão**, **5 em aberto** (3 diferente, 2 a confirmar).

| comportamento | original | porte (medido) | oráculo | veredito | teste/commit |
|---|---|---|---|---|---|
| rótulos e atrasos dos 101 estados | `+0x10/+0x18` | `anim()` = tabela | — | igual (salvo 22/25) | `weapon_pose_delays…` |
| parado 1 | `0x42bc9c` | rótulo 1, atraso 1 | quadros 0,1,1,2… × 0,0,1,1 | igual (núcleo) | — |
| pose 76 com 0xf | `0x42bcbc` | 76 e fim → 1 | teclado (eixos) não gera 0xf | a confirmar | `four_directions…` |
| correr 10 | vx ±15·8, vy −8 (`0x42afa9`) | 15 px/tique | 14 à dir., 15 à esq. (vx 120) | igual (núcleo) | `runs_at_fifteen…` |
| andar 3 (103, k 3) | `0x42af30` | vx 113/89/137 | vx 120/96/144 | igual (núcleo) | idem |
| empurrar 44 → 5043 → 46 | `0x42a9a4`, ±8 | 44 q0–2, 46 atraso 1 | idem, x 12 | igual | `push_plays_44…` |
| 44: parede a 32 px; `+0x14c = 0` | `0x42a9ce`, `0x42a9f5` | parede a 4 px; x fica | — | a confirmar | — |
| estados 2 e 45 | sem entrada no executável | não existem | — | decisão | — |
| pulo médio 22 (13, −12) | `0x429e46`: `0x406c71` a cada tique | **2 tiques/quadro**: quicava no chão | q0–q6 em 7 tiques | diferente → corrigido | `medium_and_long…` c0fbe45 |
| pulo longo 25 (25, −14), preparo 24 | `0x429c6c`, `0x429bb0` | idem 22 | vx 200, vy −112 | diferente → corrigido | idem |
| investida 25 (7, tranco) | `0x429bda` | 7, tranco, centros | — | igual | `body_check…` |
| fala na parede 3734/3740; 5053 | `0x429f24` | iguais | — | igual | `mid_air_wall…` |
| pouso 23 (k 1) → rotina do parado | `0x42a0b6` | 1 tique parado antes | 6 tiques e corre | diferente → corrigido | `landings_23_and_26…` |
| pouso 26 decide em pé | `0x429fec` | não interrompível (9 tiques) | 1 tique e corre | diferente → corrigido | idem |
| 21 parado (k 3, 122 px; fim → 27 com vy) | `0x429817` | iguais | q6–q8 vy −240/−528/−96 | igual (teto: núcleo) | `standing_jump_ends…` |
| 21 → 20 com PULAR+CIMA (q < 10) | `0x4298cb` | mesmo quadro | q3 → 20 q3 | igual | — |
| 20 para cima (216 px, teto → 47) | `0x4295a8` | iguais | q13 vy −496 → 47 | igual | `jump_heights…` (real) |
| 19 da corda (q9 vy −30) | `0x4296df` | 210 px, até o chão | — | igual | `rope_jump_flies…` |
| ataque no ar 19/20/21/27 | `0x429719/604/876`, `0x4293ef` | não havia | 21+SOCO: 114, vy 0+16; solto → 27 | não impl. → corrigido | `attacks_from_the_jumps…` 1b264f5 |
| pouso segurando o ataque | `0x4292d8` (`+0x160`) | 28 | segue no 114 no chão | diferente (aberto) | — |
| queda 27: ±8 e lado pela entrada | `0x408205`, `0x427c9c` | lado não virava no ar | vx ±64, lado 8 | diferente → corrigido | `facing_follows…` 1b264f5 |
| pouso 28 (13 q, interrompível) | `0x4294fa` | iguais | 13 quadros | igual | — |
| 29→30→31, 35; 20 px livres | `0x42951f`, `0x429555` | iguais | — | igual | `hard_landing…` |
| agachar 37, rastejar 36 (k 5, 20 px), 38 | `0x42b26e`, `0x42b3f9` | iguais | vx 80/160/200; entra no q1 | igual (núcleo) | `crawls_with_k5…` |
| barra 47 → 50 → 49; BAIXO → 51 → 47 | `0x42a29b`, `0x42a8bb` | iguais | idem | igual | `bar_grab…` |
| 49 só com CIMA fica parado | `0x42a573` (`& 0xd`) | 51→47→50 em laço | fica no 49 | diferente → corrigido | `up_alone_holds…` |
| 49 andando: 2 quadros a cada 2 tiques | `0x42a746` | 1 quadro/2 tiques | ~1 q/tique, vx 48 | diferente → corrigido | `hand_over_hand…` fdab562 |
| 49 PULAR+BAIXO, PULAR → 51 | `0x42a573` | caía | 51 | diferente → corrigido | idem |
| ataque na barra | `0x42a568` | no ar | 114 no ar | igual | `bar_attack…` |
| corda/rede 52/53, camadas 11/12 | `0x42be1e`, `0x42b01e` | iguais (I_M_109 vence) | — | igual | `real_garathe.rs` |
| esconder 54 → 56 → 55 | `0x42abf2` | 2 q/tique | idem (entra no q1) | igual (núcleo) | `hides_in_54…` |
| saída: CIMA + 0x40 → 4 → `0x428cbc` | `0x428d2c` | a cena sai do parado (sem o 4) | — | diferente (aberto) | — |
| empurrão 58 | `0x428531`, `0x42b93e` | deslocava; sem decisão | — | diferente → corrigido | `shove_does_not_move…` |
| tranco 59 (63, k 2) → 35 | `0x42b9ad` | iguais | 59 q0–19 | igual | `reactions_and_deaths` |
| derrubada 32/33, choque 121 q17 | `0x4287a6`, `0x428713` | iguais | — | igual | idem |
| queda dos 58/59/121 | `0x4292d8` | não caíam | — | diferente → corrigido | `jolt_and_quake_fall…` |
| terremoto `+0x274` (`0x42bcf8`) | 121 do q0 enquanto dura; isentos | choque elétrico repetido | — | não impl. → corrigido | `quake_staggers…` |
| `0x42836d` | só sem gravidade em 1/10/44–46/76 | — | — | decisão (morto) | — |
| mortes 61/63/67/69 | `0x428863` | rótulos iguais | — | igual | `reactions_and_deaths` |
| morte adiada em 4/11/12/25/29/32/33 | `0x42890a` | morria no ar | — | diferente → corrigido | `death_waits…` |
| 28 com veneno → 63 | `0x428950` | 61 | — | diferente → corrigido | idem |
| deslize 61/63 (k 2); cadáver com vx 0 | `0x42bb62`, `0x42b604` | sem deslize | — | não impl. → corrigido | `deaths_61_and_63…` |
| caixa `MN_COLPS` (29–33, 35, 59, mortes) | `+0x01` | caixa padrão | — | não impl. → corrigido | `collapse_box…` |
| especial 127 (16 tiques, −1) | `0x42c348` | iguais | vida 99…92, q0–22 | igual | `special_costs…` |
| especial nos personagens | < 70: derrubada, ∓15/20, 4·(19600−d)/19600; 70–139: `+0x274 = 30` | (±10, −10), sem dano | — | não impl. → corrigido | `garathe_special…` |
| garras/clava; pronto 10 tiques | `0x43b50b` | 114, 110 | 114 q0–10, 110 | igual (x +1 a confirmar) | `claws_club…` |
| guardar toca ao contrário | `0x43ba0b` → `0x423894` | para a frente | 111 entra no q9 | diferente → corrigido | `holster_plays_backwards` |
| renascer 80 (22 q, sem dano) | `0x42b861` | direto ao parado | — | diferente (aberto) | — |
| sons 3734/3740/5053/5043 | | iguais | — | igual | testes acima |

## Correções

- `1b264f5`: ataque no ar; terremoto do Garathe (estado `Quake`, contador `+0x274`, a cena
  manda o abalo do brutamontes ao contador); especial do Garathe nos personagens (outros
  jogadores e IA: derrubada com dano, terremoto a 70–139 px); 58 sem deslocamento e
  interrompível; queda dos 58/59/121; `MN_COLPS` (`Profile::collapse_brd`); morte adiada,
  deslizante, 28 com veneno; lado no ar.
- `c0fbe45`: 22/25 a cada tique; 23/26 decidem; CIMA segura o 49; guardar ao contrário.
- `fdab562`: 49 com dois quadros a cada 2 tiques; 51 sem direção; renascer zera `+0x274`.

Jake: hash da cena por tique idêntico ao de `main` nas 41 fases do Jake (900 tiques de
entrada mista), salvo **D_J_015** (muda a partir do tique 294 pelos Minotauros da IA, que
usam a mesma máquina: pulos 22/25, pousos, barra, mortes, terremoto do especial). As
mudanças valem também para os Minotauros da IA em D_J_001/007/015 e H_J_*.

## Em aberto

- **Núcleo** (para `audit/nucleo`): correndo, o original desloca 14 px à direita e 15 à
  esquerda com vx 120: o atrito do chão (7) sai antes do deslocamento e a divisão é `sar`
  (113/8 → 14; −113/8 → −15). Nos estados pelo esqueleto (36, 3, 59, 63) o deslocamento é
  cerca de **metade** da vx guardada (vx 80 → 4 px), o que o porte não faz.
- Pouso segurando o ataque: o original segue no 114 no chão (`+0x160` em px depois da
  física?); o porte vai ao 28. Comum ao Jake; depende do núcleo.
- Saída pela porta sem o estado 4 (`MNRT2BK`) antes do `0x428cbc`; renascer 80 na
  disputa; 44 com parede a 32 px e o `+0x14c = 0`; o x + 1 ao golpear.
- Fora da área: a pose 0xf não sai do teclado no original (entrada); guardar ao contrário
  e o especial do Jake (`furtividade.md` lê ±15/20 também) para Jake/armas; recarga
  `+0x294` (armas); o especial da IA não é aplicado e o abalo não chega aos personagens
  da IA (IA/inimigos).
