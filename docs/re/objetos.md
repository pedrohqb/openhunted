# Objetos das fases (engenharia reversa, M2)

## Animação e colisão vêm da própria fase

O primeiro texto de cada objeto (`LevelObject::names[0]`) é o **arquivo de animação** e o
segundo (`names[1]`) o **arquivo de colisão**. O leitor de objetos (`0x00434170`) os passa a
`0x00434ccc` (animação, campo `+0x1d8`) e `0x00434ec1` (colisão, `+0x1f8`). Para versões
antigas e alguns tipos (`0x20`, `0x22`, `0x03`, `0x07`, `0x49`), o leitor troca o nome da
colisão por um fixo; os dados da 1.00 já trazem os nomes certos.

Exemplos na `T_J_001`: Jake (tipo 1) → `JAKE.ANM` / `JK_DEFLT.BRD`; minas (tipo 102) →
`OHM00.ANM` / `OHM00.BRD`; interruptor (tipo 5) → `OSW.ANM`.

Objetos sem animação (nome vazio) são objetos de lógica: tipos 73 (388 ocorrências), 30 (32),
0 (32) e 27 (1). Um objeto de tipo 0 aponta para `EDITOR.INI` (resto do editor de fases).
Todas as demais animações de objeto e de decoração existem nos dados.

## Rótulo inicial e escala

O leitor de objetos (`0x00434170`) zera a escala (1/1) e mede o quadro de referência com
`0x0043585c` → `0x00447ff0` (`0x004358b3`). O quadro é o primeiro do rótulo cujo id está no
campo `+0x1ec` (busca em `0x004470fa`) quando o bit 0x80 dos sinalizadores `+0x178`
(`fields[3]`) está ligado; senão, o quadro 0. Só depois chama a inicialização do tipo.

Nos dados, as plataformas do `OPO.ANM` usam o próprio tipo ou uma variante (68 → 68, 70 → 70,
72 → 272), as peças do `OPL.ANM` (tipo 56) um dos rótulos 1 a 12 (uma peça por rótulo), e os
inimigos `OH1`/`OH2` um estado (50…55, 300…). O Jake e os grafites (tipo 6) não têm o bit: a
referência é o quadro 0. Com o quadro 0 em todos (M3a), a plataforma 72 saía 6,4× maior.

## Presença por modo de jogo e jogador

O laço de atualização (`0x004045a5`, em `0x00404603`) e a colisão só consideram um objeto se
`modos & modo_do_jogo ≠ 0` ou se o sinalizador 0x10 de `+0x178` está ligado. `modos` é o campo
`+0x1a8` (versão ≥ 1010; senão 0xfe, e o leitor liga o bit 0 se a versão < 1011 e `+0x19c` ≠
0). O modo do jogo é a máscara global `0x4972f8`, fixada no início da partida (`0x0043c9c2`…):
0x2 é o padrão (solo); 0x8 a 0x80 e 0xe0 aparecem nos modos de vários jogadores.

O bit 0 de `modos` marca uma **vaga de jogador**: o objeto só existe com um jogador atribuído
(`+0x19c`, zerado pelo leitor). Nas 65 missões solo do `ENGLEVEL.TXT` (jogador 1 = Jake, 2 =
Garathe) há exatamente um objeto com `modos & 3 = 3`, do tipo do jogador: é o início dele.
Outros objetos de tipo 1 com 0xfe (presentes no solo) aparecem em algumas fases (`D_J_009`,
`H_J_010`); os de vagas 0x21/0x41/0x81 (fases `C_*`) e 0xd/0x19 (fases `M_*`) são dos modos de
vários jogadores.

## Colisão entre objetos

`0x00436caf` move um objeto (m) contra cada outro (o) da lista. O outro só conta se:

1. sinalizadores de o & 0x80003 = 0x80001 (bit 0 ligado, bit 1 desligado; 0x80000 é ligado pelo
   leitor em todos, `0x0043460e`);
2. presente no modo (acima); se for vaga de jogador, com jogador atribuído;
3. mesma camada (`+0x17c`, `fields[4]`);
4. o par não é excluído: m e o com 0x2000; m e o com 0x4000 (personagens não se barram); m com
   0x10000, o sem 0x10000 e com 0x4000 (e o simétrico); m com 0x40000 e o com 0x4000 (e o
   simétrico); m e o com 0x40000 → sorteio (`0x00460a74`).

Com 0x20 no outro, é só contato (itens do `OGF`, sinalizadores 0x2b/0x2f): registra um contato
de tipo 5 em `+0x220` e não barra. Senão, as formas dos dois se cruzam, a rotina de evento de m
(`+0x1d4`) pode vetar (evento 9; a do Jake, `0x00407774`, não veta) e a velocidade é cortada.

