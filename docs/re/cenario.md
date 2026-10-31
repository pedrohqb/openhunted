# Cenário, transparência e ritmo (engenharia reversa, M2)

## Cenário

- Cada célula da fase aponta para uma animação de cenário (nas fases vistas, `WORLD1.ANM` ou
  `WORLD2.ANM`) e para um **rótulo** dela: o campo "índice" da célula é o **id do rótulo** no
  bloco `LAB:` (ex.: `10071` = rótulo `C1A07B`, quadro 115 de `WORLD1.ANM`). O fim do leitor de
  célula (`0x00432319`) carrega a animação (`0x00446ccc`), o cenário de colisão (`0x00430afb`) e
  resolve o rótulo (`0x004470fa`, a mesma rotina que as interfaces usam).
- Todos os índices de célula das 100 fases correspondem a um rótulo existente (teste
  `real_cell_indices_resolve_to_animation_labels`).
- Os quadros de `WORLD1.ANM` são de 128×128 e desenham um bitmap de bloco (ex.: `C1A07B.BMP`,
  128×128) na origem. A grade da fase é, portanto, de células de 128×128 pixels: a célula
  `(x, y, z)` fica em `(128·x, 128·y)`; o `z` é uma camada (todas as fases de um jogador vistas
  têm `z = 1`, salvo `I_J_101`, com 4).
- As posições dos objetos estão em pixels da fase (na `T_J_001`, x de 51 a 1696 para 14 células).

## Transparência

- Os bitmaps de sprite (Jake `J001.BMP`, mina `OHM00.BMP`) têm o fundo no índice 0, nos quatro
  cantos e na maior parte da área; os blocos de cenário são opacos (sem índice 0 dominante).
- Decisão provisória: índice 0 é transparente ao desenhar sprites e decorações; blocos de
  cenário são desenhados opacos. A rotina de blit da biblioteca GFX ainda não foi desmontada
  para confirmar.

## Ritmo

- As duas chamadas a `timeSetEvent` (`0x0044b7ca`, `0x0044c7c2`) pedem 50 ms periódicos com o
  mesmo callback (`0x0044d01d`), dentro da inicialização de um sistema de 16 canais — parece o
  som, não o laço do jogo.
- Não se encontrou ainda o ritmo do laço do jogo. Decisão provisória para a M2: um quadro de
  animação a cada 50 ms (20 Hz). A confirmar na M3, junto com a física.
- O executável tem um menu de depuração ("F1:Frame Rate F2:Physics F3:Borders F4:Skeleton
  F5:Modify F6:VCR F7:MOCKUP F8:RENDERS") que pode ajudar a medir o ritmo no original.
