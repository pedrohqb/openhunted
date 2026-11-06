# Itens e objetos comuns da campanha do Jake

Complementa [`itens.md`](itens.md) (coleta genérica `0x40b3f5`, aceitação `0x40b6ed`, tabela de
vida/armadura) e [`armas.md`](armas.md) (`0x409fe8`, munição). Contagens nas fases `D_J`,
`H_J`, `I_J`, `T_J`.

**Atenção aos dados**: quase todos os itens ficam na lista de objetos **de cada célula**
(`cells[i].objects` no JSON), não na lista da fase; só os objetos "ativos" (inimigos,
barris, interruptores, zonas…) estão na lista de cima. [confirmado nos JSON]

## Coletáveis (inicialização genérica `0x40b2c8`, coleta `0x40b3f5`) [confirmado]

`v` = `values[0]`. Bits de `+0x178`: `0x100` o Jake pega, `0x200` o Garathe pega.

| Tipo | ANM | Qtd. (fases) | Quem | Efeito na coleta | `v` típico |
|---|---|---|---|---|---|
| 32 | `OHE00` | 244 (39) | ambos | vida +25 (recusa com ≥ 100) | — |
| 34 | `OFO00` | 97 (33) | Jake | vida +100 (recusa com ≥ 199) | — |
| 16 | `OAC00` | 124 (33) | Jake | pistola +3·v | 25 → 75 balas |
| 17 | `OSS00` | 139 (23) | Jake | espingarda +v (cartuchos) | 10 (85×), 20, 25 |
| 13 | `OSG00` | 23 (20) | Jake | dá a espingarda + v | 10, 20, 25 |
| 18 | `ORK00` | 73 (20) | Jake | foguetes +v | 3 (31×), 5, 8, 10 |
| 14 | `ORL00` | 25 (24) | Jake | dá o lança-foguetes + v | 0 ou 5 |
| 12 | `OGN00` | 25 (22) | Jake | dá a pistola + 3·v | 0–40 |
| 26 | `OGR` | 75 (30) | ambos | granadas +v | 1–5 |
| 27 | `ONS` | 35 (15) | ambos | estrelas +v | 5, 10 |
| 37 | `OAV00` | 49 (20) | ambos | **antídoto** (zera o veneno; recusa sem veneno) | — |
| 33 / 35 | `OPH00` / `OPF00` | 11 (5) / 5 (5) | ambos | **veneno** +1 em `+0x270` (perde 1 a cada 20 tiques, `dano-vida.md`) | — |
| 76 / 77 / 78 / 79 | `OAR` (rótulos 1–4) | 30 / 33 / 34 / 32 | ambos | armadura +50 / +100 / +20 / +25 | — |
| 44, 45, 15 | `OWH00`, `OWA`, `OCL00` | 6, 39, 5 | **só o Garathe** (`0x2a9`/`0x2a1`) | armas do Garathe (`armas.md`) | — |

Munição: `0x40a36d` aplica a dificuldade (fácil ×1,5; difícil ×0,75) e limita a 99; a coleta
é recusada com 99. Os de vida/antídoto/veneno usam a tabela `0x471d10` (`itens.md`).

Em solo e cooperativo os itens **não renascem**: `0x412fec` (cria um objeto tipo 43
"Regenerate", `org.anm`) sai logo se o modo é 2 (solo) ou cooperativo. [confirmado]

Os tipos 26 e 27 colocados na fase são itens (estado 1 da granada, estados 4–5 da
estrela) com o mesmo tipo dos projéteis (`armas-2.md`): `0x40bc38` põe a estrela no
estado/rótulo 4, escala 11. 

## Chaves (tipos 62–67, `OK.ANM`) [confirmado]

Inicialização `0x40f214`: `+0x178 |= 0xa1` (sem gravidade, só contato), ordem 30, rótulo =
tipo (`ok1`…`ok6`, 10–13 quadros, quadro inicial sorteado), rotina `0x40f301` = coleta
genérica + laço. Na coleta, par `tipo − 49` (13–18): liga o "tem" e soma 1; som 5060 "Pickup
flag or key". Na campanha: 62 ×13, 63 ×23, 64 ×15, 65 ×5, 66 ×2, 67 ×1.

