// SPDX-License-Identifier: GPL-3.0-or-later
//! Jogo salvo (`*.hhg`) e lista de heróis (`hunter.fam`); formato em
//! `docs/formats/salvamento.md`, comportamento em `docs/re/opcoes-salvamento.md`.

use crate::{FormatError, Result};

const WHAT: &str = "jogo salvo hhg";
const WHAT_FAM: &str = "lista de heróis";

/// Assinatura do bloco salvo (conferida na carga, `0x440bc6`).
pub const SIGNATURE: &str = "Hunter hunted saved game";
/// Tamanho do bloco (`0x7a8` bytes a partir de `0x47f79c`).
pub const SAVE_SIZE: usize = 1960;
/// Missões com recorde no bloco (`0x47fa8c[300]`).
pub const RECORDS: usize = 300;
/// Linhas da lista de heróis.
pub const MAX_HEROES: usize = 14;

/// Deslocamentos dos campos no bloco.
mod off {
    pub const NAME: usize = 0x020;
    pub const NAME_LEN: usize = 128;
    pub const SOLO_COUNT: usize = 0x0a4;
    pub const CURRENT: usize = 0x0ac;
    pub const LAST_LOADED: usize = 0x0b0;
    pub const TOTAL: usize = 0x0b4;
    pub const HERO_ID: usize = 0x0b8;
    pub const PIECES: usize = 0x0bc;
    pub const PATH: usize = 0x0c0;
    pub const LEVEL: usize = 0x1d8;
    pub const PATH_LEN: usize = 280;
    pub const RECORDS: usize = 0x2f0;
}

/// Estado da campanha gravado (`0x47f79c`). Os campos efêmeros (o tamanho da lista em
/// `0x0a0`, `0x0a8`, o ponteiro da lista em `0x7a0` e a capacidade dela em `0x7a4`) não são
/// guardados: na escrita valem zero e na leitura são ignorados. Com `0x0a0 = 0`, o original
/// remonta a lista (`0x4406d3`) no NEXT seguinte (`0x440e95`); com a contagem e o ponteiro
/// nulo, ele leria `[0x47ff3c]` (`0x440f25`, `0x440388`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveGame {
    /// Nome do jogador (o da lista de heróis).
    pub name: String,
    /// Missões da campanha no modo solo (`0x47f840`).
    pub solo_count: i32,
    /// Índice da missão corrente (`0x47f848`).
    pub current: i32,
    /// Índice da última missão carregada (`0x47f84c`).
    pub last_loaded: i32,
    /// Pontuação acumulada (soma dos recordes, `0x47f850`).
    pub total: i32,
    /// Id desta campanha na lista de heróis (`0x47f854`; 0 = nenhum).
    pub hero_id: i32,
    /// Peças do carro de fuga (`0x47f858`, 0–15).
    pub pieces: i32,
    /// Caminho do próprio arquivo (`0x47f85c`).
    pub path: String,
    /// Fase corrente, ex. `HUNTER.HLS|T_J_001.lev` (`0x47f974`).
    pub level: String,
    /// Recorde de cada missão (índice da lista solo; 0 = não vencida); 300 entradas.
    pub records: Vec<i32>,
}

impl Default for SaveGame {
    fn default() -> Self {
        Self {
            name: String::new(),
            solo_count: 0,
            current: 0,
            last_loaded: 0,
            total: 0,
            hero_id: 0,
            pieces: 0,
            path: String::new(),
            level: String::new(),
            records: vec![0; RECORDS],
        }
    }
}

fn i32_at(d: &[u8], o: usize) -> i32 {
    i32::from_le_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]])
}

/// String ASCIIZ do campo (o resto do campo é indefinido).
fn cstr(d: &[u8]) -> String {
    let end = d.iter().position(|&b| b == 0).unwrap_or(d.len());
    d[..end].iter().map(|&b| char::from(b)).collect()
}

