# Grafite, peça do carro, teletransporte, paredes `OH`, plataformas e coletáveis

Tipos que aparecem nas `T_J_003/005/007` além do olho-laser
([`olho-laser.md`](olho-laser.md)) e do barril, fogo e cerca
([`barril-fogo-cerca.md`](barril-fogo-cerca.md)).

## Tipo 6: grafite (`OGF.ANM`, `0x00414ea4`) [confirmado]

**Não é item**: é um texto pintado na parede. Corrige a decisão provisória de `objetos.md`.

- `OGF.ANM` é uma **fonte**: o quadro `c` é o glifo do caractere `toupper(c)` (`0x46d0a4`);
  espaço usa a largura do `A`. A animação vai para `+0x288` e `+0x1e8` é zerado (não há
  sprite próprio); rótulo 0.
- Texto = linha `values[0]` do `GRAFFITI.RES` (id `17000 + values[0]`, `0x45e980`, contando de
  1). Altura `values[1]` (`+0x264`): 0 ou < 5 → 18. Escala = altura / altura do `A`; caixa
  `+0x20c..` = deslocamento do glifo e (soma das larguras escaladas, altura).
- `+0x178 |= 0x28 | 0x81 | 2`, `&= ~0xc0`: o bit 2 tira o grafite da colisão; `+0x1b4 = 10`;
  nome "Graffiti"; `+0x1c0 = 0x4150c0`, `+0x1d0 = 0x4150f9` (desenha letra a letra).
- `0x4150c0`: gatilhos verdadeiros (ou nenhum) → visível (`&= ~2`, `|= 0x40`); falsos →
  escondido (`|= 2`). O desenho não faz nada com o bit 2.

Nas fases: `T_J_005` 27 "EXPLOSIVE" (junto aos barris) e duas vezes 31 ("1"; desenha o glifo
do caractere `1`, 11 px de largura); `T_J_007` 29 "FLAMMABLE" e 57 "CAR".

## Tipo 9: peça do carro (`OCR.ANM`, `0x00414ce0`) [confirmado]

- Genérica; `OCR.ANM`/`OCR.BRD` padrão; nome "car piece"; `+0x174 = 1`; `+0x1c0 = 0x414d99`.
- Rótulo = `0x445bd6()`: zera `0x48059c`; se o índice da missão `0x47f848` ≥ total
  `0x47f840` → 0; se a missão já tem recorde (`0x47fa8c[índice] ≠ 0`) → 0; senão
  `0x480598 = 0x47f858 + 1` (peças já ganhas + 1) e devolve isso. Rótulo fora de 0…15 →
  `+0x178 |= 2` (sumida). `OCR.ANM` tem os rótulos 1–15 (`ocr01`–`ocr15`, 18 quadros cada):
  cada peça tem o seu desenho.
- Por tique (`0x414d99`): coleta genérica `0x40b3f5`; sem coleta, **flutua**: `vy` persegue
  `64 + 16·sen(2π·((tique + x) mod 22)/22) − (y mod 128)` (limitado a ±64, 1/8 px), como as
  minas: oscila 16 px em torno do meio da célula, período 22 tiques; anima o rótulo do estado.
- Coleta: som 5060 "Pickup flag or key"; `0x409fe8` → `0x445c37` (`0x48059c = 1`: peça ganha
  nesta missão, usada no fim, `fim-de-missao.md`); o item vira `+0x178 = 0x40`. Com
  `+0x1ac = 2` (objetivo "acionar"), **pegar a peça cumpre o objetivo** (`0x40` ligado).

## Tipo 46: teletransporte (`OBO.ANM`, `0x0041324c`) [confirmado]

- `OBO.ANM` padrão; estado e rótulo 2 (`obo01`, 12 quadros, parado); genérica; `|= 0x28`
  (sem gravidade, só contato), `&= ~0x80`; `+0x27c` (fase) = 0; `+0x1b4 = 80`;
  `+0x1c0 = 0x4132ce`, `+0x1d0 = 0x4137b3`.
- **Fase 0** (parado): anima em laço; quem toca (`0x4076d0`) precisa ter a caixa cruzando a
  do teletransporte (`0x405698`). Espera: se `+0x278` (último levado) toca de novo, zera
  `+0x274`; senão libera quando o tique passa de `+0x274`. Toque válido (sem `0x12` nos
  sinalizadores do tocador, não o último, tique > `+0x274`) → `0x4136ef`.
