# OpenHunted

Reimplementação livre da engine de **Hunter Hunted** (K.A.A./Dynamix, Sierra On-Line, 1996)
para Linux, em Rust com SDL3.

O objetivo é recriar o jogo original com o mesmo comportamento (física, inimigos, armas,
regras das missões), rodando nativamente, com a tela original ampliada e som funcionando.

> **É preciso o CD original.** O OpenHunted lê os dados do CD do jogo (versão Windows 95 1.00).
> Nenhum dado do jogo — imagens, sons, fases, executável — é distribuído com este projeto.

## Status

**M3b: objetos sólidos.** **M3a: o Jake corre, pula e cai.** **M2 concluída: visualizador de fase.** **M1 concluída: formatos.** A biblioteca `openhunted-formats` lê os dados do CD (pasta ou
imagem `.iso`): bitmaps, paletas, sons, textos, animações, cenários, interfaces, fontes, o
arquivo de fases e o cabeçalho das fases. O estado de cada formato está em
[`docs/formats/`](docs/formats/README.md). Na fase, o Jake já corre, pula (pulos médio, longo e
estendido), cai e colide com o cenário e com os objetos sólidos (peças de pedra, paredes e tetos que são
objetos), com as constantes e as regras do original. Os obstáculos `OPO` (gotas, bloco que cai,
portão) ainda não agem: isso vem com o comportamento dos objetos. A paleta já segue a do Windows (os
sprites não mostram mais magenta). A cena mostra só os objetos do modo solo; nas missões do Garathe ainda não há
personagem controlável (as setas rolam a tela). Os objetos ainda
ficam parados; não há armas, inimigos ativos, escadas nem furtividade.

O design e os planos estão em [`docs/superpowers/`](docs/superpowers/); as notas de engenharia
reversa, em [`docs/re/`](docs/re/).

## Visualizador de fase (M2)

O executável `openhunted` abre uma fase numa janela, com o cenário, as decorações e os objetos
animados. Ainda não há jogabilidade.

```sh
# com a SDL3 instalada no sistema (ex.: CachyOS/Arch: pacman -S sdl3)
cargo run --release -p openhunted -- --data /caminho/Hunter_Hunted.iso

# sem SDL3 no sistema: compila a SDL3 junto (exige cmake e os cabeçalhos de X11/Wayland/áudio)
cargo run --release -p openhunted --features bundled-sdl -- --data /caminho/Hunter_Hunted.iso
```

Opções: `--level T_J_001.LEV` (padrão: o primeiro tutorial do Jake), `--frames N` (fecha depois
de N quadros). Teclas: setas movem o Jake e espaço pula (correndo, o pulo escolhe a distância
conforme o chão adiante); a câmera o acompanha. Em fases sem o Jake, as setas rolam a tela,
PageUp/PageDown pulam uma tela e Home/End vão às pontas. Tab mostra as caixas de colisão e os
objetos de lógica; Esc sai. A variável `OPENHUNTED_DATA`
substitui `--data`.

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
