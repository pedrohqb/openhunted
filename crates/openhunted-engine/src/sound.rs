// SPDX-License-Identifier: GPL-3.0-or-later
//! Pedidos de som da cena e as regras do original para tocá-los (`docs/re/som.md`): nome do
//! recurso, volume e pan pela distância à vista e a fila de 4 pedidos posicionais por tique.

/// Ponto de origem de um som posicional, em px da fase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spot {
    pub x: i32,
    pub y: i32,
    pub layer: i32,
}

/// Um som pedido pela cena num tique.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cue {
    /// Id do `SOUND.RES` (`SX_%04d.WAV`).
    pub id: u16,
    /// Prioridade na disputa pelos 16 canais e pela fila do tique.
    pub priority: u16,
    /// `None`: som de interface, sem posição (volume cheio, centro).
    pub at: Option<Spot>,
}

impl Cue {
    /// Som sem posição (`0x0043c93d`).
    pub fn ui(id: u16, priority: u16) -> Self {
        Self {
            id,
            priority,
            at: None,
        }
    }

    /// Som posicional (`0x0043c2b6`) no ponto `(x, y)` da camada.
    pub fn at(id: u16, priority: u16, (x, y): (i32, i32), layer: i32) -> Self {
        Self {
            id,
            priority,
            at: Some(Spot { x, y, layer }),
        }
    }
}

/// Alcance do som posicional: a partir daqui, a vista não ouve (`d < 768`).
pub const HEARING: i32 = 768;
/// Volume cheio (escala 0–100 do original).
pub const FULL_VOLUME: u8 = 100;
/// Prioridade dos sons por quadro do `SOUND.TBL` (tiros, explosões, faísca).
pub const PRIORITY_FRAME: u16 = 9;
/// Prioridade da coleta de item e do aviso de objetivos (`0x43c93d(som, 200, …)`).
pub const PRIORITY_PICKUP: u16 = 200;
/// Pedidos posicionais por tique (`0x4970ec`).
pub const QUEUE: usize = 4;

/// Nome do recurso de um efeito (`0x00449978`/`0x004499e3`): ids 3001–9999 fora de
/// 4000–4999 viram `SX_%04d.WAV`; os demais não são efeitos.
pub fn resource_name(id: u16) -> Option<String> {
    let effect = (3001..=9999).contains(&id) && !(4000..=4999).contains(&id);
    effect.then(|| format!("SX_{id:04}.WAV"))
}

/// Volume (0–100) e pan (−128 … 128) de um som em `spot` ouvido pela vista centrada em
/// `listener` na camada `layer`: `volume = 100 − 100·d/768`, `pan = 128·dx/768`. Outra camada
/// ou `d ≥ 768` → não toca.
pub fn positional(listener: (i32, i32), layer: i32, spot: Spot) -> Option<(u8, i16)> {
    if spot.layer != layer {
        return None;
    }
    let d = crate::behaviour::mine::distance(listener, (spot.x, spot.y));
    if d >= HEARING {
        return None;
    }
    let volume = 100 - 100 * d / HEARING;
    let dx = spot.x.saturating_sub(listener.0);
    let pan = (128 * i64::from(dx) / i64::from(HEARING)).clamp(-128, 128);
    Some((volume as u8, pan as i16))
}

/// Um som pronto para o mixer: id, prioridade, volume e pan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Play {
    pub id: u16,
    pub priority: u16,
    pub volume: u8,
    pub pan: i16,
}

