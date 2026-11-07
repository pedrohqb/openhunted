# OpenHunted M7b — arsenal do Jake e itens da campanha: design

Data: 2026-11-06. Base: `docs/re/armas.md`, `armas-2.md`, `tiro-jake.md`, `projeteis.md`,
`furtividade.md` (soco), `itens.md`, `itens-campanha.md`, `logica-fase.md`, `dano-vida.md`,
`hud.md`, `som.md`. Decisões sem consulta, como nas fatias anteriores.

## Objetivo

O Jake usa as seis armas (punhos, pistola, espingarda, foguetes, estrelas, granadas), troca
de arma e, sem munição, volta à melhor disponível. Os itens da campanha funcionam: munição
das armas 3–6, vida, comida, antídoto, veneno, armaduras, chaves (abrem as portas de célula),
bandeiras (objetivo "acionar"), pistão e interruptor.

## Armas (descritores `0x47df28…0x47e1a8`, registros de 18 `i32`)

- **Arma corrente** `+0x254` no inventário; começa na melhor disponível (lista do Jake
  `0x47df0c`: 1, 6, 5, 2, 3, 4; disponível = tem ≠ 0 e munição > 0). Pegar uma arma melhor
  que a melhor disponível a torna corrente (`0x43af6c`).
- **Troca** (`0x43b0cb`): ANTERIOR/PRÓXIMA na borda (bits `0x800`/`0x1000`) → disponível
  anterior/seguinte com volta; muda → som 3090 e 7 tiques de pose "pronto" da arma nova.
- **Sem munição** (`0x43b1d0`): ATIRAR com 0 → som 3090, melhor arma, pose "pronto".
- **Ciclo comum**: estado de tiro com `t` (passo do descritor); dispara em
  `+0x0c ≤ t < +0x10 + passo`; segurando ATIRAR, recomeça com `t > +0x14`; movendo, sai após
  `+0x14`; com `t > +0x18`, recuperação → pronto (10 tiques) → guardar → parado. Rótulos,
  bocas, velocidades, janelas e cápsulas: tabelas de `tiro-jake.md`/`armas-2.md`.
- **Soco** (arma 1, ou SOCO = Z): estados 150/151 (frente), 152/153 (cima), 154/155 (baixo),
  um dos dois por sorteio; janela de golpe do registro (150: 2..3, 151: 4..5, 152: 4..5,
  153: 5..6, 154: 4..5, 155: 3..4; `+0x14`/`+0x18` 5/8, 8/11, 6/9, 10/13, 8/11, 9/12). Caixa
  (`0x43accb`): metade da frente da caixa do Jake, um terço da altura (cima, meio ou baixo).
  Acerto (`0x43afe1`, linha `(1, 0, 0)` do `DAMAGE.TBL`): 8 de dano com empurrão, som 5062.
  Ruling: um acerto por objeto por golpe (o original pode repetir; `0x43d629` não foi lido).
- **Espingarda** (20, `0x40ceec`): chumbo que se divide duas vezes (espera 2 tiques, ±0x400)
  → 4 chumbos em −11,25°, 0°, 0°, +11,25°; 10 de dano cada; cápsula 23 (sem som).
- **Foguete** (21, `0x40d678`): reto, 30 px/tique, "cima" é a diagonal; 20 diretos +
  explosão raio 64, empurrão 30, área 50 (fere o Jake).
- **Estrela** (27, `0x40bc98`): no 1º tique avança `rand % 40` / 8 px na direção do voo (o
  port aplica ao criar a estrela, onde há sorteio); onda perpendicular de período 32; parede
  → crava (item que devolve 1 estrela por 100 tiques, som 3030); objeto → 5 diretos + área 20
  (raio 30). Cravada e atingida por explosão (natureza 2) → some.
- **Granada** (26, `0x40d7ab`): gravidade, pavio 35–44, parede (`0x40707c`: contato com a
  normal mais horizontal que vertical) → inverte vx sem perder rapidez, espelha
  (`0x434815`) e toca 3042; no chão, sem som; objeto fora das exceções → explode; dano de
  explosão pendente (`0x40d819`) → explode no tique seguinte; explosão raio 55, empurrão 30,
  área 30.
- **Pistola**: cápsula com dx/dy/v do registro de cada mira (`0x47d9b4` cima, `0x47d9fc`
  diagonal para cima, `0x47da44` frente, `0x47da8c` diagonal para baixo e baixo).
- Sons por quadro: 3045 espingarda, 3035 foguete, 3110 estrela (no rótulo novo do tiro).

Ruling: impacto direto dos projéteis novos só fere quem já trata dano
(`interact::hurt`: minas, barris, olhos e o que a M7a acrescentar); os demais só os param.
Ruling: o quique da granada no chão (não lido, `0x40707c`) inverte vy com metade da rapidez;
abaixo de 1 px/tique ela pousa e espera o pavio.
Ruling: a estrela cravada é um item comum (`Pickup` tipo 27, v = 1) com prazo de 100 tiques.

