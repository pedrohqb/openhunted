# Dois jogadores: modos, tela dividida e regras

Telas em [`menus-restantes.md`](menus-restantes.md); veredito em [`logica-fase.md`](logica-fase.md);
morte em [`dano-vida.md`](dano-vida.md); IA em [`minotauro-ia.md`](minotauro-ia.md).
**[confirmado]** = lido no código/dados; **[inferido]** = dedução.

## Sem rede [confirmado]

Importa só KERNEL32, USER32, GDI32, WINMM (tempo, joystick, CD), DSOUND, COMDLG32 e ADVAPI32;
o único `LoadLibrary` nomeado é `WING32.DLL`. Nenhuma DLL ou texto de DirectPlay, winsock, IPX,
modem ou série. Os dois jogadores dividem **uma máquina e uma tela** (tela dividida).

## Modos [confirmado]

Só dois, sem equipes: **disputa** (`MMU_HEAD2HEAD`, `CLM_2PL_HEAD2HEAD`, `DMM_*`,
`process_deathmatch_interface`) e **cooperativo** (`MMU_COOPERATIVE`, `CLM_2PL_COOP`).

| Modo | `0x497300` | `0x497304` | lista `0x4974b4` | `p=` | fases |
|---|---|---|---|---|---|
| disputa | 2 | 0 | 0x5601 | 3 | 20 `M_2_*`, `M_J_*`, `M_M_*` |
| cooperativo | 2 | 1 | 0x5602 | 4 | 15: 8 `C_2_*` (Jake + Garathe), 5 `C_J_*` (2 Jakes), 2 `C_M_*` (2 Garathes) |

Máscara do modo `0x4972f8` (`0x43c95c`): índice `(pers1 + pers2 + 4·coop) · (jogadores > 1)` →
0/1 → **2** (solo); disputa 2 → **4** (Jake×Jake), 3 → **8** (Jake×Garathe), 4 → **0x10**
(Garathe×Garathe); coop 6/7/8 → 0x20/0x40/0x80, mas `0x43cbb0` troca por **0xe0** em todo
cooperativo. Bits de `+0x1a8`: 1 vaga de jogador, 2 solo, 4/8/0x10 disputa, 0x20/0x40/0x80 coop.

Nos dados: toda `M_*` tem 2 vagas Jake `0xd` e 2 Garathe `0x19` (`M_2_006` 3+3; `M_2_012` uma
`0xd9`); `C_2` uma `0x41` de cada; `C_J` duas Jake `0x21`; `C_M` duas Garathe `0x81`. Objetos
com máscaras parciais (`0x4`, `0x10`, `0xc`, `0x18`, `0x1c`, `0xfa`…) mudam o conteúdo da
disputa conforme o par; as saídas das `C_*` são `0xe2`/`0xe0`.

## Telas [confirmado]

- `SMULT` → `SLEVL` na lista do modo. Na `SLEVL`, 22016–22018 gravam `0x497300`/`0x497304`
  (`0x4413aa`) e, se o modo muda, chamam `0x440fe0` (zera campanha e recordes em memória).
- `CLM_LOAD`: disputa → `SEGOC` (estado 15, volta 5); **cooperativo carrega e vai direto ao
  briefing `SLEVD`** (sem `SEGOC`).
- `SEGOC`, um jogador por vez (`0x480380`): OK grava personagem `0x497318[i]` e cor
  `0x497320[i]`; depois do 2º carrega a fase e vai ao estado 6, que na disputa abre `SHVLD`
  (`0x4414e0`). CANCEL/Esc: no 2º volta ao 1º; no 1º, ao estado anterior. Cores: 15012 marrom
  **36**, 15013 vermelho **38**, 15014 preto **35**, 15015 branco **37**. Padrão (`0x43d33b`):
  J1 Jake/35, J2 Garathe/36. Nada impede escolhas iguais.
- `SHVLD`: `0x480560` = **tempo** (32007; 32003 +, 32004 −), `0x480564` = **pontos** (32008;
  32005 +, 32006 −), guardados ×20, passo 20, 0–1980 (mostra 0–99 com os dígitos de rolo da
  `INVBAR01`); botão do mouse preso repete. Abre com os limites anteriores (0 no começo =
  sem limite). GO → `0x497374` = minutos, `0x497378` = pontos, estado 7.

## Atribuição (`0x43cbb0`) [confirmado]

Para cada jogador `i`, candidatos (`0x43cd91`): bit 0 de `+0x1a8`, `+0x1a8 & m ≠ 0` (solo 2,
disputa **0x1c**, coop 0xe0) e tipo = `0x497318[i]` (0 aceita 1 ou 2), menos os já tomados.
Nenhum → caixa "Unable to find a human object to assign to player %d!". Disputa: sorteio; coop:
os personagens são **zerados antes** e fica o primeiro da lista montada (o último na ordem da
lista de objetos). Grava `0x497318[i]` = tipo, `+0x19c = i+1`, `+0x198` = dispositivo
(`0x43edc1(i+1)`), na disputa `+0x280 = 0x497320[i]` (a cor da `SEGOC`), `+0x27c = +0x280`,
`+0x1ac |= 8`. Depois `0x43cb28` remove o que não está no modo e as vagas sem jogador.

