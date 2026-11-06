# Som: ids, recursos, som posicional e canais (M4, base para fatia futura)

## Id → recurso (`0x00449978`, `0x004499e3`) [confirmado]

`0x449978(id)` classifica o id:

| Id | Classe |
|---|---|
| 0, 1–4, 10000 | especiais (devolvidos como estão) |
| 1000–3000 | `id / 1000` (1–3) [inferido: faixas de música/CD] |
| 4000–4999 | 4 (`SX_%04d.WAV`, ou `.MID`; o id 4999 tem tratamento próprio) |
| 3001–9999 (fora de 4000–4999) | 3: efeito **`SX_%04d.WAV`** |
| demais | −1 (nada) |

`0x4499e3` monta o nome com `"SX_%04d.WAV"` (`0x480ac8`): o som 3040 é `SX_3040.WAV`, que
está nos `RESOURCE.*` (`ohtool list`). O `SOUND.RES` é só a lista de sons a **pré-carregar**
no início (texto `id=descrição`); serve de dicionário de nomes (3040 "Pistol", 3046 "Bullet
Hitting Wall", 3090 "Changed weapon", 5057 "Pickup generic object"…).

## Tocar sem posição (`0x0043c93d(id, prioridade, sinais, pan, volume, ?)`) [confirmado]

Repassa a `0x0044dc20`. Chamadas típicas: `(3090, 0, 0, 0, −1, 0)` na troca de arma;
`(som, 200, 0, 0, −1, 0)` na coleta de itens (`−1` = volume padrão [inferido]).

- Desligado (`0x49cb0c = 0`) → nada.
- **16 canais** (`0x480f14 = 16`): sons em `0x49c560[16]`, prioridades em `0x49cb10[16]`.
- Sinais: `1` = não repetir (se o mesmo som já toca, não toca de novo); `4` = exclusivo
  (para as outras instâncias do mesmo som antes); `2` = laço; `8` repassado ao tocador.
- Sem canal livre: procura o canal de **menor prioridade** abaixo da nova (ou de mesma
  prioridade tocando o mesmo som), para-o (`0x44ded8`) e usa o canal. Sem candidato, não toca.
- Toca por `0x44b8e3(id, laço, pan, volume, ?, sinal 8)`.

## Som posicional (`0x0043c2b6(grupo, índice, x, y, camada)`) [confirmado]

Devolve 1 se o pedido entrou na fila.

1. Desligado (`0x497310 = 0`) → 0.
2. **Resolve o som**: `grupo = 0` → o próprio `índice` é o som. Senão busca a lista do grupo
   no `SOUND.TBL` (abaixo) e pega `lista[índice]` (fora da lista → 0). Em ambos os casos o
   valor de 32 bits é: bits 0–14 = **id**, bit 15 = **laço**, bits 16–31 = **prioridade**.
   Id 0 → nada.
3. **Ouvinte = vista (câmera)**, não o jogador: percorre as vistas (`0x493b70`, encadeadas
   por `+0x58`; retângulo `+0x30` x, `+0x34` y, `+0x38` largura, `+0x3c` altura, `+0x40`
   camada). Só vistas na **mesma camada** do som; `d` = distância (`0x467328`) do centro da
   vista ao ponto. Fica a vista mais próxima com **`d < 768`**; nenhuma → não toca.
4. Volume e pan, com `dx = x − centro_x` da vista:
   - **volume = 100 − 100·d / 768** (inteiro, 100 no centro, ~0 a 768 px);
   - **pan = 128·dx / 768** (−128 esquerda … 128 direita).
5. Laço: há 32 vagas em `0x496dec` (24 bytes: canal, x, y, camada, …, vista). Se já toca um
   laço no mesmo ponto/camada/vista → não repete. Senão usa uma vaga livre ou a do laço mais
   distante (parando-o, `0x44bf0a`).
6. **Fila do tique** em `0x4970ec`: **4 pedidos** de 40 bytes (id, prioridade, laço→2, pan,
   volume, vaga, camada, x, y, vista). Usa um livre ou o de prioridade menor que a nova;
   cheia de prioridades maiores → descarta.

A fila é esvaziada uma vez por tique (`0x43c5e0`…): cada pedido vira
`0x43c93d(id, prioridade, sinais, pan, volume, 0)`; os laços guardam o canal na vaga e são
reavaliados depois (vista sumiu → para) [o resto da reavaliação, inferido].

## `SOUND.TBL`: sons por quadro de animação (`0x0043c0ce`) [confirmado]

Formato: texto até `0x1a`; `i32` versão (1000); três grupos, cada um com `i32` n e n listas
(`i32` contagem + contagem valores no formato do passo 2). Grupo 0 → ids 10000/20000
(`id / 10000`), grupo 1 → ids 5001/5002, grupo 2 → **listas 0–109 indexadas pelo tipo do
objeto** (até 110; o excedente é pulado).

Uso principal: o desenho de cada objeto (`0x004354dc`/`0x0043555b`) chama, sempre que o
quadro absoluto `+0x18c` muda em relação ao último sonorizado `+0x1a4`,
`0x43c2b6(tipo, quadro, centro_x da caixa, centro_y, camada)`. Ou seja, **cada quadro de cada
tipo pode ter um som**. Para os tiros (lista 1, Jake, quadros do `JAKE.ANM`):

| Quadro | Rótulo | Som (prioridade 9) |
|---|---|---|
| 155 | `J023` (pistola, frente) | 3040 "Pistol" |
| 282, 289, 296 | `J054`, `J055`, `J056` | 3040 |
| 326, 333, 340 | `J066`–`J068` (foguete) | 3035 "Rocket" |
| 576, 582, 587, 593 | `J122`–`J125` (espingarda) | 3045 "ShotGun" |

O som da pistola sai portanto **uma vez por animação** de tiro (no 1º quadro), não por bala:
uma rajada de 3 balas toca 3040 uma vez; cada recomeço do ciclo (a cada 6 tiques segurando)
toca de novo. Lista 86 (Explosion, `OEX.ANM`): quadro 148 (`oex15`) → 3046; quadros 1, 29,
58, 59, 72, 74… → explosões 3080–3087. A lista 102 (mina) está vazia: o 3016 "Hover Mine
Floating" não sai por aqui.

Outros grupos/quadros servem ao cenário (`0x431fd6` toca por rótulo de animação de célula,
`0x4328e3`) [inferido].

## Dados conferidos na M4c

- Os 280 `SX_*.WAV` da versão 1.00 são PCM mono de 8 bits: 277 a 11 025 Hz e um a cada
  18 000, 22 050 e 44 100 Hz.
- Lista 86 (`OEX.ANM`): todos os valores têm prioridade 9 e nenhum é laço. Os rótulos 1, 2,
  3 e 4 começam nos quadros 0, 29, 58 e 72: a explosão 1 soa no 2º quadro (3080), as demais
  no 1º (3084, 3082+3083, 3086), e a 4 repete 3081 nos quadros 74, 77, 82 e 86. O 15 (faísca)
  começa no 148.
- Lista 1 (Jake) também tem passos e esforços (5100–5112, 5034, 5043–5050…, prioridades 0,
  3, 5, 10 ou 12) — ainda não tocados pelo OpenHunted.

## Falas

`0x40b9eb(obj, fala_jake, fala_outro, vol, …)` escolhe a fala pelo tipo do objeto e não
repete uma fala em curso (`itens.md`).

## Em aberto

- `0x44b8e3` (DirectSound): escala real de volume (0–100 → dB?) e de pan.
- Valor de `volume = −1` nas chamadas não posicionais; o 6º argumento de `0x43c93d`.
- Reavaliação dos laços a cada tique (se pan/volume acompanham a câmera).
- Classe 1–3 (`id/1000`) e o id 4999.
- Onde toca o 3016 da mina.