/// Grava a string como ASCIIZ no campo, truncada para caber o zero; não ASCII vira `?`.
fn put_cstr(d: &mut [u8], s: &str) {
    let n = d.len() - 1;
    for (o, c) in d.iter_mut().zip(s.chars().take(n)) {
        *o = if c.is_ascii() { c as u8 } else { b'?' };
    }
}

impl SaveGame {
    /// Lê um bloco salvo: exige 1 960 bytes começando pela assinatura (senão, o original
    /// mostra 10009 "UNABLE TO LOAD. INVALID GAME FORMAT.").
    pub fn parse(d: &[u8]) -> Result<Self> {
        if d.len() != SAVE_SIZE {
            return Err(FormatError::Truncated {
                what: WHAT,
                offset: d.len(),
            });
        }
        let sig = SIGNATURE.as_bytes();
        if &d[..sig.len()] != sig || d[sig.len()] != 0 {
            return Err(FormatError::BadMagic {
                what: WHAT,
                expected: b"Hunter hunted saved game",
                found: d[..sig.len()].to_vec(),
            });
        }
        Ok(Self {
            name: cstr(&d[off::NAME..off::NAME + off::NAME_LEN]),
            solo_count: i32_at(d, off::SOLO_COUNT),
            current: i32_at(d, off::CURRENT),
            last_loaded: i32_at(d, off::LAST_LOADED),
            total: i32_at(d, off::TOTAL),
            hero_id: i32_at(d, off::HERO_ID),
            pieces: i32_at(d, off::PIECES),
            path: cstr(&d[off::PATH..off::PATH + off::PATH_LEN]),
            level: cstr(&d[off::LEVEL..off::LEVEL + off::PATH_LEN]),
            records: (0..RECORDS)
                .map(|i| i32_at(d, off::RECORDS + 4 * i))
                .collect(),
        })
    }

    /// Monta o bloco de 1 960 bytes; os campos efêmeros e o ponteiro ficam zero.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut d = vec![0u8; SAVE_SIZE];
        d[..SIGNATURE.len()].copy_from_slice(SIGNATURE.as_bytes());
        put_cstr(&mut d[off::NAME..off::NAME + off::NAME_LEN], &self.name);
        let mut put = |o: usize, v: i32| d[o..o + 4].copy_from_slice(&v.to_le_bytes());
        put(off::SOLO_COUNT, self.solo_count);
        put(off::CURRENT, self.current);
        put(off::LAST_LOADED, self.last_loaded);
        put(off::TOTAL, self.total);
        put(off::HERO_ID, self.hero_id);
        put(off::PIECES, self.pieces);
        for (i, &r) in self.records.iter().take(RECORDS).enumerate() {
            put(off::RECORDS + 4 * i, r);
        }
        put_cstr(&mut d[off::PATH..off::PATH + off::PATH_LEN], &self.path);
        put_cstr(&mut d[off::LEVEL..off::LEVEL + off::PATH_LEN], &self.level);
        d
    }
}

/// Uma linha da lista de heróis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hero {
    pub name: String,
    pub score: i32,
    /// Id da campanha (`0x47f854` do jogo salvo).
    pub id: i32,
}

/// `hunter.fam`: até 14 heróis em ordem decrescente de pontuação.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HeroList {
    /// Último id de campanha dado.
    pub counter: u32,
    pub heroes: Vec<Hero>,
}