As formas do BRD do objeto (`+0x208`, carregado de `names[1]`) ficam relativas à posição do
objeto (`+0x14c`, `+0x150`), em px da fase: o `OPL.BRD` tem uma caixa 18 × 5 a 3 px do topo
de uma peça de 17 px de largura.

Sinalizadores típicos: Jake 0x94801; peças `OPL` 0x80089 (sólidas para o Jake); obstáculos
`OPO` 0x80089 sem BRD (não barram nada; envenenam: ver abaixo); minas saltadoras `OHS` (tipo 11, "Mine"; a sentinela é `OSE`, tipo 10 — ver `robo-sentinela.md`) e minas `OHM` 0x80801 (sólidas); inimigos com 0x4000 (`OSE`, `OSR`,
`OBR`, `ODG`, Minotauro…), que o Jake atravessa.

## Tipos 68–72: obstáculos `OPO` (`0x0040f324`) [confirmado]

Não são plataformas. A inicialização carrega o `OPO.BRD` só no 71 (os demais ficam sem BRD:
`+0x1f8 = 0`, `+0x208 = 0`), lê o `OPO.ANM`, põe o rótulo inicial = tipo (`+0x1ec`; no 71 soma
`+0x278`), chama a inicialização comum `0x0040b2c8`, liga o sinalizador 0x8 (sem gravidade),
`+0x1b4 = 0x3c`, `+0x260 = 10` se a fase deu 0, e instala por tipo (`tipo − 68`, `0x0040f3b1`):

| Tipo | `+0x1c0` | `+0x1c8` | Comportamento |
|---|---|---|---|
| 68, 69 | `0x0040f459` | `0x0040fa16` | poça de veneno: `0x00406c71(obj, 1)` (animação em laço) e o teste abaixo |
| 70 | `0x0040f46b` | `0x0040fa16` | saliência que goteja: animação em laço; se `+0x264` ≠ 0, quando o temporizador `+0x27c` vence cria um objeto de tipo 71 (gota) sob si, numa posição horizontal sorteada dentro da própria largura, com escala 8/largura (o retângulo de `0x447ff0` é x, y, largura, altura) e 0x10 ligado, e rearma o temporizador sorteando a partir de `+0x264`; liga 0x20 em si (só contato); e o teste abaixo |
| 71 | `0x0040f60a` | — | a gota: carrega `OPO.BRD` (caixa 3 × 2); estado 0 → ao fim da animação, estado 100: desliga 0x8 e 0x20 (ganha gravidade), y += 17; ao tocar algo (`0x004076d0`) ou sair da fase, se tocou o jogador aplica `0x0040b9eb(…, 0xe85, 0xe99, 200, 200)`, estado 200 (respingo), 0x28 ligado, velocidade zero; estado 200 → ao fim da animação, destruída (`0x00435b63`). Rótulo = 71 + estado (71, 171, 271). A inicialização ainda põe `+0x18c = +0x190 − +0x184`, `+0x184 = 0` e liga 0x4 |
| 72 | `0x0040f801` | `0x0040fa16` | portão: estados 1 (aberto, espera `+0x26c`), 2 (fechado, espera `+0x268`), 3 (fechando, animação invertida), 4 (abrindo); troca quando os gatilhos (`0x00435bd8` sobre `+0x11c`) disparam ou o temporizador vence; rótulos pela tabela `0x00472070` = {0, 72, 272, 172, 172}; sons 0xc60 (fechar) e 0xc62 (abrir); escala forçada a 64/altura (`+0x1f0 = 0x40`), `+0x278 = 0` e chama a rotina já na inicialização. Nos dados começa no rótulo 272 (estado 2) |

### O teste `0x0040fa16` (rotina `+0x1c8`, a cada tique por `0x004076ff`) [confirmado]

1. Só quando o contador global de tiques `0x4879c4` é múltiplo de `+0x260` (se `+0x260` ≠ 0).
   O portão no estado 2 (`+0x278 = 2`) não testa.
