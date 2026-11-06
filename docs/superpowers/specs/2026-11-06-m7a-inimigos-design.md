# M7a — inimigos principais (design)

Fatia da campanha do Jake: os cinco inimigos mais comuns das fases `D_J`, `H_J` e `I_J`,
portados das notas `docs/re/robo-seguranca.md` (OSR, tipo 8), `robo-sentinela.md` (OSE, 10),
`mina-saltadora.md` (OHS, 11), `gerador-minas.md` (OHG, 103) e `bruiser.md` (OBR, 7), com os
projéteis inimigos de `projeteis.md` e o dano de `dano-vida.md`.

## Escopo

- Comportamento por tique de cada tipo: percepção (alcance, `|dy|`, lado, linha de visão,
  esconderijo 164–166 só no OSR), movimento (flutuação/patrulha/perseguição do OSR, pulos do
  OHS, andar do OBR pela tabela de velocidades), ataques, dano recebido, morte, sons.
- Projéteis inimigos num arquivo só, `behaviour/enemy_shot.rs`: bala do Bruiser (100),
  bala da sentinela (106), bala da Morte (99, sem atirador nesta fatia) e o foguete inimigo
  (21). O raio do OSR é o tipo 36 já existente (`laser.rs`).
- Fora: Garathe/Minotauro, tipos 3/4/87/89/105/107/41, vários jogadores, escala de
  dificuldade além do normal.

## Arquitetura

- `behaviour/enemy.rs`: o comum — `Clip` (rótulo, quadro, ao contrário; um quadro por tique,
  com o fim da animação devolvido ao comportamento, como as rotinas de fim `0x407942`), sons
  por quadro absoluto do `SOUND.TBL` (tabelas por tipo nos módulos), linha de visão pelo
  mundo do Jake (células e objetos sólidos da camada dele, `0x433aa3`/`0x43735d`), alvo
  `Target` a partir do `Ctx`, dano pendente com crédito do jogador (`Wound`) e os pedidos que
  precisam de recursos (`Spawn`: tiro, mina, fogo).
- Um módulo por tipo: `security_robot.rs`, `sentinel.rs`, `hop_mine.rs`,
  `mine_generator.rs`, `bruiser.rs`; cada um com o estado em `Behaviour::<Tipo>`.
- Os sprites desses tipos ficam congelados (`object_start`): o comportamento escolhe o quadro
  (`show`), como no olho-laser.
- Cena (edições localizadas): `Out` ganha `kills`, `enemy_spawns` e `quake`; a cena cria os
  objetos pedidos (`spawn_enemy_objects`, com `OSB00/OBB00/ORO/ODS00/OHM00/OFI` carregados na
  montagem), filtra os sons de espera globais (`throttle_sounds`), conta as minas vivas de cada
  gerador antes do tique (`mine_generator::sync`) e aplica o terremoto ao Jake.
- `bullet_hits`: a bala do Jake fere esses inimigos pela `Wound` (crédito quando
  `player_shot`); no OSR soma o som 5056 "Bullet hits metal". `interact::hurt` encaminha as
  explosões a eles (sem crédito).

## Regras (Ruling)

- Ruling: dificuldade normal (`0x487854 = 1`): nenhum dano é dobrado nem dividido.
- Ruling: as distâncias `|dx|`/`|dy|` da percepção usam os centros das caixas (o original usa
  `+0x150` no `|dy|` da sentinela e da mina saltadora; os centros dão o mesmo efeito sem
  depender do quadro de referência).
- Ruling: a linha de visão amostra 16 pontos do segmento entre os centros no mundo do Jake,
  ignorando os pontos dentro das caixas de quem olha e do alvo.
- Ruling: o OSR flutua sem gravidade (como a mina, `minas.md`); a gravidade só age na queda
  (estado −2), até o contato com o cenário e velocidade < 1 px/tique.
- Ruling: os bits de passagem das células (`0x2/0x8/0x10/0x40`) do OSR são uma sonda de
  física de um passo contra as células (como `0x41f057` da mina).
- Ruling: o OSR não troca de camada pelas zonas PR ao chegar à última posição vista: esquece
  o alvo. A coleira dos gatilhos (código > 0) não é portada; o código 0 com alvo existente
  deixa o robô parado.
- Ruling: patrulha do OSR simplificada: anda no rumo atual; ao bater, sair da fase ou no
  sorteio de virar (`+0x08` por 100 000), escolhe outro rumo permitido pelo modo e livre.
- Ruling: as bocas do OSR e do Bruiser estão em px da fase (sem escala); a do OSR espelha
  pela caixa (`x' = 2·caixa.x + caixa.l − x`).
- Ruling: raios não ferem o OSR (como o olho-laser não se fere com os próprios).
- Ruling: projéteis inimigos voam retos (o foguete 21 também; `0x40d678` não lido) e só
  atingem o Jake e o cenário; não ferem outros objetos.
- Ruling: o terremoto do Bruiser vira o choque sem dano do Jake (estado 15, o mesmo do
  cambaleio), renovado enquanto o contador durar e ele estiver no chão.
- Ruling: o fogo do Bruiser nasce com a altura sorteada 25–49 e míngua 1 a cada 2 tiques.
- Ruling: o Bruiser fala uma frase por vez (sem as encadeadas), com 60 tiques de silêncio.
- Ruling: a tabela de passos do rótulo 3 do Bruiser só tem os 10 valores lidos; o resto é 0.
- Ruling: o OHS morre ao tocar o Jake (caixas a 1 px) ou ao levar qualquer dano de bala; o
  contato com outros objetos não é testado.
- Ruling: o crédito de morte vai ao jogador quando o golpe que mata vem de uma bala dele;
  explosões não creditam.
- Ruling: a explosão de morte do OHG e do OSE não deixa o objeto como destroço sólido; o OSR,
  o OSE e o OBR deixam o destroço desenhado e sem colisão.
- Ruling: o especial BAIXO+ATIRAR não fere esses inimigos (variante 9 da tabela não lida).

## Simplificações declaradas

- Sem cores pulsantes (`0x43becd`); o pulso do OSR só dispara o som 3157.
- O efeito do acerto da bala no OSR é a faísca comum (`oex15`), não `oex14`.
- O estado do OHS na água/barril e o bit `0x100000` não são tratados.

## Testes

Sintéticos por módulo: percepção (alcance, lado, esconderijo só no OSR), cadência (OSE: 3
balas por abertura; OSR: alternância dos canhões, rajada e pausa; OHG: intervalo, estoque,
limite de 4 minas), dano e morte (casulo fechado invulnerável; teto 45 do OSR; metade no
Bruiser e imune a fogo; % do gerador), padrão de pulos do OHS (5 pulinhos e um grande),
ativação do Bruiser pelos gatilhos e o terremoto. Dados reais: rótulos dos ANM batem com as
tabelas; diante de uma OSE a 100 px ela abre e atira; a pistola mata uma OHS e a cena conta;
o gerador da `I_J_005` solta minas; o OSR da `I_J_104` atira no Jake; fuzz de 2000 tiques com
entrada aleatória em toda fase solo com esses inimigos, sem pânico; as tutoriais seguem
vencíveis (o Bruiser da `T_J_007` atrás da parede) e uma missão com inimigos é vencida por
roteiro, se houver rota viável.
