# Desenho das camadas, escala por camada e transição da vista (M16)

Como a vista desenha as camadas `z` da fase. Complementa [`cenario.md`](cenario.md) (grade),
[`casca-acabamento.md`](casca-acabamento.md) (luz por célula), [`cores-sprites.md`](cores-sprites.md)
(tabelas do `DLS.TBL`), [`escada-camadas.md`](escada-camadas.md) (troca de camada do Jake) e
[`multijogador.md`](multijogador.md) (vistas). **[confirmado]** lido no código; **[inferido]** dedução.

## A vista (`0x493b70`, registros de 0x5c) [confirmado]

`+0x00/+0x04` centro no mundo; `+0x08` **distância** (20 na criação, `0x436126`); `+0x18`
**referência 30** (`0x436107`); `+0x20…+0x2c` retângulo na tela; `+0x30/+0x34` = centro −
metade da tela; `+0x38/+0x3c` = largura e altura da tela; `+0x40` camada; `+0x44` contador da
transição, `+0x48` sentido (±1), `+0x4c` distância fixa do objeto da vista na transição;
`+0x50` objeto seguido. `0x4365ee(vista)` desenha cada vista: `0x405b8c` →
`0x405c7c(&vista+0x30, vista+0x40, 0)`, depois cópia para a tela (`0x405b6c`) e a barra.

## Distância por camada (`0x436a70(camada, vista)`) [confirmado]

```
d = (vista.camada − camada)·9 + vista.distância
r = 480 com um jogador (0x497300 ≤ 1), 240 com dois
d = d·640/largura  se r < altura;  senão d·r/altura      (divisão inteira)
transição (+0x44 = c ≠ 0), k = 7 − c:
    camada da vista:  d = distância − distância·k/7      (sem a normalização acima)
    outras camadas:   d = d − 9·k/7
d = 0 → 1
```

Projeção (`0x436916`/`0x4369a2`/`0x43682e`): ponto `p` do mundo da camada → tela
`(p − centro)·30/d + metade da tela + origem da vista`, com `(v·30 + d − 1)/d` (divisão
truncada); larguras `(w·30 + d − 1)/d`. Inversa `0x43695c`/`0x436845`: `(v·d + 1 − 30)/30`.

- Solo, 640×405: `d` da própria camada = 20·480/405 = **23**, escala **30/23 ≈ 1,30**; a camada
  atrás (Δ = 1): 29·480/405 = 34, escala 0,88 — **0,68 da camada da frente**; Δ = 2: 45 → 0,51.
- Quem chama: células `0x432808` (o bloco de 128×128 é esticado ao retângulo projetado),
  decalques `0x430479` (posição relativa ao retângulo da célula), objetos `0x43558e` (caixa
  `0x434d75` projetada).
- **Zoom da câmera** (`0x436386`, por tique): alvo = `+0x60` da célula sob o objeto seguido
  (centro horizontal e topo do quadro, camada da vista), 20 se 0 ou sem célula; a distância
  anda ±1 por tique até o alvo (`0x493b80`), limitada a 1…250 (`0x436898`); `0x47d474 ≠ 0`
  desliga. Nos dados, `+0x60` (`fields[2]` das células 1006): 0 em 9 254 células, 30 em
  1 075, 20 em 239, 25 em 140, de 10 a 45 nas demais.

## O desenho de uma camada (`0x405c7c(retângulo, camada, profundidade)`) [confirmado]

O retângulo chega em coordenadas da vista; é recortado pela vista (`0x405a6c`) e vira o
recorte do desenho (`0x405a02`, x alinhado a 4). A parte do mundo coberta sai pela inversa
com o `d` da camada; as células cobertas são `x/128 … (x+w)/128` (truncado, limitado à grade).

| Prof. | Tabela de sombra (`0x43be54`) | Modo |
|---|---|---|
| 0 | 0 (identidade) | `0x80` desligado; cenário **opaco** (o bit 1 de transparência é desligado em `0x405ba9`) |
| 1…6 | **26 + prof.** (27…32) | `0x100` (tabela) e `0x80` ligados |
| ≥ 7 | 33 | idem |

