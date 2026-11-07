# OpenHunted M8 — Garathe Den jogável: design

Data: 2026-11-06. Base: `docs/re/garathe.md`, `garathe-movimentos.md` e, para o que é comum,
`jake-movimentos.md`, `jake-escalada.md`, `furtividade.md`, `tiro-jake.md`, `armas.md`,
`armas-2.md`, `ske.md`, `dano-vida.md`, `itens.md`, `itens-campanha.md`, `menu.md`.

## Objetivo

Nas fases cujo objeto-vaga do solo (`modos & 3 = 3`) é do tipo 2, o jogador controla o
Garathe (`MINOTAUR.ANM`, `MN_*.BRD`, `MINOTAUR.SKE` id 2). O shell deixa de pular as missões
`p=2` (24 missões: `T_M_*`, `I_M_*`, `H_M_*`, `D_M_*`). Fora: IA do Minotauro, dois
jogadores, telas de controle por personagem.

## Arquitetura: um jogador, dois perfis

O original tem duas máquinas (`0x422aa9` e `0x427b68`) com a **mesma** estrutura (tabela de
estados de 0x70 bytes, mesmas rotinas de tiro/armas `0x43b43e…`, mesma física e sondas). O
port mantém **uma** máquina semântica (`jake::State`: parado, correr, pulos, corda, barra…)
e põe o que difere num perfil por personagem.

- `character::Character { Jake, Garathe }` (o `hud::Character` passa a ser este) e
  `Profile` (`&'static`): corrida (12/15 px), empurrar (12/8), pulos e sondas (110/150/255;
  156/350), pulo após o preparo (estendido/longo), fatores k do esqueleto (andar 3, pulo
  parado 3/3, pulo para cima 3/2, engatinhar 2/5, barra 4/6, corda 2/2), folga para levantar
  (30/20), especial (janela 1..5 com 3/tique; 5..20 com 1/tique, fim > 24), fator de desenho
  (12/20; 587/1000), ordem das armas (1,6,5,2,3,4; 1,2,5,4,6,3), bit de coleta
  (0x100/0x200).
- **Ruling:** a struct continua `Jake` (o jogador; ~110 usos na cena e nos testes), com
  `character` (padrão Jake). Renomear seria só ruído; `jake::Player` é um apelido.
- Rótulos: `anim_of`/`reversed`/`mirrored_art` passam a métodos que despacham pelo
  personagem; a tabela do Garathe (`0x478980 + 0x70·s`) fica em `jake/garathe.rs`. Estados
  novos só do Garathe: `Rise` (35 `MNBLGTUP`) e `Jolted` (59, rótulo 63).
- `Skel` ganha o fator de desenho; `Inventory` ganha o personagem (ordem das armas e coleta).
- Armas do Garathe: descritores em `jake/weapons.rs` (`weapon_of(ch, w)`), mesmo `Reg`.

### Mapa de estados do Garathe (semântico → estado/rótulo)

Parado 1/1 (c1) · correr 10/10 · empurrar 46/46 (c1) · furtivo → **andar de luta 3/103**
(arte espelhada, k 3) · pulo parado 21 · para cima 20 (k 2) · da corda 19 · preparo 24 →
longo 25 · médio 22 · queda 27 · queda descontrolada 29 · pousos 28 (queda), 23 (médio,
k 1), 26 (longo: 24 ao contrário, k 1, som 5053), 30 (duro) → 31 · empurrado 58 · derrubado
32 (para a frente) / 33 (para trás) → 30 / `Rise` 35 · tranco `Jolted` 59 → 35 · choque 121
(a partir do quadro 17) · costas 4 / desvirar 5 · escalar 52 / 53 (k 2) · camadas 11/12 ·
saída 13 (rótulo 12) · agachar 37 (dois quadros/tique), rastejar 36 (k 5), levantar 38 ·
esconder 54/56/55 · barra 47, agarrar 50 → mão ante mão 49 (k 6), soltar 51 → 47 (não há
"subir na barra") · especial 127 · mortes 61 (veneno), 63, 67 (derrubado para a frente), 69
(para trás). Caixas: `MN_DEFLT`, `MN_CRAWL` (36–38), `MN_BARUP` (49–51).

## Regras do Garathe

- Correr: vx ±15 px/tique; FURTIVO + lado → andar 3 (pose de luta).
- Escolha do pulo: chão a 156 px → médio (13, −12); senão a 350 → preparo 24 e longo
  (25, −14); senão médio. Correndo, PULAR+CIMA+lado → longo direto. **Não há estendido.** No
  ar a animação toca uma vez e para no último quadro.
- **Investida** no pulo longo (`0x429bda`): cada objeto sólido tocado leva 7, tranco e
  empurrão pelo lado (uma vez por objeto por pulo).
- Agachado: tiro e pulo exigem 20 px livres acima.
- Reações: empurrão só em 1, 54–56, 58; tranco (novo `Pending::jolt`) em parado, correr,
  empurrar, armas e pousos → 59 → 35; nos 52/53 → queda.
