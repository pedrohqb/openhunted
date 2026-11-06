# Barril (tipo 28, `OOB`), fogo (tipo 42, `OFI`) e cerca elétrica (tipo 40, `OEF`)

Barris: 2–35 por fase; na `T_J_005` dois empilhados bloqueiam o corredor (x = 959) ao lado do
grafite "EXPLOSIVE"; na `T_J_007` dois junto da saída, ao lado de "FLAMMABLE". Fogo: dois
no fosso da `T_J_005`. Cerca: uma na `T_J_007`.

## Barril (tipo 28)

### Inicialização (`0x0040db74`) [confirmado]

- Estado `+0x180`: 3 se a fase deu 3, senão 1; rótulo inicial = estado. Genérica `0x40b2c8`.
- Colisão: `OOB02.BRD` no estado 3, senão `OOB01.BRD`.
- Vida 1; `+0x1c0 = 0x40dc00`; `+0x178 &= ~0x28` (com **gravidade** e **sólido**);
  `+0x174 = 50`; `+0x1b4 = 85`. Nos dados: sinalizadores `0x80181`/`0x80185`.

`OOB.ANM`: rótulo 1 `oob00` (1 quadro, em pé), 2 `oob01` (2, tombando), 3 `oob02` (7,
rolando), 4 `oob03` (1, não usado no código visto).

### Atualização (`0x0040dc00`) [confirmado]

Estados: **1** em pé, **2** tombando, **3** rolando, **4** explodindo (só dentro do tique).

1. Fora da fase → destruído. Fora do estado 3, anima em laço (`0x406c71(obj, 1)`).
2. Quem toca: `+0x21c` (tipo dele), ou −1 se o contato `+0x224` é de parede (`+0x220 & 2`),
   ou o tipo de `+0x224` se `+0x220 & 1` e não só-de-toque.
3. Tocado pelo **Jake (1), Garathe (2) ou Bruiser (7)** via `+0x21c`:
   - em pé → passa a tombar (estado 2), guarda x em `+0x27c`;
   - rolando e o tocador anda (`+0x15c ≠ 0`): `+0x164 −= 5` (tocador para a esquerda) ou
     `+= 7` (para a direita): o personagem **empurra** o barril.
4. Batida (tocador −1/parede, 8, 28, 50–52 ou 56): com `+0x22c` (dados da colisão:
   `+0x14` > `+0x18` → batida vertical), se `|vy|` (ou `|vx|` na horizontal) > **80** (10 px/
   tique) → **explode**; zera a componente. Barril que cai ou rola rápido contra parede,
   chão, outro barril, parede `OH` ou plataforma `OPL` explode.
5. Outro objeto vivo em contato próprio (`+0x224`): `s = |velocidade|/8`; se `s ≥ 10`, o
   objeto leva empurrão `(cos, sen)·s` na direção barril→objeto, **derrubada** (`+0x250 |= 4`)
   e `s/2` de dano; o barril para.
6. Gatilhos (`0x435bd8`) > 0 → explode (sem gatilhos, −1, não conta).
7. Dano pendente com natureza `0x2` (explosão) ou `0x4` (fogo) → **explode**. Senão, com
   reação "derrubada" (`+0x250 & 4`) e dano: `|empurrão|·2/3` vira velocidade na direção do
   empurrão (`(cos, sen)·s >> 11`). Zera natureza, reação e dano.
8. Estado 2 no quadro 0 (fim do tombo) → colisão `OOB02.BRD`, estado 3. Troca de estado →
   `0x406b6c(obj, estado, estado)` (rótulo = estado).
9. Rolando: quadro = `(x_inicial − x)/4` reduzido a `0…n` (n = quadros do rótulo): o desenho
   gira com a distância percorrida.
10. **Explosão**: `0x43d629(barril, nulo, 0)`; vida 0; liga `0x40`; se `values[0] ≠ 0`, cria
    fogo (tipo 42) no chão em volta (`0x40fb58(&pos, camada, 384)`); destrói o barril.

### Linhas do `DAMAGE.TBL` [confirmado]

Atacante 28 (a explosão): efeito `oex` sorteado 1–4, **raio 96**, empurrão 40, **dano de área
50** → alcance `R = 1,5·96 = 144` px; no centro 50 de dano e derrubada (empurrão > 7).

Vítima 28 (o que detona): bala 19 (`(19, 0)`, natureza 2), faca 25, especial do Jake
(`(1, 28, 8)`), Garathe (`(2, 28, *)`), 106 e cerca 40: natureza 2 → explode. A explosão de
área (minas, granadas, foguetes, outro barril, olho-laser) marca natureza 2 → explode
(**reação em cadeia**). Raio laser (36): natureza 10 → explode. Fogo (42): natureza 4 →
explode. Atacante 41: só empurra (reação 4, dano 1). O soco/ataque comum do Jake (linhas
`(1, 0, …)`, natureza 0, reação 2) **não** detona nem empurra.

Atirar no barril de longe (≥ 144 px entre centros) é seguro; encostar só o tomba e rola.

## Fogo (tipo 42)

### Inicialização (`0x004100a2`) [confirmado]

- Animação `OFI.ANM` (rótulo 1 `ofi00`, 16 quadros, 192×63) se a fase não der outra; estado
  0 → 1; rótulo = estado; genérica.
- Altura `+0x290`: se `+0x294 < 1000`, calcula pela escala do quadro; senão usa `values[4]`
  (`+0x270`); limita a 1…96 (≤ 0 → 32). Escala `+0x1f0/+0x1f4` = altura / altura do quadro.
