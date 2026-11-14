# Engenharia reversa

Notas sobre o comportamento do `Hunter.exe` (versão 1.00, sha256
`663e22cd9938ba6a5f4288c001e77f49558c7a8e76ed0a6d1ccf69472a44a365`), escritas para que
cada formato e cada comportamento portado seja rastreável até o original.

## Método

- A análise roda com o radare2 (`tools/re/analyze.sh <Hunter.exe>`), que gera um índice
  de funções, nomes de tipo e referências a strings em `re-out/`.
- `re-out/` fica **fora do git**: nada do executável, nem desmontagem, nem pseudocódigo
  entra no repositório.
- Os documentos desta pasta descrevem o comportamento **com palavras e endereços**. O
  código do OpenHunted é escrito a partir dessas descrições, não copiado da desmontagem.
- Endereços são endereços virtuais do executável (base `0x400000`).

## Ferramentas

O plano previa o Ghidra, mas o download das releases não estava acessível no ambiente
em que o trabalho começou; usa-se o radare2 5.5 (pacote do Ubuntu) com o módulo Python
`r2pipe`. Quem tiver o Ghidra pode usá-lo nos mesmos endereços.

## Documentos

- [`executavel.md`](executavel.md): mapa geral do executável.
- [`compressao.md`](compressao.md): compressão dos recursos.
- [`anm.md`](anm.md): animações.
- Fases: formato e endereços em `docs/formats/hls.md` e `lev.md`.
- [`cenario.md`](cenario.md): cenário, transparência e ritmo (M2).
- [`objetos.md`](objetos.md): objetos das fases e tabela de tipos (M2).
- Cenários e interfaces: endereços em [`executavel.md`](executavel.md); formato em `docs/formats/brd.md` e `itf.md`.
- [`logica-fase.md`](logica-fase.md): gatilhos, zonas de proximidade, saída e veredito da missão (M3).
- [`dicas.md`](dicas.md): dicas do tutorial (tipo 30, `WIZARD.RES`) (M3).
- [`minas.md`](minas.md): minas flutuantes (tipo 102) (M3).
- [`itens.md`](itens.md): itens genéricos e coleta de vida/armadura (M3).
- [`dano-vida.md`](dano-vida.md): vida, armadura, `DAMAGE.TBL`, explosões e morte do Jake (M3).
- [`armas.md`](armas.md): inventário, armas, munição, coleta, arma inicial e troca (M4).
- [`tiro-jake.md`](tiro-jake.md): controles, estados de tiro, mira, cadência e soco (M4).
- [`projeteis.md`](projeteis.md): criação de projéteis, bala da pistola, cápsula, impacto (M4).
- [`som.md`](som.md): ids e recursos de som, som posicional, canais, `SOUND.TBL` (M4).
- [`menu.md`](menu.md): abertura, máquina de telas, menu principal, lista de missões e "New Game".
- [`interface-itf.md`](interface-itf.md): controles das `.ITF`, botões, desenho, despacho e teclado.
- [`briefing.md`](briefing.md): briefing da missão (`SLEVD.ITF`).
- [`fim-de-missao.md`](fim-de-missao.md): anúncio, tela de resultado (`SMISS.ITF`), pontuação e próxima missão.
- [`pausa.md`](pausa.md): pausa, Esc e o menu da janela durante o jogo.
- [`hud.md`](hud.md): barra de estado (`INVBAR01.BMP`) embaixo da vista.
- [`olho-laser.md`](olho-laser.md): olho-laser (tipo 104) e raio laser (tipo 36).
- [`barril-fogo-cerca.md`](barril-fogo-cerca.md): barril explosivo (28), fogo (42) e cerca elétrica (40).
- [`objetos-tutorial.md`](objetos-tutorial.md): grafite (6), peça do carro (9), teletransporte (46), portas `OH` (50–52), plataformas `OPL` (56) e coletáveis.
- [`escada-camadas.md`](escada-camadas.md): escadas e redes (zona 4) e troca de camada (zonas 1/2) do Jake.
- [`missoes-tutorial.md`](missoes-tutorial.md): o que as `T_J_003`, `T_J_005` e `T_J_007` exigem para terminar.
- [`ske.md`](ske.md): esqueleto `.SKE` (formato em `docs/formats/ske.md`) e a "animação com deslocamento" (desenho e velocidade pelo esqueleto).
- [`jake-movimentos.md`](jake-movimentos.md): tabela de estados do Jake, despacho, pulo parado/para cima, impulso, aterrissagens 34/35/26/77–79, agachar/engatinhar, empurrar.
- [`jake-escalada.md`](jake-escalada.md): zonas PR, corda/escada (30/32) com velocidades, barra e teto (103–107).
- [`furtividade.md`](furtividade.md): FURTIVO (andar, estado 2), esconder-se (164–166) e o teste dos inimigos, especial BAIXO+ATIRAR, caixa e acerto do soco.
- [`robo-seguranca.md`](robo-seguranca.md): robô de segurança voador (tipo 8, `OSR`): variantes, flutuação, patrulha, perseguição, canhões laser, queda e explosões.
- [`robo-sentinela.md`](robo-sentinela.md): "Sentinel Robot" (tipo 10, `OSE`): casulo que abre e dispara balas 106.
- [`bruiser.md`](bruiser.md): Bruiser (tipo 7): ativação por gatilhos, estados, tiros (balas 100 e foguetes), fogo, terremoto, esmagar, morte em 30 explosões.
- [`gerador-minas.md`](gerador-minas.md): gerador de minas flutuantes (tipo 103, `OHG`).
- [`mina-saltadora.md`](mina-saltadora.md): "Mine" (tipo 11, `OHS`): pula e explode ao toque.
- [`anticorpo-manta.md`](anticorpo-manta.md): Anticorpo (105) e Manta (89), voadores com o mesmo código.
- [`inimigos-menores.md`](inimigos-menores.md): gerador de dardos (87) e dardo (29), catapulta (41), Entidade (107), Camaleão (4) e Morte (3).
- [`itens-campanha.md`](itens-campanha.md): itens da campanha (nas células), chaves 62–67 e portas, bandeiras 80–85, pistão 39, interruptor 74, tipo 0.
- [`armas-2.md`](armas-2.md): espingarda, foguetes, estrelas ninja e granadas do Jake: descritores, registros, projéteis e dano.
- [`garathe.md`](garathe.md): Garathe/Minotauro (tipo 2): inicialização, quem o controla, missões, armas e registros, `DAMAGE.TBL` do atacante 2, especial, reações, morte, falas, HUD e itens só dele.
- [`minotauro-ia.md`](minotauro-ia.md): IA de personagem sem jogador (Minotauros e Jakes inimigos): percepção, modos, ataque, perigos, busca de caminho e bits de célula.
- [`inimigos-menores-2.md`](inimigos-menores-2.md): Morte (3) e Camaleão (4) completos, natureza `0x10`, formiga (109) e formigueiro (88), zona da Entidade, trajetórias do Anticorpo/Manta.
- [`garathe-movimentos.md`](garathe-movimentos.md): tabela de 164 estados do Garathe, despacho, decisão em pé, correr/andar, pulos (com a investida), queda, agachar, empurrar, barra, corda, esconder (54–56) e velocidades do `MINOTAUR.SKE`.
- [`videos-musica.md`](videos-musica.md): filmes AVI (Indeo 3.2) da abertura, meio e final; música (CD de áudio, `SX_3000.WAV` nos menus); vozes; grupos 0/1 do `SOUND.TBL`.
- [`menus-restantes.md`](menus-restantes.md): as 16 `.ITF`, fluxo a partir do `SMAIN`, `SMULT`, `SINPU`, `SKEYB`/`SJOYS`, `SEGOC`, `SHVLD`, `SH2HS`, `SHERO`, `SHIGH`, `S_CAR`, `SABOUT`; campos do `ENGLEVEL.TXT`.
- [`opcoes-salvamento.md`](opcoes-salvamento.md): `HUNTER.INI`, tela de opções, menu da janela, dificuldade, jogo salvo `.hhg`, progresso e `hunter.fam`.
- [`multijogador.md`](multijogador.md): dois jogadores (disputa e cooperativo, sem rede): máscaras de modo, atribuição das vagas, entrada dupla, tela dividida, crédito e pontos, renascimento, itens que renascem, limites e `SH2HS`.
- [`cores-sprites.md`](cores-sprites.md): tabela de desenho `0x43becd` (`DLS.TBL`), arte "de sprite" dos 7 tipos com rotina `+0x1c4`, cor e piscar dos jogadores, Camaleão (cor, clones, esmaecer) e pulsos do robô, do gerador e das minas.
- [`casca-acabamento.md`](casca-acabamento.md): logotipo, título, história e créditos da abertura, laço de demonstração (e o `PLAY.VCR`, que o jogo não lê), dispositivos, `InputKeys`/`Joystick` do `HUNTER.INI`, bits de entrada, `SINPU`/`SKEYB`/`SJOYS` e a luz por célula.
- [`camadas-desenho.md`](camadas-desenho.md): desenho das camadas (`0x405c7c`): camada da vista opaca, buracos das células com as de trás escurecidas (27…33) no modo "atrás", distância e escala por camada (`0x436a70`), zoom por célula e transição da vista na troca de camada.
