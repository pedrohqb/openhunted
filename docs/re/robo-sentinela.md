# Robô sentinela (tipo 10, `ose.anm`)

Nome no executável: **"Sentinel Robot"** (`0x472945`). 154 objetos em 24 fases da campanha
(`D_J`, `H_J`, `I_J`), todos iguais nos dados: vida 4, `values[0] = 3`, `values[5] = 4`,
`values[10] = 1`, colisão `ose2.BRD` (3 com `ose0.BRD`), sinalizadores `0x84809`/`0x8480d`.
É um casulo fixo que se abre e dispara uma rajada de balas para o lado.

(Correção a `objetos.md`: a "sentinela" `OHS` é outro objeto, a mina saltadora do tipo 11,
[`mina-saltadora.md`](mina-saltadora.md).)

## Inicialização (`0x0041b51c`) [confirmado]

- Genérica `0x40b2c8`; `+0x1c0 = 0x41b640`; nome "Sentinel Robot"; animação `ose.anm`;
  colisão `ose2.BRD` (ponteiro `0x472908`, cache `0x488810`).
- `+0x178 |= 0x4801 | 0x8` (sólido, personagem, **sem gravidade**: fica onde a fase o põe);
  desliga `0x80` e `0x20`.
- Estado `+0x180 = 1`; `+0x1b4 = 100`; `+0x174 = 10`; vida 0 → **4**.
- Alvo `+0x268 = 0`; `+0x264 = 0`; morto `+0x270 = 0`.
- Se `values[10]` (`+0x288`) ≠ 1: `+0x288 = 1` e **`values[0]` (`+0x260`) = 3**. `values[0]` é o
  número de balas por disparo; os dados já trazem 3 com `values[10] = 1`.

## Estados e rótulos [confirmado]

`0x41b5f1(obj, estado, rótulo)`: guarda o estado anterior em `+0x264`, troca estado/rótulo
(`0x406b6c`); estado −1 toca o rótulo ao contrário (`0x406c1a`); colisão pelo rótulo
(tabela `0x472900`: 1 `ose0`, 2 `ose2`, 3 `ose2`, 4 `ose0`, 5 `ose1`).

| Estado | Rótulo (`ose.anm`) | Papel |
|---|---|---|
| 1 | 1 `ose00` (4 quadros) | fechado, parado |
| 4 | 4 `ose03` (9) | fechado, outra animação de espera |
| 2 | 2 `ose01` (8) | abrindo |
| 3 | 3 `ose02` (7) | aberto, atirando |
| −1 | 2 ao contrário | fechando |
| 5 | 5 `ose04` (1) | destruído |

## Atualização (`0x0041b640`, por tique) [confirmado]

1. Morto (`+0x270`) → nada.
2. Dano (`0x41b9c9`): com dano pendente, **só nos estados diferentes de 1 e 4** e se o
   atacante (`0x4076b7`) não for do tipo 106 (bala de sentinela), vida −= dano; sempre zera
   natureza e dano. Fechado, o casulo é **invulnerável**; aberto, abrindo ou fechando, não.
   Vida ≤ 0 fora do estado 5 → `0x43d629(sentinela, tocado, 0)`, estado 5, liga `0x40`.
3. Toca o rótulo do estado com a rotina de fim `0x41b77c` (`0x407942` modo 1; o −1 com
   `0x407989`). No estado 1, se não passaram 200 ms desde o último som do tipo (global
   `0x488820`), marca o quadro como já sonorizado (`+0x1a4 = +0x190`): o zumbido de espera não
   se acumula entre vários casulos.

## Fim de cada animação (`0x0041b77c`) [confirmado]

Primeiro procura alvo (`0x41ba5c`, abaixo). Depois, pelo estado:

- **1, 4 ou −1** (fechado ou acabando de fechar):
  - sem alerta (`+0x28c = 0`): alvo válido → alerta (`+0x28c = 1`), vira-se para o alvo
    (`+0x194 = 0` se o alvo está à esquerda, 1 se à direita) e **abre** (estado 2); sem alvo →
    estado 1 ou 4 por sorteio (50 %);
  - em alerta: alvo válido e `rand % 100 ≥ 70` (30 %) → vira-se e abre; senão 1 ou 4 (50 %).
- **2** (acabou de abrir) → **dispara** (`0x41bcdb`) e vai ao estado 3.
- **3** (acabou um disparo) → com 45 % e alvo válido, **dispara de novo** e repete o 3; senão
  fecha (estado −1).
- **5** → `+0x270 = 1`, `+0x178 |= 0x48`, `0x407009`, sem colisão, ordem de desenho 39: fica
  como destroço.

## Percepção [confirmado]

`0x41ba5c` (se não há alvo): percorre os jogadores vivos (`+0x19c ≠ 0`, vida > 0) e fica com
o válido de **menor `|dx|`** (`0x41bc93`). Válido (`0x41bacb`):

- vida > 0; `|dy|` (pela posição `+0x150`) **≤ 50**; mesma camada; `|dx|` **≤ 150**;
- nos estados 2 e 3, o alvo tem de estar do lado para onde o casulo olha (`+0x194 = 0` →
  alvo com x ≤ o seu; 1 → x ≥);
- linha livre entre os centros (`0x433aa3`/`0x43735d`).

Falhou → esquece o alvo e o alerta (`0x41bc7b`). **Não há teste de esconderijo**: o Jake nos
estados 164–166 é visto (só o tipo 8 o testa, `furtividade.md`).

## Disparo (`0x0041bcdb`) [confirmado]

Para `i` de 0 a `+0x260 − 1` (3 balas), cria um projétil **tipo 106** ("sentinel bullet",
`osb00.anm`, `projeteis.md`) com `0x40bff8`:

- posição x = `+0x14c + rand%5 ± 10` (+10 olhando à direita, −10 à esquerda); y = `+0x150`
  na primeira e, nas seguintes, a anterior + `rand%12 − 12` (cada bala 1–12 px acima da outra);
- velocidade `vx = ±20` px/tique (lado do casulo), `vy = (−rand) mod 6` (0–5 px/tique, para
  baixo);
- camada e lado do casulo.

A bala 106 (`0x40caf6`) voa reta como a da pistola (`0x40c964`) e ignora colisões com outra
bala 106. Linha `(106, 0)` do `DAMAGE.TBL`: **15 de dano**, empurrão (reação 2), efeito `oex`
sorteado 1–4 com raio 18; contra o barril (`(106, 28)`) natureza 2 → explode.

Uma abertura = 3 balas; a cada fim da animação 3 (7 quadros) há 45 % de nova rajada. Três
balas no Jake: 45 de dano.

## Destruição

Linha `(10, 0)`: sem dano direto, efeito sorteado, raio 28, empurrão 15, dano de área 10 →
`R = 42` px. Vida 4: dois tiros de pistola (2 cada) com o casulo aberto; no fácil (sem
escala de dificuldade aqui) também dois [confirmado: este tipo não lê `0x487854`].

## Sons [confirmado no `SOUND.TBL`, nomes ausentes do `SOUND.RES`]

Tipo 10: quadro 1 (espera) 3061, prioridade 3; quadro 5 (abrindo) 3062; quadro 9 (abrindo)
3060, prioridade 3; quadros 14 e 18 (atirando) 3063.

## Em aberto

- Nomes dos sons 3060–3063.
- `values[5] = 4` em todos os objetos: não lido pelas rotinas vistas.
- `0x407009` (chamado também pelos destroços dos tipos 7 e 8).
