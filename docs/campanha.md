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
  corda; das demais, só a I_M_109 foi percorrida (acima). Quinze das vinte são do Garathe: o pulo para cima
  dele sobe 216 px (o do Jake, 79) e agarra o teto de um vão de uma célula, saindo de lado
  para a laje 128 px acima ("JUMP + UP to grab the ceiling", `T_M_004`/`T_M_008`;
  `garathe-movimentos.md`). Até `611d193` o porte cortava esse pulo em 84 px (fim do Jake
  copiado), e nenhuma dessas subidas era possível; a grade da auditoria não vê pulos, e o
  resultado dela não mudou com a correção.

Resultado: **65 vencem, 0 falham**. Até a correção da `I_M_109` a D_M_010 falhava (causa 3,
abaixo); igual depois da revisão da IA da M11.

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
3. **Veneno das poças `OPO`.** Os tipos 68–70 (e o portão 72 fora do estado 2) envenenam quem
   os cruza (`0x40fa16`: `+0x270 = 1` e 1 de veneno a cada 10 tiques; `docs/re/objetos.md`).
   O porte os tinha como decoração, e a D_M_010 ("Your friend is poisoned!") ficava sem como
   envenenar o Garathe para o antídoto-objetivo (37, recusado sem veneno, `0x40b7b6`): a fase
   não tem itens de veneno (33/35), mas tem poças (68) e saliências (70). O modo deus passa
   por uma poça antes do antídoto (`audit::poison_at_ooze`) e vence. 31 das 65 missões têm
   68–70 (lista em `objetos.md`).

## A rota da I_M_109

**I_M_109** ("Garathe Tutorial 5"): **vencida por entrada** (`real_garathe.rs`,
`i_m_109_can_be_won`, 875 tiques, sem repor vida nem relógio). A porta (5, 3, 0) diante da
saída pede a chave 62 (976, 66, camada 0) e não há cordas: o pulo para cima do Garathe (20,
216 px) agarra o teto de vãos de uma célula ou, no poço de x 278–362 (camada 1), rente à
parede e com ESQUERDA no voo livre (controle no ar, `0x408205`), sai de baixo do gerador de
minas do canto do teto e pousa direto no corredor da porta de camada. Na camada 0, a sala
de x 278–618 sobe ao corredor da chave; os barris (28) que a cercam tombam ao toque e
rolam, mas não passam um pelo outro: pulo médio sobre eles, corre por cima e cai no vão da
chave (e volta igual). Na volta, a catapulta (41) do corredor de cima joga para a esquerda
quem a pisa (pulo médio sobre ela); a descida pelo poço cruza a poça `OPO` (68) e envenena
(sem antídoto na fase: 1 de vida a cada 20 tiques até a saída). Na camada 0 de baixo, a
porta de célula abre com a chave ao empurrar e a pilha de três barris antes da saída se
vence com o pulo parado e DIREITA no ar. O gerador de minas não precisa morrer; o ataque
na barra que o alcança (ATIRAR/SOCO solta e ataca, `0x42a29b` → `0x43b43e`) agora existe
no porte para o Jake e o Garathe (`claws_hit_the_mine_generator_from_the_ceiling`).

## Em aberto

- Porta de célula que pede chave ausente da fase: I_M_106 (5, 4, 2), chave 62, fora do
  caminho. (A D_J_015 começa trancada na sala do início, mas o Jake traz a 62 da fase.)
- Gatilhos para ids inexistentes (valem verdadeiro, `0x435bd8`): I_J_104 e D_J_012
  (pistão → id 3), D_J_001 e D_M_004 (robô), H_J_012 (barril), D_M_027 (teletransporte).
  Teletransportes sem destino na fase: I_M_003, H_J_036 (2), D_M_027.
- A rota real (pulos, inimigos, tempo) só se prova com vitórias roteirizadas (só a entrada
  do jogador): `real_missions.rs` (T_J_003/005/007 e, da M13, **I_J_002**: seis robôs, a
  chave 62 no corredor de cima, portas que abrem sozinhas e a porta de célula da saída) e
  `real_garathe.rs` (T_M_002 e, da M13, **I_M_102**: poço, redes, três camadas e a porta de
  empurrar que abre a de baixo pelo gatilho; e **I_M_109**, acima). Falta uma H_J/D_J: a D_J_001 (a mais curta)
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
| I_M_109 | G | 300 | só a saída | vence | vitória roteirizada (pulo 20, teto, poça de veneno) |
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
| D_M_010 | G | 175 | 1× antídoto, 1× zona | vence | veneno pela poça `OPO`; placa corrigida na M13 |
| H_J_045 | J | — | só a saída | vence |  |
| D_M_027 | G | — | só a saída | vence | 2 teletr. entre camadas |
| D_J_012 | J | 180 | 1× peça, 2× bandeira | vence |  |
| D_M_018 | G | — | só a saída | vence | 1 teletr. entre camadas |
| D_J_015 | J | — | só a saída | vence | começa trancada; o Jake traz a chave |
