// SPDX-License-Identifier: GPL-3.0-or-later
//! As telas em volta do jogo: interfaces `.ITF` (`screen`), menu principal, briefing,
//! barra de estado, pausa e resultado (`docs/re/menu.md`, `briefing.md`, `hud.md`,
//! `pausa.md`, `fim-de-missao.md`).

pub mod briefing;
pub mod generic;
pub mod hud;
pub mod results;
pub mod screen;

pub use screen::{CANCEL, Dir, Input, Placement, Screen};

use openhunted_formats::MissionInfo;

/// Ids do menu principal (`SMAIN.ITF`).
pub mod menu {
    pub const BACKGROUND: u16 = 11000;
    pub const RESTORE: u16 = 11001;
    pub const NEW_GAME: u16 = 11002;
    pub const CHOOSE_LEVEL: u16 = 11003;
    pub const OPTIONS: u16 = 11004;
    pub const INPUT_DEVICES: u16 = 11005;
    pub const HELP: u16 = 11006;
    pub const HERO_LIST: u16 = 11007;
    pub const EXIT: u16 = 11008;
    pub const MULTI_PLAYER: u16 = 11009;
    pub const SPARKS: u16 = 11011;
    pub const BLINKER: u16 = 11012;
    pub const FIRE: u16 = 11013;
    /// Botões 11001–11009 (sinais 0x81).
    pub const BUTTONS: [u16; 9] = [
        RESTORE,
        NEW_GAME,
        CHOOSE_LEVEL,
        OPTIONS,
        INPUT_DEVICES,
        HELP,
        HERO_LIST,
        EXIT,
        MULTI_PLAYER,
    ];
}

/// Ids de texto do `SYSTEM.RES` (linha `n` = id `10000 + n`).
pub mod text {
    pub const OKAY: u16 = 10001;
    pub const QUIT: u16 = 10005;
    pub const PAUSED: u16 = 10006;
    pub const IMPROVED: u16 = 10043;
    /// Base do `SYSTEM.RES`.
    pub const SYSTEM_BASE: u16 = 10000;
}

/// Missões do modo solo (`0x4406d3` com o modo 0x5600): a lista do `ENGLEVEL.TXT` sem as de
/// dois jogadores (`p=3` disputa, `p=4` cooperativo), na mesma ordem.
pub fn solo_missions(list: &[MissionInfo]) -> Vec<MissionInfo> {
    list.iter()
        .filter(|m| m.player != 3 && m.player != 4)
        .cloned()
        .collect()
}

/// Próxima missão (`0x440e95`): índice + 1, voltando a 0 no fim; se ainda há missões sem
/// recorde, pula as que já têm um.
pub fn next_mission(current: usize, records: &[i32]) -> usize {
    let n = records.len();
    if n == 0 {
        return 0;
    }
    let mut i = (current + 1) % n;
    if records.contains(&0) {
        while records[i] != 0 {
            i = (i + 1) % n;
        }
    }
    i
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(file: &str, player: u16) -> MissionInfo {
        MissionInfo {
            file: file.into(),
            player,
            extra: 0,
            title: String::new(),
            description: String::new(),
        }
    }

    #[test]
    fn solo_list_drops_two_player_missions_in_order() {
        let l = [m("A", 1), m("B", 3), m("C", 2), m("D", 4), m("E", 1)];
        let s: Vec<_> = solo_missions(&l).into_iter().map(|m| m.file).collect();
        assert_eq!(s, ["A", "C", "E"]);
    }

    #[test]
    fn next_skips_missions_with_records_while_some_remain() {
        assert_eq!(next_mission(0, &[0, 0, 0]), 1);
        assert_eq!(next_mission(2, &[0, 0, 0]), 0, "volta ao início");
        assert_eq!(next_mission(0, &[500, 300, 0]), 2);
        assert_eq!(next_mission(2, &[0, 300, 0]), 0);
        assert_eq!(
            next_mission(1, &[5, 5, 5]),
            2,
            "todas com recorde: só avança"
        );
        assert_eq!(next_mission(0, &[]), 0);
    }
}