## Teclas

Ruling: ANTERIOR/PRÓXIMA = PgUp/PgDn (como o padrão do jogador 1, `tiro-jake.md`); 1–6
escolhem a arma direto (extensão do port, bits `SELECT` = arma << 24 lidos só pelo Jake). Tab
segue como overlay. HUD: a barra mostra a arma corrente; o título mostra nome e munição.

## Itens (coleta genérica)

- Munição: 13/17 espingarda (dá/+v), 14/18 foguetes, 27 estrelas, 26 granadas, 12/16 pistola
  (3·v); recusa com 99. Dificuldade ×1 (normal).
- Vida 32 (+25) / 34 (+100); armaduras 76–79; antídoto 37 (zera o veneno; recusa sem veneno);
  veneno 33/35 (+1 em `+0x270`): a cada 20 tiques o Jake perde `+0x270` (dano de veneno, que
  a armadura não absorve); HUD com o rosto 93.
- Chaves 62–67 e bandeiras 80–85: par `tipo − 49` / `tipo − 73` (tem = 1, +1); som 5060;
  rótulo = tipo. Bandeiras com objetivo 2: coletada = ligada (o veredito já lê `on`).
- Itens não renascem (solo). Tipo 0: inerte (`Static`).

## Portas de célula (decodificado nesta fatia: `0x40ecd7`, `0x40ed62`, `0x40eed4`, `0x40f040`)

Célula com `(índice / 10) % 100 = 38`. Tabela `+0x44..+0x54` (os `s16` depois de `+0x9c`):
`+0x44 ≠ 0` começa aberta, `+0x48` chaves exigidas, `+0x4c` tiques até fechar (0 → 55; < 0 →
nunca), `+0x54` estado (1 fechada, 2 fechando, 3 abrindo, 4 aberta). Quadros: o rótulo da
célula; fechada = primeiro, aberta = último; abrindo/fechando anda um quadro por tique.
Colisão: `C%d%c%02d%c1.BRD` fechada/abrindo/fechando, `…0.BRD` aberta. Aberta, conta os
tiques e fecha quando nada cruza a faixa da coluna (`0x40ee3b`: x e largura da coluna da
porta, a primeira caixa com largura ≤ altura, na altura toda da célula; os trilhos não contam). Abrir (`0x40f040`): o Jake empurrando
(fim do rótulo 100) numa zona PR `0x20`; fechada → abre se não pede chave, ou se alguma chave
pedida está no inventário, **e** as entradas 3–4 dos gatilhos da célula dão verdadeiro;
senão fala 3715/3714 (sem chave) ou 3715/3719 (gatilho); aberta → fecha. Chave não gasta.
Ruling: as duas falas saem como dois eventos de som no mesmo tique, em ordem (sem a fila do
locutor `0x43c868`, que as encadeia e não repete enquanto uma toca).

## Pistão (39) e interruptor (74)

- Pistão (`0x4106ec`): rótulo 1, quadro 0; parado, sem espera e gatilhos > 0 (com algum
  código) → anda (+1 do quadro 0, senão −1); nos extremos espera `values[0]`. Andando, a
  caixa do quadro (2 px abaixo) contra o Jake: 35, derrubada, empurrão (±48, −3). Sons 3058
  (quadros 1/9), 3057 (2/8).
- Interruptor (`0x40e4dd`): CIMA (borda) com o Jake parado sobre zona PR `0x40` tocando a
  caixa → inverte `on`; `values[1] = 0` → só uma vez. Desenho: ligado → último quadro do
  rótulo, desligado → primeiro (Ruling: sem a animação de 3 quadros). `values[0] ≠ 0` → ligado.

## Simplificações declaradas

- Sem a pose de apertar o botão (estado 8) nem as falas como áudio de fala (eventos de som).
- Especial BAIXO+ATIRAR igual para todas as armas (como no original, registro único).
- Sem tiro no ar; projéteis do Jake não acertam o próprio Jake diretamente (só as áreas).
- Dano por dificuldade e tipos do Garathe fora.

## Testes

Sintéticos: troca (ordem, volta, som), sem munição → melhor arma; cadência e munição de
cada arma; divisão do chumbo em 4 com ±11,25°; foguete perto fere o Jake; estrela crava e vira
item recolhível que some em 100 tiques; granada quica (som), explode no pavio e ao tocar
objeto; soco fere a mina dentro da caixa e não a de fora; cada item (munição, veneno a cada
20 tiques, antídoto, chave, bandeira); porta de célula abre com a chave, recusa sem ela,
fecha sozinha; pistão anda com gatilho e derruba; interruptor liga/desliga.
Dados reais: contagens de itens por fase = `cells[i].objects`; numa fase com espingarda,
pegar o item dá munição; `D_J_009`/`H_J_018`: porta de célula com chave abre; bandeiras de
`D_J_007` ligam o objetivo; fuzz de 2000 tiques com entrada aleatória em todas as fases solo.