2. Retângulo do obstáculo: o quadro de referência na escala, na posição (`0x0043585c`).
3. Para cada objeto da lista (`0x488e7c`, encadeada por `+0x1c`) que não seja ele: sinalizadores
   & 0x80003 = 0x80001, presente no modo (`+0x1a8 & 0x4972f8` ou 0x10), com jogador atribuído
   se for vaga de jogador, mesma camada (`+0x17c`) e caixa (`0x004351b5`) cruzando o retângulo
   (`0x00405698`):
   - se é jogador (tipo 1 ou 2): com `+0x270 = 0` e `+0x19c ≠ 0` (humano), fala
     `0x0040b9eb(obj, 0xe85, 0xe99, 200, 200)` (3717 Jake / 3737 Garathe, "Oh no it's
     poison"); em todo caso **`+0x270 = 1`** (envenenado; atribui, não soma: dois venenos
     apanhados voltam a 1);
   - `0x0043d629(obstáculo, obj, 0)`: linhas `(68…71, qualquer, 0)` do `DAMAGE.TBL`, natureza
     veneno (1), **1** de dano; `(72, qualquer, 0)`, veneno, **5**.

É a única escrita de `+0x270` do jogador fora da coleta (`0x0040b6c3`) e do antídoto: os
obstáculos `OPO` envenenam (1 por 20 tiques, `0x004231f9`, até o antídoto 37). A D_M_010
("Your friend is poisoned!") depende disso: o antídoto-objetivo só se apanha envenenado e a
fase não tem itens de veneno (33/35), só poças (68) e saliências (70). A leitura antiga
("esmagamento pelo portão e pelo bloco", "só animação") estava errada.

Uso nas fases (todas as 100 do `LEVELS.VOL`): **68** em 33 fases (83 objetos), **69** em 1
(`M_2_009`), **70** em 44 (116), **71** em nenhuma (só nasce do 70), **72** em 14 (37). As 31
(de 65) missões solo com 68–70: I_J_009, I_J_010, I_J_013, I_J_104, I_J_108, I_J_110, I_M_014, I_M_109,
H_J_006, H_J_009, H_J_010, H_J_012, H_J_018, H_J_024, H_J_027, H_J_039, H_J_042, H_J_045,
H_M_004, D_J_007, D_J_009, D_J_012, D_J_015, D_M_003, D_M_004, D_M_010, D_M_018, D_M_027,
D_M_030, T_M_004, T_M_006; com 72: T_M_004, I_J_104, D_M_030, H_J_018, H_J_012, H_M_004,
D_J_009, D_J_012.

**No porte** (`behaviour::opo`): 68, 69 e 70 têm o teste (o retângulo do quadro de
referência, guardado no estado; sem caixa, para não barrar nem mudar o que a IA vê) e
envenenam pelo dano (`Pending::poisons`). Os jogadores humanos levam o dano e a fala;
barris e personagens da IA levam o dano pela área (`apply_hazards`). O 70 solta gotas (71)
e o 72 cicla pelos estados, com o teste fora do estado 2 (M17,
[`../auditoria/itens-objetos.md`](../auditoria/itens-objetos.md); medidos no oráculo).

### Em aberto

- O teste pega todo objeto presente; o porte só fere jogadores, barris e personagens da IA
  (como o fogo); a gota atravessa os personagens da IA.

## Tipo 6: grafite [confirmado, M6a]

Não é item. `OGF.ANM` (um rótulo só, 256 quadros) é uma **fonte**: o quadro `c` é o glifo do
caractere `toupper(c)`. O objeto escreve a linha `values[0]` do `GRAFFITI.RES` (contando de
1) com altura `values[1]` (< 5 → 18), sem colisão, visível enquanto os gatilhos dão
verdadeiro. Detalhes em [`objetos-tutorial.md`](objetos-tutorial.md). A decisão provisória da
M2 (sprite parado no quadro `values[0]`) foi abandonada: `values[0]` é o número da linha.

## Comportamento por tipo (para a M3)

Depois de ler um objeto, o carregador procura o tipo (campo `+0x1b8`, normalmente igual ao
tipo) na tabela de 73 pares (tipo, função) em `0x0047d47c` (busca em `0x00406b44`) e chama a
função de inicialização. Exemplos:

| Tipo(s) | Função | Observação |
|---|---|---|
| 1 | `0x0042277e` | Jake |
| 2 | `0x004279b3` | Garathe (Minotauro) |
| 102 | `0x0041e6bc` | mina flutuante (`OHM00`) |
| 30 | `0x004152d5` | "wizard" (dicas do tutorial: `SWIZARD.BMP`, `slevd.pft`) |
| 12–18, 25, 32–37, 44, 45, 76–85 | `0x0040b2c8` | "Generic Object" |
| 68–72 | `0x0040f324` | `OPO`: obstáculos de veneno e portão, não plataformas (ver acima) |
| 109 | `0x0041fe70` | `OAN` |

Cada inicialização fixa rotinas de comportamento nos campos `+0x1c0`, `+0x1c4`, `+0x1d0`.
Isso é matéria da M3 (jogabilidade).
