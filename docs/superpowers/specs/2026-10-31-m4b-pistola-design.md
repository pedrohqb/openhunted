# OpenHunted M4b — a pistola: design

Data: 2026-10-31. Decisões tomadas sem consulta (Pedro: "seguir até o fim do port").
Base: `docs/re/armas.md`, `tiro-jake.md`, `projeteis.md`, `som.md`, `dano-vida.md`.

## Objetivo

O Jake atira com a pistola: para, aponta (frente, cima, diagonais), dispara rajadas de 3
balas que voam reto, acertam as minas (que explodem) e as paredes (faísca), e gasta munição
mostrada no HUD. A dica "SHOOT them before they get you" passa a ser cumprível. Fica de fora
o resto do arsenal (punhos/soco, espingarda, foguetes, estrelas, granadas, o especial de
BAIXO), a troca de arma e o som (eventos registrados).

## Comportamento (dos documentos de engenharia reversa)

- **Inventário**: 30 pares (tem, munição) vindos de `points` do objeto do Jake; a
  inicialização força o par 1 (punhos) a (1, 20) e escolhe a melhor arma disponível (ordem
  1, 6, 5, 2, 3, 4; disponível = tem ≠ 0 e munição > 0). Na T_J_001: pistola com 80.
- **Entrada**: ATIRAR = bit 0x100 (tecla Ctrl no original; aqui Ctrl). ESQUERDA/DIREITA
  mudam o lado todo tique, mesmo atirando.
- **Mira** (`bits & 0xf`): nada, CIMA+BAIXO ou ESQ+DIR → frente; CIMA → cima (rótulo 56);
  ESQ/DIR → frente (23) e muda o lado; CIMA+ESQ/DIR → diagonal cima (54); BAIXO+ESQ/DIR →
  diagonal baixo (55); BAIXO sozinho → diagonal baixo (55) (o especial do original fica de
  fora: Ruling).
- **Quando**: de pé (parado, correndo — para na hora — ou aterrissando); no ar não
  (Ruling: os estados 16/27 do original não foram lidos). Sem munição: evento de som 3090 e
  nada mais (não há outra arma).
- **Ciclo** (contador `t` por tique, passo 1): entra no estado de tiro com `t = 0`;
  dispara em `t = 1, 2, 3` (rajada de 3, uma bala por tique, 1 de munição cada); com ATIRAR
  seguro, ao chegar a `t = 7` recomeça (`t = 0`, nova mira); solto e sem mexer, fica até
  `t > 12` → recuperação (22, `J022`) → pronto (18, `J018`) por 10 tiques → guardar (19,
  `J019`) → parado. Direção/pulo durante pronto/guardar interrompem e voltam ao controle
  normal. Som 3040 no primeiro tique de cada estado de tiro.
- **Boca e velocidade** (px, relativas à posição do Jake; dx olhando à esquerda / à
  direita): frente dx −25/25, dy 2, v (±20, 0); cima dx 9/12, dy −35, v (0, −20); diagonal
  cima dx −16/24, dy −23, v (±20, −20); diagonal baixo dx −23/27, dy 33, v (±20, 20).
  1ª e 3ª balas desviam 1 px/tique perpendicular (1ª para um lado, 3ª para o outro).
- **Cápsula** (tipo 22, `OBS00.ANM`, só contato, com gravidade): sai só com a 1ª bala em
  dx 22/−4, dy 0, v (∓10, −10) (para trás e para cima); some quando a velocidade fica
  abaixo de 1 px/tique nos dois eixos ou sai da fase; evento de som 3020/3021 ao tocar algo.
- **Bala** (tipo 19, `OBU.ANM`): rótulo pela direção (horizontal 1, diagonal 2, vertical
  3) com espelhamento (bit 1 horizontal, bit 2 vertical); tamanho-alvo 10 px; sem gravidade
  nem tempo de vida; voa até tocar: parede (células) → faísca `oex15` de 10 px e evento 3046;
  mina → a mina morre (explode) e a bala faz a faísca; outro objeto sólido (0x80001, não só
  contato) → faísca; fora da fase → some. Balas não se chocam entre si nem com o Jake.
- **Colisão com a mina**: a bala (sólida) é o que a mina vê no contato → vida 0; além disso
  a linha de dano põe 2 de dano pendente.
- **HUD**: "VIDA x  ARMADURA y  PISTOLA n".

## Arquitetura

- `jake.rs`: `Weapon { pistol_ammo }` simplificado em `Inventory` (30 pares); estados novos
  `Aim(FireState)` com `{label, t, round}` e `Ready(u32)`, `Holster`, `Recover`; `pub shots:
  Vec<Shot>` drenado pela cena (`Shot { x, y, vx, vy, mirror: u8, label: u16 }`) e
  `casings: Vec<Shot>`; `pub ammo() -> i32`.
- `behaviour/bullet.rs`: `BulletState { vx, vy (1/8 px) }`, tick com `step` sem gravidade
  contra as células; depois a cena testa entidades sólidas e o impacto.
- `behaviour/casing.rs`: `CasingState { body }`, gravidade, destruição.
- `scene.rs`: cria as entidades das balas/cápsulas a partir de `jake.shots`; impacto →
  explosão visual `oex15` (sem área) via `explosion_entity` com rótulo fixo; evento 3046.
- Viewer: Ctrl → SHOOT; HUD com munição.

## Simplificações declaradas

- Só a pistola; sem soco, troca de arma, especial de BAIXO; sem tiro no ar.
- Faísca do impacto sem som real (evento).
- A bala não acerta o Jake nem outras balas; a checagem da linha boca–parede (se a boca
  está dentro da parede, o original dá um soco) vira "não dispara".

## Testes

- Sintéticos: mira por bits; rajada de 3 em t = 1..3 com munição 80 → 77; repetição a cada
  6 tiques segurando; recuperação → pronto → guardar → parado; correr e atirar para o Jake;
  bala anda 20 px/tique e some na parede com faísca; bala mata a mina; cápsula cai e some;
  sem munição não atira.
- Dados reais: T_J_001 começa com pistola 80; roteiro "andar para a esquerda e atirar
  quando uma mina estiver à frente na mesma altura" mata ao menos 3 minas por tiro e vence
  com mais vida do que o roteiro sem tiros.
