# Garathe: estados e movimento

Complementa [`garathe.md`](garathe.md). Física, entrada, zonas PR e sondas são as do Jake
(`personagem.md`, `jake-movimentos.md`, `jake-escalada.md`, `ske.md`); aqui, o que difere.
**[confirmado]** = lido no código/dados; **[inferido]** = dedução.

## Tabela de estados `0x478980 + 0x70·s` (164 linhas) [confirmado]

Mesmo layout do Jake: `+0x01` BRD, `+0x10` rótulo do `MINOTAUR.ANM`, `+0x14` b (0 = pelo
esqueleto), `+0x18` atraso, `+0x1c` área do BRD; nome do grupo em `0x478930 + 0x70·s`.
`0x428302(obj, s)` troca (zera `+0x2a4` salvo com `0x471984`). Atalhos de animação:
`0x4289da` (uma vez, modo 10, com continuação), `0x428a39` (ao contrário, 14), `0x428a9f`
(laço, 9), todos a cada `atraso + 1` tiques.

Formato: estado: rótulo/quadros [BRD ≠ DEFLT] [atraso] grupo — rotina.

1: 1/47 c1 `MNROOTLO` — `0x42bc9c` (parado) · 2: 2 b0 — `0x42bddd` · 3: 103/13 `MNH2H1B`
— `0x42af30` (andar) · 4: 4/5 `MNRT2BK` — `0x42bd9d` (virar de costas) · 5: 4 ao contrário —
`0x42bdf9` · 10: 10/9 `JKRUN2LO` — `0x42afa9` (correr) · 11, 12: 11, 12 — `0x42b01e`,
`0x42b143` (camadas) · 13: 12 — `0x42b575` (sair pela porta) · 19: 19/15 `MNJUMP1` —
`0x4296df` · 20: 20/15 `MNJUMP1` — `0x4295a8` · 21: 21/9 `MNJUMP2` — `0x429817` · 22: 22/7 c1
`JMPMED` — `0x429e46` · 23: 23/3 c1 — `0x42a0b6` · 24: 24/3 c1 `MNJUMP4` — `0x429bb0` ·
25: 25/3 c1 — `0x429c6c` · 26: 24 c2 — `0x429fec` · 27: 27/1 `MNFALL1` — `0x4293c2` · 28:
28/13 — `0x4294fa` · 29: 29/7 COLPS `MNFALL2L` — `0x42946b` · 30: 30/9 COLPS `MNFALL3` —
`0x42951f` · 31: 31/30 COLPS — `0x429555` · 32: 32/4 COLPS c1 `BLASTFWD` — `0x42a115` · 33:
33/4 COLPS c1 `BLASTBK0` — `0x42a1b3` · 35: 35/31 COLPS `MNBLGTUP` — `0x42a248` · 36: 36/21
CRAWL `MNCRAWL` — `0x42b3f9` · 37: 37/5 CRAWL `MNRT2CRL` — `0x42b26e` · 38: 37 CRAWL —
`0x42b31b` · 44: 44/3 `PUSHHAND` — `0x42a9a4` · 45: 44 — `0x42aa5d` · 46: 46/24 c1 —
`0x42aa74` · 47: 47/1 `SWINGBAR` — `0x42a29b` · 49: 49/42 BARUP c1 `MNBRCRW0` — `0x42a4af` ·
50: 50/7 BARUP — `0x42a7c2` · 51: 50 BARUP — `0x42a8bb` · 52: 52/13 `MNCLIMB1`, 53: 53/7
`MNCLIMB2` — `0x42be1e` · 54: 54/11 `MNHIDE` — `0x42abf2` · 55: 54 — `0x42acf1` · 56: 56/1
sem BRD — `0x42ad84` · 58: 58/7 `MNDAMAGE` — `0x42b93e` · 59: 63/27 COLPS — `0x42b9ad` · 61:
61/46 COLPS `MNDIE1` · 63: 63/27 `MNDIE2B` · 65: 63 · 67: 67/8 `MNFALL3` · 69: 69/19 `MNDIE3`
(`0x42bc0a`, `0x42bae8`, `0x42bb79`, `0x42ba34`, `0x42ba8e`) · 76: 76/36 — `0x42bcbc` · 80:
80/22 — `0x42b861` (renascer) · 121: 121/22 c1 `MNQUAKE` — `0x42bd73` · 127: 127/28 `MNVICT1`
— `0x43b895` (especial).

