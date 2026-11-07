# M10 — inimigos secundários e dificuldade (design)

Os demais hostis da campanha, das notas `docs/re/inimigos-menores.md`, `inimigos-menores-2.md`
e `anticorpo-manta.md`, e a dificuldade (`opcoes-salvamento.md`, `0x487854`).

## Escopo

- Morte (3), Camaleão (4) com clones, Anticorpo (105) e Manta (89), gerador de dardos (87) e
  dardo (29), catapulta (41), Entidade (107), formigueiro (88) e formiga (109).
- `Difficulty {Easy, Normal, Hard}` na cena (padrão Normal, `Scene::difficulty` público para a
  casca): dano recebido ×2/÷2 na Morte, Camaleão, Manta, Bruiser, OSR e gerador de minas
  (dano 0 não faz nada; só um negativo vira 1, sem mínimo 1; o Anticorpo não usa) (a sentinela, a mina saltadora e os dardos não usam); munição apanhada
  ×1,5/×0,75 (`0x40a36d`); espalhamento e espera da Morte.
- Fora: dois jogadores, cores pulsantes, a transparência gradual do Camaleão.

## Arquitetura

- Um módulo por tipo em `behaviour/`: `death.rs`, `chameleon.rs`, `flyer.rs` (Anticorpo e
  Manta, o mesmo código), `dart_generator.rs`, `catapult.rs`, `the_entity.rs`, `ant.rs`
  (formiga e formigueiro); o dardo é um tiro de `enemy_shot.rs` com gravidade.
- `crate::difficulty`: o enum e as escalas; `Ctx.difficulty` leva o valor aos tiques.
- `Out` ganha `confuse` (Entidade) e `launches` (catapulta sobre barris e formigas);
  `enemy::Spawn` ganha `Dart` e `Ant`; os clones do Camaleão saem prontos em `out.spawn`
  (copiam o sprite do original).
- Cena (edições localizadas): `difficulty`, `confusion` (troca os pares de direção da entrada
  antes do tique do jogador e decresce), `chameleon::sync`/`catapult::sync` antes do tique,
  `catapult::apply` depois, criação de dardos e formigas em `spawn_enemy_objects`.
- `enemy::hurt`, `enemy::killed` e `enemy::COUNTED` cobrem os novos tipos; balas, explosões,
  raios e o especial chegam a eles pelo caminho da M7a.

## Regras (Ruling)

- Ruling: a confusão da Entidade é uma transformação da entrada na cena (`Scene::confusion`,
  `+0x264` do jogador): com ela > 0, CIMA↔BAIXO e ESQUERDA↔DIREITA trocam e ela cai 1 por
  tique; a Entidade a põe em 3 (se ≤ 1). `jake.rs` não muda.
- Ruling (revisão): a "vista do alvo" (`0x41299d`) é a câmera da cena — 640 × 405 centrada no
  centro do Jake e presa à fase por `clamp_camera`, a mesma conta do visualizador
  (`the_entity::view`); a confusão exige a **caixa** da Entidade cruzando-a (`0x405698`) e a
  distância entre centros menor que o alcance da confusão da variante.
- Ruling: a Entidade com `values[2] = 0` vagueia na horizontal com gravidade contra as células
  da camada, na velocidade da variante, virando na parede, na beira do chão e, ao fim de cada
  laço do rótulo, com a chance de virar da variante; com `values[2] ≠ 0` fica parada. Um golpe
  só a empurra: 5–12 px/tique para longe do Jake (o original usa a velocidade de quem tocou).
- Ruling: a Morte oculta (estado −1) fica viva e presente, sem desenho nem colisão (nem
  recolhe dano de explosão: `interact::blast` a salta com `0x2`); a
  aparição procura, nos dois lados da caixa do alvo, um lugar livre nas células da camada dele
  com chão até 15 px abaixo, recuando de 10 em 10 px (até 4 recuos); sem lugar, rearma a espera.
- Ruling: a radiação da Morte atinge só o jogador (a linha `(3, 1)` só tem dano para ele).
- Ruling: Camaleão simplificado aos estados com papel lido: 34 (parado), 4 (anda 3 px/tique,
  inferido), 28 (vira), 13 (morde), 7 (olha), 12 (recua ferido, recarga 40), 18 (morre),
  26/27 (cai/aterrissa) e 3 (preso: `vy += 10`). Sem alvo, no fim de 34/27/7: 50 % anda se o
  caminho está livre, senão vira; 25 % vira; 25 % fica. A troca de camada (39/40) não é portada.
- Ruling: o Camaleão invisível continua presente, sólido e ferível; o esmaecer (limiar 0xf7
  → 0x91, 7 por quadro) é feito pela tabela de cor desde a M12 (`2026-11-07-m12-cores-design.md`).
- Ruling (revisão): clones e formigas criadas, como as minas de gerador, entram no total
  enquanto vivos e saem dele (e nunca contam nos mortos) quando destruídos — o original os
  libera ao destruir (`0x435b63`) e o `0x43cfcf` só percorre as listas; a morte do original
  zera a vida dos clones.
- Ruling: o Anticorpo com `values[6] & 0xff = 0` dorme (o tique `0x41129d` sai sem fazer nada);
  a Manta usa a vida no lugar desse byte. Nenhum dos dois sente gravidade (a velocidade pedida
  a compensa); as velocidades do passeio e do mergulho estão em 1/8 px.