Uso (Jake, `0x425d7c`, ao passar ao estado 101 [inferido: encostar na porta]) sobre uma zona
PR com o bit `0x20`: monta a
máscara `bit i = (quantidade do par 13+i ≠ 0)` e chama `0x40f040(célula, máscara, 1)`: a
porta da célula (estado `+0x54 = 1`, fechada) com chaves exigidas (`+0x48 ≠ 0`) abre se
`exigidas & máscara ≠ 0` — **basta uma** das chaves pedidas — e os gatilhos da porta
(`+0xa0`, 2 entradas) dão verdadeiro; senão o Jake fala (3715 com 3714 ou 3719; o Garathe
3730 com 3732 ou 3741). Abre por `0x40eed4(célula, 3)`. A chave **não é gasta** [inferido:
nenhuma escrita no par]. A trapaça `0x49738c` dispensa as chaves.

## Bandeiras (tipos 80–85, `OFL.ANM`) [confirmado]

Rótulo = tipo (`ofl01`…`ofl06`); par `tipo − 73` (7–12) no inventário; som 5060. Todas as
17 da campanha têm **objetivo 2 ("acionar")**: o item coletado vira `+0x178 = 0x40`, o que o
veredito (`logica-fase.md`) lê como "acionado". Logo **é preciso recolher todas** para a
saída aceitar.

## Pistão (tipo 39, `OPP.ANM`) [confirmado]

32 objetos em 18 fases. `0x4106ec`: rótulo/estado 1 (`opp00`, 10 quadros, 174 × 25), quadro
0, `+0x178 |= 0x28` (sem gravidade, só contato), ordem 70, caixa = retângulo do quadro;
`+0x1c0 = 0x4107c8`, `+0x1c8 = 0x4108c0`.

- Parado (`+0x27c = 0`), sem espera (`+0x278`) e com gatilhos **> 0** (sem gatilhos dá −1 e
  ele **nunca** anda): parte para a frente (+1) se está no quadro 0, senão para trás (−1).
- Andando: quadro += sentido por tique; no quadro 0 ou no último, para e espera `values[0]`
  tiques (10 em 19 objetos).
- Enquanto anda, `0x4108c0` testa a sua caixa de colisão (2 px mais baixa) contra o
  retângulo de cada objeto presente na camada e chama `0x43d629(pistão, obj, 0)` com vx =
  ±sentido: linhas `(39, 2)` e `(39, 0)`: **derrubada, 35 de dano**, empurrão fixo (54 ou
  48, −3). Esmaga o Jake a cada tique de contato.
- Sons `SOUND.TBL` tipo 39: quadros 1/9 → 3058, 2/8 → 3057; a rotina só deixa tocar na metade
  do curso no sentido do movimento (`+0x1a4`).

## Interruptor (tipo 74, `OSW.ANM`) [confirmado; ver `logica-fase.md`]

64 objetos em 19 fases. É o mesmo "Toggle Switch" da saída, sem o objetivo: o Jake o aciona
com CIMA no estado 3 sobre uma zona PR `0x40` (`0x40e4dd`), invertendo `0x40`; o estado é lido
pelos **gatilhos** de outros objetos (Bruiser adormecido, pistões, geradores de dardos,
catapultas, portas). `values[0]` = começa ligado (6 dos 64); `values[1] ≠ 0` = pode voltar
(40) — senão aciona uma vez só (24). Rótulos `osw01`…`osw07` (3 quadros) e `osw08` (12, a
saída).

## Tipo 0 [confirmado]

22 objetos em 12 fases, sem animação nem colisão, sinalizadores `0x80000` (bit 0 desligado:
fora de colisão e de varreduras), id de gatilho 0. A tabela de tipos tem o par `(0, nulo)`:
**nenhuma inicialização**. Restos do editor (um traz `EDITOR.INI` em outras fases) [inferido].

## Em aberto

- Nomes das falas 3714/3715/3719/3730/3732/3741 (fora do `SOUND.RES`).
- Estrutura completa da porta de célula (`+0x48`, `+0x54`, `+0xa0`) e `0x40eed4`.
- Se alguma fase da campanha usa portas com chaves que o Jake não encontra (conferir
  máscaras `+0x48` contra as chaves presentes).
