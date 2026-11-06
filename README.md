# OpenHunted

Reimplementação livre da engine de **Hunter Hunted** (K.A.A./Dynamix, Sierra On-Line, 1996)
para Linux, em Rust com SDL3.

O objetivo é recriar o jogo original com o mesmo comportamento (física, inimigos, armas,
regras das missões), rodando nativamente, com a tela original ampliada e som funcionando.

> **É preciso o CD original.** O OpenHunted lê os dados do CD do jogo (versão Windows 95 1.00).
> Nenhum dado do jogo — imagens, sons, fases, executável — é distribuído com este projeto.

## Status

**M5: as telas em volta do jogo.** O jogo abre no menu principal original (`SMAIN.ITF`, com
fogo, faíscas e botões animados, mouse ou teclado); "New Game" começa a campanha solo na
primeira missão do `ENGLEVEL.TXT`, com o briefing (`SLEVD.ITF`: título e descrição, rolagem,
GO/CANCEL) sobre o primeiro quadro da fase. A partida ocupa 640×405 com a barra de estado
original embaixo (`INVBAR01.BMP`: vida, rosto, armadura, armas com munição, cronômetro de
rolo); P ou Pause pausa ("GAME PAUSED"). No fim, a tela de resultado (`SMISS.ITF`) conta
mortos, pontos e bônus de vitória, guarda o recorde da missão na sessão e oferece REPLAY,
NEXT (a próxima missão solo) e MAIN MENU. Decisão do port: enquanto o Garathe não existe, o
NEXT pula as missões sem o Jake controlável (as do Garathe); uma fase aberta com `--level` fora
da lista de missões não grava recorde e o NEXT dela volta ao menu.

**M4c: efeitos sonoros.** **M4b: a pistola.** **M4a: a primeira missão é jogável de ponta a ponta.** Na `T_J_001`, o
Jake corre, pula e cai, colide com o cenário e com os objetos sólidos, apanha a vida e a
armadura, lê as dicas do tutorial, atira com a pistola (rajadas de 3, mira para cima e nas
diagonais, munição no HUD) nas minas flutuantes que o perseguem (ou elas explodem nele,
tiram vida, empurram e derrubam), chega à saída, aperta CIMA e vence; se a vida acaba, morre
e perde. Os efeitos sonoros tocam (tiro, cápsulas, faíscas, explosões, coleta, dicas, choque
da saída), posicionados em relação à câmera, com volume e pan pela distância e 16 canais com
prioridade, como no original. Tudo com as
constantes e as regras lidas do executável original (`docs/re/`).

Etapas anteriores: M1 formatos (a biblioteca `openhunted-formats` lê os dados do CD, pasta ou
`.iso`: bitmaps, paletas, sons, textos, animações, cenários, interfaces, fontes, fases;
estado em [`docs/formats/`](docs/formats/README.md)); M2 visualizador de fase; M3a o Jake
corre e pula; M3b objetos sólidos e presença por modo de jogo.

Ainda não há: as outras armas, o soco e a troca de arma; inimigos ativos, escadas, furtividade,
música, os passos e demais sons por quadro do Jake e do cenário, o zumbido das minas, a
abertura (logotipo, filme, história, créditos), os demais botões do menu (carregar, escolher
missão, opções, dispositivos, heróis, ajuda, dois jogadores), tesouros e segredos, gravação de
recordes e de jogo, outras missões jogáveis, o Garathe. Os obstáculos `OPO` (gotas, bloco que cai,
portão) ainda não agem. Nas missões do Garathe não há personagem controlável (as setas rolam a
tela).

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

Opções: `--level T_J_001.LEV` (pula o menu e abre o briefing dessa fase), `--frames N` (fecha
depois de N quadros). A variável `OPENHUNTED_DATA` substitui `--data`.

Nas telas: o mouse aponta e clica; as setas levam o ponteiro ao botão mais próximo naquela
direção e Enter ou espaço clicam; Esc cancela (no briefing, volta; no resultado vindo do jogo,
nada; no menu, sai do jogo). No briefing, CIMA/BAIXO rolam o texto longo. Na tela de
resultado, uma tecla salta a contagem.

No jogo: setas movem o Jake, espaço pula (correndo, o pulo escolhe a distância conforme o chão
adiante), Ctrl atira (segurando, rajadas; com as setas mira para cima e nas diagonais) e CIMA
usa a saída quando ele está parado diante dela; a câmera o acompanha. P ou Pause pausa (Enter,
espaço ou OKAY retomam). A dica do tutorial pausa o jogo e fecha ao soltar uma tecla. Esc não
faz nada (como no original). Em fases sem o Jake, as setas rolam a tela, PageUp/PageDown pulam
uma tela, Home/End vão às pontas e Esc volta ao menu (sem personagem, a fase nunca acaba).
Quando a janela perde o foco, o jogo pausa. Só o botão esquerdo do mouse clica. Tab mostra as caixas de colisão e os objetos de lógica. O
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