- `0x4136ef`: só leva tipos **1, 2, 8 e 109**. Liga `0x40` no teletransporte; cria um
  **clone** (cópia de 0x2b0 bytes), sem colisão, `0x10`, `+0x1b4 = 81`, `+0x278` =
  passageiro, rótulo/estado 1 (`obo00`, 21 quadros), fase 1, `+0x274` = tique; o passageiro
  recebe `+0x178 |= 2` (some).
- **Fase 1** (clone): depois do tique de criação, anima o rótulo 1; da metade em diante
  `+0x1a4` = quadro; ao voltar ao quadro 0 → `0x41347e`: sorteia um **destino** entre as
  entradas da lista de gatilhos com alvo (o código do gatilho não importa) e move o clone
  para lá: x centrado no destino, y deslocado em células inteiras (mantém a altura dentro da
  célula), camada do destino; o passageiro vai junto (mesmo deslocamento, camada, velocidades
  zeradas, `+0x154/+0x158` atualizados). Se o destino é outro tipo 46, ele ganha 40 tiques
  de espera e `+0x278` = passageiro. A vista do jogador (`0x436635`) passa ao centro e
  camada do passageiro. Rótulo 1 de novo (`0x406c71` modo 5), fase 2, `+0x1a4 = −1`.
- **Fase 2**: anima com o modo 4 (de trás para a frente [inferido]: guarda `+0x1a4` só abaixo
  da metade e termina no quadro 0); ao chegar ao quadro 0 o passageiro reaparece (`&= ~2`); o
  clone volta ao rótulo 2/fase 0 e, por ter `0x10`, é destruído. Total ≈ 42 tiques.

Porte [ruling]: o destino é sorteado na partida; sem destino válido não há viagem (nem
feixe, nem os 42 tiques ocupados); dois pedidos no mesmo tique: vale o primeiro da lista.
O destino vale em qualquer camada e o passageiro passa a ela na metade da viagem (M13; 20
missões solo têm teletransportes entre camadas, `docs/campanha.md`). O
destino tipo 46 conta os 40 tiques a partir da metade da viagem e, enquanto o passageiro
viaja (invisível), não o esquece: o par id 1 ↔ 2 da `I_J_104` não o devolve em laço.

Na `T_J_005`: teletransporte no fundo do primeiro fosso (374, 367), gatilho `(1, 11)` → a
zona 73 de id 1 em (438, 98), no poço de cima da célula (3, 0). Dica 15: "To activate
teleporters STEP ON THEM" (zona 2, no fundo do fosso).

## Tipos 50–52: paredes e portas `OH` (`0x0040e690`) [confirmado]

Não são decoração estática. Sinalizadores `|= 0x89`, `&= ~0x40`; `+0x1b4 = 10`; conta os
quadros do rótulo (`+0x184` primeiro, `+0x188` último). Estado 0 → `values` padrão
(10, 0, 0, 1). Rótulo 0 → o primeiro da animação. Colisão:

- rótulo < 100: `OH_W%03d.BRD` com `(tipo − 49)·100 + rótulo − 49` (ex.: tipo 50, rótulos 54
  e 55 → `OH_W105`, `OH_W106`); y alinhado ao topo da célula;
- rótulo ≥ 100, `rótulo % 100`: 0 → `OH_F.BRD` (x ao início da célula, y = topo + 116),
  1 → `OH_C.BRD` (x e y ao início da célula), 2 → `OH_B.BRD` (como F).

