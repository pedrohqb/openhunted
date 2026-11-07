# M7b — arsenal do Jake e itens da campanha: plano de implementação

**Goal:** as seis armas do Jake, a troca de arma e os itens da campanha (munição, veneno,
antídoto, chaves e portas de célula, bandeiras, pistão, interruptor).

**Spec:** `docs/superpowers/specs/2026-11-06-m7b-arsenal-itens-design.md`

## Global Constraints

- GPL-3.0-or-later/SPDX; português; nada de dados do jogo; `/home/claude/check.sh` verde
  antes de cada commit; constantes com endereço.
- A M7a corre em paralelo: não tocar `mine.rs`, `laser*.rs`, `fire.rs`, `barrel.rs`; em
  `scene.rs`, funções novas e edições mínimas nos pontos de entrada.

## Review Focus

1. Munição nunca negativa; troca com nada disponível não panica (devolve os punhos).
2. Projéteis que nascem fora da fase ou dentro da parede somem sem panicar.
3. Explosão de foguete/granada ao lado do Jake o fere (área), com derrubada.
4. Porta de célula: colisão fechada volta ao fechar; não fecha sobre o Jake.
5. Veneno não é absorvido pela armadura; antídoto recusado sem veneno.

---

### Task 1: descritores das armas, inventário e troca (`jake/weapons.rs`)

`Weapon` (pronto, guardar, passo, projétil, cápsula, estados por mira com registro
`Reg { window, again, reeval, recover, muzzle, vel, casing }`); `Inventory::{weapon, best,
cycle, available}`; a arma corrente nasce na melhor. `Jake` passa a tirar estado/rótulo da
arma; `Shot { kind: ShotKind }`. Troca na borda de PRÓXIMA/ANTERIOR e por `SELECT`. Sem
munição → melhor. Testes: ordem 1,6,5,2,3,4; volta; cadências; munição.

### Task 2: soco

Estados 150–155 com as janelas do registro; caixa do golpe; `Jake::punch: Option<Rect>` no
tique da janela; a cena fere quem `interact::hurt` aceita (8, empurrão), som 5062.

### Task 3: projéteis novos (`behaviour/pellet.rs`, `rocket.rs`, `star.rs`, `grenade.rs`)

Entidades a partir dos `Shot`; tique de cada um; `Scene::weapon_hits` (impacto direto,
`Blast` de área na lista existente, faísca). Estrela cravada vira `Pickup`. Testes por arma.

### Task 4: itens da campanha

`PICKUPS` ganha 13, 17, 18, 27, 33, 35, 37, 62–67, 80–85; `take_weapon` para 13/17/18/27,
chaves e bandeiras; veneno/antídoto no `Health`+Jake (`poison`); som 5060. Rótulo = tipo de
chaves e bandeiras. Testes por item.

### Task 5: portas de célula

`scene.rs`: `CellDoor` por célula 38 (BRD aberta/fechada, quadros, tempos, gatilhos); pedido
do Jake (`door_push`) na zona `0x20`; tique das portas; caixas da porta no mundo. Testes
sintéticos (abre com chave, recusa, fecha) e com dados (`D_J_009`).

### Task 6: pistão e interruptor

`behaviour/piston.rs`, `behaviour/switch.rs`; CIMA no interruptor pela mesma rota da saída.
Testes sintéticos.

### Task 7: viewer, HUD, dados reais, fuzz, documentação

PgUp/PgDn, 1–6; HUD com a arma corrente e o rosto do veneno; título com arma e munição;
testes de dados reais (contagens, coleta de munição, porta com chave, bandeiras) e fuzz de
2000 tiques; README (M7b), ajuda das teclas, `docs/re/itens-campanha.md` (portas de célula).
