# Bruiser (tipo 7, `obr.anm`)

Nome no executável: **"Bruiser"** (`0x472550`). 59 objetos em 23 fases da campanha; robô
bípede de 100 × 95 px. O leitor força a colisão `OBR0.BRD`. Sinalizadores `0x84801`. Vida
100 em 53 objetos (150 em 3); 27 têm objetivo "destruir" (`+0x1ac = 4`); 16 têm gatilhos.

## Inicialização (`0x00416af4`) [confirmado]

- Genérica; `+0x1c0 = 0x416c68`; nome; `obr.anm`; `OBR0.BRD`; `+0x178 |= 0x4801`, desliga
  `0x20` e `0x80` (**com gravidade**).
- Estado/rótulo **15** (`obr15`, 1 quadro: adormecido). `+0x1b4 = 125`, `+0x174 = 255`; vida
  0 → 100. A armadura da fase (`+0x25c`, 50 em 5 objetos) **não é usada**.
- `+0x27c = 1` (olhando à direita; o sprite sem espelho olha à direita); alvo `+0x280 = 0`;
  `+0x290` (morto) = 0; fala `+0x274..+0x278 = 0`; contadores `+0x294`, `+0x298` = 0.
- **Capacidades** `values[0..3]` (`+0x260..+0x26c`), postas todas a 1 se `values[15] ≠ 1`:
  `+0x260` fogo no chão, `+0x264` foguetes, `+0x268` balas, `+0x26c` terremoto. Nos dados:
  49, 57, 48 e 45 dos 59 têm cada uma.

`0x416c0c(obj, estado, rótulo)`: troca (`0x406b6c`); estado 6 zera vx; estados −1, −2, −3
tocam ao contrário; estados 6 e 1 zeram a espera do fogo `+0x299`.

## Rótulos e estados [confirmado]

`OBR.ANM`: 1 `obr01` (12 quadros), 2 `obr02` (20), 3 `obr03` (14), 4 `obr04` (10), 5 `obr05`
(22), 6 `obr06` (14), 7 `obr07` (22), 10 `obr10` (10), 11 `obr11` (6), 12 `obr12` (13), 13
`obr13` (5), 15 `obr15` (1). Estado = rótulo, salvo os invertidos.

| Estado | Papel | Fim da animação (`0x417061`) |
|---|---|---|
| 15 | adormecido | continua 15 |
| −3 | acordar (rótulo 11 ao contrário) | → 13 |
| 13 → 12 | olhar em volta | 13 → 12; 12 sem alvo: 80 % de novo 12, senão −2 (13 ao contrário) |
| −2 | — | sem alvo: 70 % → 13; senão anda (1) se puder, ou vira (6); com alvo → ataque |
| 1 → 2 → 3 | andar: começo, passo, parada | 1 → 2 se puder andar, senão 3; 2 idem; 3 → 6 ou 13 (50 %) ou ataque |
| 4 | atirar | sem alvo → 6; com alvo → ataque |
| 5 | fogo no chão | cria o fogo; depois como 4 |
| 7 | terremoto | como 4 |
| 6 | virar-se | esmaga, encara o alvo, −1 |
| −1 | fim da virada (6 ao contrário) | sem alvo: anda se puder, senão 6/13; com alvo → ataque |
| 10 | tranco (atingido) | sem alvo → 6; com alvo → ataque |
| 11 | morrer | explode |
| 8, 9, 14 | pulo (vy −10 px/tique) | 14 → 8 → 9 → 14; rótulos ausentes do `OBR.ANM` e nenhum caminho leva a eles [inferido: resto] |

## Atualização (`0x00416c68`, por tique) [confirmado]

1. Morto (`+0x290`) → nada.
2. **Ativação**: enquanto `+0x27a = 0`, testa os gatilhos (`0x435bd8`); verdadeiros (ou sem
   gatilhos: −1) → `+0x27a = 1`, estado −3. Até lá, nada mais.
3. `+0x299−−` (espera do fogo). **Dano** (`0x417ae0`, abaixo).
4. `+0x279` (contador; ao zerar: com alvo → ataque, senão estado 6) [inferido: não visto
   quem o arma].
5. Fala (`0x416ed6`) fora do estado 11; procura alvo (`0x417d90`).
6. No estado 2, se x não mudou por 6 tiques e não há alvo → 50 % estado 6, senão 13.
7. Toca o rótulo do estado (`0x407942`, rotina de fim `0x417061`). Nos estados 1–3 a
   velocidade é refeita a cada tique (`0x417a68`): `vx = ±tabela[quadro absoluto]` (1/8 px,
   `0x472498`), `vy = −16`. Tabela: rótulo 1: 9, 20, 22, 22, 22, 22, 22, 22, 20, 22, 0, 18;
   rótulo 2: 27, 27, 27, 27, 16, 16, 18, 18, 16, 13, 13, 13, 11, 11, 25, 29, 27, 27, 27, 11;
   rótulo 3: 22, 20, 18, 67, 43, 40, 0, 27, 34, 0… (≈ 2–3 px/tique). No estado 6 a rotina da
   virada, no quadro final, chama o terremoto; no 7, cinco quadros antes do fim.

