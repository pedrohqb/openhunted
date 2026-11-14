# Auditoria M17: movimentos do Jake

Tabela de estados `0x473300 + 0x70·s` (BRD `+0x01`, rótulo `+0x10`, atraso `+0x18`) lida
do executável e conferida com `docs/re/jake-movimentos.md`; rotinas em `/tmp/hunter.asm`.
**Oráculo**: o original no Wine (`oraculo.md`), entradas em `T_J_001` (corredor plano,
x 1696, parede à direita em x ≈ 1770), `T_J_003` (corda) e `T_J_005`; porte com
`ohtool render --trace-json`, posições comparadas com `--defasagem 1` (a máquina de
estados do original roda depois da física: área do núcleo). **Porte**: medido em
`crates/openhunted-engine/tests/real_jake_moves.rs` (o Jake real da T_J_005 em cenários
sintéticos). Correções e pendências em [`jake-movimentos-2.md`](jake-movimentos-2.md).

Placar (61 linhas): **igual 17 · diferente/não implementado corrigidos 29 · diferente de
outra área 10** (física 4, núcleo 4, armas 1, Bruiser 1) **· não implementado 3 · decisão 1
· a confirmar 1**. Na coluna Veredito, "física", "núcleo" e "outra área" são `diferente`
deixados para a área dona; "corrigido" é `diferente`/`não implementado` resolvido aqui.

