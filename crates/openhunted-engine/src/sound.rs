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

/// Vagas da fila do locutor (`0x497190`, 10 × 36 bytes).
pub const SPEECH_SLOTS: usize = 10;

/// Uma sequência de até 3 falas, tocadas uma depois da outra pela fila do locutor
/// (`0x0043c868(id1, id2, id3, p1, p2, p3)`); id 0 encerra.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Chain {
    /// (id, prioridade) de cada fala.
    pub lines: [(u16, u16); 3],
    /// Só entra se nenhuma das falas já está na fila (os chamadores testam `0x0043c814`
    /// antes, como a porta de célula trancada).
    pub unique: bool,
}

impl Chain {
    /// Sequência das falas dadas (as que passam de 3 são ignoradas).
    pub fn new(lines: &[(u16, u16)], unique: bool) -> Self {
        let mut c = Chain {
            unique,
            ..Default::default()
        };
        for (o, l) in c.lines.iter_mut().zip(lines) {
            *o = *l;
        }
        c
    }

    fn ids(&self) -> impl Iterator<Item = u16> + '_ {
        self.lines.iter().map(|l| l.0).filter(|&id| id != 0)
    }
}

/// Fila do locutor (`0x497190`): 10 vagas, cada uma com uma sequência de falas. Por tique
/// (`0x0043c8cd`), a vaga cuja fala da frente ainda não começou a toca (sem posição, pela
/// prioridade dela); a que começou e já parou de tocar passa à fala seguinte, que toca no
/// tique seguinte. Sem vaga livre, a sequência se perde.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Announcer {
    slots: [Option<(Chain, bool)>; SPEECH_SLOTS],
}

impl Announcer {
    /// A fala está em alguma vaga (`0x0043c814`).
    pub fn contains(&self, id: u16) -> bool {
        self.slots
            .iter()
            .flatten()
            .any(|(c, _)| c.ids().any(|x| x == id))
    }

    /// Fila vazia (`0x0043c84a`).
    pub fn is_empty(&self) -> bool {
        self.slots.iter().all(Option::is_none)
    }

    /// Põe a sequência na primeira vaga livre (`0x0043c868`); `false` se não entrou.
    pub fn enqueue(&mut self, c: Chain) -> bool {
        if c.ids().next().is_none() || (c.unique && c.ids().any(|id| self.contains(id))) {
            return false;
        }
        match self.slots.iter_mut().find(|s| s.is_none()) {
            Some(s) => {
                *s = Some((c, false));
                true
            }
            None => false,
        }
    }

    /// Um tique da fila (`0x0043c8cd`): `playing(id)` diz se o som ainda toca no mixer.
    pub fn tick(&mut self, mut playing: impl FnMut(u16) -> bool) -> Vec<Cue> {
        let mut out = Vec::new();
        for slot in self.slots.iter_mut() {
            let Some((c, started)) = slot.as_mut() else {
                continue;
            };
            if !*started {
                out.push(Cue::ui(c.lines[0].0, c.lines[0].1));
                *started = true;
            } else if !playing(c.lines[0].0) {
                c.lines = [c.lines[1], c.lines[2], (0, 0)];
                *started = false;
                if c.lines[0].0 == 0 {
                    *slot = None;
                }
            }
        }
        out
    }
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

    #[test]
    fn announcer_plays_chains_in_order_after_each_line_ends() {
        let mut a = Announcer::default();
        assert!(a.enqueue(Chain::new(&[(3715, 200), (3714, 200)], true)));
        assert!(a.contains(3714) && !a.contains(3719));
        // a mesma fala na fila: o chamador não repete
        assert!(!a.enqueue(Chain::new(&[(3715, 200), (3719, 200)], true)));
        let ids = |v: Vec<Cue>| v.iter().map(|c| (c.id, c.priority)).collect::<Vec<_>>();
        assert_eq!(ids(a.tick(|_| true)), vec![(3715, 200)]);
        assert_eq!(ids(a.tick(|_| true)), vec![], "ainda tocando");
        assert_eq!(ids(a.tick(|_| false)), vec![], "acabou: passa à seguinte");
        assert_eq!(ids(a.tick(|_| false)), vec![(3714, 200)]);
        assert_eq!(ids(a.tick(|_| false)), vec![]);
        assert!(a.is_empty(), "a vaga se libera depois da última");
        // sem exigir unicidade (as provocações da Morte), duas vagas tocam juntas
        assert!(a.enqueue(Chain::new(&[(3760, 0)], false)));
        assert!(a.enqueue(Chain::new(&[(3760, 0), (3761, 0), (3762, 0)], false)));
        assert_eq!(ids(a.tick(|_| true)), vec![(3760, 0), (3760, 0)]);
        // dez vagas: a 11ª sequência se perde
        let mut b = Announcer::default();
        for _ in 0..SPEECH_SLOTS {
            assert!(b.enqueue(Chain::new(&[(3760, 0)], false)));
        }
        assert!(!b.enqueue(Chain::new(&[(3761, 0)], false)));
        assert!(!b.enqueue(Chain::new(&[], false)), "vazia");
    }
}