impl HeroList {
    /// Lê o arquivo (`0x444e3b`): contador em claro, chave, entradas com XOR da chave.
    pub fn parse(d: &[u8]) -> Result<Self> {
        if d.len() < 5 {
            return Err(FormatError::Truncated {
                what: WHAT_FAM,
                offset: d.len(),
            });
        }
        let counter = u32::from_le_bytes([d[0], d[1], d[2], d[3]]);
        let key = d[4];
        let body: Vec<u8> = d[5..].iter().map(|&b| b ^ key).collect();
        let mut heroes = Vec::new();
        let mut p = 0;
        while p < body.len() && heroes.len() < MAX_HEROES {
            let Some(z) = body[p..].iter().position(|&b| b == 0) else {
                break;
            };
            let name = cstr(&body[p..p + z]);
            p += z + 1;
            if p + 8 > body.len() {
                return Err(FormatError::Truncated {
                    what: WHAT_FAM,
                    offset: 5 + p,
                });
            }
            let score = i32_at(&body, p);
            let id = i32_at(&body, p + 4);
            p += 8;
            heroes.push(Hero { name, score, id });
        }
        Ok(Self { counter, heroes })
    }

    /// Monta o arquivo com a chave `key` (o original a sorteia a cada gravação).
    pub fn to_bytes(&self, key: u8) -> Vec<u8> {
        let mut d = self.counter.to_le_bytes().to_vec();
        d.push(key);
        for h in self.heroes.iter().take(MAX_HEROES) {
            let mut e: Vec<u8> = h
                .name
                .chars()
                .filter(|c| *c != '\0')
                .map(|c| if c.is_ascii() { c as u8 } else { b'?' })
                .collect();
            e.push(0);
            e.extend_from_slice(&h.score.to_le_bytes());
            e.extend_from_slice(&h.id.to_le_bytes());
            d.extend(e.into_iter().map(|b| b ^ key));
        }
        d
    }

    /// A pontuação entra na lista (`0x445036`): há menos de 14 linhas ou ela supera a 14ª.
    pub fn qualifies(&self, score: i32) -> bool {
        self.heroes.len() < MAX_HEROES || self.heroes.last().is_some_and(|h| score > h.score)
    }

