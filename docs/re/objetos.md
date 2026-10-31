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
inimigos `OH1`/`OH2` um estado (50…55, 300…). O Jake e os itens de tipo 6 não têm o bit: a
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

Sinalizadores típicos: Jake 0x94801; peças `OPL` e plataformas `OPO` 0x80089 (sólidas para o
Jake, mas as 41 `OPO` dos dados não têm BRD: a forma delas vem do comportamento, não do
arquivo); sentinelas `OHS` e minas `OHM` 0x80801 (sólidas); inimigos com 0x4000 (`OSE`, `OSR`,
`OBR`, `ODG`, Minotauro…), que o Jake atravessa.

## Tipo 6: itens genéricos

`OGF.ANM` tem um rótulo só cobrindo os 256 quadros de `OGF00.BMP` (216 deles são marcadores de
1×1; os outros, itens diferentes). Cada objeto do tipo 6 mostra **um** desses quadros, escolhido
por um campo ainda não decifrado; `values[0]` acerta em 62 dos 215 objetos. Decisão provisória
(M2): objetos do tipo 6 ficam parados no quadro `values[0]` quando ele é válido, senão no 0.

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
| 68–72 | `0x0040f324` | `OPO` |
| 109 | `0x0041fe70` | `OAN` |

Cada inicialização fixa rotinas de comportamento nos campos `+0x1c0`, `+0x1c4`, `+0x1d0`.
Isso é matéria da M3 (jogabilidade).
