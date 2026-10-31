# OpenHunted M3a — Jake corre e pula: design

Data: 2026-10-31. Decisões tomadas sem consulta, a pedido de Pedro (continuar sem intervenção).
Base: `docs/re/personagem.md` (engenharia reversa do controlador de personagem).

## Objetivo

Na `T_J_001`, o jogador controla o Jake: parado, correndo, pulando (médio, longo, estendido),
caindo e aterrissando, colidindo com o cenário, com a câmera acompanhando. É a primeira fatia
da M3 da spec geral; furtividade, agarrar, escadas, esconder, troca de camada, armas e inimigos
ficam para as próximas fatias.

## Comportamento (do documento de engenharia reversa)

- Tique fixo de 50 ms (confirma o provisório da M2). Velocidades em 1/8 px por tique;
  posição += velocidade >> 3.
- Gravidade 2 px/tique²; sem velocidade terminal.
- Correr: 12 px/tique, sem aceleração, para na hora; −1 px/tique vertical enquanto corre.
- Pulos balísticos ao apertar PULAR correndo: médio (10, −11), longo (10, −15), estendido
  (17, −15) px/tique; escolha por sondagem de chão a 110/150/255 px. No ar, apertar o lado
  oposto inverte vx.
- Queda: estado de queda quando desce a ≥ 5 px/tique sem chão.
- Colisão: caixa do Jake = caixas do `JK_DEFLT.BRD`; formas das células sólidas dos quatro
  lados; chão quando a normal aponta para cima; resposta: componente normal com meio rebote
  acima de 4 px/tique, senão zerada; atrito 7/8 px/tique² no contato.
- Animações (rótulo do `JAKE.ANM`, quadro avança a cada c+1 tiques): parado 1 (c=1), correr 10,
  pulo parado 27, médio 29, longo 28, estendido 31→80, aterrissagens 35/34, queda 25,
  aterrissagem da queda 26, empurrar 100.
- Desenho: arte do JAKE olha para a esquerda; espelhada quando o Jake olha para a direita.
- Escala de objetos (vale para todos, corrige a M2): os dois últimos campos do objeto (A, B)
  dão o tamanho-alvo; se A ≥ B, escala = A / largura do quadro de referência, senão
  B / altura (Jake: 47/88).

## Simplificações declaradas

- Colisão por caixas alinhadas (tipo 2), eixo a eixo; arestas de polígono (tipos 3 e 4, 12 de
  cada nos dados) ignoradas nesta fatia.
- Pulo parado e aterrissagens tocam a animação sem o deslocamento vindo do `JAKE.SKE`.
- Sem armas, dano, inimigos, troca de camada.

## Arquitetura

- `openhunted-engine`: `input.rs` (bits de entrada do original), `physics.rs` (corpo, colisão
  com caixas, sondagem de chão), `jake.rs` (máquina de estados da fatia), escala e espelhamento
  no `render.rs`; `Scene` ganha o Jake controlável (objeto de tipo 1) e `tick(input)`.
- `openhunted`: teclado → bits de entrada (setas, espaço = pular), câmera segue o Jake.

## Testes

Unitários com cenários sintéticos (cair e pousar numa caixa, correr até a parede, pulo médio
com a trajetória exata dos constantes, inversão no ar, escolha do pulo pela sondagem); com dados
reais, o Jake da `T_J_001` pousa no chão e corre até a saída sem atravessar o cenário; render
com escala conferido visualmente.
