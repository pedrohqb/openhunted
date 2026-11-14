# Auditoria M17: itens, objetos de fase e colisões de objetos

Método em [`README.md`](README.md). Original: endereço do `Hunter.exe` e, quando medido, o
oráculo (`tools/oraculo/`, [`oraculo.md`](oraculo.md)). Porte: medido por teste (unitário ou
`tests/real_objects.rs`) ou por `ohtool render --trace-json` + `compare.py`.

**Placar** (33 linhas): igual 19 · diferente 6 (3 corrigidos) · não implementado 5 (4
corrigidos) · decisão 1 · a confirmar 2.

## Inventário (as 100 fases do `LEVELS.VOL`, objetos da fase e das células)

`tipo:instâncias/fases`. Fora de inimigos e jogadores: 0:33/23 · 5:100/100 · 6:215/55 ·
9:15/15 · 12:68/46 · 13:64/50 · 14:59/52 · 15:44/36 · 16:206/58 · 17:224/51 · 18:133/44 ·
25:56/28 · 26:233/80 · 27:141/55 · 28:589/72 · 30:32/12 · 32:590/94 · 33:25/16 · 34:259/73
· 35:16/13 · 37:163/57 · 39:74/44 · 40:51/20 · 41:135/30 · 42:162/43 · 44:67/45 · 45:150/46
· 46:218/58 · 50:479/75 · 51:136/41 · 52:1/1 · 56:170/30 · 62–67: 25/41/25/9/4/5 ·
68:83/33 · 69:1/1 · 70:116/44 · 72:37/14 · 73:388/79 · 74:189/55 · 76–79: 97/96/82/82 ·
80–85: 11/11/7/2/1/2 · 102:408/37 · 104:318/64. Os itens (12–37, 44, 45, 62–67, 76–85)
estão quase todos nas listas das células. O 71 e o 43 só nascem em jogo. Tipos sem
comportamento nos dados: só o **0** (par `(0, nulo)` em `0x47d47c`) e um **27 sem ANM/BRD**
(`T_M_008`, (431, 413)): `0x40bc38` usa a animação da fase, nula → invisível e sem caixa,
nos dois. A tabela de tipos (73 pares) foi conferida: todos os tipos das fases têm rotina.

## Comportamentos