## Entrada [confirmado]

Dispositivos `0x499014[1…6]`: `Keyboard1`, `Keyboard2`, `Joystick1–4`. Chaves `Player1Input`
e `Player2Input` (o byte 6 do nome é trocado). `0x43edc1`: J1 = o do INI ou teclado 1; J2 = o do
INI se ≠ J1; igual → teclado 2; vazio → joystick 1 se existe e J1 não o usa, senão teclado 2;
ausente → teclado 1/2. Teclado: copia as 10 teclas de `0x480048` (J1) ou `0x480098` (J2) para
`+0xb4…+0xbd` (padrões em `tiro-jake.md`), leitor `0x448503`; joystick: botões de `0x480224…`.
Os dois teclados são conjuntos de teclas do mesmo teclado. Cada personagem lê o seu
dispositivo (`+0x198`) a cada tique (`0x407e2d`).

## Tela dividida [confirmado]

- Vistas em `0x493b70` (0x5c bytes, `+0x58` próxima): `+0x00/+0x04` centro, `+0x08` zoom (20),
  `+0x20…+0x2c` retângulo na tela, `+0x30…+0x3c` no mundo, `+0x40` camada, `+0x50` objeto,
  `+0x54` barra. Uma por jogador (`0x43632b` → `0x4360a0`), ordenadas por `+0x19c`.
- Layout `0x4361c8`: `n` vistas com jogador; colunas = 2 se `n > 2`, senão 1; linhas =
  `(n+1)/2` ou `n`. Com dois: **J1 em cima (640×240), J2 embaixo**. Refeito ao redimensionar
  (`0x401d80`) e ao liberar o objeto da vista (`0x433e2c` → `0x43615a`).
- Barra (`0x40a4ef`): com `0x497300 ≥ 2` alturas pela metade (`0x471cd0`): 37 px, desenhada
  achatada (`0x45aee4`). Jogo em 640×203 em y = 0 e 240; barras em y = 203 e 443. Cada barra
  mostra o seu jogador; o cronômetro (global) aparece nas duas no coop.
- Escala por camada (`0x436a70`): altura de referência **240** (480 no solo): a ampliação é
  praticamente a do solo (240/203 ≈ 480/405); cada metade mostra metade da altura de mundo.
- Câmera (`0x436386`), a mesma para cada vista: alvo = centro horizontal e topo do quadro do
  objeto na posição do tique anterior (`+0x154/+0x158`); célula com bit 1 de `+0x5c` fixa o ponto (`+0x64/+0x68`); tremor ±5
  com `+0x274`; zoom ±1 até o `+0x60` da célula (padrão 20); passo máximo **18 px/tique**.
- Ouvinte do som = vista mais próxima (`som.md`); a dica do mago usa a 1ª vista (`dicas.md`).

## Regras comuns [confirmado salvo indicação]

- Tique = 1/20 s (cronômetro `0x4972fc` = segundos × 20).
- Crédito `+0x1b0`: ao aplicar dano com vítima viva (`0x43d6e3`), atacante tipo 1/2 com
  jogador → número dele; senão herda o `+0x1b0` do atacante (projéteis, explosões
  `0x40d649`); com dois jogadores um crédito 0 também sobrescreve. Vivo, o personagem zera o
  seu crédito todo tique (`0x4232ec`, `0x4284b9`): **só o golpe fatal conta**.
- Estatísticas `0x49732c`: 3 registros de 24 bytes (`k` = 0 totais, 1, 2): `+0` segredos,
  `+4` inimigos mortos, `+8` **F** (eliminações do outro), `+0xc` **S** (autoeliminações),
  `+0x10` tesouros, `+0x14` peças (tipo 9). `0x43cfcf` conta por crédito.
- Fogo amigo: nenhum dos 30 usos de `0x497304` está no dano ou nos projéteis → no coop
  os jogadores se ferem como na disputa [inferido pela ausência].
- Coleta (`0x40b43a`): solo e coop só com `0x100`/`0x200` do personagem; disputa, qualquer
  um (a aceitação `0x40b6ed` vale igual).

## Cooperativo [confirmado]

- Morte: cadáver, sem renascer; a vista segue o cadáver.
- Derrota quando **todos** os jogadores estão mortos (`0x43ce57`) ou o campo 1004 esgota.
  A saída consulta `0x43cdf0(0)` como no solo: **basta um** entrar; os dois vencem.
- Itens não renascem. Anúncio: vitória 3780 + 3788; tempo 3776 + 3787; outra 3777 + 3787.

## Disputa [confirmado salvo indicação]