    /// Grava um herói (`0x445098`): com `id ≠ 0` e uma linha desse id, atualiza a linha;
    /// senão cria uma com id = contador + 1 (com 14, substitui a última). Reordena e devolve
    /// o id da campanha.
    pub fn record(&mut self, name: &str, score: i32, id: i32) -> i32 {
        let id = match self.heroes.iter_mut().find(|h| id != 0 && h.id == id) {
            Some(h) => {
                h.name = name.to_string();
                h.score = score;
                id
            }
            None => {
                self.counter = self.counter.wrapping_add(1);
                let id = self.counter as i32;
                if self.heroes.len() >= MAX_HEROES {
                    self.heroes.truncate(MAX_HEROES - 1);
                }
                self.heroes.push(Hero {
                    name: name.to_string(),
                    score,
                    id,
                });
                id
            }
        };
        // estável: em empate, quem já estava fica na frente
        self.heroes.sort_by_key(|h| std::cmp::Reverse(h.score));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> SaveGame {
        let mut records = vec![0; RECORDS];
        records[0] = 1234;
        records[5] = 77;
        records[299] = -1;
        SaveGame {
            name: "JAKE".into(),
            solo_count: 40,
            current: 5,
            last_loaded: 4,
            total: 1311,
            hero_id: 3,
            pieces: 2,
            path: "/home/x/saves/a.hhg".into(),
            level: "HUNTER.HLS|T_J_003.lev".into(),
            records,
        }
    }

    #[test]
    fn save_round_trips_with_fields_at_their_offsets() {
        let s = sample();
        let b = s.to_bytes();
        assert_eq!(b.len(), SAVE_SIZE);
        assert_eq!(&b[..25], b"Hunter hunted saved game\0");
        assert_eq!(&b[0x20..0x25], b"JAKE\0");
        assert_eq!(&b[0xac..0xb0], &5i32.to_le_bytes());
        assert_eq!(&b[0xb4..0xb8], &1311i32.to_le_bytes());
        assert_eq!(&b[0xbc..0xc0], &2i32.to_le_bytes());
        assert_eq!(&b[0x1d8..0x1d8 + 22], b"HUNTER.HLS|T_J_003.lev");
        assert_eq!(&b[0x2f0..0x2f4], &1234i32.to_le_bytes());
        assert_eq!(&b[0x2f0 + 20..0x2f0 + 24], &77i32.to_le_bytes());
        assert_eq!(&b[0x79c..0x7a0], &(-1i32).to_le_bytes());
        assert_eq!(SaveGame::parse(&b).unwrap(), s);
    }

    #[test]
    fn ephemeral_fields_and_pointer_are_zero_and_ignored() {
        let mut b = sample().to_bytes();
        // 0x0a0 = 0: o original só remonta a lista (`0x4406d3`) com 0x47f83c = 0; com a
        // contagem e o ponteiro nulo, o NEXT (`0x440f25`) lê [0x47ff3c] → falha
        assert_eq!(&b[0xa0..0xa4], &[0; 4], "tamanho da lista nunca gravado");
        assert_eq!(&b[0xa8..0xac], &[0; 4]);
        assert_eq!(
            &b[0x7a0..0x7a8],
            &[0; 8],
            "ponteiro e capacidade nunca gravados"
        );
        // um arquivo do original com lixo nesses campos lê igual
        b[0x7a0..0x7a8].copy_from_slice(&[0xde, 0xad, 0xbe, 0xef, 1, 2, 3, 4]);
        b[0xa0] = 40;
        b[0xa8] = 9;
        assert_eq!(SaveGame::parse(&b).unwrap(), sample());
    }

    #[test]
    fn invalid_saves_are_refused() {
        assert!(SaveGame::parse(&[0; 100]).is_err());
        let mut b = sample().to_bytes();
        b[0] = b'h';
        assert!(SaveGame::parse(&b).is_err());
        let mut b = sample().to_bytes();
        b.push(0);
        assert!(SaveGame::parse(&b).is_err());
    }

    #[test]
    fn long_strings_are_truncated_with_terminator() {
        let s = SaveGame {
            name: "N".repeat(300),
            ..Default::default()
        };
        let b = s.to_bytes();
        assert_eq!(b[0x20 + 127], 0);
        assert_eq!(SaveGame::parse(&b).unwrap().name.len(), 127);
    }

    #[test]
    fn hero_list_xor_round_trip() {
        let mut l = HeroList::default();
        assert_eq!(l.record("ANA", 500, 0), 1);
        assert_eq!(l.record("BIA", 900, 0), 2);
        let b = l.to_bytes(0x5a);
        assert_eq!(&b[..5], &[2, 0, 0, 0, 0x5a]);
        assert_eq!(b[5], b'B' ^ 0x5a, "entradas com XOR, a maior primeiro");
        assert_eq!(HeroList::parse(&b).unwrap(), l);
        assert_eq!(HeroList::parse(&l.to_bytes(0)).unwrap(), l);
        assert!(HeroList::parse(&[1, 0]).is_err());
        assert_eq!(
            HeroList::parse(&[0, 0, 0, 0, 7]).unwrap(),
            HeroList::default()
        );
    }

    #[test]
    fn hero_list_updates_by_campaign_and_keeps_fourteen() {
        let mut l = HeroList::default();
        for i in 0..14 {
            l.record(&format!("H{i}"), 100 + i, 0);
        }
        assert_eq!(l.heroes.len(), 14);
        assert!(!l.qualifies(100), "não supera a 14ª");
        assert!(l.qualifies(101));
        let id = l.record("NOVO", 1000, 0);
        assert_eq!(id, 15);
        assert_eq!(l.heroes.len(), 14);
        assert_eq!(l.heroes[0].name, "NOVO");
        assert!(l.heroes.iter().all(|h| h.name != "H0"), "a última saiu");
        // mesma campanha: atualiza a linha
        assert_eq!(l.record("NOVO2", 50, 15), 15);
        assert_eq!(l.heroes.len(), 14);
        assert_eq!(l.heroes.last().unwrap().name, "NOVO2");
    }
}