| comportamento | original | porte (medido) | veredito | teste/commit |
|---|---|---|---|---|
| tipo 0 | sem inicialização, `0x80000` | `Static`, sem caixa | igual | `inv` |
| 56 `OPL` | `0x40e60c`: `OPL.BRD` com rótulo ≤ 6, senão `OPL07.BRD`; `&~0x20`, `|8`, rotina vazia | BRD de `names[1]`: os 170 batem com a regra | igual | inventário |
| 12–18, 25–27, 44, 45 coleta | `0x40b6ed`/`0x409fe8`: teto 99, dificuldade ×1,5/×0,75, "tem" | idem | igual | `pickup::weapons_*`, `ammo_*` |
| 32/34/76–79/33/35/37 efeito | tabela `0x471d10` | idem | igual | `pickup::health_*`, `poison_and_antidote` |
| fala da aceitação | `0x40b6ed`: vida/comida/antídoto 3700 (o outro 3735 ou 3738, `rand & 1`); veneno 3717/3737 | não falava | diferente → corrigido | 3315f90 |
| quem não pode pegar | aceitação (com fala) antes do bit 0x100/0x200 (`0x40b49a`) | não falava | diferente → corrigido | 3315f90 |
| item-segredo (`+0x1ac & 1`, 22 nas fases) | som 3093 no lugar do da coleta e fala 3718/3731 | som de coleta | diferente → corrigido | 3315f90 |
| vida tocada por jogador morto | aceita e some sem curar (`0x40b57b` exige vida > 0) | morto não está ativo: nada | decisão | — |
| quadro inicial dos itens | `0x40b2c8`: `+0x184 = rand % n` (oráculo: sorteado) | quadro 0 | diferente | núcleo (RNG) |
| queda sem 0x8 | física genérica | `pickup::tick` cai contra o mundo | igual | `pistol_falls_*` |
| chaves 62–67, bandeiras 80–85 | `0x40f214` (`|0xa1`, par `tipo−49`), som 5060; bandeira = objetivo 2 | idem | igual | `campaign_ammo_keys_and_flags` |
| peça do carro 9, grafite 6, dica 30, zona 73, saída 5, interruptor 74 | `objetos-tutorial.md`, `logica-fase.md` | idem | igual | testes dos módulos |
| pistão 39, teletransporte 46, cerca 40, fogo 42, catapulta 41 | `itens-campanha.md`, `barril-fogo-cerca.md`, `objetos-tutorial.md` | idem | igual | testes dos módulos |
| minas 102, olho 104, portas 50–52, portas de célula 38 | `minas.md`, `olho-laser.md`, `objetos-tutorial.md`, `itens-campanha.md` | idem | igual | testes dos módulos |
| renascer 43 (disputa) | `0x412fec`: 300 tiques, `org.anm` | `players::schedule_regen` | igual | `scene/players/tests.rs` |
| itens pelos personagens da IA | `0x40b3f5` aceita qualquer tipo 1/2 com jogador | a IA não apanha | não implementado | área IA |
| 68/69 poça | `0x40fa16`: a cada `values[0]` tiques, veneno 1, `+0x270 = 1`, fala | idem | igual | `opo::poisons_*` |
| 70 goteja | `0x40f46b`: `values[7]` tiques; gota em `x + rand % (l − 8)`, `y + a/2`; rearma `rand%(v/4+1) + v − v/8`; liga 0x20 | não gotejava | não impl. → corrigido | ce8be5a |
| 71 gota | `0x40f60a`: rótulo 71 (4 q.), cai `+17` com gravidade (171), respinga (271, 5 q.) e some; fere o jogador (linha 71, veneno) | idem; **oráculo `I_J_104`**: y 54,56,60,66,74…, respingo q0–4, some no 6.º tique — igual | não impl. → corrigido | ce8be5a, `the_drop_*` |
| escala da gota | `8 / largura` (`0x447ff0` devolve x, y, **largura**, altura) | 8/13 | igual | `docs/re/objetos.md` dizia altura |
| 72 vapor | `0x40f801`: estados 1–4, `values[1..4]`, gatilhos (4 entradas), sons 3168/3170, bit 0x40, teste fora do estado 2 com o retângulo do rótulo atual (5 de veneno) | parado em 272 | não impl. → corrigido | ce8be5a |
| 72 medido | **oráculo `D_J_012`**, 80 tiques, os 2 vapores: rótulo e quadro iguais em todos os tiques | idem | igual | `steam_*` |
| 72 tique da troca 2→3 | `+0x184 = 7` (quadro 56, fora do rótulo 172) | mesmo quadro 56; o rastro do porte o lê como 272/0 | igual (desenho) | — |
| 28 barril com `values[0] ≠ 0` (117) | `0x40fb58(&pos, camada, 384)`: fogo no chão até paredes/±192, células fechadas embaixo (bit 2), fogos ≤ 256 px | não criava | não impl. → corrigido | e150e7c |
| 28 barril (batida, cadeia, empurrão) | `0x40dc00` | idem | igual | `barrel::*`, `interact::*` |
| gota × personagem da IA | contato de objeto sólido → respinga e fere | a gota atravessa (a IA não está no mundo da física) | diferente | em aberto |
| gota × barril/objeto sólido | respinga, linha 71 | respinga; perigo 1 px em volta | igual | `fall`/`splash` |
| barril × barril, × porta `OH`, × `OPL` | batida > 80 explode | idem | igual | `barrel::*` |
| itens sobre pistão | pistão só contato (`|0x28`): nada pousa | idem | igual | — |
| contato só-de-toque (0x20) | não barra; contato tipo 5 | itens não barram | igual | `objects::blocks_flags` |
| ativação por distância | ver abaixo | todos ativos | diferente | núcleo |
| barril-fogo no oráculo | — | — | a confirmar | sem fase com barril-fogo perto do início |
| fogo do barril: ordem (175/55) e espelho | sorteados | espelho sim; ordem de desenho não | a confirmar | — |

## Achado transversal: ativação por distância (`0x435e7f`)

Medido no oráculo (`I_J_104`, `--todos`): os 10 vapores, 3 das 6 saliências, 3 OPL e os
pistões ficam com `+0x178` sem `0x80000` e **não atualizam** (vapor parado em 272 por 120
tiques; no porte cicla). A cada tique, para cada vista: objeto com `0x100000` ou dos tipos
1, 2, 19–21, 36, 42, 50–52, 71, 86, 99, 100, 106, 108 → raio ∞; 10, 11, 28, 29, 40, 89, 102,
104, 105, 107, 109 → **400 px**; os demais (e > 109) → **600 px** (tabela de bytes
`0x435efb`, saltos `0x435f69`). Raio finito e camada fora de {vista, vista + 1} e não
desenhada (`0x4879e4`) → inativo. Ativo se `(x − cx)² + (y − cy)² < r²` (posição do
objeto, centro da vista). Só objetos ativos têm física, colisão e rotina (`0x40460e`). O
porte atualiza tudo: muda o tempo de vapores, pistões, minas, barris e inimigos longe da
vista. Fica para o núcleo (afeta todas as áreas).

## Correções

- ce8be5a: gotas (71) e vapor (72) em `behaviour/opo.rs`; o 70 e o 72 em `entity.rs`.
- 3315f90: falas da aceitação e item-segredo em `behaviour/pickup.rs`.
- e150e7c: `fire::floor_fires` + `Scene::spawn_floor_fires`; `Out::floor_fires`.

## Em aberto

- Ativação por distância (acima) e quadro inicial sorteado: núcleo.
- Itens apanhados pela IA; a gota que cai sobre um personagem da IA.
- Contagem de segredos achados (`Stats::secrets`): quem soma não foi lido (`0x436358`?).
- Oráculo, `D_J_012` t0: o Jake começa com vida 99 e veneno 1 e o tipo 3 (Morte) salta para
  a posição dele no t1; no porte, vida 100 e veneno 0 (área de inimigos).