Estados (`0x40ebdc`): **1 fechada** (quadro inicial, sólida: `&= ~0x20`; recarrega
`+0x270 = values[0]`, `+0x274 = values[1]`); **2 fechando** e **3 abrindo** (sólida;
avançam um quadro por tique até 1 ou 4); **4 aberta** (último quadro, `|= 0x60`: só contato
→ passável; `+0x278 = values[2]`; se `+0x1ac & 1` e ainda não aberta: som 3093 "Secret
discovered" e fala 3718/3731). Nos gatilhos, "ligada" para 50–52 = estado > 2.

Por tique (`0x40e952`):

- **Empurrar**: com `values[0] ≠ 0`, fechada, (`values[3] ≠ 0` ou ainda não aberta), um tipo
  1/2 tocando (não-jogador só se não for segredo) conta `+0x270`; ao zerar → abre.
- **Gatilho**: fechada, gatilhos verdadeiros, `values[1] ≠ 0`, ninguém tocando, (`values[3]`
  ou não aberta) → conta `+0x274`; ao zerar → abre.
- **Fechar**: aberta, gatilhos verdadeiros, ninguém tocando nem dentro (`0x435d1c`) e
  `values[2] ≠ 0` → conta `+0x278`; ao zerar → fecha [condição do gatilho conferida, sentido
  em aberto].

Na `T_J_007`: duas (`OH1.ANM`, camada 1, x = 635 e 944) com `values` (0, 1, 0, 1, 0, 1) e
gatilho `(1, 11)` → o id 1 é a **saída**. Não abrem por toque; só abrem quando a saída liga,
isto é, no fim da missão. A de x = 635 corta o corredor da camada 1: é preciso contorná-la
pela camada 2 (`missoes-tutorial.md`). A célula (8, 2, 1), atrás da de x = 944 (id 2, onde
está o Bruiser), tem gatilho `(2, 11)`.

## Tipo 56: plataforma (`OPL.ANM`, `0x0040e60c`) [confirmado]

Genérica; nome "Platform"; `OPL.ANM`; colisão `OPL.BRD` (rótulo ≤ 6) ou `OPL07.BRD`;
`&= ~0x20` (sólida), `|= 0x8`; `+0x1b4 = 50`; `+0x1c0 = 0x40e688`, rotina **vazia**: estática.
Na `T_J_007`, três peças (rótulo 1) em y = 93 sob o início do Jake.

## Tipo 7: Bruiser (`obr.anm`, `0x00416af4`)

Inimigo (nome "Bruiser", `OBR0.BRD`, vida 100, `+0x1c0 = 0x416c68`, `|= 0x4801`, sólido).
Na `T_J_007` fica atrás da parede id 2, que só abre no fim: não interfere. Não decifrado.

## Coletáveis genéricos (`0x40b2c8`; coleta em `itens.md`, `armas.md`) [confirmado]

| Tipo | ANM | Efeito (Jake) | Nos dados |
|---|---|---|---|
| 12 | `OGN00` | dá a pistola + 3·v | `T_J_003`: v 20 (+60) |
| 16 | `OAC00` | munição da pistola + 3·v | v 25 (+75) |
| 14 | `ORL00` | dá o lança-foguetes + v | `T_J_007`: v 5 |
| 26 | `ogr` | dá granadas (arma 6) + v | `T_J_007`: v 3 e 2 |
| 32 | `OHE00` | vida +25 (até 100) | |
| 34 | `OFO00` | vida +100 (até 199) ("FOOD") | |
| 76 / 77 / 78 | `OAR` rótulos 1/2/3 | armadura +50 / +100 / +20 | |

Munição soma com o ajuste de dificuldade e teto 99 (`armas.md`). Os genéricos só animam em
laço (12 quadros, começando num quadro sorteado): **não flutuam**; quem cai ou não depende
do bit `0x8` vindo da fase (pistola e comida da `T_J_00x` caem; munição, armadura e
lança-foguetes não). O tipo 26 tem inicialização própria (`0x40d754`, "grenade", `ogr.brd`,
estado 1 = coletável no rótulo 1; estados 0/2 são a granada lançada).

## Em aberto

- Bit `0x4` dos sinalizadores dos coletáveis; `0x4137b3` (desenho do teletransporte).
- Rótulo 0 da peça do carro (missão já vencida): desenha algo?
- Sentido do gatilho no fechamento das portas `OH`; papel do gatilho de célula `(2, 11)`.
- Comportamento do Bruiser (tipo 7).
- Derrubada do barril em qualquer objeto (não só no Jake) e o atrito depois dela.
- Linha de visão do olho-laser através de objetos sólidos (hoje só as células barram).
- Porta `OH` da `I_J_107` que nasce sobre o Jake; e o "rótulo 0 → o primeiro da animação"
  das `OH`: é o primeiro rótulo do `.ANM` ou o de menor número? (pergunta de RE)
