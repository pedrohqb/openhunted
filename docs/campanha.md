# Campanha solo: vencibilidade das missões (M13)

As 65 missões solo do `ENGLEVEL.TXT` (`p=1` Jake, `p=2` Garathe), na ordem do arquivo,
auditadas com `ohtool audit --god --reach` (`openhunted_engine::audit`; desenho em
`docs/superpowers/specs/2026-11-07-m13-auditoria-design.md`). O teste
`tests/real_audit.rs` repete a auditoria inteira a cada `check.sh`.

## O que a auditoria confere

- **Modo deus** (`god_mode`): o jogador fica invulnerável e é levado a cada objetivo;
  apanha itens e peças, aperta CIMA nos interruptores, espera nas zonas, fere pelo caminho
  normal de dano quem é "destruir" (acordando antes o Bruiser pelos gatilhos dele) e explode
  os barris "acionar"; por fim aperta CIMA diante da saída e espera o veredito. Confere a
  lógica de objetivos, gatilhos e veredito, **não** a rota.
- **Alcance** (`reach`): busca na grade de células da IA a partir do início (cair, andar,
  portas de camada, teletransportes e, otimista, subir a qualquer célula sem teto); as
  portas de célula com chave só abrem depois de alcançada a chave (ou se o jogador já a
  traz). Aviso, não prova: a grade não vê pulos nem caixas internas. Hoje nenhuma missão tem
  objetivo ou saída fora do alcance. `--strict` tira a subida livre (só cordas e escadas,
  como a IA): 20 missões dependem de pulos para cima (T_M_006, I_M_003, I_J_004, I_M_109,
  I_M_008, I_M_106, I_M_011, H_M_003, D_M_030, H_J_018, H_J_024, H_M_004, D_M_024, H_M_027,
  D_M_004, D_M_003, H_J_039, D_M_027, D_J_012, D_M_018) — na T_M_006 basta o pulo até a
  corda; as demais não foram percorridas.

Resultado: **64 vencem, 1 falha** (D_M_010, abaixo); igual depois da revisão da IA da M11.

## Causas achadas e corrigidas

1. **Teletransporte entre camadas.** O porte só aceitava destinos na camada do
   teletransporte (ruling da M6a, de antes da troca de camada do Jake). O original
   (`0x41347e`) leva o clone e o passageiro à camada do destino. Nove missões ficavam sem
   caminho até a saída ou um objetivo (marcadas abaixo); 20 têm teletransportes entre
   camadas. Agora `Jake::start_teleport_to` muda a camada na metade da viagem.
2. **Zona com o jogador na porta de saída.** A zona 73 (`0x40e14e`) conta o jogador por
   sinalizadores, presença e vida, não pelo estado; a porta de saída liga `0x200008`, fora da
   máscara. O porte deixava de contá-lo ao entrar na porta, e a placa da D_M_010 (duração
   30, rearma ocupada) desligava antes dos 40 tiques do veredito.

## Em aberto

- **D_M_010** ("Your friend is poisoned!"): o antídoto é objetivo "acionar", mas a coleta
  recusa o antídoto sem veneno (`0x40b7b6`: `+0x270 = 0`), e a fase solo não tem item de
  veneno (33/35). A única escrita que soma em `+0x270` do jogador é a coleta
  (`0x40b6c3`); as gotas (70/71) só falam "Oh no it's poison" e ferem com natureza veneno,
  sem envenenar. Indício de que a missão também não se vence no original 1.00; **a conferir
  no Wine**. O porte segue o original; NEXT no resultado leva adiante mesmo na derrota.
- **I_M_109** (suspeita): a porta (5, 3, 0) diante da saída pede a chave 62, que fica no
  corredor de cima (981, 74, camada 0). Do início, a rota sobe 128 px duas vezes sem corda
  (vão no teto da sala do início, x 416–480; poço x 288–352). No porte o pulo para cima do
  Garathe sobe 84 px (o original: 88, `garathe-movimentos.md`) e a barra 47 não tem puxada:
  sem rota conhecida. A conferir no Wine.
- Porta de célula que pede chave ausente da fase: I_M_106 (5, 4, 2), chave 62, fora do
  caminho. (A D_J_015 começa trancada na sala do início, mas o Jake traz a 62 da fase.)
- Gatilhos para ids inexistentes (valem verdadeiro, `0x435bd8`): I_J_104 e D_J_012
  (pistão → id 3), D_J_001 e D_M_004 (robô), H_J_012 (barril), D_M_027 (teletransporte).
  Teletransportes sem destino na fase: I_M_003, H_J_036 (2), D_M_027.
- A rota real (pulos, inimigos, tempo) só se prova com vitórias roteirizadas (só a entrada
  do jogador): `real_missions.rs` (T_J_003/005/007 e, da M13, **I_J_002**: seis robôs, a
  chave 62 no corredor de cima, portas que abrem sozinhas e a porta de célula da saída) e
  `real_garathe.rs` (T_M_002 e, da M13, **I_M_102**: poço, redes, três camadas e a porta de
  empurrar que abre a de baixo pelo gatilho). Falta uma H_J/D_J: a D_J_001 (a mais curta)
  é um quebra-cabeça de interruptores (ids 3, 8, 9) e portas por gatilho na camada 1, ainda
  não percorrido.

## Missões

Personagem J/G; tempo em segundos (campo 1004); objetivos além da saída.

