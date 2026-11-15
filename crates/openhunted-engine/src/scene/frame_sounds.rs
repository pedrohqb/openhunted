// SPDX-License-Identifier: GPL-3.0-or-later
//! Sons por quadro de animação do `SOUND.TBL` (`docs/re/som.md`) que as rotinas próprias da
//! cena não tocam: os do cenário animado (grupos 0 e 1, `0x004328c2`: quando o quadro da
//! célula muda, `0x43c2b6(grupo, quadro, centro da célula, camada)`) e os passos, esforços e
//! vozes dos jogadores (listas 1 e 2, `0x004354dc`). Os sons de arma dos jogadores saem pela
//! tabela das armas (`jake::weapons`) e os demais tipos pelas rotinas deles.

use openhunted_formats::SoundTable;

use super::{Layer, Scene};
use crate::behaviour::Event;
use crate::sound::Cue;

/// Sons dos jogadores que a cena já toca por outro caminho (armas e pedidos do Garathe).
const PLAYER_ELSEWHERE: [u16; 9] = [3040, 3045, 3035, 3110, 3034, 3097, 3098, 5043, 5053];

/// Grupo do cenário pela animação da célula (`0x449978`: rótulos 10000+ → grupo 0 de
/// `WORLD1.ANM`, 20000+ → grupo 1 de `WORLD2.ANM`).
fn scenery_group(anim: &openhunted_formats::Animation) -> Option<usize> {
    let id = anim.labels.first()?.id;
    match id / 10000 {
        g @ 1..=2 => Some(usize::from(g) - 1),
        _ => None,
    }
}

/// O som do quadro `frame` no grupo `group` (0–1: cenário; 2: tipo `kind`).
fn lookup(t: &SoundTable, group: usize, kind: usize, frame: usize) -> Option<(u16, u16, bool)> {
    let v = if group < 2 {
        *t.groups[group].first()?.get(frame)?
    } else {
        return t
            .frame_sound(kind, frame)
            .map(|s| (s.id, s.priority, s.looped));
    };
    openhunted_formats::TableSound::decode(v).map(|s| (s.id, s.priority, s.looped))
}

impl Scene {
    /// Depois das animações do tique: os sons dos quadros novos do cenário e dos jogadores.
    pub(super) fn frame_sounds(&mut self) {
        let Some(t) = self.sound_table.clone() else {
            return;
        };
        let n = self.sprites.len();
        self.sprite_sounded.resize(n, None);
        let mut cues = Vec::new();
        for (k, sp) in self.sprites.iter().enumerate() {
            if sp.layer != Layer::Scenery {
                continue;
            }
            let f = sp.player.frame();
            if self.sprite_sounded[k] == Some(f) {
                continue;
            }
            // a primeira passagem só marca o quadro (o desenho da carga já o "sonorizou")
            let first = self.sprite_sounded[k].is_none();
            self.sprite_sounded[k] = Some(f);
            let (Some(g), Some(Some(cell))) = (scenery_group(&sp.anim), self.sprite_cells.get(k))
            else {
                continue;
            };
            if first {
                continue;
            }
            if let Some((id, p, false)) = lookup(&t, g, 0, f) {
                let c = (sp.x + super::CELL / 2, sp.y + super::CELL / 2);
                cues.push(Cue::at(id, p, c, cell.cell.2));
            }
        }
        let n = self.players.len().max(1);
        self.player_sounded
            .resize(n.max(self.player_sounded.len()), None);
        for i in 0..n {
            let (Some(j), Some(sp)) = (self.player_jake(i), self.player_sprite(i)) else {
                continue;
            };
            let f = sp.player.frame();
            let kind = j.character().kind() as usize;
            let b = j.body.bounds();
            let layer = j.layer();
            if self.player_sounded[i] == Some(f) {
                continue;
            }
            let first = self.player_sounded[i].is_none();
            self.player_sounded[i] = Some(f);
            if first {
                continue;
            }
            if let Some((id, p, false)) = lookup(&t, 2, kind, f)
                && !PLAYER_ELSEWHERE.contains(&id)
            {
                cues.push(Cue::at(id, p, (b.x + b.w / 2, b.y + b.h / 2), layer));
            }
        }
        self.events.extend(cues.into_iter().map(Event::Sound));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_weapon_sounds_stay_with_the_weapon_table() {
        for id in [3040, 3045, 3035, 3034] {
            assert!(PLAYER_ELSEWHERE.contains(&id));
        }
        assert!(!PLAYER_ELSEWHERE.contains(&5100), "passo: pela tabela");
    }
}
