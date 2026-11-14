// SPDX-License-Identifier: GPL-3.0-or-later
//! O gerador pseudoaleatório do original (`rand`, `0x00460a74`): aditivo atrasado de 56
//! palavras de 32 bits (`0x00483920`), índices `j` (`0x00483918`, começa em 24) e `k`
//! (`0x0048391c`, começa em 55): `t[k] += t[j]`, devolve os 16 bits baixos de `t[k]`, e os
//! dois índices avançam módulo 56. O executável nunca o semeia (a rotina que descartaria `n`
//! valores, `0x00460aca`, não tem chamadas), então a sequência é a mesma em toda partida a
//! partir do estado inicial; o porte o repõe na carga da fase e antes do tique 0, como o
//! oráculo (`docs/auditoria/oraculo.md`): no original o briefing ainda sorteia conforme o
//! tempo que fica aberto, o que o porte não reproduz.

/// Estado inicial da tabela, como está na seção de dados do executável (`0x00483920`,
/// 56 × `u32`). São constantes do programa (código/dados do `Hunter.exe`), não dados do jogo
/// (fases, animações, sons).
const INITIAL: [u32; 56] = [
    0xdd23, 0x9533, 0xf3cd, 0x4bc2, 0xf7e2, 0xa2df, 0xaa70, 0x60aa, 0x144b, 0x8948, 0x192a, 0x6b0d,
    0x9e0b, 0xfbf1, 0x8607, 0x465d, 0xeaf4, 0x69fd, 0x6c44, 0x1b27, 0xdb71, 0xe38f, 0xabb4, 0xdc47,
    0x0fde, 0x4d8e, 0xfc7d, 0x8d38, 0x5948, 0xe36f, 0xaa22, 0x2f26, 0x43fa, 0xb5a5, 0x20eb, 0xed85,
    0xf5c3, 0x819f, 0x2d9c, 0xb26d, 0xe92b, 0x97d5, 0x3da1, 0x89ed, 0xe3f4, 0x74b5, 0x002c, 0xd35f,
    0x3b08, 0x0df5, 0xf27e, 0xc270, 0x2589, 0x2e8b, 0x6352, 0xda2d,
];
/// Índices iniciais (`0x00483918`, `0x0048391c`).
const J0: usize = 24;
const K0: usize = 55;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rng {
    t: [u32; 56],
    j: usize,
    k: usize,
}

impl Default for Rng {
    fn default() -> Self {
        Self::original()
    }
}

impl Rng {
    /// O estado inicial do executável.
    pub fn original() -> Self {
        Self {
            t: INITIAL,
            j: J0,
            k: K0,
        }
    }

    /// O estado inicial com `skip` valores descartados (o que `0x00460aca(n)` faria): outras
    /// sequências do mesmo gerador, para testes e sorteios independentes.
    pub fn new(skip: u32) -> Self {
        let mut r = Self::original();
        for _ in 0..skip % 4096 {
            r.next_u16();
        }
        r
    }

    /// Volta ao estado inicial.
    pub fn reset(&mut self) {
        *self = Self::original();
    }

    /// Índices `(j, k)` (`0x00483918`, `0x0048391c`), como o oráculo os grava.
    pub fn indices(&self) -> (usize, usize) {
        (self.j, self.k)
    }

    /// Próximo valor de 16 bits (`rand()`).
    pub fn next_u16(&mut self) -> u32 {
        self.t[self.k] = self.t[self.k].wrapping_add(self.t[self.j]);
        let v = self.t[self.k] & 0xffff;
        self.k = (self.k + 1) % 56;
        self.j = (self.j + 1) % 56;
        v
    }

    /// `rand() % n` (0 se `n == 0`; o sorteio é consumido do mesmo jeito).
    pub fn below(&mut self, n: u32) -> u32 {
        let v = self.next_u16();
        if n == 0 { 0 } else { v % n }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rng_is_deterministic_and_bounded() {
        let mut a = Rng::new(7);
        let mut b = Rng::new(7);
        let xs: Vec<u32> = (0..50).map(|_| a.below(100)).collect();
        let ys: Vec<u32> = (0..50).map(|_| b.below(100)).collect();
        assert_eq!(xs, ys);
        assert!(xs.iter().all(|&x| x < 100));
        assert!(xs.iter().any(|&x| x != xs[0]));
        assert_eq!(Rng::new(1).below(0), 0);
    }

    /// Os primeiros valores do `rand` do original a partir do estado do executável; os 14
    /// primeiros `% 13`/`% 12` são os quadros iniciais medidos no oráculo na `T_J_001`
    /// (5 objetos antes das minas, 7 minas de 13 quadros, armadura e vida de 12).
    #[test]
    fn matches_the_original_sequence() {
        let mut r = Rng::original();
        let xs: Vec<u32> = (0..14).map(|_| r.next_u16()).collect();
        assert_eq!(&xs[..4], &[59915, 10929, 37296, 33029]);
        let mines: Vec<u32> = xs[5..12].iter().map(|x| x % 13).collect();
        assert_eq!(mines, [11, 5, 10, 2, 8, 8, 8]);
        assert_eq!((xs[12] % 12, xs[13] % 12), (4, 6));
        // os índices dão a volta em 56
        let mut r = Rng::original();
        assert_eq!(r.indices(), (24, 55));
        r.next_u16();
        assert_eq!(r.indices(), (25, 0));
        for _ in 0..55 {
            r.next_u16();
        }
        assert_eq!(r.indices(), (24, 55));
    }
}
