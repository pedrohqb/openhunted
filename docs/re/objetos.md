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
