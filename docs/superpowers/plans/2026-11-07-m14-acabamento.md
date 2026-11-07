# M14 — acabamento da casca e entrada: plano

Spec: `docs/superpowers/specs/2026-11-07-m14-acabamento-design.md`. Cada tarefa em TDD
(teste falhando → código → `check.sh` → commit).

1. **RE e formatos**: `docs/re/casca-acabamento.md`, `docs/formats/vcr.md`;
   `openhunted_formats::vcr` (parse/to_bytes, teste no arquivo real), `ohtool info`.
2. **Entrada no motor** (`input.rs`): `Device`, `Bindings` (padrões, troca `0x442537`),
   `JoyMap`, `DeviceState` → bits (`0x407e2d`), espera da combinação; testes com dispositivo
   falso.
3. **Configuração** (`shell/config.rs`): `InputKeys1/2`, `Joystick1/2`, `PlayerNInput`.
4. **Telas** (`shell/keys.rs`, `shell/devices.rs`): `SKEYB`/`SJOYS` (espera, pisca, RESET,
   DONE) e `SINPU` (grupos, cinza, troca, CUSTOMIZE).
5. **Abertura** (`shell/intro.rs`): `Splash` (logotipo/título) e `Scroller` (história e
   créditos, substitui `credits.rs`).
6. **App** (`casca.rs`, `shell.rs`, `main.rs`): passos novos da sequência, laço e
   ociosidade, paletas da abertura, telas novas, VK da SDL, joysticks da SDL, bits pela
   configuração.
7. **Luz por célula** (`scene/light.rs` + ganchos em `scene.rs`): estado, tique, desenho;
   teste em `H_J_006`; determinismo da cena.
8. **Docs**: README (status, controles, "Ainda não há"), `docs/re/README.md`,
   `docs/formats/README.md`; `check.sh --full`.
