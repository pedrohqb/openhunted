# Jake: escadas e redes (zona 4) e troca de camada (zonas 1 e 2)

As zonas PR das células (`props`, `kind` + retângulo relativo à célula) são lidas por
`0x004075af(obj)`: devolve o **OU dos tipos** das zonas cujo retângulo (espelhado conforme a
célula, `0x405775`) cruza a caixa de colisão do objeto (`0x4351b5`), percorrendo as células
que a caixa cobre na camada do objeto (`+0x17c`) [confirmado]. Tipos vistos nas
`T_J_003/005/007`:

| Tipo | Uso no Jake | Onde |
|---|---|---|
| 1 | CIMA → camada − 1 | `T_J_007`: camadas 1 e 2 |
| 2 | BAIXO → camada + 1 | `T_J_007`: camadas 0 e 1 |
| 4 | escada/rede: CIMA ou BAIXO agarra | `T_J_003` (12 redes), `T_J_005` (3), `T_J_007` (2) |
| 0x40 | saída (`logica-fase.md`) | todas |

(`0x20` é testado no estado 101, `0x425d7d`; não aparece nestas fases.)

Bits de entrada (`0x487a2c`, `tiro-jake.md`): `1` CIMA, `2` BAIXO, `4/8` ESQ./DIR.,
`0x100` ATIRAR, `0x200` PULAR, `0x40000` SOCO.

## Em pé (`0x004239f5`, parte do tratamento comum de "em pé") [confirmado]

Só se PULAR **não** está pressionado e CIMA ou BAIXO está:

1. Há zonas sob o Jake:
   - CIMA e zona 1 e camada > 0 → `+0x268 = 7`, `0x4263ee` (sair para a camada de cima);
   - BAIXO e zona 2 e camada < `0x488e74` (= `grid_z` da fase) → `+0x268 = 0`, `0x4262c9`;
   - CIMA e (zona 4, ou zona 0x40 sendo jogador) → `0x427089`: com ATIRAR/SOCO atira; senão
     **estado 3** (`J003`, 7 quadros; retorno `0x4270c9`).
2. Sem zona útil: BAIXO → `0x426519` (agachar); CIMA → `0x425e82` (o "encostar na parede" da
   dica 5 do `WIZARD.RES`, "Press UP to hide against a wall" [inferido; não decifrado]).

No **estado 3** (`0x423b8e`): CIMA + zona 0x40 → saída (`0x40e4dd`); CIMA + zona 1 / BAIXO +
zona 2 → troca de camada como acima; CIMA + zona 4 → **agarra** (`0x42710a`). Isto é,
segurar CIMA diante de uma rede leva a 3 e, no tique seguinte, à escalada.

**No ar**: dez chamadas iguais em outras rotinas de estado (consultas em `0x424292`,
`0x424328`, `0x424d6d`, `0x424eb6`, `0x4250df`, `0x4253e9`, `0x4254f5`, `0x4256a2`, `0x425a5c`,
`0x425b0a`) [inferido: estados de pulo e queda]: sem PULAR pressionado e com CIMA ou BAIXO, zona 4 → agarra. Daí a dica 4:
"JUMP + UP to grab the ladder" (pular e, no ar, soltar PULAR com CIMA).

## Escalada (`0x0042710a`, por tique) [confirmado]

1. Sem zona 4 sob o Jake → desce da rede (`0x4270e5`: `0x423b2b` tenta pôr de pé, senão
   cai); o mesmo com ATIRAR/SOCO. Antes de sair: liga a gravidade (`&= ~8`) e `+0x1b4 = 120`.
2. PULAR mantido: com ESQ./DIR. → pulo lateral (`0x4254df` com CIMA, senão `0x425144`); só CIMA
   → `0x424dd0`; sem direção → solta (`0x4270e5`).
3. Senão testa cada direção com `0x40718d(obj, dx, dy)` (passo de prova de 4 px; livre se não
   houver contato ou o contato não tiver o bit 2): CIMA (0, −4) → bit 1; se o contato é
   exatamente 2 → `0x424498` (sobe para a borda [inferido]); BAIXO (0, +4) → 2; ESQ. (−4, 0)
   → 4; DIR. (+4, 0) → 8.