Armas (rotinas comuns): pronto `0x43b99b` 81, 100, 110, 113, 128, 140, 152; guardar
`0x43ba0b` 82, 101, 111, 129, 141, 153; recuperação `0x43ba54` 85, 130, 142, 154; golpe/tiro
`0x43b50b` 95–98, 104–109, 114, 116, 118, 135–138, 147–150, 159–162; 103 → `0x43b43e`.
Os demais (115, 117, 119 e os sem rótulo) caem no parado. Despacho: bytes `0x427d42`
(163 entradas), alvos `0x427de5`.

## Tique (`0x00427b68`) [confirmado]

Igual ao do Jake (`personagem.md` §5.1): célula inexistente, compensação zerada a cada
`atraso+1`, entrada, `+0x178 |= 0x4000`, `+0x1c8 = 0`, lado. Sem `0x200000`: recarga
`+0x294`, troca de arma `0x43b0cb`, depois, na ordem: dano `0x4283b7`, queda `0x4292d8`,
terremoto `0x42bcf8`, fim do choque `0x42876e`, terremoto em 1/10/44–46/76 `0x42836d`.

Queda (`0x4292d8`): gravidade ligada e `+0x160 ≥ 40` → 27. Isentos 19–22, 25, 27, 29, 32,
33; nos de golpe/tiro (95–98, 104–109, 114, 116, 118, 135–138, 147–150, 159–162) só cai sem
ATIRAR/SOCO (com, e no chão, aterrissa).

**Espelhados** (fim do tique nega `+0x16c` e `+0x164`, espelhamento ao contrário): 3, 81, 82,
85, 95–98, 100, 101, 103–111, 113, 114, 116, 118, 128–130, 135–138, 140–142, 147–150,
152–154, 159–162.

## Decisão em pé (`0x00428ae8`) [confirmado]

1. Entrada = 0xf → 76 (pose; no fim, 1).
2. ATIRAR/SOCO: com `+0x294 = 0` ataca (`0x43b43e`); **com recarga, devolve 0 sem testar o
   resto**.
3. ESQ/DIR: sonda ±4 px; parede → empurrar 44; FURTIVO → **andar 3**; senão correr 10.
4. Sem PULAR, CIMA/BAIXO com zonas: CIMA + zona 1 + camada > 0 → 12 (`+0x268 = 7`); BAIXO +
   zona 2 + camada < máx. → 11 (`+0x268 = 0`); CIMA + zona 4 (ou 0x40 com jogador) → 4.
   Sem zona: BAIXO → 37 (agachar); CIMA → 54 (esconder).
5. PULAR: com CIMA → 20; senão 21.

## Correr (10) e andar (3) [confirmado]

- **Correr**: `vx = ±15 px/tique` (`0x47d144`), `vy += −1` por tique (`0x47d148`); soltar a
  direção zera e para. Testes `0x428f56`: 0xf → 76; parede no tique → 44; ataque para e
  ataca; FURTIVO + lado → 3; zonas como acima, mas CIMA + zona 4 → direto para os testes de
  costas (`0x42bddd`: a escalada); PULAR + CIMA + lado → longo (25); PULAR + CIMA → 20;
  PULAR → escolha (`0x429a0b`); BAIXO → 37.
- **Andar** (3, rótulo 103 = pose de luta): velocidade pelo esqueleto, k = 3, só x.
  Testes `0x429104`: soltar FURTIVO → 10 com lado, senão parado; ataque congela a posição.

