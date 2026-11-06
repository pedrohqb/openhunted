// SPDX-License-Identifier: GPL-3.0-or-later
//! Lista de gatilhos de um objeto (`0x00435bd8`; `docs/re/logica-fase.md`).

/// A lista `(id do alvo, código)` está satisfeita? O código é lido em grupos de dois dígitos
/// decimais, do fim para o começo: unidades = número da entrada (1–4) cujo alvo se testa,
/// dezenas = 1 → ligado, senão desligado. Grupos de uma entrada combinam-se com E; as
/// entradas, com OU. Entradas de código 0 (ou ausente) não contam; nenhuma entrada contada →
/// verdadeiro; alvo inexistente (`on` devolve `None`) → verdadeiro.
pub fn satisfied(list: &[(i32, Option<i32>)], on: &dyn Fn(i32) -> Option<bool>) -> bool {
    let mut evaluated = false;
    for &(_, code) in list {
        let mut c = code.unwrap_or(0);
        if c <= 0 {
            continue;
        }
        evaluated = true;
        let mut all = true;
        while c > 0 {
            let group = c % 100;
            c /= 100;
            let (entry, want_on) = (group % 10, group / 10 == 1);
            let target = usize::try_from(entry)
                .ok()
                .filter(|&e| (1..=list.len()).contains(&e))
                .map(|e| list[e - 1].0);
            let state = target.and_then(on);
            if let Some(s) = state {
                if s != want_on {
                    all = false;
                }
            }
        }
        if all {
            return true;
        }
    }
    !evaluated
}

/// A lista tem alguma entrada contada (código > 0)? Sem nenhuma, `0x435bd8` devolve −1 e
/// quem só reage a "> 0" (o barril) não dispara.
pub fn armed(list: &[(i32, Option<i32>)]) -> bool {
    list.iter().any(|&(_, c)| c.unwrap_or(0) > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trigger_codes_from_doc() {
        let on = |id: i32| match id {
            1 => Some(true),
            2 => Some(false),
            _ => None,
        };
        assert!(satisfied(&[(1, Some(11))], &on)); // alvo da entrada 1 ligado
        assert!(!satisfied(&[(2, Some(11))], &on)); // desligado
        assert!(satisfied(&[(2, Some(1))], &on)); // "desligado" pedido e está desligado
        // 1203: entrada 3 desligada E entrada 2 ligada; a entrada 2 aponta para o id 1
        assert!(satisfied(
            &[(9, Some(1203)), (1, Some(0)), (2, Some(0))],
            &on
        ));
        assert!(!satisfied(
            &[(9, Some(1203)), (2, Some(0)), (2, Some(0))],
            &on
        ));
        assert!(satisfied(&[], &on)); // sem gatilhos
        assert!(satisfied(&[(5, Some(0))], &on)); // só código 0
        assert!(satisfied(&[(42, Some(11))], &on)); // alvo inexistente
        // OU entre entradas
        assert!(satisfied(&[(2, Some(11)), (1, Some(12))], &on));
        assert!(armed(&[(0, Some(0)), (1, Some(11))]));
        assert!(!armed(&[(0, Some(0)), (1, None)]));
    }
}
