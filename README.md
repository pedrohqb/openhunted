# OpenHunted

Reimplementação livre da engine de **Hunter Hunted** (K.A.A./Dynamix, Sierra On-Line, 1996)
para Linux, em Rust com SDL3.

O objetivo é recriar o jogo original com o mesmo comportamento (física, inimigos, armas,
regras das missões), rodando nativamente, com a tela original ampliada e som funcionando.

> **É preciso o CD original.** O OpenHunted lê os dados do CD do jogo (versão Windows 95 1.00).
> Nenhum dado do jogo — imagens, sons, fases, executável — é distribuído com este projeto.

## Status

**M15: dois jogadores numa máquina.** "Multi Player" leva à disputa ou ao cooperativo: a
lista de fases do modo, a escolha de personagem e cor de cada jogador e os limites de tempo e
de pontos (disputa) ou o briefing (coop), a partida em tela dividida (jogador 1 em cima, cada
metade com a sua câmera, camada e transição (M16) e a barra achatada; na disputa, `F − S` no lugar do cronômetro) e o
resultado por jogador (NEXT, REPLAY, MAIN). O motor (M15a) cuida de vagas, alvos, crédito,
renascimento e itens que renascem. As partidas a dois não mexem na campanha solo. Regras em
`docs/superpowers/specs/2026-11-08-m15b-multijogador-casca-design.md`.

**M14: acabamento da casca e entrada.** A abertura do original (logotipo, filme, título,
história, créditos) repete até uma tecla, também depois de 30 s de menu parado; as teclas e o
joystick se configuram em "Input Devices" (`SINPU`, `SKEYB`, `SJOYS`) e ficam no `hunter.ini`;
as salas apagadas por gatilho somem com os objetos e voltam clareando (luz por célula). Regras
em `docs/superpowers/specs/2026-11-07-m14-acabamento-design.md`.

**M13: auditoria da campanha.** `ohtool audit --god --reach`: das 65 missões solo, 64
vencem (a D_M_010 não: antídoto sem veneno, como no original). Corrigidos os teletransportes
entre camadas e a placa da saída. Tabela em [`docs/campanha.md`](docs/campanha.md).

**M12: as cores dos sprites** pelas tabelas do `DLS.TBL` (piscar dos jogadores, Camaleão,
pulsos de robôs, geradores e minas).

**M11: personagens do computador.** Minotauros e Jakes sem jogador agem pela IA do original
(veem a 400 px, caçam, recuam, vagam, golpeiam ou atiram, pulam buracos, fogem de perigos,
seguem o jogador pelas células), com o corpo do jogador: ferem e são feridos. Regras em `docs/superpowers/specs/2026-11-07-m11-ia-personagens-design.md`.

**M10: inimigos secundários e dificuldade.** Morte (aparece ao lado do jogador, irradia,
atira, some), Camaleão (morde, fica invisível, se clona), Anticorpo e Manta (laços, mergulho,
bote), dardos, catapulta, Entidade (inverte os controles de perto) e formigueiro. A dificuldade
das opções vale no jogo: dano nos inimigos ×2/÷2 e munição ×1,5/×0,75. Regras em
`docs/superpowers/specs/2026-11-06-m10-inimigos-secundarios-design.md`.

**M8: o Garathe Den jogável.** Nas missões do Minotauro (`T_M`, `I_M`, `H_M`, `D_M`) o
jogador controla o Garathe, com os movimentos dele (corrida a 15 px/tique, pose de luta, pulo
longo com investida, rastejar, barra, esconder, especial) e as armas dele (garras, clava,
chicote, facas, granadas, estrelas). A `T_M_002` é vencida de ponta a ponta. Regras em
`docs/superpowers/specs/2026-11-06-m8-garathe-design.md`.

**M9: a casca completa** (carregar, escolher missão, opções, heróis, "Sobre"); `S_CAR`,
filmes, `hunter.fam` e `.hhg` compatíveis com o original; música dos menus e da partida.

**M7b: o arsenal do Jake** (soco, pistola, espingarda, lança-foguetes, estrelas, granadas;
troca na ordem do original) **e os itens da campanha** (munição, comida, veneno e antídoto,
armaduras, chaves, bandeiras, pistão, interruptor).

**M7a: os inimigos principais da campanha.** Robô de segurança voador (`OSR`), sentinela
(`OSE`), mina saltadora (`OHS`), gerador de minas (`OHG`) e Bruiser (acorda pelos gatilhos,
atira balas e foguetes, fogo, terremoto, esmaga), com as mortes no resultado e os sons por
quadro. Regras em `docs/superpowers/specs/2026-11-06-m7a-inimigos-design.md`.

