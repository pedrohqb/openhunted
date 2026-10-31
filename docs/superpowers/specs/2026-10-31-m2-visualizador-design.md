# OpenHunted M2 — visualizador de fase: design

Data: 2026-10-31
Status: decisões tomadas sem consulta, a pedido de Pedro ("ir sem minha intervenção"); a revisar.
Base: `docs/superpowers/specs/2026-10-31-openhunted-design.md` (spec geral), etapa M2.

## 1. Objetivo

Abrir a fase-alvo `T_J_001.LEV` ("TUTORIAL: Jake Gets a Gun") numa janela SDL3, com o cenário
completo, rolagem livre e os objetos desenhados com a animação certa, animando no ritmo do
original. Sem jogabilidade: ninguém se move por conta própria além das animações.

Decisão (opção 1 da conversa): os objetos aparecem com a animação de verdade, o que exige
decifrar no executável a ligação tipo de objeto → animação. Esse trabalho serve também à M3.

## 2. O que já se sabe do corpo das fases (protótipo validado na T_J_001)

Depois do bloco `LEV:` vêm:

1. **Lista global de objetos** (`0x00434170`): marca opcional `"OB"`, `s16` versão (≤ 1013),
   `s16` contagem e, por objeto, `u32` tipo, `i32` x, `i32` y, e uma série de campos
   dependentes da versão (inclui pares de pontos, uma tabela de 16 valores, uma tabela de
   gatilhos e dois nomes ASCIIZ). A T_J_001 tem 13 objetos (tipos 30, 73, 5, 1 e 102).
2. **x × y × z células** (`0x00432319`): marca opcional `"CE"`, `s16` versão (≤ 1010), nome da
   animação do cenário (ex.: `WORLD1.ANM`), nome do arquivo de colisão (ex.: `C1_07.BRD`), um
   índice (`10071` na primeira célula da T_J_001), vários campos, e três listas próprias:
   objetos (mesmo leitor da lista global), uma lista com marca `"PR"` (`0x0042ff4c`) e uma com
   marca `"DC"` (`0x00430278`).

## 3. Escopo

Dentro:
- Decodificar o corpo inteiro das fases (objetos, células, listas PR e DC) para as 100 fases.
- Decifrar: como o índice da célula escolhe o quadro do cenário; a tabela tipo de objeto →
  animação (ao menos para todo tipo que aparece em alguma fase, com marcador para os demais);
  a cor transparente dos sprites; o ritmo das animações (timer `timeSetEvent`).
- Crate `openhunted-engine` (sem SDL): monta a cena de uma fase (camadas de cenário e objetos),
  avança as animações por tick e desenha num framebuffer indexado de 640×480 por software.
- Executável `openhunted` (SDL3): janela, rolagem (setas, PageUp/PageDown, Home/End), ampliação
  inteira mantendo a proporção, caixa de colisão ligada/desligada (Tab), sair (Esc).
- `ohtool render <fase> --out x.png [--camera X,Y]`: renderiza um quadro sem janela (testes e
  comparação com o original).

Fora: jogabilidade, entrada de jogador, som, música, menus, HUD.

## 4. Arquitetura

- `openhunted-formats`: ganha `level_body.rs` (objetos, células, PR, DC) e preenche
  `Level::objects` e `Level::cells`, mantendo `Level::body` apenas se sobrar algo não lido
  (deve sair vazio nas 100 fases).
- `openhunted-engine` (novo): `assets` (cache de bitmaps, animações e paleta a partir do
  `ResourceArchive`), `scene` (fase → instâncias com posição, animação e quadro atual),
  `anim` (intérprete mínimo do roteiro ANM: opcodes 0, 1 e 2; os outros, ignorados e
  registrados), `render` (framebuffer 640×480 indexado, blit com transparência e recorte),
  `camera`. Determinístico; tempo só por `tick()`.
- `openhunted` (novo binário): camada SDL3 fina (janela, teclado, textura RGBA a partir do
  framebuffer e da paleta, laço de tempo fixo). Linka a SDL3 do sistema por padrão; a feature
  `bundled-sdl` compila a SDL3 a partir do código (CI e máquinas sem SDL3).

## 5. Erros e dados

- Fase ou recurso ausente/corrompido: mensagem clara (caixa de mensagem do SDL no executável,
  stderr no `ohtool`), sem pânico.
- Tipo de objeto sem animação conhecida: desenhado como marcador (retângulo com o número do
  tipo) e listado no stderr uma vez.

## 6. Testes

- Formatos: as 100 fases decodificam e o leitor consome o corpo inteiro (sobra = 0 bytes);
  fatos da T_J_001 (13 objetos, 7 do tipo 102, 14 células, primeira célula `WORLD1.ANM` /
  `C1_07.BRD`).
- Engine: testes unitários com assets sintéticos (blit, transparência, recorte, avanço de
  animação); com dados reais, o render da T_J_001 na câmera (0,0) tem hash de referência
  (fixado depois da inspeção visual) e desenha todos os objetos da área visível.
- Executável: compila com `bundled-sdl` na CI; teste manual na máquina do Pedro.
- Pendência declarada: comparar o render com capturas do jogo original (Wine/DOSBox) — exige
  Pedro ou um ambiente com Wine.

## 7. Riscos

- A tabela tipo → animação pode estar espalhada em código (um `switch` por tipo) em vez de uma
  tabela. Nesse caso, mapear os tipos das fases de tutorial primeiro e os demais em lote.
- O roteiro ANM pode depender de opcodes 3 e 4 para o cenário; se sim, decifrá-los nesta etapa.
