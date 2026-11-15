# Auditoria de fidelidade (M17)

Comparação sistemática, área por área, entre o porte e o `Hunter.exe` original: cada
movimento, item, arma, colisão e inimigo. Pedido do Pedro (nov/2026): "teste cada um dos
movimentos e itens, colisões, e compare", o mais detalhado possível.

## Método

1. **Inventário.** Listar todos os comportamentos da área (estados, rótulos, constantes,
   transições, colisões, sons, dano) a partir do executável (tabelas de estado, registros,
   rotinas) e dos docs em `docs/re/`. Nada fica de fora por "parecer óbvio".
2. **Valor do original.** Para cada item, o valor ou a regra do executável, com o endereço.
   Quando o oráculo (`tools/oraculo/`, o original rodando no Wine com leitura da memória)
   estiver disponível, medir também no original.
3. **Valor do porte.** Medido de verdade (teste, `ohtool render --trace`, harness), nunca
   lido do código só.
4. **Veredito por linha:** `igual`, `diferente` (bug), `não implementado`, `decisão`
   (Ruling documentado) ou `a confirmar` (sem como provar sem o oráculo).
5. **Correção.** Cada `diferente`/`não implementado` que se resolve na área é corrigido com
   teste que falha antes (TDD). O resto vira "Em aberto" com evidência.

## Relatórios

Um arquivo por área, `docs/auditoria/<area>.md`, < 12 KB: tabela
`comportamento | original (endereço/valor) | porte (medido) | veredito | teste/commit`,
depois "Correções", "Em aberto". O README desta pasta lista as áreas e o placar.

## Áreas

| Área | Arquivo | Situação |
|---|---|---|
| Oráculo (original no Wine) | `oraculo.md` | pronto: `tools/oraculo/` + `ohtool render --trace-json` |
| Movimentos do Jake | `jake-movimentos.md`, `jake-movimentos-2.md` | feito: 17 iguais, 29 corrigidos, 10 de outras áreas, 3 não implementados |
| Movimentos do Garathe | `garathe-movimentos.md` | feita: 24 igual, 18 corrigidas, 2 decisão, 5 em aberto |
| Armas, projéteis e reações a dano | `armas.md`, `armas-projeteis.md` | feita (32 corrigidos, 7 em aberto, 6 a confirmar) |
| Itens, objetos de fase e colisões de objetos | `itens-objetos.md` | feito: 19 iguais, 7 corrigidos, 6 em aberto |
| Inimigos principais | `inimigos.md` | a seguir |
| Inimigos secundários e IA dos personagens | `inimigos-2.md` | feita: 11 igual, 17 corrigidas, 1 decisão, 7 em aberto, 4 a confirmar |
| Física e colisão do cenário, HUD e som | `fisica-hud-som.md` | a seguir |
