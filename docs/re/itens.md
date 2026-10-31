# Itens: objetos genéricos e coleta (M3)

Tipos 12–18, 25, 32–37, 44, 45, 76–85 usam a inicialização "Generic Object" `0x0040b2c8`
(que outras inicializações também chamam primeiro). Na `T_J_001`: **armadura** (tipo 79,
`OAR.ANM`/`oar04.brd`, célula 10, (1355, 67), rótulo 4) e **vida** (tipo 32,
`OHE00.ANM`/`OHE.BRD`, célula 12, (1542, 71), rótulo 1), ambos com sinalizadores `0x803a9`.

## Inicialização genérica (`0x40b2c8`) [confirmado]

- Zera `+0x2a0..+0x2a8`; liga `0x81` e desliga `0x40` em `+0x178`; `+0x1b4 = 30`.
- Tipos 80–85: o rótulo inicial `+0x1ec` passa a ser o próprio tipo.
- Nome "Generic Object"; `+0x16c = +0x170 = 0`.
- Conta os quadros do rótulo (`+0x188`, a partir do primeiro, `+0x18c`) e começa num quadro
  **sorteado** dele (`+0x184 = rand % n`), para os itens não piscarem em sincronia.
- `+0x1c0 = 0x40b3d5`; `+0x1c4..+0x1cc = 0`; `0x434fc3(obj)`.

Sinalizadores `0x803a9` dos dois itens: `0x1` presente/sólido, `0x8` sem gravidade, `0x20`
**só contato** (não barra, registra contato de tipo 5), `0x80` rótulo da fase, `0x100` o
**Jake pode pegar**, `0x200` o **Garathe pode pegar**, `0x80000` (leitor). [confirmado pelo
uso de `0x100`/`0x200` em `0x40b3f5`]

## Rotina por tique (`0x40b3d5`)

Chama a coleta `0x40b3f5`; se não houve coleta, anima em laço (`0x406c71(obj, 1)`).

## Coleta (`0x40b3f5(item)`) [confirmado]

1. Quem toca: `0x4076d0(item)` devolve `+0x21c` (quem me tocou) ou, se `+0x220 & 1`, o
   `+0x224`. Um tipo 1 ou 2 **sem jogador** (`+0x19c = 0`) não pega nada.
2. Som de coleta: se quem toca é vaga de jogador (`+0x1a8 & 1`): Jake → 5057 ("Pickup
   generic object") se o item tem `0x100`; Garathe → 5057 se tem `0x200`. Fora do modo solo
   (`0x4972f8 & 2 = 0`) e sem cooperação, qualquer um pega.
3. `0x40b6ed(quem, item)` decide se aceita (abaixo); recusa → nada.
4. Sem som escolhido no passo 2 → nada (o item continua lá).
5. Tipos 62–67, 80–85 e 9 trocam o som para 5060 ("Pickup flag or key").
6. Item com `+0x1ac & 1` (**segredo**) ainda não ligado: som 3093 ("Secret discovered") e
   fala `0x40b9eb(quem, 3718 "Ah hah!", 3731, 200, 200)`.
7. Toca o som (`0x43c93d(som, 200, …)`, não posicional).
8. `0x40b57b(quem, item)`: **vida/armadura/veneno** pela tabela abaixo.
9. `0x409fe8(quem, item)`: armas e munição [inferido; não lido].
10. `0x436358(quem, 0x20)` (aviso ao HUD [inferido]); `0x412fec(item)`.
11. O item vira `+0x178 = 0x40` (todos os bits apagados, inclusive o `0x1`: some do jogo) e
    `+0x1b0 = +0x19c` de quem pegou. Devolve 1.

`0x40b9eb(obj, fala_jake, fala_outro, vol, …)` **não é dano**: toca uma fala conforme o tipo
do objeto (Jake → a primeira, senão a segunda), se ela já não estiver tocando
(`0x44e130`).

### Aceitação (`0x40b6ed(quem, item)`) — casos relevantes [confirmado]

| Tipo do item | Recusa se | Ao aceitar |
|---|---|---|
| 32 (vida, `OHE`) | vida `+0x258 ≥ 100` | fala 3700 (Jake "Ahh!") / 3735 ou 3738 (sorteio) |
| 34 (vida grande) | vida ≥ 199 | idem |
| 33, 35 (veneno) | — | fala 3717 (Jake "Oh no it's poison") / 3737 |
| 37 (antídoto) | `+0x270 = 0` (não envenenado) | fala de vida |
| 77 (armadura grande) | armadura `+0x25c ≥ 199` | — |
| 76, 78, 79 (armadura) | armadura ≥ 100 | — |
| 12–18, 25–27, 44, 45 | munição da arma correspondente (`+0x38..+0x5c`, pares "tem/quantidade") ≥ 99 | — |

Os tipos que dão a arma (12, 13, 14, 25, 26, 27, 44) aceitam sempre quando o "tem" é 0; os
26/27 usam pares diferentes para Jake e Garathe. Os demais tipos sempre aceitam.

## Efeito (`0x40b57b`, tabela `0x00471d10`) [confirmado]

Nove registros de 20 bytes: tipo, vida, armadura, veneno (soma em `+0x270`), antídoto.

| Tipo | Vida | Armadura | Veneno | Antídoto |
|---|---|---|---|---|
| 32 | +25 | | | |
| 33 | | | +1 | |
| 34 | +100 | | | |
| 35 | | | +1 | |
| 37 | | | | sim |
| 76 | | +50 | | |
| 77 | | +100 | | |
| 78 | | +20 | | |
| 79 | | +25 | | |

Regras de soma:

- **Vida**: valor < 100 → só se `0 < vida < 100`, soma e limita a **100**; valor ≥ 100 → só
  se `0 < vida < 199`, soma e limita a **199**. (Morto não se cura.)
- **Armadura**: valor < 100 → só se armadura < 100, limita a **100**; valor ≥ 100 → só se
  < 199, limita a **199**.
- Veneno: `+0x270 += valor`. Antídoto: `+0x270 = 0` e `+0x27c = +0x280`.

Na `T_J_001`, o Jake começa com vida 50 e armadura 0 (`fields[14]`, `fields[15]` do Jake):
a vida (+25) leva a 75; a armadura (+25) a 25.

## Caminho do contato

Os itens têm `0x20`: na colisão entre objetos (`0x436caf`, ver [`objetos.md`](objetos.md)),
o Jake que se move contra um item não é barrado e registra um contato de tipo 5. A coleta
lê o contato pelo lado do item (`+0x21c`/`+0x224`); que o `+0x21c` do item recebe o Jake
nesse momento é [inferido] pela simetria com a mina.

## Em aberto

- `0x409fe8` (armas/munição), `0x412fec` e o evento `0x436358(…, 0x20)`.
- Onde exatamente `+0x21c` do objeto passivo é preenchido.
- Tipos 62–67/80–85/9 (bandeiras, chaves, peças do carro?).
