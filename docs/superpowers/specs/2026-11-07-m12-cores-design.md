# OpenHunted M12 — cores dos sprites: design

Data: 2026-11-07. Base: `docs/re/cores-sprites.md`, `docs/formats/tbl.md`.

## Objetivo

Os sprites dos 7 tipos com rotina de desenho (`+0x1c4`) — Jake, Garathe, Camaleão,
Anticorpo, robô de segurança, gerador de minas e mina flutuante — passam a ser desenhados
pela tabela de cor que o original monta a cada objeto (`0x43becd`), e não com os índices
crus (ruído de arco-íris). Entram também o piscar dos jogadores (`0x4081ae`/`0x408129`), o
ciclo de cor da clonagem e o esmaecer do Camaleão. Fora: luz por região (sombra ≠ 0), cor da
`SEGOC` na disputa, truques.

## Arquitetura

- `tint.rs` (novo, motor): `Tint { table, shift, cutoff }` (o par do `0x43becd` mais o
  limiar do Camaleão); `Dls` (48 tabelas do `DLS.TBL`) com `table(tint, sombra) -> [u8; 256]`
  pela composição documentada (metade alta deslocada com tabela ≥ 0x22 e fator ≠ 0; limiar
  zera `T[i ≥ c]`). Constantes das variantes copiadas das tabelas do executável (endereço
  em cada uma).
- **Ruling: a cor é calculada no desenho, do estado do comportamento** (`tint::entity(e,
  índice, tique)`), como no original (a rotina `+0x1c4` roda no desenho). Assim nenhum
  comportamento muda e não há campo novo em `Entity`/`Sprite` (40 literais de `Entity`, ramos
  paralelos M10/M11). O "endereço do objeto" do pulso vira o índice na lista de entidades
  (estável: a cena só acrescenta no fim); o robô usa a fase que já tem (`phase`), a mesma
  do zumbido.
- Jogador: `Jake.flash: tint::Flash` (estado `+0x28c/+0x290`), ligado nos pontos do
  original: dano com reação/explosão (0x2e, 4), veneno a cada tique (0x2f, 4), choque na
  entrada (0x25, 10), efeito do especial (0x27, 4); `Flash::tick(envenenado)` no fim do
  tique. Cor-base 0 (solo): `tint::player` dá 35 (Jake) ou 36 (Garathe).
- Desenho: `Framebuffer::blit_scaled_mapped` (transparência testada no valor remapeado, como
  `0x44f641`); `scene::draw_sprite` recebe `Option<&[u8; 256]>`. `Assets::dls()` lê o
  `DLS.TBL` uma vez (sem ele, desenha cru, como hoje).
- Camaleão: o limiar sai do `fade` já existente (`0xf7 − 7·fade`, limitado a 0x91…0xf7); o
  Camaleão esmaecido **continua desenhado** (`Entity::visible` deixa de escondê-lo): sobram
  os pixels < 0x91, como no original. Clones: cor `+0x34` da variante, sem esmaecer.
  **Ruling:** o ciclo da clonagem (38, 39, 40, 41, 40, 39, normal; passo de 3 tiques) é
  derivado da contagem restante (`cloning`), sem o `rand % 5` inicial (não muda a sequência
  do gerador de números).

## Testes

- Unidade (`tint.rs`): composição (identidade, deslocamento ±, limites ±127, tabela fora de
  0–47, limiar), pulso (fórmula inteira, extremos ±A), piscar (alternância normal e com
  veneno, não reinicia em curso, troca 35↔37), cor de cada comportamento.
- `render.rs`: `blit_scaled_mapped` (remapeia, 0 remapeado é transparente, espelho/escala).
- Dados reais: para Garathe (D_M_030), Jake (T_J_001), Camaleão, Anticorpo, robô, gerador e
  mina, desenhar um quadro com a tabela e conferir que todo pixel opaco da arte na metade alta
  sai na rampa da tabela (`T[128..256]`) e nunca com o índice cru; Camaleão invisível só
  deixa pixels de origem < 0x91.
- Visual: `ohtool render … --overlay` antes/depois (PNG no rascunho).
