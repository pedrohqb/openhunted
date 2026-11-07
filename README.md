# OpenHunted

Reimplementação livre da engine de **Hunter Hunted** (K.A.A./Dynamix, Sierra On-Line, 1996)
para Linux, em Rust com SDL3.

O objetivo é recriar o jogo original com o mesmo comportamento (física, inimigos, armas,
regras das missões), rodando nativamente, com a tela original ampliada e som funcionando.

> **É preciso o CD original.** O OpenHunted lê os dados do CD do jogo (versão Windows 95 1.00).
> Nenhum dado do jogo — imagens, sons, fases, executável — é distribuído com este projeto.

## Status

**M12: as cores dos sprites.** O Jake, o Garathe, o Camaleão, o Anticorpo, o robô de
segurança, o gerador de minas e as minas flutuantes são desenhados pela tabela de cor do
`DLS.TBL` que o original monta para cada um (antes saíam com ruído de arco-íris): o Jake e o
Garathe piscam ao serem feridos, envenenados ou eletrocutados; o Camaleão tem a cor da
variante, cicla de cor ao se clonar e, invisível, só deixa ver o brilho do corpo; robôs,
geradores e minas pulsam. Regras em `docs/superpowers/specs/2026-11-07-m12-cores-design.md`.

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

**M9: a casca completa.** Todos os botões do menu principal levam às telas originais:
carregar (Restore), escolher missão (`SLEVL`, com a marca nas vencidas e o retrato do
personagem), opções (`SOPTI`: dificuldade, brilho, volumes da música e dos efeitos, gravadas
na hora), dispositivos e teclas (`SINPU`/`SKEYB`, só consulta), heróis (`SHERO`), "Sobre"
(no botão Help) e dois jogadores (`SMULT`, com o aviso de que esses modos não existem). Depois
de cada vitória que dá uma peça do carro aparece `S_CAR`; a 7ª peça traz o filme do meio
(com as vozes) e a 15ª o final e os créditos. Na contagem do resultado, a soma dos recordes
entra na lista de heróis (`hunter.fam`, pedindo o nome) e o jogo é gravado num `.hhg`
compatível com o original (o nome do arquivo é pedido na primeira vez). A música dos menus
(`SX_3000.WAV`) toca em laço; na partida, as faixas de uma pasta tomam o lugar das do CD. Os
filmes AVI (Indeo 3.2) tocam pelo `ffmpeg`, se houver. Regras e simplificações em
`docs/superpowers/specs/2026-11-06-m9-casca-design.md`.

**M7b: o arsenal do Jake e os itens da campanha.** Além da pistola, o Jake soca (Z, ou
ATIRAR com os punhos: 8 de dano na frente), atira com a espingarda (o chumbo se divide em
quatro, com cartucho), o lança-foguetes (20 diretos e explosão de raio 64 que também fere o
Jake de perto), as estrelas ninja (voo em onda; na parede cravam e podem ser recolhidas) e as
granadas (quicam, explodem pelo pavio ou ao tocar um objeto). PgUp/PgDn trocam de arma na
ordem do original (1–6 escolhem direto, extensão do port); sem munição, ele passa à melhor
arma. Os itens da campanha funcionam: munição das armas, comida, veneno (perde vida a cada
20 tiques) e antídoto, armaduras, chaves (abrem as portas de célula empurrando, sem serem
gastas), bandeiras (objetivo "acionar"), o pistão (anda pelos gatilhos e esmaga) e o
interruptor (CIMA diante dele).

**M7a: os inimigos principais da campanha.** O robô de segurança voador (`OSR`) flutua,
patrulha, persegue o Jake a 80–200 px e dispara raios laser alternando os dois canhões, em
rajadas; não vê o Jake escondido contra a parede; atingido, a vida cai a 45 e, abaixo de 0,
ele cai e explode dez vezes. A sentinela (`OSE`) abre o casulo diante do Jake e dispara
rajadas de três balas (fechada, é invulnerável). A mina saltadora (`OHS`) dá cinco pulinhos e
um pulo grande para o lado em que olha e explode ao tocar o Jake. O gerador (`OHG`) solta
minas flutuantes em levas, até quatro vivas. O Bruiser acorda pelos gatilhos, anda, atira
balas e foguetes, põe fogo no chão, faz o chão tremer (o Jake cambaleia) e esmaga quem está
embaixo; leva metade do dano, é imune a fogo e morre em 30 explosões. As mortes por bala do
jogador contam no resultado; os sons de cada um tocam pelos quadros (`SOUND.TBL`). Regras e
simplificações em `docs/superpowers/specs/2026-11-06-m7a-inimigos-design.md`.

**M6b** os demais movimentos do Jake (furtivo, pulos, cordas, escadas, camadas, esconder,
agachar, barra, especial); **M6a** os objetos dos tutoriais (`T_J_003/005/007` vencíveis);
**M5** menu, briefing, barra, pausa e resultado.

Etapas anteriores: M4 a `T_J_001` de ponta a ponta, com a pistola e o som posicional; M1
formatos (`openhunted-formats` lê o CD, pasta ou `.iso`; [`docs/formats/`](docs/formats/README.md));
M2 visualizador; M3a o Jake corre e pula; M3b objetos sólidos e presença por modo.

Ainda não há: o equilíbrio na beirada (162/163), os passos e
demais sons por quadro do Jake e do cenário, o zumbido das minas, o logotipo, o título e a
história da abertura (só o filme toca) e o laço de demonstração, a troca de teclas e o joystick,
os modos de dois jogadores (e a cor escolhida para cada um), tesouros e segredos, a luz por
região sobre os objetos. Os obstáculos `OPO` (gotas, bloco que cai, portão) ainda não agem.

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
`--frames N` (fecha depois de N quadros), `--no-intro` (sem o filme de abertura). A variável
`OPENHUNTED_DATA` substitui `--data`.

Arquivos do jogador (criados na primeira gravação):

- `~/.config/openhunted/hunter.ini` (ou `$XDG_CONFIG_HOME/openhunted/`): as opções, com as
  chaves do `HUNTER.INI` original (`Difficulty`, `MusicVolume`, `SfxVolume`, `Brightness`)
  e `MusicDir`, a pasta das faixas de música;
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

No jogo: setas movem o personagem (o Jake ou, nas missões do Minotauro, o Garathe), espaço pula (correndo, o pulo escolhe a distância conforme o chão
adiante; com CIMA, para cima), Ctrl atira (segurando, rajadas; com as setas mira para cima e
nas diagonais; BAIXO+Ctrl é o especial), X anda furtivo (o Garathe, em pose de luta), Z é o soco (o Garathe usa a clava se a tem), PgUp/PgDn trocam de arma (1–6 escolhem direto), CIMA
usa a saída ou o interruptor quando ele está parado diante deles, agarra cordas e escadas, passa pelas portas
de camada ou o esconde, e BAIXO agacha (e engatinha com as setas) ou desce de camada; a
câmera o acompanha. P ou Pause pausa (Enter,
espaço ou OKAY retomam). A dica do tutorial pausa o jogo e fecha ao soltar uma tecla. Esc não
faz nada (como no original). Em fases sem personagem, as setas rolam a tela, PageUp/PageDown pulam
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
