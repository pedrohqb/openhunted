# Oráculo: o `Hunter.exe` original no Wine, medido tique a tique

`tools/oraculo/` roda o executável original (a cópia do CD do usuário, versão 1.00, md5
`20bd0feca616fb59f9360cbfb77544db`) num X virtual, entra numa fase, aplica uma entrada
roteirizada **tique a tique** e grava o estado dos objetos lido da memória do processo. O
rastro sai no mesmo formato do `ohtool render --trace-json` do porte, e `compare.py` mostra
as diferenças. Ferramenta de medição; nada do jogo nem do prefixo do Wine vai para o
repositório.

## Instalação

```
tools/oraculo/instalar.sh /caminho/do/cd [/home/claude/oraculo/prefix]
```

O que o script faz (Ubuntu 24.04, root), e por quê:

1. `dpkg --add-architecture i386` e `apt-get install wine wine32:i386 xdotool x11-utils
   imagemagick xauth` (Wine 9.0; o `Xvfb` já existe). O jogo é PE32, usa WinG/GDI (não
   DirectDraw) e DirectSound; roda no Wine sem nenhuma DLL extra.
2. `oraculo.py setup`: prefixo `WINEARCH=win32` (`wineboot -i`, ~30 s); copia `Hunter/`
   do CD (5 arquivos, 66 MB) para `C:\Sierra\Hunter` e mais:
   - `\WINDOWS\SIERRA.INI` com `[Hunter] InstallPath=C:\Sierra\Hunter` e
     `CDPath=C:\Sierra\Hunter`. Sem ele: caixa "Hunter Hunted not Installed" (`0x4031db`).
     `InstallPath\hunter.exe` tem de existir; `CDPath` é onde estão os `RESOURCE.*`.
   - `sierra.inf` (do CD) na pasta do jogo. Sem ele o executável relança
     `"%s\hunter.exe -i%c"` e sai (`0x4032f3`), em laço infinito.