/// Resolve os sons de um tique como o original: os de interface tocam já, com volume cheio;
/// os posicionais passam pela vista (`positional`) e pela fila de 4 do tique, que aceita um
/// pedido numa vaga livre ou no lugar do de prioridade menor que a dele (cheia de
/// prioridades maiores ou iguais → descarta). A fila sai depois dos sons de interface.
pub fn resolve(cues: &[Cue], listener: (i32, i32), layer: i32) -> Vec<Play> {
    let mut now = Vec::new();
    let mut queue: Vec<Play> = Vec::with_capacity(QUEUE);
    for c in cues {
        match c.at {
            None => now.push(Play {
                id: c.id,
                priority: c.priority,
                volume: FULL_VOLUME,
                pan: 0,
            }),
            Some(spot) => {
                let Some((volume, pan)) = positional(listener, layer, spot) else {
                    continue;
                };
                let p = Play {
                    id: c.id,
                    priority: c.priority,
                    volume,
                    pan,
                };
                if queue.len() < QUEUE {
                    queue.push(p);
                } else if let Some(slot) = queue
                    .iter_mut()
                    .filter(|q| q.priority < p.priority)
                    .min_by_key(|q| q.priority)
                {
                    *slot = p;
                }
            }
        }
    }
    now.extend(queue);
    now
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spot(x: i32, y: i32) -> Spot {
        Spot { x, y, layer: 0 }
    }

    #[test]
    fn ids_map_to_effect_resources() {
        assert_eq!(resource_name(3040).as_deref(), Some("SX_3040.WAV"));
        assert_eq!(resource_name(3001).as_deref(), Some("SX_3001.WAV"));
        assert_eq!(resource_name(9999).as_deref(), Some("SX_9999.WAV"));
        assert_eq!(resource_name(5057).as_deref(), Some("SX_5057.WAV"));
        for not in [0, 1, 4, 1000, 3000, 4000, 4500, 4999, 10000] {
            assert_eq!(resource_name(not), None, "{not}");
        }
    }

    #[test]
    fn volume_and_pan_follow_the_distance_to_the_view() {
        let c = (1000, 500);
        assert_eq!(positional(c, 0, spot(1000, 500)), Some((100, 0)));
        // 384 px à direita: metade do volume, meio pan
        assert_eq!(positional(c, 0, spot(1384, 500)), Some((50, 64)));
        assert_eq!(positional(c, 0, spot(616, 500)), Some((50, -64)));
        // na vertical: volume cai, pan fica no centro
        assert_eq!(positional(c, 0, spot(1000, 692)), Some((75, 0)));
        // 3-4-5: d = 500, dx = 300
        assert_eq!(positional(c, 0, spot(1300, 900)), Some((35, 50)));
        // inteiros como no original: 100 − 100·767/768 = 1
        assert_eq!(positional(c, 0, spot(1767, 500)), Some((1, 127)));
        assert_eq!(
            positional(c, 0, spot(1768, 500)),
            None,
            "d = 768 já não toca"
        );
        assert_eq!(positional(c, 0, spot(0, 0)), None);
        assert_eq!(
            positional(c, 1, spot(1000, 500)),
            None,
            "outra camada não ouve"
        );
    }

    #[test]
    fn ui_sounds_play_at_full_volume_and_before_the_queue() {
        let cues = [
            Cue::at(3040, 9, (1384, 500), 0),
            Cue::ui(5057, 200),
            Cue::at(3046, 9, (5000, 500), 0),
        ];
        let p = resolve(&cues, (1000, 500), 0);
        assert_eq!(
            p,
            vec![
                Play {
                    id: 5057,
                    priority: 200,
                    volume: 100,
                    pan: 0
                },
                Play {
                    id: 3040,
                    priority: 9,
                    volume: 50,
                    pan: 64
                },
            ]
        );
    }

    #[test]
    fn tick_queue_keeps_four_and_evicts_the_lowest_priority() {
        let mut cues: Vec<Cue> = (0..4).map(|i| Cue::at(3020 + i, i, (0, 0), 0)).collect();
        // prioridade 0 não tira ninguém (só quem tem prioridade menor que a nova)
        cues.push(Cue::at(3100, 0, (0, 0), 0));
        // prioridade 9 tira o de menor prioridade (o 3020, prioridade 0)
        cues.push(Cue::at(3040, 9, (0, 0), 0));
        let ids: Vec<u16> = resolve(&cues, (0, 0), 0).iter().map(|p| p.id).collect();
        assert_eq!(ids, vec![3040, 3021, 3022, 3023]);
    }
}