## Escolha do ataque (`0x0041750b`) [confirmado]

1. Alvo do lado de trás (centro do alvo ≤ o seu e olhando à direita, ou o contrário) →
   **virar** (6).
2. `|dx|` entre centros **≤ 65** e `+0x260` → **fogo no chão** (5).
3. Pode andar e `rand % 100 < 30` → anda (1).
4. Com 75 %: com 75 % e (`+0x264` ou `+0x268`) → **dispara** (`0x417e47`): disparou → 4,
   senão 13; nos outros 25 %, `+0x26c` → **terremoto** (7), senão 13.
5. Senão 13.

"Pode andar" (`0x417836`): um passo de 17 px à frente não bate em parede (linha e passo de
física `0x432e5d`) e há chão adiante (`0x41817d`, sonda de queda de 15 px). Não cai de beiradas.

## Percepção [confirmado]

`0x417d90`: entre os jogadores vivos, o válido de menor `|dx|`. Válido (`0x417c0d`): vida >
0, `|dy| ≤ 50`, mesma camada, `|dx|` **≤ 350**, linha livre entre os centros. Sem teste de
esconderijo; vê para os dois lados.

## Ataques [confirmado]

- **Tiro** (`0x417e47`): direção ±1 (espelhado → −1), `v = (20·dir, rand' & 3)` px/tique.
  Bocas (olhando à direita / à esquerda): A (x+55, y+12) / (x, y+12); B (x+72, y+15) /
  (x−12, y+15); C (x+68, y+5) / (x−10, y+5).
  - 35 %: com `+0x268`, som 3143 e **bala tipo 100** ("bruiser bullet") em B; mais 48 % de
    outra em A. Linha `(100, 0)`: **15** de dano direto, efeito sorteado raio 28, empurrão de
    área 30, dano de área 10 (`R = 42`).
  - 65 %: com `+0x264`, som 3143 e **foguete tipo 21** em C. Linha `(21, 0)`: **20** direto,
    raio 64, empurrão 30, área **50** (`R = 96`).
- **Fogo no chão** (`0x4180d0`, fim do estado 5): se `+0x299 ≤ 0`, cria fogo (tipo 42,
  `0x40fb58`, `barril-fogo-cerca.md`) 50 px à frente do centro, na base, tamanho 25–49, e
  `+0x299 = 70`.
- **Terremoto** (`0x418229`): som 3131 "Bruiser - Earth Quake"; todo Jake/Garathe vivo e
  presente a menos de **300 px** (centros) recebe `+0x274 = 20 + rand%20`. No Jake
  (`0x426f6a`), enquanto `+0x274 > 0` e no chão, ele cambaleia (estado 15) [inferido pelo
  destino da tabela de saltos].
- **Esmagar** (fim da virada, `0x417308`): se o x do alvo cai dentro da largura do Bruiser,
  `0x43d629(bruiser, alvo, 0)` → linha `(7, 0, 0)`: derrubada, **15** de dano, empurrão fixo
  (48, −3).

## Dano recebido (`0x00417ae0`) [confirmado]

- Natureza `0x4` (fogo) → **imune**. Dano 1 só passa quando o empurrão vem pelo lado em que
  ele olha [inferido: pelas costas].
- Dificuldade (fácil ×2, difícil ÷2), depois **metade**: `vida −= dano·50/100`. Pistola (2)
  → 1 por bala: 100 balas no normal.
- Sem veneno/fogo e fora dos estados 10, 6, 5 → **tranco** (estado 10). Zera natureza,
  reação e dano (não há empurrão).
- Vida ≤ 0 → estado 11, liga `0x40`.

## Morte (fim do estado 11) [confirmado]

`+0x290 = 1`, desliga `0x8`, liga `0x40` e `0x20` (só contato), `0x407009`, ordem 125, e
**30 explosões** `0x43d629(bruiser, nulo, 2)` em pontos sorteados a ±30 px do canto
superior esquerdo da caixa. Linha `(7, 0, 2)`: raio 48, empurrão 15, área 10 (`R = 72`).
Juntas, até ~300 de dano perto do canto [inferido do cálculo].

## Fala e sons [confirmado]

`0x416ed6`: com alvo e calado, sorteio `rand % 1000 ≤ 10` por tique escolhe uma frase 1–3 e,
às vezes, uma ou duas seguintes; toca-as em ordem (não posicionais), esperando a anterior
acabar: 1 → 3127 "Halt", 2 → 3125 "Put down your weapon", 3 → 3126 "You can not win". Sem
alvo, cala. `SOUND.TBL` tipo 7: passos 3107 (quadros 1, 12, 22, 40), 3111 (2, 96), 3105
(58, fogo), 3106 (83, 95), 3143 (115, tranco), 3109 (124, morte), 3140 (130, 144).

## Em aberto

- Quem arma `+0x279`; os estados 8, 9, 14 (pulo) sem rótulo.
- Linha `(7, 0, 1)` (empurrão, 10 de dano): nenhuma chamada com variante 1 vista.
- Semântica exata de `+0x184/+0x188` (quadro corrente/contagem) nas chamadas do terremoto.
- Nomes de 3105–3111, 3140, 3143.