**M6b** os demais movimentos do Jake (furtivo, pulos, cordas, escadas, camadas, esconder,
agachar, barra, especial); **M6a** os objetos dos tutoriais (`T_J_003/005/007` vencíveis);
**M5** menu, briefing, barra, pausa e resultado.

Etapas anteriores: M4 a `T_J_001` de ponta a ponta, com a pistola e o som posicional; M1
formatos (`openhunted-formats` lê o CD, pasta ou `.iso`; [`docs/formats/`](docs/formats/README.md));
M2 visualizador; M3a o Jake corre e pula; M3b objetos sólidos e presença por modo.

Ainda não há: o equilíbrio na beirada (162/163), os passos e demais sons por quadro do Jake e
do cenário, o zumbido das minas, tesouros e segredos e a navegação das telas pelo joystick.
Os obstáculos `OPO` (gotas, bloco que cai, portão) ainda não agem.

O design e os planos estão em [`docs/superpowers/`](docs/superpowers/); as notas de engenharia
reversa, em [`docs/re/`](docs/re/).

## Jogar

O executável `openhunted` abre o menu principal do jogo.

```sh
# com a SDL3 instalada no sistema (ex.: CachyOS/Arch: pacman -S sdl3)
cargo run --release -p openhunted -- --data /caminho/Hunter_Hunted.iso

# sem SDL3 no sistema: compila a SDL3 junto (exige cmake e os cabeçalhos de X11/Wayland/áudio)
cargo run --release -p openhunted --features bundled-sdl -- --data /caminho/Hunter_Hunted.iso
```

Opções: `--level T_J_001.LEV` (pula a abertura e o menu e abre o briefing dessa fase),
`--frames N` (fecha depois de N quadros), `--no-intro` (sem a abertura). A variável
`OPENHUNTED_DATA` substitui `--data`.

Arquivos do jogador (criados na primeira gravação):

- `~/.config/openhunted/hunter.ini` (ou `$XDG_CONFIG_HOME/openhunted/`): as opções, com as
  chaves do `HUNTER.INI` original (`Difficulty`, `MusicVolume`, `SfxVolume`, `Brightness`,
  `Player1Input`/`Player2Input`, `InputKeys1/2` em códigos VK, `Joystick1/2`) e `MusicDir`,
  a pasta das faixas de música;
- `~/.local/share/openhunted/` (ou `$XDG_DATA_HOME/openhunted/`): `hunter.fam` (lista de
  heróis) e `saves/*.hhg` (jogos salvos, no formato do original).

Filmes: com o `ffmpeg` no PATH (decodificador `indeo3`), a abertura e os filmes da 7ª e da
15ª peça tocam ampliados na janela; qualquer tecla ou clique pula. Sem ele, são pulados.

Música: os menus tocam `SX_3000.WAV`. As faixas de áudio do CD não estão na imagem de dados;
para ouvi-las na partida, extraia-as do CD para uma pasta e aponte `OPENHUNTED_MUSIC` (ou
`MusicDir`) para ela: os arquivos tocam em ordem de nome e recomeçam do primeiro (`.wav`
direto; `.ogg`, `.mp3`, `.flac` e outros pelo `ffmpeg`). Com o número da faixa no nome
(`track02.wav`), o título da janela mostra o nome original ("Shock Wave"…).

Nas telas: o mouse aponta e clica; as setas levam o ponteiro ao botão mais próximo naquela
direção e Enter ou espaço clicam; Esc cancela (no briefing, volta; no resultado vindo do jogo,
nada; no menu, sai do jogo). No briefing, CIMA/BAIXO rolam o texto longo. Na tela de
resultado, uma tecla salta a contagem. Em "escolher missão", CIMA/BAIXO mudam a missão e
Enter carrega. Nos campos de nome, digita-se pelo teclado, Backspace apaga, Enter confirma e
Esc desiste (no nome do arquivo, desistir não grava). "Restore" já traz o nome do jogo salvo
mais recente.