- Ruling: o bote fere o alvo no fim do rótulo de ataque sem novo teste de distância (como
  `0x412249`/`0x422416`); natureza `0x10` é dano comum. Rótulo de ataque do Anticorpo: 2.
- Ruling (revisão): a perseguição segue as etapas `+0x273` dos dois voadores (`0x411e95`,
  `0x422031`): etapa inicial pelo lado da posição do alvo; três pontos sobre o ponto-base
  (Anticorpo: centro do alvo, −75/0/+75 ou +75/0/−75, mordendo ao passar pelo centro; Manta:
  centro x e topo − `rand%10` a cada tique, −60/0/+75 ou +75/0/−60, com o rótulo do bote a
  < 40 px sem sair da perseguição); no fim, casa espelhada em x e volta. A mordida da Manta
  sai no fim do rótulo 4, em qualquer modo, pela linha `(89, 0, v)` do `DAMAGE.TBL`
  (empurrão pela velocidade dela, em px). O tempo da perseguição conta todo tique (o
  original, metade). A velocidade para um ponto dobra toda componente < 10, negativas
  incluídas (`abs(v < 10)`). O byte dormente do Anticorpo é com sinal (≤ 0 dorme).
- Ruling: o formigueiro espera os gatilhos; a cada `values[2]` tiques (0 → tabela: 100/50/55)
  cria uma formiga na sua posição enquanto `values[1]` (0 → 9/5/3; −1 = sem fim) não zera
  (`0x414aed`); variante da formiga = campo 5 − 1; ela nasce olhando para o lado contrário
  ao espelho do formigueiro (`0x414c13`). A formiga anda 2 px/tique [velocidade inferida];
  vira (rótulo 3, parada) depois de 3 tiques no mesmo x (`0x420153`) e, no fim de cada passo,
  se 0x40 px à frente não há chão até 15 px abaixo ou há um sólido que ela ainda não toca
  (`0x42081d`; a física já deixa sair do sólido em que nasceu). Nascida (ou posta) com a
  caixa nas células da camada — a caixa do `OAN.BRD` no ponto do formigueiro cruza a parede
  em várias fases —, no 1º tique vai ao lugar livre mais perto, até 64 px para os lados
  (primeiro o que olha) ou para cima [Ruling: o original não mostra esse empurrão]; com o jogador a ≤ 60 px, à
  vista, para e morde 5 a cada 10 tiques. Entrar no formigueiro (estados 6/−1), o desvio de
  outra formiga e a alternância 4/5 da mordida não são portados. No tique em que os gatilhos
  se cumprem o formigueiro só liga (`0x414b1b` → `0x414bb8`, sai).
- Ruling: a catapulta lança o jogador (derrubada com empurrão `values[0..1]`, x espelhado
  quando ela se vira) e o primeiro barril ou formiga a ≤ 30 px (centros), no quadro 2 do
  rótulo 1; caixa = retângulo do quadro.
- Ruling: o dardo nasce com a faísca 14 e some ao tocar parede, objeto sólido ou o jogador
  (1 de dano com empurrão).
- Ruling (revisão): dano 0 (a borda de uma explosão, `explosion::area_effect`) não fere nem
  dá tranco em nenhum inimigo da M10, no Bruiser nem no robô; `Difficulty::hit` concentra a
  regra dos seis pontos que leem a dificuldade (só a Manta entre os voadores).
- Ruling: o estado "ferido" do Camaleão, os sons ambientes 5120–5122 e as falas da Morte
  seguem as notas; prioridades sem valor no `SOUND.TBL` usam a de quadro.

## Simplificações declaradas

- Sem cores pulsantes (Anticorpo, clonagem) nem transparência gradual.
- Camaleão: sem fuga por porta (39/40), sem a 2ª mordida (`0x41ac2d`) e sem a sonda de 128/256 px.
- Formiga: estados 1–6 reduzidos a andar/morder/derrubada.

## Testes

Sintéticos: escalas da dificuldade (dano ×2/÷2, 0 ignorado, só o negativo vira 1; munição ×1,5/×0,75; espera e
espalhamento da Morte); máquina da Morte (aparece ao lado do alvo, radiação 4 a < 45 px,
tiro 99, oculta sem colisão, morte com fala 3764); Camaleão (morde 30 a ≤ 50 px, recua com
dano, clones ≤ 3 com metade da vida, morrem com ele, invisível continua presente); voadores
(passeio em laços, mergulho, bote, Manta com empurrão, dormente); dardos (intervalo, munição,
gravidade, 1 de dano); catapulta (lança o Jake e o barril); Entidade (inverte a entrada perto,
invulnerável, empurrada); formigueiro e formiga. Dados reais: rótulos dos `.ANM`; a Morte da
`D_J_012` aparece e fere; Camaleão da `H_J_013`; Anticorpos da `H_J_009`; Mantas da `H_J_027`;
dardos da `D_J_007`; catapulta da `H_J_012`; Entidade da `D_J_009`; formigueiro da `T_M_008`
cria formigas. O fuzz do `real_enemies.rs` cobre os novos tipos com os mesmos invariantes.