3. Na execução: `Hunter.INI` com `Difficulty` (padrão 1), `Player1Input=Keyboard1`,
   música desligada; `WINEDLLOVERRIDES=winealsa.drv,winepulse.drv=d` (sem áudio: o jogo
   segue sem DirectSound); `WINEDEBUG=-all`. Os filmes (`Movies\`) não são copiados: a
   abertura é interrompida por tecla. Memória do jogo: ~50 MB.

## Como funciona

- **Sincronismo sem tocar no código.** O `oraculo.py` rastreia a thread principal do Wine
  com `PTRACE_SEIZE` e usa os **registradores de depuração** (DR0–DR2, execução), não `int3`:
  - DR0 = `0x4043a3`, no laço `0x40433c` logo depois da leitura dos dispositivos
    (`0x448366`) e antes da física. Em cada parada o estado visível é o do **fim do tique
    anterior** (depois da efetivação `0x404bab`), que vira a linha `t` do rastro; então a
    entrada do tique seguinte é escrita.
  - DR1 = `0x43124b` (carga da fase): troca o nome `HUNTER.HLS|T_J_001.lev` pela fase
    pedida e reinicia o `rand`.
  - DR2 = `0x4156f7`, fim da dica modal do "mago" (`dicas.md`): a dica relê o teclado de
    verdade (nada apertado) no meio do tique; o oráculo repõe a entrada atual e a anterior,
    como um jogador que continua segurando as teclas.
- **Entrada.** A estrutura do teclado do jogador (`[0x499014 + 4·n]`, n = `+0x198`):
  eixos `+0x94/+0x96` = ±128, botões `+0x98…+0x9d` (atirar, pular, furtivo, próxima,
  anterior, soco). `0x407e2d` os converte em `0x487a2c` dentro da atualização do Jake;
  como o jogo copia `+0x8c…+0x9f` para `+0xa0` antes de ler, as bordas (0x10…0x80,
  0x2000…) saem certas. O teclado real é lido pelo jogo (`0x449e38`:
  `GetAsyncKeyState`/`GetKeyState`) mas sobrescrito antes do uso. Mesma sintaxe de letras do `ohtool render --input`.
- **Reprodutível.** O `rand` (`0x460a74`, aditivo atrasado de 56 palavras em `0x483920`,
  índices `0x483918/0x48391c`) nunca é semeado pelo relógio: o oráculo grava nele o estado
  do arquivo na carga da fase **e** no tique 0 (o briefing sorteia conforme o tempo aberto).
  Duas execuções com a mesma entrada dão rastros idênticos (conferido com 110 tiques,
  dica modal e minas explodindo no meio).
- **Rápido.** `0x471988 = 0` zera a espera de 50 ms do fim do laço (`0x404580`); o passo
  continua um tique por volta e o rastro é idêntico ao do tempo real (`--tempo-real`
  mantém os 50 ms). 5–30 s de menus (conforme a carga da máquina) + ~5 ms por tique.
- **Menus.** Return a cada segundo: interrompe a abertura (qualquer tecla) e aciona o botão
  padrão do menu (New Game). DR1 troca a fase; o briefing põe o ponteiro no centro do GO
  (`briefing.md`) e o oráculo clica ali (Return a cada 3 s de reserva) até o tique 0.
  Falha nos menus deixa a tela em `/tmp/oraculo-falha.png`.
- **Objetos.** Como `0x43cfcf`: a lista global `0x488e7c` (ordenada por `+0x1b4`,
  `"lista": "g"`), as listas das células (`[0x488e84]` com `gx·gy·gz` ponteiros, lista em
  `+0xd0`, `"c"`: itens, decoração animada) e a dos destruídos `0x488e80` (`"m"`). Item
  apanhado continua na célula só com `+0x178 = 0x40`.

## Uso

```
# original (prefixo e display padrão: /home/claude/oraculo/prefix e :77; Xvfb sobe sozinho)
tools/oraculo/oraculo.py run --level T_J_001 --input "20*,J,40*,30*L" --ticks 90 \
    --out orig.jsonl [--shots pasta --shot-every 10] [--todos] [--inicial] [--difficulty 1]
# porte, mesma entrada
ohtool --data /caminho/do/cd render T_J_001.LEV --out /tmp/x.png --ticks 90 \
    --input "20*,J,40*,30*L" --trace-json porte.jsonl
# diferenças
tools/oraculo/compare.py orig.jsonl porte.jsonl [--objetos] [--defasagem 1] \
    [--campos x,y,vx,vy,rot,q] [--tudo] [--linhas 80]
```

Rastro (JSON lines): uma linha `meta` (fase, entrada, md5, dificuldade, índices do `rand`
no tique 0, deslocamentos) e uma por tique: `t`, `in` (bits aplicados), `in_jogo`
(`0x487a2c` da última atualização), `tique` (`0x4879c4`), `rng`, `modal` (o tique abriu uma
dica), `vista` (centro x, y e distância `d` da câmera da 1ª vista, `[0x493b70]`) e `obj`. Cada objeto: `id` (estável na execução), `lista`, `tipo` (`+0x18`), `x`/`y`
(`+0x14c/+0x150`), `vx`/`vy` (`+0x164/+0x168`, 1/8 px por tique), `fl` (`+0x178`), `cam`
(`+0x17c`), `est` (`+0x180`), `q` (`+0x184`, quadro dentro do rótulo), `rot` (`+0x1ec`),
`esp` (`+0x194`), `vida`/`arm` (`+0x258/+0x25c`). O jogador (`+0x19c ≠ 0`) traz ainda
`jogador`, `arma` (`+0x254`), `veneno` (`+0x270`), `lado` (`+0x288`: 4 esq., 8 dir.),
`recarga` (`+0x294`), `c260/c264/c268` e `inv` (pares não nulos de `+0x28`, índice = arma).
`--todos` inclui os objetos de lógica (tipos 0, 27, 30, 73). Campos internos de um tipo:
`ORACULO_CAMPOS="ori=0x278,qa=0x190,hx=0x26c:b" ORACULO_TIPOS=8` (deslocamento e formato
`struct`, padrão `i`) os acrescenta aos objetos desses tipos (`memoria.py`).

`compare.py` casa os objetos pelo tipo e pela posição no primeiro tique (`--raio`, 24 px);
no jogador compara posição, velocidade, rótulo, quadro, camada, vida, armadura, arma,
veneno, lado e inventário, e mostra lado a lado `est` do original (número) e do porte
(nome). `--defasagem K` compara porte[t] com original[t + K].

## Limites

- Só o caminho solo "New Game" (a fase pedida entra no lugar da `T_J_001`; inventário e
  vida vêm da fase, como no início de campanha). Dois jogadores e jogo salvo, não.
- O `rand` do porte é um LCG (`rng.rs`) com outra semente: tudo que sorteia (quadro
  inicial das minas e itens, rumo das minas, par de estados do tiro) diverge por
  construção. Para alinhar, o porte teria de usar o gerador do original com a tabela
  inicial do executável e as mesmas chamadas.
- W1…W6 (escolha direta de arma) é extensão do porte: ignorado no original.
- O porte não tem velocidade nem estado numérico por entidade no `--trace-json`; dos objetos
  comparam-se posição, camada, vida, rótulo, quadro e presença. `fl` existe nos dois, mas o
  porte não reproduz todos os bits (ex.: 0x4).
- Os objetos sem rótulo no original (`rot` 0, ex. minas 102) têm em `q` o quadro absoluto.
- Morte ou fim de missão param o laço: o rastro é truncado (aviso no fim).
- Teclas que o jogo trata fora do laço (F1 ajuda) não são enviadas; a pausa (`0x47119c`)
  é detectada e não conta tique.

## Demonstração: Jake, pulo parado e corrida na `T_J_001`

Entrada `20*,J,40*,30*L` (20 tiques parado, PULAR por 1 tique, 40 parado, 30 para a
esquerda), 90 tiques; `compare.py`: x difere em 29 tiques, y 7, vx 29, vy 37, q 17, vida
1; rótulo, camada, arma, inventário iguais.

| t | in | original rót/q, (x,y), (vx,vy) | porte rót/q, (x,y), (vx,vy) |
|---|---|---|---|
| 0–4 | 0 | queda 1→25→26: y 48, 52, 58, 66, 69 | idêntico (posição, vy, rótulos) |
| 20 | J | 27/0 (1696,69) (0,0) | 27/0 (1696,69) (0,0) |
| 28 | 0 | 27/8 (1696,69) (0,0) | 27/8 (1696,69) (0,0) |
| 29 | 0 | 27/9 (1696,**69**) (0,**−144**) | 27/9 (1696,**53**) (0,**−128**) |
| 30 | 0 | 27/10 (1696,53) (0,−144) | 27/10 (1696,37) (0,−128) |
| 31–34 | 0 | y 37, 27, 26, 34 | y 27, 26, 34, 48 |
| 35 | 0 | 27/15 (1696,48) (0,216) | 27/15 (1696,69) (0,−116) |
| 36 | 0 | 27/16 (1696,69) (0,48) | 27/16 (1696,69) (0,−32) |
| 46–60 | 0 | 1/1, 1/1, 1/2, 1/2 … 1/8 | 1/0, 1/0, 1/1, 1/1 … 1/7 |
| 61 | L | 10/0 (**1696**,69) (−96,−8) | 10/0 (**1684**,69) (−89,0) |
| 62–89 | L | −12 px/tique a partir de t 62 | −12 px/tique a partir de t 61 |
| 73 | L | vida 50 (pega o OHE em t 74) | vida 75 |

Leitura:

1. **Velocidade da máquina de estados um tique antes.** No original a máquina de estados
   roda depois da física (`personagem.md` §1.2): a velocidade que o estado grava (pulo em
   q9, corrida em t 61) só move o corpo no tique seguinte. O porte move no mesmo tique. A
   queda inicial (velocidade só da gravidade, aplicada na física) bate exatamente. Com
   `--defasagem 1`, x fica igual em toda a corrida e o arco do pulo coincide
   (69→53→37→27→26→34→48→69). Efeito: tudo que depende de posição (coleta da vida em t 73
   × 74, colisões) chega um tique antes no porte.
2. **`vy` guardado.** O original guarda em `+0x168` a velocidade antes da gravidade da
   física seguinte (−144 + 16 = −128 → 16 px); o porte guarda a já aplicada. Na corrida
   o original tem (−96, −8) e o porte (−89, 0), com o mesmo deslocamento de 12 px.
3. **Animação parada (rótulo 1).** Ao entrar no rótulo 1, o original fica 1 tique no
   quadro 0 e depois 2 tiques por quadro (0,1,1,2,2…); o porte, 2 tiques já no quadro 0
   (0,0,1,1…). Saindo do pulo (t 46) o original entra direto no quadro 1.
4. **Pulo correndo.** Com `30*L,J` (PULAR sem direção logo depois da corrida), o original
   vai ao rótulo 29 (pulo correndo, vx −80) e o porte ao 27 (pulo parado).
5. **Garathe** (`T_M_002`, `10*,20*R`): o original corre 14 px/tique (vx 120), o porte 15
   (vx 113), também um tique antes.
6. **Objetos.** Minas (102) e itens (tipos 79 e 32) começam em quadros sorteados no
   original (minas 11, 5, 10, 2, 8…; tipo 32 no 6, 79 no 4) e no 1 no porte; posições de
   algumas minas divergem depois (sorteio do rumo).

Os itens 1–6 são entradas para as áreas `jake-movimentos.md`, `garathe-movimentos.md` e
`itens-objetos.md`: o oráculo só mede.
