# Auditoria: HUD, câmera, som e geometria de colisão do cenário (M17)

Medido no oráculo (`tools/oraculo/`, capturas `--shots` e a câmera da 1ª vista, campo
`vista` = `[0x493b70]` +0/+4/+8, novo neste ramo) contra `ohtool render --jogo` (quadro da
partida: câmera do original, escala absoluta, barra de estado, paleta `WORLD_G1.PAL` do brilho
padrão 1402) e `ohtool render --sons` (sons e falas pedidos por tique). Fases: `T_J_001`,
`T_J_003`, `T_J_005`, `I_J_104`, `H_J_006`. A área de cliente da janela do Wine fica em (80, 60)
das capturas 800×600.

## Placar

| Veredito | HUD | Câmera/desenho | Som | Colisão | Total |
|---|---|---|---|---|---|
| igual | 5 | 2 | 2 | 4 | 13 |
| corrigido | 0 | 6 | 4 | 0 | 10 |
| diferente (em aberto) | 0 | 1 | 1 | 3 | 5 |
| não implementado (em aberto) | 0 | 2 | 1 | 0 | 3 |
| a confirmar | 3 | 1 | 1 | 0 | 5 |
| decisão | 0 | 0 | 1 | 0 | 1 |

## HUD (`docs/re/hud.md`)

| Comportamento | Original | Porte (medido) | Veredito |
|---|---|---|---|
| Barra 640×75 em y 405, fundo, faixas e ícones dos pares 7–18 | captura t 40 `T_J_001`, `I_J_104`, `H_J_006` | **0 pixels diferentes** na faixa 405–479 nas três | igual |
| Vida/armadura: moldura 14, rosto 15, quadro 16, 3 dígitos com zeros à esquerda, estilos 19/18/17 por faixa | 050 verde, 000 vermelho, 090/100 | idêntico | igual |
| Armas: ícone com/sem a arma (+1 a corrente), caixa de munição 2 dígitos estilo 50/51 | pistola 80 (`T_J_001`), punhos (`I_J_104`) | idêntico | igual |
| Cronômetro `0000` nas fases sem limite | `0000` | idêntico | igual |
| Paleta da partida pelo brilho (`Brightness` 1402 → `WORLD_G1.PAL`) | o oráculo grava 1402 | o jogo já usava; o `ohtool` passou a usar em `--jogo` | igual |
| Rolo do cronômetro numa fase com limite (`r = (t % 20)/2`) | não medido | `timer_frames` pelo RE | a confirmar |
| Rosto 93 com veneno (`+0x270`) | não medido | `HudState::of` | a confirmar |
| Caixa da dica (`SWIZARD.BMP`) e da pausa | não medidas (o oráculo fecha a dica) | — | a confirmar |

O estado da barra saiu do visualizador para o motor (`HudState::of(cena, jogador)`), usado pelo
jogo e pelo `ohtool`.

## Câmera e desenho (`0x436386`, `0x436898`, `0x436916`)

| Comportamento | Original | Porte antes | Veredito / commit |
|---|---|---|---|
| Alvo = centro horizontal (`w/2`) e **topo** do retângulo de referência (`0x43585c`: quadro 0 × escala + posição) | vista = (x + 11, y) do Jake em todos os tiques medidos (queda, corrida a 12 px/t, pulo) | centro da caixa de colisão | corrigido `15d59da` |
| Sem atraso: a câmera usa a posição do próprio tique | `vista` t 0 = (1707, 48), y = 48 | — | igual |
| Passo máximo 18 px pelo **comprimento** do vetor (`dx·18/d`) | `0x4365ae` | solo: direto; tela dividida: 18 por eixo | corrigido `15d59da` |
| Sem limite na borda da fase (o vazio aparece) | capturas: preto além da última célula | `clamp_camera` | corrigido `15d59da` |
| Escala absoluta da camada da vista 30/23 ≈ 1,30 | corredor de 104 px ocupa 135 px | 1:1 | corrigido `15d59da` (`camera::solo_view`, `View::reference = 30`, também na tela dividida) |
| Largura/altura projetadas à parte, `(v·30 + d − 1)/d` (`0x436916` → `0x43682e`) | bloco de 128 px = 167 px em toda posição | diferença dos cantos (166 ou 167) | corrigido `15d59da` |
| Decalques espelhados com a célula (`0x4305ac`: retângulo por `0x405775`, desenho com `+0x40` XOR `+0x30` do decalque) | seta "<<<" na `T_J_001` | "> > >", posição sem espelho | corrigido `15d59da` (hash de referência da `T_J_001` atualizado) |
| Câmera na `I_J_104` e na `H_J_006` (d = 25) | (641, 325), (154, 837) | idênticas depois da correção | igual |
| Amostragem do bitmap esticado (128 → 167) | colunas ≈ `⌊(i·127 + 150)/167⌋`, linhas ≈ `⌊i·127/166⌋` (ajuste nas capturas) | `⌊i·23/30⌋` | diferente: ~10 % dos pixels do cenário em padrão de grade; rotina de blit da GFX (`0x447530`) não desmontada |
| Deslocamento residual de 1 px (o original sai 1 px abaixo, às vezes 1 à direita) | três fases | — | a confirmar (arredondamento da âncora ou da amostragem) |
| Tremor ±5 px com `+0x274` (só tipos 1/2, `0x471980`) | `0x4364e4` | não há | não implementado |
| Ponto fixo da célula (`+0x5c` bit 0, `+0x64/+0x68`) | `0x436490` | não há | não implementado |