| Missão | P | Tempo | Objetivos | Modo deus | Observação |
|---|---|---|---|---|---|
| T_J_001 | J | — | só a saída | vence |  |
| T_M_002 | G | — | só a saída | vence |  |
| T_J_003 | J | — | só a saída | vence |  |
| T_M_004 | G | — | só a saída | vence |  |
| T_J_005 | J | — | só a saída | vence |  |
| T_M_006 | G | — | só a saída | vence |  |
| T_J_007 | J | — | 1× peça | vence |  |
| T_M_008 | G | — | só a saída | vence |  |
| I_J_101 | J | — | 1× espingarda, 1× granada, 1× estrela | vence |  |
| I_J_005 | J | — | 1× bandeira | vence | 4 teletr. entre camadas (**presa antes da M13**) |
| I_M_003 | G | 190 | só a saída | vence |  |
| I_J_110 | J | — | 1× peça | vence |  |
| I_J_002 | J | — | 6× robô | vence | vitória roteirizada |
| I_M_102 | G | — | só a saída | vence | vitória roteirizada |
| I_J_104 | J | — | 2× bandeira | vence |  |
| I_J_001 | J | 250 | 1× peça | vence |  |
| I_J_004 | J | — | só a saída | vence |  |
| I_M_109 | G | 300 | só a saída | vence | suspeita: chave da porta sem rota (acima) |
| I_J_013 | J | 240 | 1× peça | vence |  |
| I_J_010 | J | — | 1× peça, 1× chave | vence |  |
| I_M_008 | G | — | só a saída | vence | 1 teletr. entre camadas |
| I_J_107 | J | — | 6× granada | vence |  |
| I_M_106 | G | — | 1× peça | vence | 1 porta sem chave na fase (fora do caminho) |
| I_J_009 | J | — | 1× robô, 1× barril | vence |  |
| I_M_011 | G | — | só a saída | vence | 2 teletr. entre camadas |
| I_J_108 | J | 125 | 1× chave | vence |  |
| I_J_103 | J | 110 | 2× bandeira | vence |  |
| I_M_014 | G | 222 | só a saída | vence | 4 teletr. entre camadas (**presa antes da M13**) |
| I_J_016 | J | — | só a saída | vence | 6 teletr. entre camadas (**presa antes da M13**) |
| I_J_105 | J | 120 | 1× peça, 2× bandeira | vence |  |
| H_M_003 | G | — | 1× Jake IA | vence | 2 teletr. entre camadas |
| H_J_009 | J | — | 7× Bruiser, 1× peça | vence | 3 teletr. entre camadas (**presa antes da M13**) |
| D_M_030 | G | 180 | 1× zona | vence |  |
| H_J_018 | J | — | 3× bandeira | vence | 1 teletr. entre camadas |
| H_J_024 | J | 420 | 1× peça, 1× interruptor | vence |  |
| D_J_001 | J | 235 | só a saída | vence | 2 teletr. entre camadas |
| H_M_021 | G | — | 5× Jake IA, 3× robô | vence |  |
| H_J_012 | J | 420 | 1× Minotauro IA | vence |  |
| H_M_004 | G | — | 3× Jake IA, 1× peça | vence |  |
| H_J_010 | J | — | 6× Jake IA | vence | 4 teletr. entre camadas (**presa antes da M13**) |
| D_M_024 | G | — | 1× peça, 6× bandeira | vence |  |
| D_J_007 | J | 250 | 3× bandeira | vence |  |
| H_M_027 | G | — | 6× Jake IA, 1× bandeira | vence | 1 teletr. entre camadas |
| H_J_030 | J | — | 6× Bruiser | vence |  |
| H_J_007 | J | — | 3× Minotauro IA, 2× Bruiser, 6× gerador | vence |  |
| D_M_004 | G | 225 | 4× bandeira | vence | 3 teletr. entre camadas (**presa antes da M13**) |
| H_J_013 | J | — | 2× Bruiser, 1× peça | vence |  |
| H_J_006 | J | — | 3× Minotauro IA | vence | 2 teletr. entre camadas |
| D_M_003 | G | — | 1× zona | vence | 2 teletr. entre camadas (**presa antes da M13**) |
| H_J_042 | J | — | só a saída | vence |  |
| H_J_015 | J | — | 8× Bruiser, 1× peça | vence |  |
| D_M_006 | G | 300 | 1× zona | vence |  |
| H_J_001 | J | — | 1× Minotauro IA, 2× Bruiser | vence | 2 teletr. entre camadas |
| D_J_009 | J | — | 1× bandeira | vence | 2 teletr. entre camadas (**presa antes da M13**) |
| H_J_027 | J | — | 1× peça | vence | 3 teletr. entre camadas (**presa antes da M13**) |
| H_J_039 | J | — | só a saída | vence |  |
| D_M_021 | G | — | só a saída | vence | 1 teletr. entre camadas |
| H_J_033 | J | — | 1× Jake IA | vence |  |
| H_J_036 | J | — | 1× bandeira | vence |  |
| D_M_010 | G | 175 | 1× antídoto, 1× zona | FALHA | antídoto sem veneno; placa corrigida na M13 |
| H_J_045 | J | — | só a saída | vence |  |
| D_M_027 | G | — | só a saída | vence | 2 teletr. entre camadas |
| D_J_012 | J | 180 | 1× peça, 2× bandeira | vence |  |
| D_M_018 | G | — | só a saída | vence | 1 teletr. entre camadas |
| D_J_015 | J | — | só a saída | vence | começa trancada; o Jake traz a chave |
