# Missões tutoriais `T_J_003`, `T_J_005` e `T_J_007`: o que é preciso para terminar

Continua [`logica-fase.md`](logica-fase.md) (que cobre a `T_J_001`). Dados das fases
(`/tmp/lv/T_J_00x.lev.json`); comportamentos em [`olho-laser.md`](olho-laser.md),
[`barril-fogo-cerca.md`](barril-fogo-cerca.md), [`objetos-tutorial.md`](objetos-tutorial.md),
[`escada-camadas.md`](escada-camadas.md), [`minas.md`](minas.md), [`itens.md`](itens.md) e
[`armas.md`](armas.md). Coordenadas em px da fase; célula = (x/128, y/128, camada).

Regra comum [confirmado]: o veredito (`0x43cdf0`) exige todos os objetos com `+0x1ac`
satisfeitos: a saída (tipo 5, bit 2, ignorada até o Jake apertar CIMA nela) e o Jake vivo
(bit 8). A saída só aceita o CIMA (estado 3 sobre a zona PR 0x40) quando os demais objetivos
já estão cumpridos; senão dá o choque de 20.

## `T_J_003` — "Jake Tutorial 2"

"Get to the exit. Use the 'up' key while in front of the ropes to climb." Grade 8×5×1.

- **Objetivos** [confirmado nos dados]: saída (888, 155), célula (6, 1), zona PR 0x40 na célula
  (7, 1) em x 971–984, y 181–219; Jake (`+0x1ac = 8`). Nada mais.
- **Jake**: (91, 196), célula (0, 1); vida 99, armadura 0; `points`: punhos (1, 20) e pistola
  (0, 90) → começa **de punhos**. A pistola (tipo 12, v = 20) está ao lado, em (62, 200): dá o
  "tem" e soma 60 (teto 99). Vida (32) em (39, 201) e (659, 451); munição (16, v = 25) em
  (376, 451) e (646, 453).
- **Rota** [inferido dos dados e do `ohtool render`]: a fase é uma escada de salas com redes
  (zona 4) em 12 células. Do início desce-se (caindo ou pelas redes das colunas 1–3) até o
  fundo, células (3, 4)/(4, 4); daí é **obrigatório subir** três redes em degrau: x 560–592
  (y 434–619), x 688–720 (y 306–491) e x 815–847 (y 178–363), até a sala da saída (linha 1).
  Sem a escalada (CIMA diante da rede → estado 3 → 30) a missão não termina.
- **Perigos**: olho-laser Alpha no teto da sala inicial (104, 12), alcance 400, 5 por raio
  (dica 28, zonas 2 e 3 junto ao início: atirar de baixo com ATIRAR+CIMA); 8 minas sobre as
  redes: 5 Alpha e 3 **Beta** (`values[0] = 1`: velocidade 15, alcance 250, pairam; vida 1
  vinda da fase), estas em (438, 440), (487, 562) e (519, 584).
- **Dica 16** ("Press UP to climb the ladder"): zona 1 no fundo, (566, 607).

## `T_J_005` — "Jake tutorial 3"

"Get to the exit. Surprising how far a guy can jump when he really needs to." Grade 12×3×1,
um corredor na linha 1 com dois fossos embaixo.

- **Objetivos**: saída (1298, 156), zona PR 0x40 na célula (10, 1) em x 1326–1348,
  y 187–219; Jake vivo. Nada mais.
- **Jake**: (104, 197); vida 100, armadura 0; pistola (1, 20) e granadas (1, 2) → pistola.
- **Rota** [inferido]: andar para a direita pelo corredor.
  1. Fosso 1 (células (2, 2)–(3, 2)): pular. Quem cai encontra no fundo a dica 15 (zona 2,
     "To activate teleporters STEP ON THEM") e o **teletransporte** (374, 367), cujo destino é
     a zona id 1 em (438, 98), no poço de cima da célula (3, 0); de lá se cai de volta no
     corredor. Armadura grande (77) no fosso, (422, 319).
  2. Buraco com rede na célula 5 (x 687–720, y 161–296): pular ou descer/subir pela rede.
  3. **Barris** empilhados em x = 959 (y 179 e 207, 22×29 cada), ao lado de "EXPLOSIVE" e de
     duas minas Alpha (925, 189) e (993, 169): ocupam o corredor [inferido]. Atirar de
     ≥ 144 px (explosão: raio 96, dano 50, cadeia barril–barril–minas) ou empurrá-los
     (encostar tomba; rolando, o Jake empurra; batendo a > 10 px/tique explodem).
  4. **Fosso de fogo** (células (8, 2)–(9, 2)): a dica 21 (zona 5, (1010, 233)) manda
     PULAR+DIREITA. Caindo: dois fogos estáveis de 32 px (x 1050–1212, y 349–381) tiram 1 por
     tique; a dica 4 (zona 4 no fundo) manda PULAR+CIMA para agarrar a rede da célula (9, 1)
     (x 1182–1231, y 161–256) e sair. Duas vidas (32) em (1214, 292/331).
  5. Saída.
