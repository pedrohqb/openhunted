# OpenHunted M16 — desenho das camadas como no original: design

Data: 2026-11-07. Base: `docs/re/camadas-desenho.md`, `casca-acabamento.md` (luz),
`cores-sprites.md` (tabelas), `escada-camadas.md` (troca de camada), `multijogador.md`.

## Objetivo

A cena desenha, para cada vista, só a camada dela e, pelos buracos das células, as de trás
escurecidas e menores, como `0x405c7c`; objetos de outras camadas só aparecem pelos buracos
(escurecidos) ou não aparecem (camadas à frente). A troca de camada do Jake ganha a
transição da vista (`+0x44/+0x48/+0x4c`).

## API

- `scene::view::View { screen: Rect, anchor, center, layer, distance, transition, players,
  norm, reference: Option<i32> }`: retângulo da vista no quadro, ponto da tela onde cai o
  centro, centro no mundo, camada, distância (`+0x08`), transição, número de vistas
  (`0x497300`), altura usada na normalização e referência (`None` = camada da vista em 1:1;
  `Some(30)` = zoom do original).
- `View::layer_distance(camada)` (`0x436a70`), `View::project`/`unproject`, `View::scale(camada)`.
- `Scene::view_of(i, camera, screen)`: a vista do jogador `i` com a câmera (canto superior
  esquerdo, como hoje), o estado de vista do jogador (camada, distância, transição) e o
  retângulo na tela. `Scene::draw_view(assets, fb, &View, overlay)` desenha uma vista; a
  M15b chama uma vez por jogador com metade da tela.
- `Scene::draw(assets, fb, camera, overlay)` = `draw_view` da vista do jogador 0 na tela
  inteira, com o centro na vista de jogo (640×405, a barra cobre o resto).
- `ViewState { layer, distance, counter, dir, focus_distance }` por jogador
  (`Scene::views`), atualizado no fim de `tick_players`: troca de camada (rotinas 11/12 do
  Jake, lidas pelo estado antes e depois do tique), tique da transição (`0x436b20`) e zoom
  ±1 até o `+0x60` da célula (padrão 20).

## Desenho (`draw_view`)

Recursivo por camada, com recorte (retângulo na tela) e profundidade:

- Prof. 0: fora da grade e células ausentes/invisíveis → cor 245; cenário **opaco** (índice 0
  escreve 0), com a sombra da luz 18 + s; buracos → camada − 1, prof. + 1, recorte = buraco
  (aumentado 1 px, projetado, recortado pela vista); depois decalques de trás (`+0x24 = 0`),
  objetos da camada (entidades, depois os jogadores) e decalques da frente.
- Prof. > 0: tabela 26 + prof. (≥ 7 → 33) em tudo, modo **atrás** (só escreve no destino 0):
  decalques da frente, objetos, decalques de trás, cenário (sem luz), buracos.
- Objetos: célula do centro invisível → não desenha; sombra da célula `s ≠ 0` → 18 + s em
  vez da da profundidade; as cores próprias (`+0x1c4`) compõem com a sombra.
- Escala: `reference / layer_distance(camada)` composta com a escala do sprite; o jogador da
  vista em transição usa `focus_distance`.
- `Framebuffer::blit_view(frame, x, y, escala, espelho, mapa, modo, recorte)` com
  `Blend::{Opaque, Transparent, Behind}`; `fill_rect` com recorte.
- Overlay: como hoje, na camada da vista, projetado.

## Testes

- Unidade: `layer_distance` (23/34/45 no solo 640×405; transição; 2 jogadores 240),
  tabela da profundidade (27…33), projeção ida e volta, máquina da transição (descer, subir,
  meia-volta), zoom ±1, blit atrás/opaco com recorte, cena sintética de duas camadas: buraco
  mostra a de trás pela tabela 27, objeto atrás só no buraco, objeto à frente some.
- Dados reais: `T_J_007` pixel da camada de trás visível pelo buraco = arte pela tabela 27;
  objeto de outra camada escondido/visível; hash de referência da `T_J_001` (uma camada)
  revisto só depois de conferir a imagem; PNGs antes/depois de `T_J_007`, `I_J_104`, `H_J_006`.

## Fora do escopo

Zoom absoluto da câmera (1,30) e ponto fixo das células na câmera do porte; `+0x1b4`;
separação lista da célula/global; tremor.