## Som (`docs/re/som.md`)

Lido o `SOUND.TBL` inteiro (`openhunted_formats::SoundTable`): versão 1000, grupos 0 e 1 (cenário
de `WORLD1`/`WORLD2`, por quadro) e 110 listas por tipo; 26 tipos com sons (1, 2, 3, 4, 5, 7, 8,
10, 11, 25, 27, 39, 42, 43, 46, 50, 51, 68, 70, 71, 72, 86, 89, 103, 104, 107). O oráculo roda
mudo: os disparos saem do binário e do `ohtool --sons`.

| Comportamento | Original | Porte (medido) | Veredito |
|---|---|---|---|
| Fila de 4 posicionais por tique, volume `100 − 100·d/768`, pan `128·dx/768`, ouvinte = centro da vista | `0x43c2b6` | `sound::resolve`; o ouvinte do solo agora é o centro da câmera nova | igual |
| **Fila do locutor** `0x43c868(id1, id2, id3, p1, p2, p3)`: 10 vagas de 36 bytes em `0x49718c`; `0x43c8cd` por tique (e no laço da dica, `0x402d9e`) toca a fala da frente sem posição e, quando `0x44e130` diz que parou, passa à seguinte; sem vaga, perde; `0x43c814` = "está na fila" | `0x43c814…0x43c93c` | não havia (falas no mesmo tique) | corrigido `e185432` (`sound::Announcer`, `Event::Speech`, visualizador) |
| Porta trancada: 3715 → 3714 (ou 3719), prioridade 200, só se nenhuma já está na fila (Garathe 3730 → 3732) | `0x40f08c…0x40f1af` | as duas no mesmo tique | corrigido `e185432` |
| Passos, esforços e vozes dos jogadores (listas 1 e 2: 5100–5112, 5034, 5042–5050, 37xx…) quando o quadro absoluto muda | `0x4354dc` | só os sons de arma | corrigido `e185432` (`scene/frame_sounds.rs`; correndo: 5101/5100 alternados a cada 6 tiques, prioridade 3) |
| Cenário animado (grupos 0/1, centro da célula, quando o quadro da célula muda: `0x4328c2`) | ex.: 5024, 5027, 5041 em `WORLD1` | não havia | corrigido `e185432` |
| 3016 "Hover Mine Floating" | o id não aparece no executável (nenhum `push`, nenhum dword) nem no `SOUND.TBL`: **nunca toca** | não toca | igual |
| Provocações da Morte: 40 % → 3763 direto (prioridade 5, se não está tocando); senão uma **sequência de 2 ou 3** de 3760/3761/3762 (permutação sorteada, prioridade 0) pela fila | `0x41592a…0x415b52` | uma só, sorteada, prioridade 9 | diferente, em aberto (`behaviour/death.rs`, área de inimigos-2) |
| Laços do `SOUND.TBL` (bit 15: tipo 42 quadro 1 = 5041; tipo 72 quadro 12 = 3167) e as 32 vagas de laço `0x496dec` | `0x43c2b6` passo 5 | não há laço posicional | não implementado |
| 16 canais e despejo pela menor prioridade | `0x44dc20` | `audio::Mixer` (16 canais) | a confirmar (despejo não medido) |
| Música do CD | — | Ruling do Pedro: sem as faixas | decisão |