- **Outros**: munição (406, 190), comida 34 (520, 199, +100), armadura 76 (825, 191, +50),
  mina Alpha (255, 197) e (1475, 188). Grafites "1" em (378, 317) e (443, 70).
- **Precisa no OpenHunted**: barril (tipo 28) com explosão e linhas do `DAMAGE.TBL`; fogo
  (42); teletransporte (46) com destino pela lista de gatilhos; grafite (6, só desenho);
  agarrar a rede no ar; coletáveis 34/76/77.

## `T_J_007` — "Jake Tutorial 4"

"Find the exit. Follow the arrows." Notas do autor: "the first multi-planer level … no
threat". Grade 9×4×**3**: camada 0 completa; camada 1 nas células (2..8, 2); camada 2 em
(3..6, 2).

- **Objetivos** [confirmado nos dados]: saída (818, 284) na **camada 1**, zona PR 0x40 na
  célula (6, 2, 1) em x 784–799, y 328–360; Jake vivo; **peça do carro** (tipo 9,
  `+0x1ac = 2`) em (781, 301) na **camada 2**: precisa ser pega antes da saída.
- **Jake**: (182, 49), camada 0, de pé sobre três plataformas `OPL` (y = 93); vida 150,
  armadura 100; pistola (1, 30).
- **Zonas de camada** (todas 5×32): (451, 322) zona 2 na camada 0 e zona 1 na camada 1; (568,
  313) zona 2 na camada 1 e (579, 322) zona 1 na camada 2; (707, 322) zona 2 na camada 1 e
  zona 1 na camada 2. Escada (zona 4) x 567–599, y 310–497 na camada 0.
- **Paredes `OH1`** na camada 1 em x = 635 (34×108) e 944 (80×128), fechadas e sólidas;
  só abrem quando a saída liga (gatilho `(1, 11)`), ou seja, no fim.
- **Rota** [inferido]:
  1. Descer do início até o corredor de baixo (linha 3); passar sob o olho-laser (259, 396,
     alcance 150) e pelas minas da célula (3, 3, 0): Alpha (457, 441) e (504, 441), Beta
     (481, 441), Gamma (476, 396).
  2. Subir a escada em x ≈ 583 até a sala da linha 2; ir à esquerda até x ≈ 451 e apertar
     **BAIXO** → camada 1.
  3. Na camada 1, ir à direita até x ≈ 568 e apertar BAIXO → camada 2 (a parede de x = 635
     impede seguir na camada 1).
  4. Na camada 2: minas Delta (687, 277) e Alpha (777, 317), (835, 317); pegar a **peça do
     carro** em (781, 301) (dica 34, zona 5). Voltar a x ≈ 707 e apertar **CIMA** → camada 1,
     já à direita da parede.
  5. Andar até x 784–799 e apertar CIMA na saída. Os barris (813, 318/347) estão colados à
     saída, ao lado de "FLAMMABLE": não atirar neles de perto (R = 144).
- **Perigos**: olho-laser Alpha logo acima do início (198, 12), alcance 150; cerca elétrica A
  (328, 372) pendurada sobre o corredor de baixo [inferido: fora do caminho no chão]. O
  Bruiser (1049, 298, camada 1) fica atrás da parede id 2. A dica 5 ("Press UP to hide
  against a wall", zona 3 em (169, 494)) ensina a se esconder dos tiros.
- **Coletáveis**: granadas (254, 77, v 3) e (763, 291, camada 2, v 2); lança-foguetes
  (239, 199, v 5); vida (296, 324); munição (388, 329) e (602, 320, camada 1); comida
  (373, 323, camada 1); armadura 78 (473, 319, camada 2).
- **Precisa no OpenHunted**: troca de camada pelas zonas 1/2 (estados 11/12) e presença e
  colisão por camada; escada; portas `OH` (50) sólidas; peça do carro (9) como objetivo;
  olho-laser; minas Beta/Gamma/Delta; barris; plataformas `OPL` estáticas.

## Resumo das dependências

| Mecânica | 003 | 005 | 007 |
|---|---|---|---|
| Escada/rede (zona 4) | obrigatória | para sair do fogo | obrigatória |
| Troca de camada (zonas 1/2) | | | obrigatória |
| Olho-laser (104 + raio 36) | perigo | | perigo |
| Minas variantes 1–3 | Beta | | Beta, Gamma, Delta |
| Barril (28) | | bloqueio | perto da saída |
| Fogo (42), teletransporte (46), grafite (6) | | sim | grafite |
| Peça do carro (9), portas `OH` (50), `OPL` (56) | | | sim |
| Cerca elétrica (40), Bruiser (7) | | | perigo / inerte |

## Em aberto

- Se o Jake consegue pular a pilha de barris da `T_J_005` (altura do pulo não medida).
- Geometria exata das salas (rotas inferidas dos dados e de renderizações, sem jogar).
- Se a zona 73 id 4 da `T_J_007` (252, 108) serve a algo (nenhum gatilho a usa).
