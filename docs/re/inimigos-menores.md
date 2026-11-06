# Objetos hostis menores: dardos, catapulta, Entidade, Camaleão e Morte

Tipos com poucas ocorrências ou mecânica simples na campanha do Jake. O Anticorpo e a
Manta estão em [`anticorpo-manta.md`](anticorpo-manta.md).

## Gerador de dardos (tipo 87, `ODG.ANM`) e dardo (tipo 29) [confirmado]

55 geradores em 11 fases. Nome por variante "… Generator" (`0x473080`, registros de 0x30).
Sprite mínimo (4 × 9, rótulos 1 `odg02` e 2 `odg03`): é um furo de parede.

- Inicialização `0x420a14`: variante `values[0]` (0–4); `+0x174 = 1`; sólido, sem gravidade,
  personagem (`0x4809`), desliga `0x40`; **orientação** `values[1]`: 0 → rótulo/estado 1
  (atira na horizontal), 1 → 2 (na vertical); vida 0 → `+0x18`; `values[3]` (`+0x26c`,
  munição; −1 = infinita) 0 → `+0x24`; `values[4]` (`+0x270`, intervalo) 0 → `+0x20`;
  `values[2]` (`+0x268`, tolerância perpendicular) 0 → 15; `values[5]` (`+0x274`, alcance)
  0 → 350.

| Variante | Vida | Linha de visão | Intervalo | Munição | % dano recebido | Vel. do dardo |
|---|---|---|---|---|---|---|
| 0 Alpha | 35 | não | 15 | 50 | 100 | 20 |
| 1 Beta | 30 | não | 15 | 40 | 70 | 25 |
| 2 Gamma | 25 | não | 10 | 100 | 60 | 25 |
| 3 Delta | 50 | **sim** | 5 | 100 | 40 | 30 |
| 4 All | 40 | não | 2 | 500 | 50 | 35 |

- Atualização `0x420bc2`: até ligar `0x40`, espera os gatilhos (sem gatilhos: liga logo).
  Dano × % (sem dificuldade); vida ≤ 0 → `0x43d629(ger, nulo, 0)` (não há linha 87: nada) e
  some. Alvo: o jogador vivo mais próximo, na mesma camada, a ≤ alcance, com `|dy|` (ou `|dx|`
  na vertical) ≤ tolerância e, no Delta, linha livre. Com alvo e munição: a cada intervalo,
  munição −1 e **um dardo** de `(x+8, y+5)`/`(x−7, y+5)` (horizontal, para o lado do alvo)
  ou `(x+5, y+13)`/`(x+5, y−6)` (vertical), à velocidade da variante.
- Dardo (`0x421154` + `0x40d978`): "Dart", `oda.anm`/`oda.brd`, rótulo 1 ou 2, escala 12,
  sólido, **com gravidade**; nasce com `0x2` e, no 1º tique, `0x43d629(dardo, nulo, 1)`
  (faísca 14). Ao tocar algo ou sair da fase: avança pela velocidade medida,
  `0x43d629(dardo, tocado, 0)` e some. Linhas `(29, …)`: **1** de dano com empurrão.

## Catapulta (tipo 41, `OCA00.ANM`) [confirmado]

72 em 9 fases. Não fere: é uma **plataforma de lançamento** (74 × 18, faixas amarelas).

- `0x4137e4`: "Catapult"; sem gravidade; `0x4821` (só contato); sem colisão própria.
- Liga e desliga com os gatilhos (`0x40` acompanha o resultado; sem gatilhos fica ligada).
- No fim de cada animação procura o primeiro **Jake/Garathe** vivo e o primeiro **barril
  (28)** ou **formiga (109)** vivos a ≤ **30 px** (centros). Estado 0 com alguém → (se
  `values[3]`, vira-se para ele) estado 1, som 3137 "catapult". No quadro 2 do rótulo:
  `0x43d629(catapulta, alvo, 0)` (linhas `(41, …)`: **derrubada**, dano 0 — 1 no barril) e
  impõe o empurrão `(values[0], values[1])` (px/tique, x com o sinal do lado da catapulta):
  nos dados vx 3–27, vy −10…−43. `values[2] = 1` → dispara uma vez só.