No jogo (teclas padrão do original, trocáveis em "Input Devices" → CUSTOMIZE, onde também
se escolhe um joystick: eixos ou chapéu movem, meia inclinação anda, botões 1–6 atiram,
pulam, andam furtivo, trocam de arma e socam): setas movem o personagem (o Jake ou, nas
missões do Minotauro, o Garathe), **Shift** pula (correndo, o pulo escolhe a distância conforme o chão
adiante; com CIMA, para cima), Ctrl atira (segurando, rajadas; com as setas mira para cima e
nas diagonais; BAIXO+Ctrl é o especial), X anda furtivo (o Garathe, em pose de luta), Z é o soco (o Garathe usa a clava se a tem), PgUp/PgDn trocam de arma (1–6 escolhem direto), CIMA
usa a saída ou o interruptor quando ele está parado diante deles, agarra cordas e escadas, passa pelas portas
de camada ou o esconde, e BAIXO agacha (e engatinha com as setas) ou desce de camada; a
câmera o acompanha. P ou Pause pausa (Enter,
espaço ou OKAY retomam). A dica do tutorial pausa o jogo e fecha ao soltar uma tecla. Esc não
faz nada (como no original). Em fases sem personagem, as setas rolam a tela, PageUp/PageDown pulam
uma tela, Home/End vão às pontas e Esc volta ao menu (sem personagem, a fase nunca acaba).
Quando a janela perde o foco, o jogo pausa.
Com dois jogadores, o 2 usa por padrão I/M/J/K (mover), Espaço (atirar), Enter (pular), `;`
(furtivo), `/` (soco) e Insert/Delete (armas) no mesmo teclado, ou o dispositivo escolhido em
"Input Devices"; a pausa e o foco perdido param a partida inteira, inclusive o limite de tempo. Só o botão esquerdo do mouse clica. Tab mostra as caixas de colisão e os objetos de lógica. O
som sai pelo dispositivo de áudio padrão; sem ele, o jogo avisa uma vez e segue mudo.

Para renderizar sem janela: `ohtool --data … render T_J_001.LEV --out quadro.png [--camera X,Y]`.

## ohtool

Ferramenta de linha de comando para inspecionar e extrair os dados:

```sh
# listar os recursos (NOME, VOLUME, TAMANHO)
cargo run -p ohtool -- --data /caminho/Hunter_Hunted.iso list --type BMP

# ver um recurso (blocos, quadros; animações, cenários e interfaces em JSON)
cargo run -p ohtool -- --data /caminho/Hunter_Hunted.iso info SABOUT.ANM

# extrair: bitmaps em PNG com a paleta escolhida, fontes como folha de glifos,
# formatos estruturados em JSON, o resto como está
cargo run -p ohtool -- --data /caminho/Hunter_Hunted.iso extract --out extraido --palette SMAIN.PAL

# auditar as missões solo: objetivos, portas, chaves; --god vence cada uma no modo deus
cargo run -p ohtool -- --data /caminho/Hunter_Hunted.iso audit --god --reach

# listar as missões do hunter.hls (e gravar o cabeçalho de cada fase em JSON)
cargo run -p ohtool -- --data /caminho/Hunter_Hunted.iso levels --out fases
```

`--data` aceita a raiz do CD montado, a pasta `Hunter/`, o `RESOURCE.MAP` ou a imagem `.iso`.

## Compilar

Requer Rust estável (1.85 ou mais recente).

```sh
cargo build --release            # formatos, engine e ohtool (não precisa de SDL3)
cargo build --release -p openhunted   # visualizador: precisa da SDL3
```

O visualizador linka a SDL3 do sistema. Sem ela, o link falha com
`rust-lld: error: unable to find library -lSDL3`. Duas saídas:

- instalar a SDL3 (CachyOS/Arch: `pacman -S sdl3`; Fedora: `dnf install SDL3-devel`);
- ou compilar a SDL3 junto: `cargo build --release -p openhunted --features bundled-sdl`. Isso
  exige `cmake` e os cabeçalhos de X11, Wayland e áudio (no Ubuntu/Debian: `libx11-dev
  libxext-dev libxcursor-dev libxrandr-dev libxi-dev libxfixes-dev libxss-dev libxtst-dev
  libxkbcommon-dev libwayland-dev wayland-protocols libasound2-dev libpulse-dev libdrm-dev
  libgbm-dev libegl-dev libgl-dev`, a mesma lista da CI).

## Testes

```sh
cargo test                                            # sem o visualizador
cargo test --workspace --features openhunted/bundled-sdl   # tudo (como na CI)
```

Os testes sobre os dados reais do jogo só rodam quando a variável `OPENHUNTED_DATA` aponta
para os dados — a raiz do CD montado, a pasta `Hunter/` do CD ou uma imagem `.iso` do CD:

```sh
OPENHUNTED_DATA=/caminho/Hunter_Hunted.iso cargo test --workspace
```

Sem a variável, esses testes são pulados com um aviso.

## Licença

GPL-3.0-or-later. Veja [`LICENSE`](LICENSE).

Hunter Hunted é marca de seus respectivos titulares. Este projeto não tem relação com a
Sierra, a Dynamix ou seus sucessores.