## Geometria de colisão do cenário

| Comportamento | Original | Porte (medido) | Veredito |
|---|---|---|---|
| Caixa espelhada na célula `x' = 2·cx + 128 − (x + w)` | `0x405775` | `x' = 128 − x − w` | igual |
| Só caixas (tipo 2) nas células: sem rampas, degraus ou plataformas de mão única | 225 BRD | idem | igual |
| Pouso no chão (encosta) | y 69 (`T_J_001`), 197 (`T_J_003/005`), 325 (`I_J_104`) | idêntico | igual |
| Parede à **esquerda** (encosta) | x 20 (`T_J_003`, `T_J_005`), 404 (`I_J_104`) | idêntico | igual |
| Parede à **direita**: para 1 px antes | x 1749 (`T_J_001`), 853 (`I_J_104`) | 1750, 854 | diferente, em aberto (abaixo) |
| Forma do Jake: octógono `JK_DEFLT` (arestas tipo 3) + caixas; caixas com borda inclusiva (`x + w − 1`, `0x438d55`) | `0x43339c` | caixa envolvente única | diferente, em aberto |
| Teto: a caixa de topo do Jake (8, 3) entra 1 px na caixa do teto | pulo para cima 16 na `T_J_001`: y 8 (topo 11, teto até 12) | y 9 (encosta) | diferente, em aberto (abaixo) |

**A matemática da varredura** (para o núcleo): `0x438d55` (caixa × caixa) e `0x439177`/`0x439709`
(com arestas) dão o deslocamento até o contato **em px inteiros**, com as caixas em borda
inclusiva (`x + w − 1`; "encostado" já conta como contato) e as arestas na coordenada exata do
vértice (a do Jake em x + 20); `0x433999` então multiplica por 8 e encurta o vetor em 1/8 px
(`v·(L − 1)/L`, `L` = comprimento inteiro de `0x467328`), guarda em `+0x15c/+0x160` e soma
`>> 3`. Daí: correndo (12, 1) contra o chão sai (95, 7) → 11 px para a direita e −12 para a
esquerda; contra a parede da direita, a aresta em x + 20 para 1 px antes (1749 = 1770 − 21).
Caixa × caixa conta como separadas as que só dividem a linha de borda (`A.y ≥ B.y + B.h − 1`):
subindo, o encurtamento de 1/8 px arredonda para o contato e a caixa de topo para dentro da
última linha do teto (y 8 em vez de 9, medido na auditoria do Jake); descendo e para a direita,
a caixa encosta; para a esquerda, a aresta do octógono (x + 2) encosta antes da caixa. O porte
usa uma caixa envolvente exclusiva para tudo: as três diferenças (teto, parede à direita,
corrida) somem com as formas do `JK_DEFLT` e essas duas regras.

## Correções

- `15d59da` câmera da partida como o original (`openhunted_engine::camera`: `target`, `chase`,
  `solo_view`); `View::project_rect` com o tamanho projetado à parte; decalques espelhados com a
  célula; `HudState::of`; `ohtool render --jogo/--sons`; `SoundTable`; oráculo grava `vista`.
- `e185432` fila do locutor e falas da porta; sons por quadro do cenário e dos jogadores.

Capturas lado a lado (original | porte) em `/home/claude/scratch/a-hud/lado_*.png`.

## Em aberto

1. Parede à direita 1 px e corrida 11 px: a varredura acima é do núcleo (`physics::step`); o
   `audit/nucleo` já reproduz a corrida empiricamente.
2. Provocações da Morte em sequência pela fila (`Event::Speech`/`Chain::new(.., false)`), em
   `behaviour/death.rs`.
3. Laços posicionais (bit 15 e as 32 vagas), tremor e ponto fixo da câmera.
4. Amostragem do esticamento e o 1 px residual: desmontar `0x447530`/`0x45c4b3`.
5. Medir no oráculo o cronômetro de uma fase com limite, o rosto 93, a dica e a pausa.
