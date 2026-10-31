// SPDX-License-Identifier: GPL-3.0-or-later
//! Gerador de números pseudoaleatórios determinístico (LCG). O original usa um gerador
//! aditivo atrasado (24, 55) de 16 bits (`0x00460a74`); aqui só a distribuição importa.

#[derive(Debug, Clone)]
pub struct Rng(u32);

impl Rng {
    pub fn new(seed: u32) -> Self {
        Self(seed)
    }

    /// Próximo valor de 16 bits.
    pub fn next_u16(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        (self.0 >> 16) & 0xffff
    }

    /// Valor em `0..n` (0 se `n == 0`).
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 { 0 } else { self.next_u16() % n }
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
}