| Comportamento | Original (endereço / oráculo) | Porte (medido) | Veredito |
|---|---|---|---|
| Rótulos e BRDs da tabela | 197 registros; `+0x10` como no RE | `jake_anim` | igual |
| Parado 1 (atraso 1) | 2 tiques/quadro; oráculo 1/0, 1/1, 1/1… | 1/0, 1/0, 1/1… | núcleo (quadro 0) |
| Parado com punhos 6 (rót. 146) | oponente a < 40 px (`0x42dc43`) | sempre 1 | não impl. |
| Pose 4 (entrada = 0xf) | teclado: eixo R−L = 0, inalcançável (oráculo: `in_jogo` 0) | idem (`DeviceState`) | igual |
| Correr 10 | vx ±96, vy −8 (`0x426296`); oráculo 12 px/tique à esq., **11 à dir.** | 12 e 12 | física |
| Duas setas (motor) | ESQ tem prioridade (`0x422a62`); teclado nunca as dá juntas | a mais recente | decisão |
| Furtivo 2 (rót. 149, k = 3) | Δ 0,3,9,15,9,9,9,9,12; oráculo anda 0,2,5,8,5,5,5,5,6 | 3,9,15,9… | física |
| Engatinhar 86 (k = 2) | 106 px/ciclo; oráculo ≈ metade (5,8,4,3…) | 10,16,8,6… | física |
| Agachar 85 (2 quadros/tique) | oráculo: entra no q1, 86 no 3º tique | q0, 86 no 4º | núcleo |
| Levantar 87 | oráculo 85/4…0, parado no 6º tique | igual | igual |
| Esconder 164/165/166 | oráculo 164 q1,3,5 → 165; 166 = 164 ao contrário | +1 tique no 164; 166 igual | núcleo |
| Pulo parado 27 | 26 quadros, vy −18 no q9, pico 43 px (oráculo 69→26) | igual (def. 1) | igual |
| PULAR segurado | `0x200` é o botão (`0x407f4b`); oráculo: 27 a cada 26 tiques | era a borda | corrigido |
| Controle no ar no chão (27/16) | `0x408205` sai no chão: sem deslize | deslizava 8 px/tique | corrigido |
| Controle no ar, limite | compara com o deslocamento real `+0x15c`; oráculo: 8 px a cada 2 tiques | 8 px todo tique | corrigido |
| Pulo para cima 16 | k = 3, pico 79 px; teto `+0x220 = 2` → 103 (oráculo y 69→56→16→8) | igual; pendura 1 px abaixo | igual (1 px: física) |
| Pulo da corda 17 | parado sem gravidade até q9, vy −20 (`0x424ee2`); oráculo: 10 tiques, −18…, pico 90, 26 no chão | esqueleto, parado no ar | corrigido |
| Ataque no ar 16/17/25/27 | `0x43b43e` (`0x424ce3`, `0x424df3`, `0x424263`, `0x424f89`) | não atacava | corrigido |
| Correndo, PULAR sem seta | escolha (`0x423f4d`); oráculo 29, vx −80 | 27 | corrigido |
| PULAR+CIMA+lado | longo 28 (`0x423f35`); oráculo 28 (80, −120) | escolha (29) | corrigido |
| Pulo médio 29 | (10, −11), 9 q, pico 25 px | igual (def. 1) | igual |
| Pulo longo 28 | (10, −15), pico 49 (oráculo 69→20) | igual (def. 1) | igual |
| 28/29: fim do rótulo | param no último quadro (oráculo 29/8 até o pouso) | reiniciavam o rótulo | corrigido |
| 28/29: chão só no fim | `0x42557d`/`0x42572a` (degrau no meio do voo) | pousava na hora | corrigido |
| 28/29: parede no fim | som 3712 + queda 25 (`0x4255a8`) | não tinha | corrigido |
| 28/29: teto no fim | pose 1 (`0x4255ef`) | não tinha | corrigido |
| Ricochete na parede | oráculo vx −29 | −40 | física |
| Escolha do pulo | faixa ½ largura × 3 px em y+h+1, a ¼ + alcance (`0x425144`) | caixa inteira, 8 px, exige vão livre | corrigido |
| Impulso 31 → 80 | 0, 16 px; (17, −15) (`0x425382`) | igual (testes M6b) | igual |
| 80: contato no ar | pose 1 (`0x4254c7`) | seguia até 76 | corrigido |
| 80 fim no ar → 76; chão → 77 | `0x4254ab`, `0x425481` | igual | igual |
| Queda 25 | vy ≥ 5 px/tique, isenções (`0x42415c`) | igual | igual |
| 25: controle no ar | `0x42424d` → `0x408205`; lado segue a seta | só invertia vx | corrigido |
| Aterrissagem 26 | decisão, senão 11 q → parado; oráculo 26/0… | igual | igual |
| Aterrissagens 35/34 | 2 quadros/tique; oráculo entra no q1: 35 dura 1 tique, 34 4 | 2 e 5 tiques | núcleo |
| 77 → 78/79/86 | sonda de 25 px (`0x4243fc`, `0x424428`) | 30 px | corrigido |
| Beirada 162/163 | 77 com o chão sem cobrir a caixa (`0x407d09`) | não tem | não impl. |
| Queda descontrolada 76 | corda; 77 no chão | igual | igual |
| Derrubada 81/82 | 81 se empurrado para onde olha (`0x42365e`); fora de 11/12 | pelo sinal do empurrão | corrigido |
| 81: fim | chão → 77; ar → 76 (`0x425ab6`) | 26 a qualquer momento | corrigido |
| 82 → 83 → 84 | `0x425b99`, `0x425bdf` | 26 | corrigido |
| Tranco 169 → 84 | `0x4233e3` (estados da tabela), corda → 25; oráculo 169 ×16, 84 ×18 | só o Garathe | corrigido |
| Empurrado 168 | só em 1, 6, 79, 164–166 (`0x423366`) | 1, 10, 100 | corrigido |
| Choque 15 | 9 tiques, quadro 0 parado (`0x4235c4` consome o tique) | animava | corrigido |
| Cambalear `+0x274` | 15 com atraso 1 (`0x426f6a`) | o terremoto vira choque | outra área (Bruiser) |
| Mortes 173/177/179/171 | 177 (rót. 77) se empurrado para onde olha | pelo sinal | corrigido |
| Empurrar 100/101/102 | oráculo: 100 ×6, laço 101 (5 q), 102 ao contrário ×6; vx 32; PULAR ignorado; som 5043 | laço do 100, pulava, vx 96, sem som | corrigido |
| Corda: agarrar em pé / no ar | 3 → 30; no ar CIMA/BAIXO na zona 4 | igual | igual |
| Escalar 30/32 | 42 px/13 q (oráculo 212→…→170 em 13 tiques); lado 50/7 | igual (k = 2) | igual |
| Sair pelo alto da corda | 8 e a decisão `0x423b2b`: CIMA na zona → 30 (oráculo) | 8 → 3 → queda | corrigido |
| SOCO na corda solta | `0x40100` (`0x427161`) | só ATIRAR | corrigido |
| Barra 103–107 | `jake-escalada.md` | testes M6b/I_M_109 | igual |
| Camadas 11/12 (contador `+0x268`) | `escada-camadas.md` | testes M6b | igual |
| Especial 9 | 5 tiques; fim sem tiro → pose pronta (oráculo 9 → 18 ×10 → 19) | 9 → parado | corrigido |
| Atrasos de pose (146, 147, 109…) | `+0x18 = 1` (oráculo: 147 dura 16 tiques) | 0 | corrigido |
| 147 ao contrário, 150 parado no fim | oráculo 147/7…0; 150/5 segura | para a frente; laço | outra área (armas) |
| Caixa padrão | `JK_DEFLT` (`0x422878`), mesmo com `JK_EXTND` no objeto (M_2_048) | BRD do objeto | corrigido |
| Caixas `JK_EXTND`/`JK_COLPS` | 76/77/162; 78, 81–84, 163, 169–179 | caixa padrão | não impl. |
| Sons de estado | só 3712 (28/29) e 5043 (empurrar) no código do Jake | — | corrigido |
| Entrar na saída 13, teletransporte | `logica-fase.md`, `itens` | testes | igual |
| Ordem do desenho (`+0x1b4`) | 0x28/0x6e/0x78/0x79 | sem campo | a confirmar |