4. `+0x178 |= 8` (sem gravidade), `+0x1b4 = 40`.
5. Por combinação (tabela `0x0042736c`):

| Bits | Estado (rótulo) | Animação | Velocidade |
|---|---|---|---|
| 0 | mantém 30 ou 32 (outro → 30) | parado | `0x406ea2` em `+0x16c`, `0x407009` [inferido: para] |
| 1 CIMA | 30 (`J030`, 13 q.) | para a frente (modo 9) | pela animação |
| 2 BAIXO | 30 | para trás (modo 0xd) | pela animação |
| 4 ESQ. / 8 DIR. | 32 (`J032`, 7 q.) | modo 9 | pela animação (lado pelo espelho [inferido]) |
| 5, 9 / 6, 10 (diagonais) | 30 | 9 / 0xd | vertical da animação; `vx = ±vy` |

`0x406fc0(obj, 2, modo, atraso)` tira o deslocamento do **quadro corrente** da tabela de
pontos da animação (`0x406ea2`, diferença entre quadros, espelhada pelo lado `+0x288 & 8`) e
grava `vx = 2·8·dx`, `vy = −2·8·dy` (1/8 px) [confirmado no código; valores dependem do
`JAKE.ANM`]. O atraso é o campo `+0x18` do registro do estado em `0x473300 + 0x70·estado`,
**0** para 3, 11, 12, 30 e 32 (um quadro por tique; nenhuma escrita a ele no executável).

## Troca de camada [confirmado]

Duas rotinas de estado, cada uma de **8 tiques** (contador `+0x268`):

| Rotina | Estado (rótulo) | Contador | Camada |
|---|---|---|---|
| `0x4262c9` (zona 2, BAIXO) | 11 (`J011`, 12 q.) | 0 → 7 | `+0x17c += 1` quando o contador vale 3 (se < `grid_z`) |
| `0x4263ee` (zona 1, CIMA) | 12 (`J012`, 12 q.) | 7 → 0 | `+0x17c −= 1` quando vale 4 (se > 0) |

- Cada tique: zera `+0x274`/`+0x284`, alinha `+0x154/+0x158` à posição e avança a animação
  (`0x4238fa`: troca de estado na primeira vez; depois um quadro a cada `atraso + 1` tiques).
- **Meia-volta**: durante a 11, CIMA + zona 1 + camada > 0 → passa à 12 (com o contador
  onde está); durante a 12, BAIXO + zona 2 + camada < `grid_z` → passa à 11. As entradas são
  zeradas (`0x487a2c = 0`) nessa troca.
- Fim (contador > 7 ou < 0): zera o contador e volta a "em pé" (`0x426ecc`).
- A vista do jogador (`0x436635`) acompanha: guarda a camada antiga e um contador de
  transição (`+0x44` = 1 ou 7, `+0x48` = ±1, `+0x4c = 0x436a70(camada, vista)`) e, no fim,
  `+0x40` = camada nova [inferido: efeito de transição da vista].

Não há troca de camada sem zona; o Jake não muda de camada por colisão. Os objetos só se
enxergam e colidem na mesma camada (`objetos.md`), inclusive minas, olho-laser e fogo.

Qual camada fica "na frente" na tela não foi conferido: o código só diz que BAIXO leva a
`z + 1`. Nos dados, as portas aparecem aos pares no mesmo ponto de células vizinhas em z
(`T_J_007`: zona 2 na célula (3, 2, 0) e zona 1 na (3, 2, 1), ambas em (67, 66, 5×32)).

## Em aberto

- Rotinas `0x424498` (topo da escada), `0x425e82` (CIMA sem zona) e `0x426519` (agachar).
- Valores da tabela de pontos do `JAKE.ANM` para `J030`/`J032` (velocidade real da escalada).
- Se `z + 1 = grid_z` é barrado em outro lugar (a checagem é `z < grid_z`).
- Zona 0x20 (estado 101) e o efeito visual da transição de camada.