**Modo `0x80` = desenhar atrás** (`0x44f68b`/`0x44f6d4`, entradas 4–7 da tabela de rotinas
`0x481148`): só escreve onde o destino **é 0** e a origem (já remapeada) não é 0. Por isso a
camada da frente sai opaca (os índices 0 dela ficam 0 na tela) e as de trás só preenchem
esses buracos; numa camada de trás, o primeiro a desenhar fica na frente.

Ordem, profundidade 0 (camada da vista):
1. Se a área toca a borda da grade: fora da grade, cor **245** (`0x405899`).
2. Passo 0, célula a célula: célula visível (bit 0 de `+0x38`) → cenário (`0x432808`), com
   a sombra da luz `18 + s` se `+0x58 = s ≠ 0`; célula ausente ou invisível → retângulo
   cheio (modo 8) de cor **245** (`0xf5`; 2 ou 255 com o `0x4879e0` de depuração, que nada
   grava) sobre a parte visível da célula, aumentada de 1 px à esquerda e em cima e 3 à
   direita e embaixo.
3. Passo 1, célula a célula (visível): o **buraco** da célula. `+0x6c = 0` e o retângulo
   `+0x70…+0x7c` (x, y, largura, altura; relativo à célula, espelhado com `+0x40` por
   `0x405775`) com largura e altura ≠ 0 (senão `+0x6c` passa a 1). Recorta-se o buraco pela
   parte visível da célula, aumenta-se 1 px de cada lado, projeta-se com o `d` desta camada
   e, se `0 < camada < grid_z`, desenha-se **`camada − 1` com profundidade + 1** nesse
   retângulo (recursão; o recorte novo é só o buraco, recortado pela vista).
4. Recorte de volta à vista; decalques com `+0x24 = 0`; objetos da lista de cada célula
   (`+0xd0`); objetos globais (`0x488e7c`, ordenada por `+0x1b4`) com `+0x1b4 < 200`;
   decalques com `+0x24 ≠ 0`; o resto dos objetos (`+0x1b4 ≥ 200`).

Profundidade > 0 (camadas de trás, tudo no modo atrás e com a tabela da profundidade):
decalques `+0x24 ≠ 0`, objetos globais (todos), objetos das células, decalques `+0x24 = 0`,
cenário das células visíveis (sem a luz; ausente ou invisível → nada, fica 0), buracos.

- **Camadas à frente da vista nunca são desenhadas**; a de trás só aparece pelos buracos.
  Fora dos buracos, os índices 0 da camada da frente ficam 0 (preto do sistema).
- Nos dados: 692 células com buraco (6 na camada 0); `T_J_007` (56, 60, 29×40) na porta
  de camada; `I_J_101` (36, 13, 92×95) etc. Decalques: 4 853 com `+0x24 = 0`, 836 com 1
  (`+0x24` é o 5º `s16` do item DC, `fields[4]`).

### Objetos (`0x43558e(retângulo, lista)`) [confirmado]

Liga a transparência (bit 1); desenha cada objeto em jogo com **`+0x17c` = a camada em
desenho** (`0x48bab8`). Escala: o objeto da vista em transição usa `+0x4c`; os outros, o
`d` da própria camada. Célula do centro da caixa (camada do objeto): invisível → não
desenha; sombra `s ≠ 0` → tabela `18 + s` (substitui a da profundidade; depois o modo
`0x100` é **desligado**, e os objetos seguintes da mesma camada de trás saem sem a tabela —
peculiaridade não reproduzida); sem célula → desenha. Objetos com rotina `+0x1c4` compõem a
cor com a tabela corrente (`T = S[A[i]]`), então também escurecem atrás.

`0x4879e4[camada]` marca as camadas desenhadas no quadro (zerado na 1ª vista); `0x435fef`
o usa no raio de atividade dos objetos (fora do escopo do desenho).

## Transição da vista na troca de camada [confirmado]