- **Ruling:** o `+0x14c = 0` do empurrar (`0x42a9f5`, provável erro: teleporta a x = 0) não
  é copiado; o empurrar só para.
- Especial 127: BAIXO+ATIRAR com vida > 25; na janela 5..20 perde 1/tique e o efeito vale a
  cada tique (destrói projéteis e fere os demais a < 70 px; jogadores a < 70 derrubados; a
  70–140 atordoados — no solo não há outro jogador). Fim com `t > 24`.

## Armas do Garathe (`0x47e994…0x47ec14`)

| Par | Arma | Pronto/guardar | Estados (cima, diag., frente, baixo) | Golpe / disparo | `+0x14/+0x18` | Projétil |
|---|---|---|---|---|---|---|
| 1 | garras | 100/101 | 106/107, idem, 104/105, 108/109 (sorteio) | 3..4 | 8/8 | — |
| 2 | clava | 110/111 | 116, 116, 114, 118 | 5..7 | 11/12 | — |
| 3 | chicote | 152/153 | 162, 160, 159, 161 | 8..10 / 9 | 24/24 | 108 |
| 4 | facas | 81/82 | 98, 96, 95, 97 | 8 | 12/20 | 25 |
| 5 | granadas | 140/141 | 150, 148, 147, 149 | 8 | 12/20 | 26 |
| 6 | estrelas | 128/129 | 138, 136, 135, 137 | 8 | 12/20 (cima 20/24) | 27 |

- Passo 1 (garras, clava) ou 2; recuperação 154/85/142/130; bocas e velocidades de
  `garathe.md`. SOCO (ou boca na parede) usa a clava se tem o par 2, senão as garras.
- Golpes (`DAMAGE.TBL`, atacante 2, vítima "qualquer"): garras 15 com empurrão; clava 15
  com derrubada (20, 30) / (20, 10) / (20, −10); chicote 18 com derrubada; som 5062.
- **Chicote**: a linha até a boca só é testada até a metade. Projétil 108 (`owb.anm`): reto,
  20 de dano e derrubada; após 7 tiques (Ruling: o meio de 6–8) gera dois filhos a ±11,25°
  com a mesma velocidade, até a 4ª geração; some na parede.
- **Faca** 25 (`okn.anm`): reta; num objeto 12 com empurrão e some; na parede crava e vira
  item. **Ruling:** a cravada devolve 1 faca (o valor não foi lido) e dura 100 tiques, como
  a estrela.
- Coleta só do Garathe (bit 0x200): 15 → clava, 25 → facas, 44 → chicote (+v), 45 → +v no
  chicote; 26 → par 5, 27 → par 6 (no Jake, 6 e 5).

## Cena, shell, HUD e som

- `Scene::build`: o objeto-vaga do solo de tipo 1 **ou** 2 vira o jogador; o tipo escolhe
  ANM, BRDs, `.SKE` (id 1/2) e perfil. `has_controllable_player` (tipo 1 ou 2) substitui o
  teste no shell; `has_controllable_jake` fica para os testes do Jake.
- HUD: tabela de armas pelo personagem do jogador (`0x471ce9`).
- Falas pelo personagem (`0x40b9eb`): 3700→3735, 3717→3737, 3704→3736, 3718→3731,
  3715→3730, 3714→3732, 3719→3741, 3712→3734/3740; empurrar 5043. Vitória/derrota 3786/3785.
- Inimigos que olham o esconderijo (`is_hidden`) já cobrem 54–56: o teste é semântico.

## Simplificações declaradas

- `MN_COLPS` não é usado (pousos duros, derrubada e morte ficam com a caixa padrão).
- Estado 2, pose 76 (entrada 0xf), terremoto 0x42bcf8 e renascer 80 (disputa) ficam de fora.
- Investida e tranco: um acerto por objeto por pulo; o "módulo 20" dos empurrões não é
  modelado (como no Jake).
- O preparo 24 não move pelo esqueleto.

## Testes

- Sintéticos (perfil Garathe, esqueleto sintético): correr 15 px/tique; pulos médio/longo
  com as velocidades e o preparo; pulo parado k 3 e para cima k 2; andar de luta k 3 com arte
  espelhada; rastejar k 5 e folga de 20; barra 47→50→49 (k 6)→51; esconder 54/56/55;
  empurrão 58, derrubada 32/33 → 30/35, tranco 59, choque 121; mortes 61/63/67/69; especial
  127 (16 de vida, fim em t > 24); cada arma com rótulos, janelas, projéteis e munição; SOCO
  → clava; coleta 15/25/44/45/26/27; faca cravada recolhida; chicote divide em 2/4/8.
- Dados reais: `T_M_002` (o primeiro tutorial do Garathe; não há `T_M_001`) carrega com o
  Garathe controlável, corre e pula pelas tabelas, e é **vencida com entrada roteirizada**;
  fuzz de 2000 tiques com entrada aleatória em todas as fases `p=2` solo (sem pânico, nunca
  dentro de sólidos); todos os testes do Jake verdes.