- Caixa `+0x20c..` = (0, 0, `altura·512/169`, altura) (≈ 3,03 × altura).
- Liga `0x28` (sem gravidade, só contato), desliga `0x80`; `+0x1b4 = 175`.
- `+0x280` = `values[0]` ou, se 0, `−values[1]`; `+0x284 = |+0x280|/10`; `+0x26c ≥ 0`.
- Âncora: `+0x288` = centro x, `+0x28c` = base y. `+0x294 = 1000` (já iniciado).
- `+0x1c0 = 0x41028c`, `+0x1c8 = 0x4104a3`, `+0x1d0 = 0x4105df` (desenho).

### Atualização (`0x0041028c`) [confirmado]

- Gatilhos falsos → esconde (`+0x178 |= 2`; ao esconder chama `0x43c774` na âncora). Sem
  gatilhos (−1) ou verdadeiros: liga `0x40`, mostra, anima em laço.
- **Crescer** (`+0x280 > 0`): a cada `values[0]/10` tiques soma `values[0] % 10` à altura; ao
  passar do máximo (`values[3]`, 0 → 96) fixa no máximo, `+0x284 = values[2]`,
  `+0x280 = −values[1]`.
- **Minguar** (`+0x280 < 0`): a cada `values[1]/10` tiques tira `values[1] % 10`; ≤ 0 →
  `0x43c774` e destrói (apagou). Com `values[1] = 0` o fogo fica estável.
- Refaz caixa e posição a partir da âncora (centro x, base y); som por quadro
  (`0x43c2b6(42, quadro, âncora, camada)`, via `SOUND.TBL`).

Na `T_J_005` (`values` = 19, 0, 999, 32, 38, …): cresce 9 px por tique até 32 e fica.
Caixas: centro 1164 → x 1116–1212; centro 1098 → x 1050–1146; y 349–381 (fundo do fosso).

### Queimar (`0x004104a3`, rotina `+0x1c8`) [confirmado]

Se o fogo está presente (`0x80003 → 0x80001`, modo de jogo, vaga): para cada outro objeto
presente na mesma camada cuja caixa de colisão (`0x4351b5`, ou a do quadro se vazia) cruza a
caixa do fogo → `0x43d629(fogo, objeto, 0)`. Linha 42: natureza `0x4`, **1 de dano** direto,
sem reação nem efeito. Por tique em que o Jake está dentro: −1 (a armadura absorve conforme
`dano-vida.md`). Barris dentro do fogo explodem.

## Cerca elétrica (tipo 40)

### Inicialização (`0x00413f40`) [confirmado]

- Ignora se `+0x17a & 2` (`0x20000`, já iniciada) ou variante `values[0] < 0`; tira o bit 2 de
  `+0x194` (espelho vertical) se houver.
- Genérica; `OEF.ANM`/`OEF.BRD`; `+0x1c0 = 0x4140ab`, `+0x1d0 = 0x414363`; estado/rótulo 1;
  nome da variante (tabela `0x004722d0`, 36 bytes: "A/B/C Electric Fence" + 4 inteiros).
- `+0x178`: desliga `0x80`/`0x40`, liga `0x8` e `0x4821` (sem gravidade, só contato, `0x4000`:
  não barra personagens); `+0x1b4 = 165`.
- Não sendo cópia (`0x10`): cria o **par** (`0x4145a5`): cópia tipo 40 com `0x10`, x + 10,
  y ± 5, rótulo 3, ligada em `+0x274`; liga `0x20000`.

Variantes (`+0x26c` passo, `+0x270`, `+0x264` curso, `+0x268`): A (1, 32, 50, 7),
B (2, 15, 50, 3), C (3, 0, 100, 6); copiadas por `0x414739` a cada tique (variante < 3).

### Atualização (`0x004140ab`) [confirmado]

- Desligada (`0x40` apagado): liga quando os gatilhos dão verdadeiro (sem gatilhos: já no
  primeiro tique).
- Ligada: uma faísca `+0x27c` (byte) vai e volta de 0 a `curso − 4` de `+0x26c` px por tique.
- `+0x280` (recarga): quando 0 e no estado 2 (faiscando), `0x41494b` procura vítimas vivas
  com o bit `0x80000`: mesma camada, centro x a ≤ 8 px de `caixa.x + 5 + faísca` e caixa
  vertical contendo `caixa.y + (byte +0x27f) − 24` (+32 se espelhada) [geometria inferida];
  cada uma recebe `0x43d629(cerca, vítima, 0)`: linha 40 → natureza `0x20` (elétrico),
  **5 de dano** (barril: natureza 2, explode). Com acerto, recarga de 10 tiques e som em laço
  3135/3136 "Electric Fence Zap" (sorteio, `0x8c3f/0x8c40`); sem acerto e sem recarga,
  `0x43c774` (cala).
- Ao fim de cada ciclo do rótulo (`0x414272`): estado 1 → 20% de ir a 2; estado 2 → 40% de
  continuar; rótulos 1/2 (`oef01` parado, `oef02` faíscas, 12) e 3/4 no par.

No Jake, natureza elétrica → estado 15 de choque por 10 tiques (`dano-vida.md`).

## Em aberto

- `0x40fb58` (fogo deixado por barril com `values[0] ≠ 0`) e o rótulo 4 `oob03`.
- Campo `+0x22c` da colisão (as duas larguras comparadas) e o sinal do "−5/+7".
- Geometria exata da faísca (`+0x27e/+0x27f`) e o desenho `0x414363`.
- `0x43c774` (parar som em laço no ponto?).
