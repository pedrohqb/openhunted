# OpenHunted M3b — objetos sólidos e presença por modo de jogo: design

Data: 2026-10-31. Decisões tomadas sem consulta, a pedido de Pedro (continuar sem intervenção).
Base: desmontagem do `Hunter.exe` (`0x00434170` leitor de objetos, `0x004045a5` laço de
atualização, `0x00436caf` colisão entre objetos), registrada em `docs/re/objetos.md`.

## Objetivo

O Jake fica de pé sobre as plataformas e peças que são objetos (o `OPL` da `T_J_007`, onde
hoje ele nasce e cai 404 px) e é barrado pelos objetos que o original trata como sólidos para
ele. De quebra, a cena passa a mostrar só os objetos que existem no modo solo, e o Jake
controlável passa a ser o objeto de início do jogador (na `D_J_009` hoje é escolhido o errado).

## O que o original faz (desmontagem)

**Campos do objeto envolvidos.** Sinalizadores `+0x178` (`fields[3]`); camada `+0x17c`
(`fields[4]`); máscara de modos `+0x1a8` (versão ≥ 1010; senão 0xfe, com o bit 0 ligado se a
versão < 1011 e `+0x19c` ≠ 0).

**Presença por modo.** O laço de atualização e a colisão só consideram um objeto se
`modos & modo_do_jogo ≠ 0` ou o sinalizador 0x10 está ligado. O modo do jogo é uma máscara
global (`0x4972f8`): 0x2 no solo (Jake ou Garathe), 0x8…0x80 nos modos de vários jogadores.
O bit 0 dos modos marca **vaga de jogador**: o objeto só existe com um jogador atribuído
(`+0x19c`). Nas 61 fases solo há exatamente um objeto de jogador com modos & 3 = 3: o Jake nas
fases J, o Garathe nas M.

**Quem colide com quem** (`0x00436caf`, para o objeto que se move contra cada outro):
1. o outro tem sinalizadores & 0x80003 = 0x80001 (bit 0 ligado, bit 1 desligado; 0x80000 é
   ligado pelo leitor em todos);
2. o outro está presente no modo (regra acima) e, se for vaga de jogador, ativo;
3. mesma camada (`+0x17c`);
4. regras de par sobre os sinalizadores dos dois (m = quem se move, o = outro): pula se
   m e o têm 0x2000; se m e o têm 0x4000; se m tem 0x10000, o não, e o tem 0x4000; se o tem
   0x10000, m não, e m tem 0x4000; se m tem 0x40000 e o tem 0x4000; se m tem 0x4000 e o tem
   0x40000; se os dois têm 0x40000, sorteia;
5. com o sinalizador 0x20 no outro, é só contato (itens apanháveis): registra e não barra;
6. senão as formas dos dois se cruzam e a velocidade é cortada (barra). A rotina de evento do
   Jake (`0x00407774`) não veta nenhum objeto nesse ponto.

As formas do BRD do objeto (`+0x1f8`, `names[1]`) ficam relativas à posição do objeto, já em
px da fase (o `OPL.BRD` tem uma caixa 18 × 5 no topo de uma peça de 17 px de largura).

## O que a M3b faz

- `LevelObject` ganha `modes` (máscara de modos, com o padrão e o ajuste de versão acima).
- `Scene::build` usa o modo solo (constante `SOLO = 0x2`) e:
  - **descarta** (não desenha, não colide) objetos ausentes no modo e vagas de jogador que não
    sejam a do jogador;
  - escolhe como Jake o objeto de tipo 1 com modos & 3 = 3; os demais objetos de tipo 1
    presentes são desenhados parados, como hoje.
  - **Fases sem jogador solo** (as de vários jogadores, `C_*` e `M_*`, abertas no visualizador):
    sem filtro de modo, e o Jake é o primeiro objeto de tipo 1, como na M3a.
- O mundo do Jake passa a incluir as caixas (tipo 2) dos objetos que colidem com ele pelas
  regras 1 a 5, na posição inicial do objeto. Contato (0x20) não entra.

## Simplificações declaradas

- Objetos sólidos ficam **parados** onde a fase os põe: plataformas que sobem e descem,
  inimigos que andam etc. são da fatia de comportamento dos objetos.
- Contatos (itens, gatilhos) são ignorados: nada é apanhado.
- Arestas de polígono (tipos 3/4) continuam ignoradas, nos objetos como nas células.
- O sorteio da regra de par 0x40000 × 0x40000 não se aplica ao Jake (ele não tem 0x40000).
- Só o modo solo; os modos de vários jogadores ficam sem filtro no visualizador.

## Testes

- Sintéticos: regra de presença (modo, 0x10, vaga de jogador); regras de par; caixa de objeto
  sólido barra e sustenta o Jake; objeto de contato (0x20) não barra; camada diferente não
  barra.
- Dados reais: em toda fase solo há exatamente um objeto de jogador; na `D_J_009` o Jake é o
  de (573, 581); na `T_J_007` o Jake fica de pé no início (não cai mais de 16 px em 40 tiques
  parado); render da `T_J_001` inalterado; fuzz de entradas aleatórias sem o Jake dentro de
  sólidos (células e objetos).

## Fora do escopo

Movimento e comportamento dos objetos, dano, coleta, gatilhos, escadas e cordas (próximas
fatias).