Na rotina do estado 11 (BAIXO, `0x4262c9`): se a vista não está em transição e o contador
do Jake é 0 → `+0x4c = 0x436a70(camada do Jake)`, `+0x44 = 1`; sempre `+0x48 = +1`. No fim
(contador > 7): `+0x44 = 0`, `+0x40` = camada do Jake. Estado 12 (CIMA, `0x4263ee`): início
com o contador 7 → `+0x44 = 7`; `+0x48 = −1`. Meia-volta 11 → 12 (`0x42632b`, antes do
incremento do contador do Jake): `+0x48 = −+0x48`, `+0x44` = contador do Jake, e a rotina
do estado 12 roda no mesmo tique (`0x426347`); 12 → 11: `+0x48 = −+0x48` e, se `+0x44 = 1`
e camada > 0, `+0x40 −= 1`.

Por tique da vista (`0x436b20`, chamado de `0x4363b8`, depois dos objetos): `+0x48 = −1` →
`+0x44 −= 1`, ao chegar a 0 `+0x40 −= 1` (se > 0); `+0x48 = +1` → com `+0x44 = 1`
`+0x40 += 1` (se < `grid_z` − 1), `+0x44 += 1`, e 7 vira 0.

Efeito: descendo (z + 1) a vista passa à camada nova no 1º tique; ela começa enorme
(`d` = 6, 9, 12, 15, 18 nos tiques 2–6, contra 23) e recua, e a antiga (vista pelo buraco)
vem de 28 a 33 até 34. Subindo, a camada da vista avança para a câmera (`d` 18 → 3) enquanto
a de trás se aproxima (34 → 27); no fim a vista passa a ela (23). O Jake fica no tamanho
`+0x4c` o tempo todo; quando a camada dele difere da da vista, sai na profundidade 1
(escurecido e só pelos buracos).

## No OpenHunted [inferido / decisões]

- `Scene::view_of(i, câmera, retângulo, tamanho)` monta a vista do jogador `i` (estado em
  `Scene::views[i]`, atualizado no fim de `tick_players`) e `Scene::draw_view(assets, fb,
  &View, overlay)` desenha uma vista (a tela dividida chama uma por jogador); `Scene::draw`
  é a do jogador 1 na tela inteira. A vista carrega câmera, retângulo na tela, camada,
  distância, transição e a referência. **Referência**: o porte desenha a camada da vista em
  1:1 (`s` = o `d` normal dela), e as outras na razão `d_vista / d_camada`; o zoom absoluto do
  original (30/23 ≈ 1,30) fica para a fatia da câmera (`View::reference = Some(30)` já projeta
  como o original).
- O porte não limita o desenho às células cobertas pela área (`x/128 … (x+w)/128`): todo
  sprite da camada que cruza o recorte sai, inclusive a parte de um decalque que passa da
  célula dele quando a célula está fora da área (no original, cortada). Custo medido: 0,1 a
  0,5 ms por quadro (release) nas fases de três camadas.
- Sem campo `+0x1b4` no porte, todos os objetos contam como `< 200` (os vistos nos documentos
  vão de 10 a 175); os objetos das células e os globais saem na ordem das entidades.
- Ruling: fora de transição e sem o Jake trocando de camada, a camada da vista acompanha a
  do jogador (teletransporte, renascimento).
- O overlay de depuração sai por cima de tudo, projetado com o `d` da camada da vista: as
  caixas das células, as portas de célula e as zonas só dessa camada; os sólidos dos objetos
  (do mundo da camada do Jake) e os marcadores (sem camada), sempre.
- Objetos de sombra própria numa camada de trás: o porte usa sempre a tabela da célula ou a
  da profundidade (sem a peculiaridade do modo `0x100` desligado, acima).

## Em aberto

- Célula presente cuja arte não cobre os 128×128: o original só pinta 245 nas células
  ausentes ou invisíveis (`0x405b8c` não limpa o quadro; `0x45cfae` só guarda o recorte), então
  o resto ficaria com o que havia no quadro; o porte pinta 245 em toda a área antes do cenário.
- Campo `+0x1b4` (ordem de desenho) e a separação lista da célula / global no porte.
- Zoom absoluto e ponto fixo da câmera (`+0x5c` bit 0, `+0x64/+0x68`) na câmera do porte.
- O tremor (`+0x274`, ±5 px) e o `0x47d474`.