## Entidade (tipo 107, `OES.ANM`) [confirmado na estrutura]

8 em 6 fases. "Alpha…Delta Entity" (`0x4721f0`, registros de 0x28: alcance 200/150/250/150,
alcance da confusão 250/200/300/200, `+0x278` 1/2/2/1, chance de virar 100/50/50/50,
velocidade 4/4/2/8).

- Sólida, personagem, com gravidade; `+0x278` (byte) = viva. **Não recebe dano**: um toque
  só a empurra (`0x41293a`: velocidade medida de quem tocou × 5–12). Só some fora da fase.
- `values[2] = 0` → vagueia pela grade de células como o robô de segurança (`0x412a1c`).
- Alvo: jogador vivo na mesma camada, a ≤ alcance, com linha livre. Com alvo: rótulo 2 e
  depois 3 em laço; se ela estiver na zona do alvo (`0x41299d`) e a distância for menor que
  o alcance da confusão, põe **`+0x264 = 3` no jogador** (se ≤ 1): o Jake fica com os
  **controles invertidos** enquanto estiver perto (`tiro-jake.md`, confusão).
- Sons do `SOUND.TBL` (tipo 107): 3113, 3114, 3115.

## Camaleão (tipo 4, `OCH.ANM`) [parcial]

11 em 5 fases, vida 50 nos dados. "Alpha/Beta/Gamma Chameleon" (`0x47256c`, registros de
0x38; as variantes 3 e 4 repetem o nome "Gamma"). Inicialização `0x41853c`: `+0x1c0 =
0x418704`, `+0x1c4 = 0x419391`; colisão `och0.brd` (tabela de 40 nomes por rótulo em
`0x472680`); personagem, com gravidade; estado/rótulo 34; escala 45; `+0x174` e vida padrão
da variante (vida 80/50/80/80/80). `OCH.ANM` tem 294 quadros em 38 rótulos. Sons 3154
"chameleon spawns" e 3155 "chameleon invisible" (fica invisível). Linha `(4, 1)`: natureza
`0x10`, **30** de dano, som 3147. Não se achou o chamador de `0x43d629` com atacante 4 no
código do tipo [em aberto].

## Morte (tipo 3, `ode.anm`) [parcial]

4 objetos em 4 fases (vida 100/80; 3 com gatilhos). "Death"; o leitor força `ODE.BRD`.
Inicialização `0x415700`: `+0x178 |= 0x4803` (o `0x2` a tira das colisões), `+0x174 = 20`,
estado −1, vida 0 → 100.

- **Radiação** (`0x416a48`): todo objeto na camada a menos de 45 px em x **e** em y recebe
  `0x43d629(morte, obj, 0)`: linha `(3, 1)`, **4** de dano por chamada no jogador, empurrão
  fixo (3, 0); toca 3156 "death radiation".
- **Tiro** (`0x416696`): bala tipo 99 ("death bullet") de `(x+30 ou x−20, y+2)`, `vx = ±20`,
  `vy` sorteado em ±s/2 com `s = max(10, (2 − dificuldade)·15/2)`. Linha `(99, 0)`: **15**,
  empurrão, efeito sorteado raio 28.
- Dano recebido com dificuldade (fácil ×2, difícil ÷2); vida ≤ 0 → estado 11 (`ode10`, 15
  quadros), liga `0x40`, fala 3764. Sons 3051 "Death ascending", 3763.

## Em aberto

- Camaleão: máquina de estados, invisibilidade e o ataque (linha 4).
- Morte: estados e o papel dos gatilhos (aparição?).
- Entidade: `0x41299d`/`0x436635` (zona do alvo) e o vaguear exato.
