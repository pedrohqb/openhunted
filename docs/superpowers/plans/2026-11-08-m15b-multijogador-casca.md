# M15b — dois jogadores na casca: plano

Spec: `docs/superpowers/specs/2026-11-08-m15b-multijogador-casca-design.md`. Cada tarefa em
TDD (teste falhando → código → `check.sh` → commit). O solo não muda.

1. **Motor, telas** (`shell/`): `Party` e `missions_for`; `views::layout`; barra achatada e
   pontos da disputa (`hud.rs`, estilo 195); `Reply` novos; `SMULT`/`SLEVL` com o modo;
   `SEGOC` (`choose.rs`), `SHVLD` (`limits.rs`), `SH2HS` (`duo.rs`) com testes de tela.
2. **Viewer** (`viewer.rs`): `from_level_with(mode)`; duas entradas e batidas; câmeras por
   vista; `draw_view` (ponto único) e quadro dividido; ouvinte pela vista mais perto;
   relógio real; testes de geometria e de som.
3. **App** (`dois.rs` + ganchos em `shell.rs`/`casca.rs`): estado do modo, fluxos de disputa
   e coop, `SINPU` a partir da `SEGOC`, resultado, NEXT/REPLAY/MAIN, entrada dos dois
   dispositivos; testes de fluxo com dados reais, teclado e joystick falsos, limite de tempo.
4. **Docs e capturas**: README (status condensado, controles do jogador 2), `docs/re`
   (`hud.md` estilo 195, `multijogador.md` aberto/fechado), exemplo `shots` com as telas
   novas; `check.sh --full`; merge de `main` sem commit.
