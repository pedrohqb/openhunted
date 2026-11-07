# OpenHunted M9 — casca completa: opções, salvamento, telas, música e vídeos: design

Data: 2026-11-06. Base: `docs/re/menus-restantes.md`, `opcoes-salvamento.md`,
`videos-musica.md`, `menu.md`, `interface-itf.md`, `fim-de-missao.md`;
`docs/formats/salvamento.md`.

## Objetivo

Todos os botões do `SMAIN` levam a uma tela; o progresso é gravado e carregado (`.hhg`,
`hunter.fam`); as opções valem e ficam gravadas; a música dos menus toca em laço; os filmes
tocam quando há `ffmpeg`; a peça do carro mostra `S_CAR` e, na 7ª e na 15ª, o filme.

## Fluxo (a partir do `SMAIN`)

| Botão | Destino |
|---|---|
| Restore 11001 | `SHIGH` com o título 10041 "Load Game:" → carrega → briefing |
| Choose Level 11003 | `SLEVL` → LOAD → briefing; CANCEL/Esc → menu |
| Options 11004 | `SOPTI` → DONE/Esc → menu |
| Input Devices 11005 | `SINPU` → CUSTOMIZE (jogador 1) → `SKEYB` → DONE → `SINPU` → DONE → menu |
| Hero List 11007 | `SHERO` → OK/Esc → menu |
| Help 11006 | `SABOUT` (diálogo) → DONE → menu |
| Multi Player 11009 | `SMULT`; HEAD2HEAD/COOPERATIVE → caixa "não disponível"; CANCEL → menu |

Fim da vitória: anúncio → [`S_CAR` se ganhou peça] → [filme do meio ou final + créditos] →
`SMISS` → contagem a 100 → [`SHIGH` nome do herói] → [`SHIGH` nome do arquivo] → grava.

Ruling: os modos de dois jogadores (`SMULT`, 22017/22018 da `SLEVL`, CUSTOMIZE do jogador
2) abrem a caixa genérica com "TWO PLAYER MODES ARE NOT AVAILABLE." (em inglês,
como os textos do jogo). `SEGOC`, `SHVLD`, `SH2HS`, `SJOYS`: só se testa que
montam e desenham.
Ruling: Help abre o `SABOUT` (não há arquivo de ajuda); 33002 mostra a versão do port.

## Opções (`SOPTI`) e configuração

- Grupos de opção (dificuldade 12001–12003, brilho 12004–12008): o controle marcado mostra o
  quadro 2 do rótulo, o realçado pelo ponteiro o 1, os outros o 0. Clique (ou Enter) marca.
- Barras 12020 (música) e 12018 (efeitos), 0–100: a alça (rótulos 5 e 19) desenhada em
  `x = barra.x + v·(barra.w − alça.w)/100`; setas 12024/12025 e 12021/12022 (botões de 3
  quadros) somam ±10; clique na barra põe o valor pela posição.
- Grava na hora. Ruling: arquivo `$XDG_CONFIG_HOME/openhunted/hunter.ini`
  (padrão `~/.config/...`), seção `[Hunter]`, chaves do `HUNTER.INI` (`Difficulty`,
  `MusicVolume`, `SfxVolume`, `Brightness`, `NoMusicStart`) e a do port `MusicDir`; chaves
  desconhecidas são mantidas; valores fora da faixa voltam ao padrão (1, 25, 75, 1402).
- Volumes: < 10 = desligado; senão o volume mestre de cada saída é o valor (0–100).
- Brilho: Ruling: a paleta `WORLD_G{b−1401}.PAL` vale para o briefing e a partida; as telas
  `.ITF` seguem com a `WORLD.PAL` (`interface-itf.md`).
- Ruling: a dificuldade é gravada e lida, mas seus efeitos no jogo (tabela de
  `opcoes-salvamento.md`) ficam para a fatia que mexer nos inimigos (motor intocado aqui).

## Escolher missão (`SLEVL`)

Ruling: as sete áreas 22019–22025 são a lista de missões solo (títulos, `slevd.pft`, cor
211; a escolhida em 221), com a marca (rótulo 5) à esquerda das vencidas; rolagem pelas
setas 22007/22008 e a barra 22009 (alça rótulo 15). Clique numa linha escolhe; CIMA/BAIXO do
teclado mudam a escolha; ESQUERDA/DIREITA navegam os botões; Enter clica no ponteiro (que
começa em LOAD). SINGLE PLAYER fica marcado. Retrato 22010: rótulo 17 (Jake) ou 18
(Garathe) pelo `p=`, em laço, recolorido pela tabela `DLS.TBL` do 2º número do `p=`
(`Screen::set_remap`); Ruling: sem o sorteio de poses.

## Jogo salvo e lista de heróis

- `.hhg` (formato em `openhunted-formats::save`): bloco de 1 960 bytes; lê se o tamanho é
  1 960 e começa pela assinatura; grava nome, missão corrente, última carregada, pontuação
  acumulada, id de herói, peças, caminho, fase (`HUNTER.HLS|arquivo`), recordes `i32[300]`.
  Ruling: `0x0a0` recebe o tamanho da lista solo, `0x0a8` 0, e os campos efêmeros/ponteiro
  `0x7a0`/`0x7a4` são **sempre zero** (o original gravava um ponteiro).