- Campo 1004 ignorado (`0x488e54 = 0`); `0x497474` = `timeGetTime` no início.
- Fim da morte (`0x4268ac` Jake, `0x42b604` Garathe): corpo clonado (`0x433d80`, cópia de
  0x2b0 bytes; `+0x178 |= 0x8038`, sem colisão, `+0x19c = +0x198 = +0x1a8 = 0`, sem rotinas,
  ordem 0x27). Crédito `c`: 0 → nada; `c` = vítima → `S_c += 1`, limitado a `F_c`; outro →
  `F_c += 1`.
- Renascer: `y −= 20`, apaga `0x200008`, ordem 0x78/0x79, posição `0x43d2b5`, estado 181
  (`0x426b09`) / 80 (`0x42b861`): zera os 30 pares (armas, munição, chaves, bandeiras),
  veneno, atordoamento, choque e piscar; melhor arma (punhos/garras); **vida 100**; a
  armadura fica a de antes. Barra redesenhada, vista recentrada na hora. Sem invencibilidade
  vista.
- Pontos de renascimento (`0x43d22a`, no início): células com bit 4 do `+0x38` (`fields[14]`
  do JSON), até **16** em `0x4973b0` (12 bytes: `128·cx + 64`, `128·cy + 32`, camada), total
  `0x497470`. Sorteio uniforme, sem olhar o rival. As 20 `M_*` têm 2–17 (`M_2_002`: 17).
- **Itens renascem** (`0x412fec`, só na disputa, salvo `0x10`): cria "Regenerate" (tipo 43,
  `org.anm` rótulo 1) do tamanho do item; espera **300 tiques (15 s)**, anima e no quadro 8
  devolve o `+0x178` do item. Só a coleta genérica a chama: chaves e bandeiras não [inferido].
- Saída: interruptor comum, sem teste de objetivos.
- Fim (`0x43cdf0`): limite de pontos ≠ 0 → ignora objetivos; fim quando `F_k − S_k ≥` limite.
  Limite de tempo ≠ 0 → fim quando `timeGetTime > início + min·60 000` (relógio real).
  Sem limite de pontos valem os objetivos: ligar a saída dá 1; os dois mortos juntos, 2.
  Sem limite e sem saída ligada, só o menu encerra. A espera de 40 tiques vale.
- Resultado: limite → 2 (`MH2H_FAILURE`); saída → 1. Anúncio pelo `F − S`: J1 maior 3790,
  J2 maior 3792, empate 3773 [inferido: "jogador 1/2 vence", "empate"].
- Barra (`0x40b089`): no lugar do cronômetro, quadro 194 à direita e `max(0, F − S)` no
  estilo 195.

## Resultado `SH2HS` (`0x443c30`) [confirmado]

Por jogador `k`: `NKILLS`/`SKILLS` (×30)/`PKILLS` (% do total), `NTREAS`/`STREAS` (×65)/
`PTREAS`, `NSEC`/`SSEC` (×215)/`PSEC`; só na disputa `KILLS` = `S_k`, `KILLO` = `F_k`; `SCORE`
= 30·mortos + 65·tesouros + 215·segredos (sem F nem bônus). Total 0 → 100%. Conta 5/quadro
com 3093; tecla salta. 30033 título; cursor em NEXT. REPLAY recarrega (estado 6); NEXT
`0x440e95`; MAIN → 2; Esc → 2 salvo vindo do jogo. Sem recorde, gravação nem heróis. O corpo
clonado conta como inimigo do matador (tipo 1/2 sem vaga): cada F soma 1 a `NKILLS` e ao
total [inferido do caminho, não testado].

## O que o motor precisa [inferido]

1. N jogadores por índice (`+0x19c`, dispositivo, cor, estatísticas), sem "o jogador".
2. Vistas: lista, layout, câmera e barra por vista; escala de referência 240.
3. Alvos: Minotauro/Jake IA (dois primeiros), minas, olho, robôs, gerador, dardos, Camaleão e
   Entidade já escolhem entre jogadores; conferir cada "jogador único" restante.
4. Crédito por golpe fatal, clone de cadáver, renascimento e itens que renascem.
5. Veredito, barra, anúncio e `SH2HS` por modo.

## Fatias (estimativa)

1. Modos e atribuição: máscaras, filtro da lista, `SMULT`/`SLEVL`/`SEGOC`/`SHVLD`, vagas.
2. Entrada dupla: teclados 1/2, joystick, `Player*Input`, `SINPU`/`SKEYB`/`SJOYS`.
3. Tela dividida: vistas, câmera por vista, barra achatada, ouvinte do som.
4. Cooperativo: veredito, saída por um, coleta, anúncio, `SH2HS` coop.
5. Disputa: crédito, F/S, renascimento, regeneração, limites, barra de pontos, `SH2HS`.
6. Auditoria de IA e inimigos com dois alvos; testes de ponta a ponta.

## Em aberto

- Ordem real J1/J2 no coop (lista por `+0x1b4`?); quem é J1 em `C_2`.
- Animação dos estados 181/80 e se o renascer é instantâneo na tela.
- Se a pausa congela o limite de tempo (relógio real).
- Nomes dos sons 3773, 3787, 3788, 3790, 3792.
- "Next" no multijogador pula índices com recorde do solo (mesmo `0x47fa8c`)?