## Costas, porta e camadas [confirmado]

4 toca `MNRT2BK` e passa a `0x428cbc`: zona 0x40 + jogador → `0x40e4dd`: −1 → 13 (entra:
`0x200008`, 10 tiques, encolhe `+0x1f0` até 30, colisão nula); 0 → choque de 20 (elétrico),
sons 3772 e 3135; senão 5 (desvira, rótulo 4 ao contrário). Zona 4 + CIMA → escalar.
11/12: `+0x268` conta 0→7 (camada +1 em 3) ou 7→−1 (camada −1 em 4), animação em laço, zera
`+0x274/+0x284`; CIMA/BAIXO contrários invertem no meio; fim → parado.

## Pulos [confirmado salvo indicação]

| Pulo | Estado | vx, vy (px/tique) | Constantes | Alcance |
|---|---|---|---|---|
| médio | 22 | 13, −12 | `0x47d128/0x47d12c` | 156 px |
| longo | 24 → 25 | 25, −14 | `0x47d130/0x47d134` | 350 px |
| terceiro | — | 15, −15 | `0x47d138/0x47d13c` | 225 (só a IA, `0x42d2xx`) |

- Escolha `0x429a0b` (alcances calculados uma vez, `0x47d140`, em `0x4888a4/a8/ac`): sonda
  como a do Jake; chão ao alcance do médio → 22; senão do longo → 24 (preparo, uma vez) →
  25; senão 22. **Não há pulo estendido.**
- 22/25: fora dos estados 52/53, sem chão a 4 px → queda 27. No ar, uma vez a animação; no
  fim: chão → 23 (médio; desliza k = 1) ou 26 (longo; 24 ao contrário, k = 1, som 5053);
  parede → som 3734/3740 e 27; outro contato → 1. Inverter vx no ar como no Jake.
- **Investida no pulo longo**: em 25, `+0x1cc = 0x00429bda`: quem ele toca (objeto, não
  só-toque) recebe tranco (`+0x250 = 1`), **7 de dano** e empurrão por `0x435951`; volta a
  `0x407796` no fim.
- 20 (PULAR+CIMA): exige chão; ataque zera `+0x168`; até o quadro 10 PULAR + lado → 25 (com
  CIMA) ou escolha; teto (`+0x220 = 2`) → barra 47; zona 4 → escalar; esqueleto k = 2 nos
  dois eixos; fim no chão → 28; controle no ar `0x408205`.
- 21 (PULAR parado): igual, k = 3; até o quadro 10 PULAR + CIMA sem lado → 20 no mesmo
  quadro (rótulo `0x479250`); fim → 1 e queda.
- 19 (PULAR+CIMA na corda): sem gravidade e parado até o quadro 9; então `vy = −30` px/tique
  e gravidade.

## Queda, derrubada e levantar [confirmado]

27: chão → 28 (aterrissagem interrompível, depois parado); ataque; zona 4 → escalar;
controle no ar; desliga 0x8 e 0x4000. 32/33: v = 8·empurrão, uma vez; 32 → 30 no chão ou 29
(queda descontrolada → 30); 33 → 35 no chão. 30 (`0x42951f`): com 20 px livres acima
(`0x40718d(obj, 0, −20)`), decisão em pé; sem eles só anima; fim → 31. 31 e 35 (`0x429555`,
`0x42a248`): teto a menos de 20 px ou BAIXO → 36; senão decisão em pé; fim → parado.

## Agachar, rastejar, empurrar [confirmado]

- 37 (BAIXO): dois quadros por tique → 36; CIMA → 38 (levantar); ataque e pulos de pé.
- 36: ataque e PULAR só com **20 px** livres (Jake 30); soltar BAIXO livre → 38; lado com 4
  px livres → esqueleto **k = 5** só x; parado → só a compensação.
