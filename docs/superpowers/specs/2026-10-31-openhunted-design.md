# OpenHunted — design

Data: 2026-10-31
Status: aprovado em conversa; aguardando revisão desta spec

## 1. Objetivo

OpenHunted é uma reimplementação livre da engine de **Hunter Hunted** (K.A.A./Dynamix,
Sierra On-Line, 1996, Windows 95) para Linux, em Rust com SDL3. A engine lê os dados
originais do CD do usuário; nenhum dado original é distribuído.

O alvo é **fidelidade ao jogo original**: o mesmo comportamento (física, IA, armas,
regras de missão), rodando nativamente, com a tela original ampliada e som funcional.
Melhorias (alta resolução, widescreen, rede) ficam para depois e não orientam o design
inicial, exceto pela separação engine/front-end que as torna possíveis.

### Situação anterior (pesquisa de 2026-10-31)

Não existe port, reimplementação nem engine aberta do jogo. O motor DGDS do ScummVM
cobre as aventuras da Dynamix, não este jogo. Hoje ele roda via Wine (instalador do
Lutris) ou via Windows 95 emulado no DOSBox. A referência mais próxima é o
[open-scrantic](https://github.com/glauberlima/open-scrantic) (Rust, MIT), que lê o
mesmo formato de volume de recursos da Dynamix no Johnny Castaway.

### Primeira entrega (critério de sucesso)

Uma missão para um jogador, a primeira do Jake, jogável de ponta a ponta: menu
mínimo → carregar a missão → Jake correndo, pulando, agarrando, escondendo-se e
atirando → inimigos com a IA dessa missão → vitória ou morte. O comportamento é
conferido lado a lado com o original rodando no Wine.

### Fora da primeira entrega

Música (faixas de áudio do CD), vídeos Indeo 3.2, modo para dois jogadores (que é
local, no mesmo PC: o executável não tem bibliotecas de rede), Garathe (Minotauro),
demais missões, editor, Flatpak, alta resolução.

## 2. O que se sabe dos dados (versão 1.00, ISO "Hunter-Hunted")

| Arquivo | Conteúdo |
|---|---|
| `Hunter/Hunter.exe` | PE32 i386, C++ com RTTI; importa GDI32, USER32, WINMM, DSOUND (sem DirectDraw, sem rede). Classes de compressão `LZHuffman` e `RLERStream`. |
| `Hunter/RESOURCE.MAP` | Índice: 4 bytes de cabeçalho, `u16` número de volumes; por volume, nome com 13 bytes, `u16` contagem e entradas `(u32 hash, u32 offset)`. 2.312 entradas em 2 volumes. |
| `Hunter/RESOURCE.001`, `.002` | Volumes: em cada offset, nome com 13 bytes, `u32` tamanho e dados. |
| `Hunter/hunter.hls` | Volume próprio com o índice embutido no início, 201 entradas; provavelmente as fases (`.LEV`, listadas em `LEVELS.RES`), comprimidas. |
| `Movies/*.AVI` | 9 vídeos em Indeo 3.2 (`IV32`). |
| Faixas de áudio do CD | Música, via `mciSendString`; não estão na ISO. |

Tipos de recurso no volume (contagem): BMP 1654, WAV 280 (RIFF comum), BRD 225, ANM 87,
PAL 26 (RIFF PAL comum), ITF 16, RES 13 (texto), PFT 5 (fonte, magic `PFNT`), TBL 3,
SKE 2 (esqueleto), VCR 1.

Os recursos próprios são sequências de blocos `TAG:` + `u32` tamanho (`BMP:`, `ANM:`
com `TAG:`/`LAB:`/`PAL:`, `HED:` + `BDR:` nos BRD, `ITF:` + `TAG:`, `VCR:` + `TAG:`).
Ainda não se sabe onde entram LZHuffman e RLE.

## 3. Abordagem

Os formatos são decifrados primeiro, a partir das rotinas de leitura do `Hunter.exe`
no Ghidra. Depois a lógica do jogo é desmontada **um sistema por vez**, na ordem que a
missão-alvo exige, e cada sistema é conferido contra o original no Wine.

Abordagens descartadas:
- **Imitação por observação** (desmontar só os formatos e medir o comportamento):
  mais rápida, mas a física e a IA ficam aproximadas.
- **Recompilação estática do x86**: idêntica ao original, mas gera código ilegível,
  impossível de manter e dependente de Win32.

## 4. Arquitetura

Workspace Cargo com quatro crates:

### `openhunted-formats` (biblioteca, sem SDL)
Lê os dados originais e entrega estruturas Rust simples.
- `volume`: `RESOURCE.MAP` + volumes; `hunter.hls`; busca de recurso por nome
  (sem diferenciar maiúsculas de minúsculas).
- `compress`: LZHuffman e RLE.
- `chunk`: leitor de blocos `TAG:`.
- Decodificadores: `Bitmap` (indexado, 8 bits), `Palette` (256 × RGB), `Animation`,
  `Border` (BRD), `Level`, `Interface` (ITF), `Font` (PFT), `Sound` (PCM), `TextList` (RES).
- Fonte de dados abstrata (`DataSource`): uma pasta ou um arquivo `.iso` (leitor
  mínimo de ISO 9660/Joliet), para que o usuário não precise extrair nada.
- Erros tipados (`thiserror`); nenhum pânico com dados corrompidos.

### `ohtool` (binário de linha de comando)
`list`, `extract` (PNG/WAV/JSON), `info <recurso>`. Serve para conferir formatos e
apoiar a engenharia reversa. Depende só de `openhunted-formats`.

### `openhunted-engine` (biblioteca, sem SDL)
A simulação, determinística e com passo fixo no ritmo do timer original (a frequência
exata sai do executável: `timeSetEvent`).
- Entrada por tick: `PlayerInput` com ações abstratas por jogador (direções, pular,
  atirar, furtividade, trocar arma).
- Saída por tick: lista de desenho (sprite, posição, quadro, camada, espelhamento)
  e eventos de som.
- Mundo, entidades, colisão, física, armas, dano, IA e regras de missão.
- Sem relógio de parede, sem aleatoriedade fora de um RNG semeado: replays são
  reproduzíveis.

### `openhunted` (binário, camada SDL3)
Janela, mapeamento de teclado e controle para `PlayerInput`, renderizador (lista de
desenho → framebuffer 640×480 de 8 bits → conversão pela paleta → textura ampliada com
proporção preservada), mixer de som sobre o áudio do SDL3, laço de tempo fixo,
configuração e localização dos dados.

### Documentação no repositório
- `docs/formats/<formato>.md`: especificação de cada formato.
- `docs/re/<sistema>.md`: notas de engenharia reversa com os endereços das funções no
  `Hunter.exe`, para cada comportamento portado ser rastreável até o original.

## 5. Fluxo de dados

```
CD/pasta/.iso ──DataSource──▶ volume ──▶ decodificadores ──▶ assets carregados
                                                                │
SDL eventos ─▶ PlayerInput ─▶ engine.tick() ─▶ lista de desenho + sons
                                                    │              │
                                     renderizador 8 bits      mixer SDL3
```

## 6. Etapas

- **M0, fundação:** README (os dados vêm do CD do usuário), LICENSE
  GPL-3.0-or-later, workspace, CI (fmt, clippy, testes), notas iniciais do executável.
- **M1, formatos:** volume → compressão → BMP + PAL; meta: `ohtool` extrai as 1.654
  imagens em PNG corretas. Depois ANM, BRD, ITF, PFT, RES e as fases do `.hls`, cada um
  com sua página em `docs/formats/`.
- **M2, visualizador de fase** (marco interno): o executável abre a missão-alvo com
  cenário, rolagem e objetos animados, sem jogabilidade.
- **M3, Jake:** controles, movimento, pulo, agarrar no teto e na escada, esconder na
  parede, furtividade, colisão, animações, câmera.
- **M4, combate:** armas, projéteis, dano (`DAMAGE.TBL`), itens, efeitos sonoros.
- **M5, primeira entrega:** inimigos e IA da missão, objetivo, vitória e morte, menu
  mínimo.

Antes de começar a M3, revisar se todo o comportamento do Jake deve ser desmontado ou
se partes podem ser medidas no original, conforme a legibilidade do executável no Ghidra.

## 7. Dados, configuração e erros

- Dados: `--data <caminho>` (pasta do CD montado, cópia de `Hunter/` ou `.iso`); o
  caminho fica gravado na configuração. Na abertura, conferir tamanhos e hashes contra
  a versão 1.00 conhecida e avisar se a versão for outra.
- Configuração em `~/.config/openhunted/config.toml`; saves em
  `~/.local/share/openhunted/` (XDG).
- Faltando dado ou com um formato falhando, o executável mostra uma caixa de mensagem
  do SDL com o motivo, e não só uma saída no terminal.

## 8. Testes

1. Unitários com dados sintéticos (podem ficar no repositório).
2. De integração sobre os dados reais, ativados por `OPENHUNTED_DATA=<caminho>` e
   pulados na CI; hashes de referência das imagens decodificadas.
3. De replay na engine: uma sequência de entradas gravada produz sempre o mesmo estado.

## 9. Cuidados com a engenharia reversa e os dados

- Nenhum dado original e nenhum trecho do executável entra no repositório.
- O código é reescrito a partir das notas de `docs/re/`, e não copiado do pseudocódigo
  do Ghidra.
- Licença: GPL-3.0-or-later.

## 10. Dependências previstas

`sdl3` (camada SDL), `thiserror` (erros); no `ohtool`, `png`, `serde`, `serde_json`;
`toml` para a configuração. As versões se fixam no plano de implementação.