- Ruling: pasta `$XDG_DATA_HOME/openhunted/saves/` (padrão `~/.local/share/...`); o nome
  vem do `SHIGH` (título 10042 "Save Game As:"), limpo para letras, dígitos, espaço, `-` e
  `_` (até 32), com `.hhg`. Primeira gravação da campanha pergunta; depois grava direto no
  mesmo arquivo (como o original). Ruling: arquivo existente é substituído sem a pergunta
  10040; Esc no nome = não grava; falha de escrita → aviso no terminal.
- Carga: `SHIGH` "Load Game:" com o nome do jogo mais recente; inválido → caixa 10009. Depois:
  missão corrente (fora da lista → 0); se já tem recorde, a próxima sem recorde
  (`next_mission`); briefing; Cancel volta ao menu.
- `hunter.fam` (`openhunted-formats::save::HeroList`), Ruling: na pasta de dados. Até 14,
  decrescente; XOR com chave sorteada a cada gravação. Na contagem a 100 (uma vez): se a
  soma entra (`0x445036`), `SHIGH` pede o nome e grava (atualiza a linha do id da
  campanha ou cria uma). Ruling: só se o recorde subiu nesta vitória; com nome já dado na
  campanha, atualiza sem perguntar.
- `SHERO`: uma linha por quadro com o som 3093, nome à esquerda e pontos à direita, cor 209,
  14 linhas de 21 px na área 31002.
- Sem a gravação ligada (testes antigos, `App::new`), nada disso acontece (sessão só).
- `SHIGH`: campo 21002 com o texto e um "_" de cursor; letras pela entrada de texto da SDL,
  Backspace apaga, Enter/DONE confirma, Esc cancela; até 31 caracteres ASCII. Ruling: a
  arte traz "CONGRATULATIONS!…"; com título, o port a cobre com a cor do painel.

## Peças do carro, `S_CAR`, filmes e créditos

- Peça ganha = vitória de missão da lista solo **sem recorde anterior** cujo objeto tipo 9
  foi apanhado (entidade morta com `on`). Peças = min(15, peças + 1).
- `S_CAR`: 38002 com o rótulo peças + 3, uma vez, parando no último quadro; monitores em laço;
  ponteiro em DONE.
- 7ª: tela preta 1 000 ms, voz `SX_4102`, filme `MID*`, preta, voz `SX_4103` (cada passo
  pula com tecla/clique). 15ª: filme `OUT*` e os créditos. Depois, o resultado.
- Abertura: ao iniciar, o filme `HH*` (Ruling: logotipo, título, história e o laço de
  demonstração ficam para depois). Ruling: créditos = fundo `SCREDIT.ANM` rótulo 1 e as
  linhas do `CREDITS.RES` centradas em `credits.pft`, subindo 2 px por tique; tecla encerra.
- Variante (Ruling): 1402 escuro, 1403–1404 normal, 1405–1406 claro **nos três filmes**; o
  original inverte a tabela do meio/final (`0x480488`), erro não copiado.
- Ruling: decodificação pelo `ffmpeg` do PATH (`-f avi -i pipe:0`, vídeo `rgb24` a 15 qps,
  áudio `s16le` estéreo 22 050 Hz), o AVI lido da pasta `Movies/` dos dados; quadro 320×240
  ampliado 2× (ocupa 640×480). Sem `ffmpeg` ou com erro: o filme é pulado em silêncio.
  Qualquer tecla ou clique pula. O tocador é testado com um decodificador falso.

## Música

- Telas cheias (`SMAIN`, `SOPTI`, `SLEVL`, `SINPU`, `SKEYB`, `SHERO`, `SMULT`, `SMISS`,
  `S_CAR`): `SX_3000.WAV` em laço (não recomeça se já toca). Briefing e partida: a param.
- Ruling: CD ausente → pasta de faixas (`OPENHUNTED_MUSIC`, senão `MusicDir`): arquivos em
  ordem de nome tocados em sequência com volta ao início, durante a partida; `.wav` lido
  direto, outros pelo `ffmpeg`. Nome da faixa: dígitos do nome do arquivo (`track02.wav` →
  faixa 2) → `SYSTEM.RES` 10010 + n ("Shock Wave"…), senão o nome do arquivo; aparece no
  título da janela (não há menu da janela).
- O mixer ganha laço por som, `stop`, `is_playing` e volume mestre. Ruling: duas saídas SDL
  (efeitos e música); filmes e vozes saem pela de música a volume 100.

## Testes

Salvo byte a byte; `HeroList` XOR; `Config`; `Mixer` em laço, `stop`, mestre;
`MusicPlayer` com saída falsa (menu em laço, playlist com volta); `Movie` com decodificador
falso (tempo a 15 qps, 2×, pular); todas as `.ITF` do CD montam e desenham; fluxos no `App`
com dados reais: menu → opções → muda volume → volta (gravado e relido), escolher missão,
vitória → nome → arquivo gravado e recarregado igual, peça → `S_CAR`.