- 38: lado → 10; BAIXO + zona 2 → 11 (com `+0x268 = 7`); BAIXO → 37; fim → parado.
- 44: testes `0x428edb` (ataque, lado oposto ou nenhum → para); sem parede a 32 px ou sem chão
  a 4 px → **`+0x14c = 0`** e `vx = 0` (`0x42a9f5`, provável erro: zera o x da posição) e
  parado; senão `vx = ±8`, toca 44 e, no fim, som 5043 → 46. 46 (laço, atraso 1): BAIXO →
  37; zona 0x20 na entrada → `0x40f040(célula, máscara +0x94…+0xbc, 0)`; `vx = ±8`, `vy =
  0`. 45 = 44 ao contrário → parado.

## Barra, corda, esconder [confirmado]

- 47 (pendurado; sem gravidade, `+0x1b4 = 0x6e`): sem teto a 4 px → 27; PULAR+BAIXO
  exatos, BAIXO ou PULAR → solta; ataque solta e ataca; CIMA/lado → 50 (agarrar, uma vez) →
  49. 49 (mão ante mão): lado com 4 px livres e teto contínuo (linha de 4 px acima, x ±3) →
  anima dois passos e esqueleto **k = 6** em x; sem teto → 27; sem direção → 51 (50 ao
  contrário) → 47.
- 52/53 (`0x42be1e`; sem gravidade, `+0x1b4 = 0x28`): sai da zona 4 ou ataca → 5. PULAR:
  com lado → longo/escolha; com CIMA → 19; senão 5. Sondas de 4 px: CIMA → 52, k = 2 (y);
  CIMA contra teto → 47; BAIXO → 52 ao contrário; lado → 53 k = 2 (x); diagonais → 52 com
  `vx = ±vy`; nada → parado no quadro.
- 54 (CIMA sem zona; `+0x1b4 = 0x28`, dois quadros por tique) → 56 (escondido: sem
  gravidade, parado; soltar CIMA → 55); 55 = 54 ao contrário → parado (CIMA → 54). Ataque,
  PULAR e lado saem como no Jake 164–166. São os estados que o tipo 8 ignora.

## Velocidade pelo esqueleto [confirmado na fórmula; px inferidos]

`px = k · trunc(Δraiz · S/K)`, com **S = 39 × 58** e **K = 28 × 50**; + = para a frente/baixo
(já com o espelhamento do 3):

- 3 (rótulo 103): 0, 15, 12, 12, 15, 12, 18, 18, 12, 12, 12, 27, 24 (189 px/ciclo).
- 20: vy 0 ×13, −62, −26. 21: vy 0 ×6, −30, −66, −12.
- 23: 0, 6, 6. 26: 0. 36: 0, 10 ×6, 20, 20, 10, 10, 5, 10, 10, 10, 20, 25 ×4, 20 (295 px).
- 49: ciclos de 0 e 6 (132 px em 42 quadros). 52: vy 0, −4, −16, −8, −2, 0, −2, −4, −20, −10,
  −2, 0, 0 (68 px). 53: 0, 22, 18, 4, 12, 8, 0 (64 px).
- 59/63/65 (rótulo 63) e 61: recuo de 108 e 112 px.

## Colisão (`MN_*.BRD`) [confirmado]

`MN_DEFLT`: octógono (10,8) (16,5) (24,5) (30,8) (30,52) (24,55) (16,55) (10,52) e caixas
corpo (10,8) 20×44, topo (16,5) 8×3, base (16,52) 8×4 (pés em y + 56; Jake y + 47).
`MN_COLPS` (10,30) 20×26; `MN_CRAWL` (10,20) 20×36; `MN_BARUP` (10,5) 20×21.

## Em aberto

- Medir no original os valores do esqueleto (k altos: 5 no rastejar, 6 na barra).
- Confirmar no jogo o `+0x14c = 0` do empurrar.
- Estado 2 (b = 0, sem rótulo próprio): quem entra nele.
