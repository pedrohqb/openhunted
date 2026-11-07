# OpenHunted

Reimplementação livre da engine de **Hunter Hunted** (K.A.A./Dynamix, Sierra On-Line, 1996)
para Linux, em Rust com SDL3.

O objetivo é recriar o jogo original com o mesmo comportamento (física, inimigos, armas,
regras das missões), rodando nativamente, com a tela original ampliada e som funcionando.

> **É preciso o CD original.** O OpenHunted lê os dados do CD do jogo (versão Windows 95 1.00).
> Nenhum dado do jogo — imagens, sons, fases, executável — é distribuído com este projeto.

## Status

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

**M6b: os demais movimentos do Jake.** O esqueleto `JAKE.SKE` é lido e move o Jake como no
original: o andar furtivo (X), o impulso do pulo estendido, os pulos parado e para cima (com a
tabela de velocidades e o controle no ar), as aterrissagens que deslizam, a escalada, o
engatinhar e a barra; o desenho acompanha o deslocamento da captura. CIMA diante de uma
corda, escada ou rede (zona PR 4) agarra e escala (em pé ou no ar, sem PULAR), na vertical,
de lado e nas diagonais; PULAR solta em pulo. As portas de camada (zonas 1 e 2) levam o Jake
de uma camada à outra com CIMA/BAIXO, e o cenário sólido passa a ser o da camada nova. CIMA
sem zona esconde o Jake contra a parede; BAIXO agacha e engatinha (sob teto baixo não
levanta); o pulo para cima contra um teto de cenário pendura na barra (mão ante mão, puxar o
corpo). BAIXO+ATIRAR com vida acima de 25 é o especial: custa 3 de vida e destrói ou fere o
que estiver a menos de 70 px. Com os objetos da M6a, as missões tutoriais `T_J_003`, `T_J_005`
e `T_J_007` podem ser vencidas de ponta a ponta (testes `t_j_00x_can_be_won`).

**M6a: os objetos das missões tutoriais `T_J_003/005/007`.** O olho-laser varre o teto, vê
o Jake dentro do cone do feixe e dispara rajadas de raios (5 de dano com tranco; morre com um
tiro e explode); o barril tomba quando o Jake encosta, rola empurrado, explode com tiro, fogo,
raio, explosão ou batida rápida, em cadeia; o fogo cresce e queima 1 por tique; a cerca
elétrica faísca e dá choque; o grafite escreve o texto do `GRAFFITI.RES`; a peça do carro
flutua e, apanhada, cumpre o objetivo; o teletransporte some com o Jake e o põe no destino; as
paredes e portas `OH` abrem por empurrão ou gatilho e fecham por tempo; a pistola, a munição,
a comida, as armaduras, o lança-foguetes e as granadas são apanhados.

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

Ainda não há: os demais inimigos (Morte, Camaleão,
catapulta, dardos, Anticorpo, Manta, Entidade, Minotauro), o equilíbrio na beirada (162/163),
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
adiante; com CIMA, para cima), Ctrl atira (segurando, rajadas; com as setas mira para cima e
nas diagonais; BAIXO+Ctrl é o especial), X anda furtivo, Z é o soco, PgUp/PgDn trocam de arma (1–6 escolhem direto), CIMA
usa a saída ou o interruptor quando ele está parado diante deles, agarra cordas e escadas, passa pelas portas
de camada ou o esconde, e BAIXO agacha (e engatinha com as setas) ou desce de camada; a
câmera o acompanha. P ou Pause pausa (Enter,
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
