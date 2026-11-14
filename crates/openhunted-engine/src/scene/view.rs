// SPDX-License-Identifier: GPL-3.0-or-later
//! Vistas (`docs/re/camadas-desenho.md`): distância e escala por camada (`0x436a70`),
//! projeção (`0x436916`, `0x43695c`), tabela de sombra da profundidade (`0x405e9a`), zoom
//! por célula (`0x436386`) e transição da vista na troca de camada (`0x4262c9`,
//! `0x4263ee`, `0x436b20`).

use crate::jake::State;
use crate::physics::Rect;

/// Referência da escala (`+0x18`, `0x436107`) e distância inicial (`+0x08`, `0x436126`).
pub const REFERENCE: i32 = 30;
pub const DEFAULT_DISTANCE: i32 = 20;
/// Distância a mais por camada para trás (`0x436a8f`).
pub const LAYER_STEP: i32 = 9;
/// Limites da distância (`0x4368d5`).
pub const MIN_DISTANCE: i32 = 1;
pub const MAX_DISTANCE: i32 = 250;
/// Passos da transição (`+0x44` vai de 1 a 7).
pub const TRANSITION: i32 = 7;
/// Cor das células vazias e de fora da grade na camada da vista (`0x4061ee`, `0x4058e8`).
pub const EMPTY_COLOUR: u8 = 245;

/// Tabela de sombra do `DLS.TBL` da profundidade (`0x405e91`): 0 → identidade, 1–6 →
/// 26 + prof., ≥ 7 → 33.
pub fn depth_shadow(depth: i32) -> usize {
    match depth {
        ..=0 => 0,
        1..=6 => (26 + depth) as usize,
        _ => 33,
    }
}

/// Estado da vista de um jogador (`+0x08`, `+0x40`, `+0x44`, `+0x48`, `+0x4c`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewState {
    pub layer: i32,
    pub distance: i32,
    /// Contador da transição (0 = parada; 1…6 em curso).
    pub counter: i32,
    /// Sentido (+1 descendo para z + 1, −1 subindo).
    pub dir: i32,
    /// Distância fixa do jogador da vista durante a transição (`+0x4c`), guardada **sem** a
    /// normalização pelo tamanho da vista (`ViewState::raw_distance`); a vista que desenha a
    /// normaliza (`View::focus_distance`). O original guarda já normalizado, pela própria vista
    /// (o tamanho não muda no meio da transição); aqui o tique da cena não conhece a vista.
    pub focus_distance: i32,
}

impl ViewState {
    pub fn new(layer: i32) -> Self {
        Self {
            layer,
            distance: DEFAULT_DISTANCE,
            counter: 0,
            dir: 0,
            focus_distance: 0,
        }
    }

    /// Distância da camada `layer` antes da normalização pelo tamanho da vista (`0x436a70`
    /// sem a transição): `(camada da vista − layer) · 9 + distância`.
    pub fn raw_distance(&self, layer: i32) -> i32 {
        (self.layer - layer)
            .saturating_mul(LAYER_STEP)
            .saturating_add(self.distance)
    }

    /// O jogador da vista passou de `prev` a `now` neste tique (rotinas 11 e 12 do Jake).
    /// `start` = distância da camada dele no começo (`0x436a70`, para `+0x4c`).
    pub fn follow(&mut self, prev: Option<State>, now: Option<State>, layer: i32, start: i32) {
        use State::{LayerDown, LayerUp};
        match (prev, now) {
            // meia-volta 11 → 12 (`0x42632b`)
            // (`+0x48 = −+0x48`: o sentido era +1)
            (Some(LayerDown(c)), Some(LayerUp(_))) => {
                self.counter = c;
                self.dir = -1;
            }
            // meia-volta 12 → 11 (`0x426456`), seguida de `0x4262c9` no mesmo tique: com a
            // transição parada e o contador do Jake em 0 (último tique da subida), começa
            // outra (`0x426352…0x42637c`)
            (Some(LayerUp(_)), Some(LayerDown(c))) => {
                if self.counter == 1 && self.layer > 0 {
                    self.layer -= 1;
                }
                if self.counter == 0 && c == 0 {
                    self.focus_distance = start;
                    self.counter = 1;
                }
                self.dir = 1;
            }
            (p, Some(LayerDown(_))) => {
                let entering = !matches!(p, Some(LayerDown(_)));
                if entering && self.counter == 0 {
                    self.focus_distance = start;
                    self.counter = 1;
                }
                self.dir = 1;
            }
            (p, Some(LayerUp(_))) => {
                let entering = !matches!(p, Some(LayerUp(_)));
                if entering && self.counter == 0 {
                    self.focus_distance = start;
                    self.counter = TRANSITION;
                }
                self.dir = -1;
            }
            // fim da rotina (`0x4263d5`): a vista fica na camada do jogador
            (Some(LayerDown(_) | LayerUp(_)), _) => {
                self.counter = 0;
                self.layer = layer;
            }
            // Ruling: parada, a vista acompanha a camada do jogador (teletransporte, renascer)
            _ if self.counter == 0 => self.layer = layer,
            _ => {}
        }
    }

    /// Tique da vista depois dos objetos: transição (`0x436b20`) e zoom ±1 até `target`
    /// (`+0x60` da célula; 0 = 20) (`0x436898`).
    pub fn tick(&mut self, layers: i32, target: i32) {
        if self.counter != 0 {
            if self.dir == -1 {
                self.counter -= 1;
                if self.counter == 0 && self.layer > 0 {
                    self.layer -= 1;
                }
            } else {
                if self.counter == 1 && self.layer < layers - 1 {
                    self.layer += 1;
                }
                self.counter += self.dir;
                if self.counter == TRANSITION {
                    self.counter = 0;
                }
            }
        }
        let target = if target == 0 {
            DEFAULT_DISTANCE
        } else {
            target
        };
        self.distance =
            (self.distance + (target - self.distance).signum()).clamp(MIN_DISTANCE, MAX_DISTANCE);
    }
}

/// Uma vista: onde fica no quadro, que ponto do mundo mostra e como projeta cada camada.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct View {
    /// Retângulo da vista no quadro (recorte de tudo).
    pub screen: Rect,
    /// Ponto do quadro onde cai o centro (`+0x20 + largura/2`).
    pub anchor: (i32, i32),
    /// Centro no mundo (`+0x00/+0x04`).
    pub center: (i32, i32),
    /// Tamanho de tela usado na normalização da distância (`+0x38/+0x3c`).
    pub size: (i32, i32),
    /// Número de vistas (`0x497300`).
    pub views: i32,
    pub state: ViewState,
    /// Jogador seguido (usa `focus_distance` na transição).
    pub player: Option<usize>,
    /// `None`: a camada da vista em 1:1 (o porte); `Some(30)`: a escala do original.
    pub reference: Option<i32>,
}

impl View {
    /// Vista de tamanho `size` em `screen`, com a câmera `camera` (canto superior esquerdo
    /// da camada da vista em 1:1) e o centro a meia altura de `size`.
    pub fn new(screen: Rect, size: (i32, i32), camera: (i32, i32), state: ViewState) -> Self {
        let half = (size.0 / 2, size.1 / 2);
        Self {
            screen,
            anchor: (screen.x + half.0, screen.y + half.1),
            center: (
                camera.0.saturating_add(half.0),
                camera.1.saturating_add(half.1),
            ),
            size,
            views: 1,
            state,
            player: None,
            reference: None,
        }
    }

    /// Normalização da distância pelo tamanho da vista (`0x436a70`): `d·640/largura` se a
    /// altura passa de 480 (uma vista) ou 240 (duas), senão `d·480/altura` (ou `d·240/altura`).
    fn normalise(&self, d: i64) -> i64 {
        let r: i64 = if self.views <= 1 { 480 } else { 240 };
        let (w, h) = (i64::from(self.size.0.max(1)), i64::from(self.size.1.max(1)));
        if r < h { d * 640 / w } else { d * r / h }
    }

    /// Distância normal da camada (`0x436a70` sem a transição).
    fn base_distance(&self, layer: i32) -> i64 {
        self.normalise(i64::from(self.state.raw_distance(layer)))
    }

    /// Distância do jogador da vista durante a transição (`+0x4c`), normalizada por esta
    /// vista.
    pub fn focus_distance(&self) -> i32 {
        let d = self.normalise(i64::from(self.state.focus_distance));
        if d == 0 {
            1
        } else {
            d.clamp(-(1 << 30), 1 << 30) as i32
        }
    }

    /// Distância da camada (`0x436a70`), com a transição.
    pub fn layer_distance(&self, layer: i32) -> i32 {
        let mut d = self.base_distance(layer);
        if self.state.counter != 0 {
            let k = i64::from(TRANSITION - self.state.counter);
            let t = i64::from(TRANSITION);
            if layer == self.state.layer {
                let dist = i64::from(self.state.distance);
                d = dist - dist * k / t;
            } else {
                d -= i64::from(LAYER_STEP) * k / t;
            }
        }
        if d == 0 { 1 } else { d as i32 }
    }

    /// Numerador da escala (`+0x18`): no porte, a distância normal da camada da vista.
    pub fn reference(&self) -> i32 {
        self.reference
            .unwrap_or_else(|| self.base_distance(self.state.layer).max(1) as i32)
    }

    /// Ponto do mundo de uma camada a distância `d` → quadro (`0x4369a2`: `(v·s + d − 1)/d`).
    pub fn project(&self, p: (i32, i32), d: i32) -> (i32, i32) {
        let s = self.reference();
        let f = |v: i32, c: i32, a: i32| {
            let v = i64::from(v) - i64::from(c);
            let o = if s == d {
                v
            } else {
                (v * i64::from(s) + i64::from(d) - 1) / i64::from(d.max(1))
            };
            (o + i64::from(a)).clamp(-(1 << 30), 1 << 30) as i32
        };
        (
            f(p.0, self.center.0, self.anchor.0),
            f(p.1, self.center.1, self.anchor.1),
        )
    }

    /// Retângulo do quadro → mundo da camada a distância `d`, por fora (para escolher o que
    /// desenhar; a inversa do original, `0x436845`, arredonda para dentro).
    pub fn unproject(&self, r: Rect, d: i32) -> Rect {
        let s = i64::from(self.reference());
        let d = i64::from(d.max(1));
        let lo = |v: i32, a: i32, c: i32| {
            ((i64::from(v) - i64::from(a)) * d).div_euclid(s) + i64::from(c)
        };
        let hi = |v: i32, a: i32, c: i32| {
            -((-(i64::from(v) - i64::from(a)) * d).div_euclid(s)) + i64::from(c)
        };
        let x0 = lo(r.x, self.anchor.0, self.center.0);
        let y0 = lo(r.y, self.anchor.1, self.center.1);
        let x1 = hi(r.x.saturating_add(r.w), self.anchor.0, self.center.0);
        let y1 = hi(r.y.saturating_add(r.h), self.anchor.1, self.center.1);
        let c = |v: i64| v.clamp(-(1 << 30), 1 << 30) as i32;
        Rect {
            x: c(x0),
            y: c(y0),
            w: c(x1 - x0),
            h: c(y1 - y0),
        }
    }

    /// Retângulo do mundo (distância `d`) → quadro, pelos dois cantos.
    pub fn project_rect(&self, r: Rect, d: i32) -> Rect {
        let (x0, y0) = self.project((r.x, r.y), d);
        let (x1, y1) = self.project((r.x.saturating_add(r.w), r.y.saturating_add(r.h)), d);
        Rect {
            x: x0,
            y: y0,
            w: x1 - x0,
            h: y1 - y0,
        }
    }
}

/// A célula de um sprite de cenário ou decalque.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpriteCell {
    /// (x, y, z) na grade.
    pub cell: (i32, i32, i32),
    /// Decalque da frente (`+0x24 ≠ 0`), desenhado depois dos objetos.
    pub front: bool,
}

/// O que o desenho das camadas lê das células.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CellMap {
    /// Tamanho da grade (x, y, z); (0, 0, 0) numa cena montada à mão.
    pub grid: (i32, i32, i32),
    /// Buracos (`+0x6c = 0`, `+0x70…+0x7c`), em px da fase, com a célula.
    pub holes: Vec<((i32, i32, i32), Rect)>,
    /// Distância da câmera da célula (`+0x60`), só as ≠ 0.
    pub zoom: std::collections::HashMap<(i32, i32, i32), i32>,
}

impl CellMap {
    /// Lê a célula `key` (campos na ordem de `openhunted_formats::Cell::fields`).
    pub fn add(&mut self, cell: &openhunted_formats::Cell, key: (i32, i32, i32)) {
        let o = usize::from(cell.version >= 1006);
        let f = |i: usize| cell.fields.get(o + i).copied().unwrap_or(0);
        if f(1) != 0 {
            self.zoom.insert(key, f(1));
        }
        let (x, y, w, h) = (f(5), f(6), f(7), f(8));
        if f(4) == 0 && w != 0 && h != 0 {
            // espelhada com a célula (`0x405775`)
            let x = if cell.mirrored {
                super::CELL - x - w
            } else {
                x
            };
            self.holes.push((
                key,
                Rect {
                    x: key.0 * super::CELL + x,
                    y: key.1 * super::CELL + y,
                    w,
                    h,
                },
            ));
        }
    }

    /// Distância da câmera na célula sob `p` na camada (0 = padrão).
    pub fn zoom_at(&self, p: (i32, i32), layer: i32) -> i32 {
        if p.0 < 0 || p.1 < 0 {
            return 0;
        }
        let c = super::CELL;
        self.zoom
            .get(&(p.0 / c, p.1 / c, layer))
            .copied()
            .unwrap_or(0)
    }
}

/// Interseção de dois retângulos (vazia → largura ou altura ≤ 0).
pub fn intersect(a: Rect, b: Rect) -> Rect {
    let x0 = a.x.max(b.x);
    let y0 = a.y.max(b.y);
    let x1 = a.x.saturating_add(a.w).min(b.x.saturating_add(b.w));
    let y1 = a.y.saturating_add(a.h).min(b.y.saturating_add(b.h));
    Rect {
        x: x0,
        y: y0,
        w: x1 - x0,
        h: y1 - y0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solo(layer: i32) -> View {
        View::new(
            Rect {
                x: 0,
                y: 0,
                w: 640,
                h: 480,
            },
            (640, 405),
            (0, 0),
            ViewState::new(layer),
        )
    }

    #[test]
    fn layer_distance_follows_0x436a70() {
        let v = solo(2);
        // 20·480/405 = 23; 29·480/405 = 34; 38·480/405 = 45
        assert_eq!(v.layer_distance(2), 23);
        assert_eq!(v.layer_distance(1), 34);
        assert_eq!(v.layer_distance(0), 45);
        // dois jogadores, 640×203: referência 240
        let mut h = solo(1);
        (h.size, h.views) = ((640, 203), 2);
        assert_eq!(h.layer_distance(1), 20 * 240 / 203);
        // vista mais alta que a referência: normaliza pela largura (640/640)
        let mut t = solo(0);
        t.size = (640, 600);
        assert_eq!(t.layer_distance(0), 20);
    }

    #[test]
    fn transition_moves_the_layers() {
        let mut v = solo(1);
        v.state.counter = 2;
        // camada da vista: 20 − 20·5/7 = 6, sem normalizar; a de trás: 34 − 9·5/7 = 28
        assert_eq!(v.layer_distance(1), 6);
        assert_eq!(v.layer_distance(0), 28);
        v.state.counter = 6;
        assert_eq!((v.layer_distance(1), v.layer_distance(0)), (18, 33));
    }

    #[test]
    fn depth_shadow_tables() {
        let t: Vec<usize> = (0..9).map(depth_shadow).collect();
        assert_eq!(t, vec![0, 27, 28, 29, 30, 31, 32, 33, 33]);
    }

    #[test]
    fn own_layer_is_one_to_one_and_back_layers_shrink_around_the_centre() {
        let mut v = solo(1);
        v.center = (1000, 500);
        let d = v.layer_distance(1);
        assert_eq!(v.project((1000, 500), d), (320, 202));
        assert_eq!(v.project((1010, 480), d), (330, 182));
        let b = v.layer_distance(0);
        // 34 px do mundo de trás = 23 px na tela
        assert_eq!(v.project((1034, 500), b), (343, 202));
        let r = v.unproject(v.screen, b);
        assert!(r.x <= 1000 - 320 * 34 / 23 && r.w >= 640 * 34 / 23, "{r:?}");
        // a escala do original: 30/23 na própria camada
        v.reference = Some(REFERENCE);
        assert_eq!(v.project((1023, 500), d), (350, 202));
    }

    fn walk(v: &mut ViewState, states: &[State], layers: &[i32], grid: i32) -> Vec<(i32, i32)> {
        let mut prev = Some(State::Idle);
        let mut out = Vec::new();
        for (s, &l) in states.iter().zip(layers) {
            v.follow(prev, Some(*s), l, 23);
            v.tick(grid, 0);
            out.push((v.layer, v.counter));
            prev = Some(*s);
        }
        out
    }

    #[test]
    fn going_down_switches_the_view_first_and_settles_in_seven_ticks() {
        use State::LayerDown as D;
        let mut v = ViewState::new(0);
        let s = [D(1), D(2), D(3), D(4), D(5), D(6), D(7), State::Idle];
        let l = [0, 0, 1, 1, 1, 1, 1, 1];
        let got = walk(&mut v, &s, &l, 3);
        assert_eq!(v.focus_distance, 23);
        assert_eq!(
            got,
            vec![
                (1, 2),
                (1, 3),
                (1, 4),
                (1, 5),
                (1, 6),
                (1, 0),
                (1, 0),
                (1, 0)
            ]
        );
    }

    #[test]
    fn going_up_keeps_the_view_until_the_counter_runs_out() {
        use State::LayerUp as U;
        let mut v = ViewState::new(1);
        let s = [U(6), U(5), U(4), U(3), U(2), U(1), U(0), State::Idle];
        let l = [1, 1, 1, 0, 0, 0, 0, 0];
        let got = walk(&mut v, &s, &l, 3);
        assert_eq!(
            got,
            vec![
                (1, 6),
                (1, 5),
                (1, 4),
                (1, 3),
                (1, 2),
                (1, 1),
                (0, 0),
                (0, 0)
            ]
        );
    }

    #[test]
    fn half_turn_reverses_the_transition() {
        use State::{LayerDown as D, LayerUp as U};
        let mut v = ViewState::new(0);
        // desce dois tiques (a vista já passou à 1), volta com o contador do Jake (2)
        let got = walk(
            &mut v,
            &[D(1), D(2), U(2), U(1), U(0), State::Idle],
            &[0; 6],
            3,
        );
        assert_eq!(got, vec![(1, 2), (1, 3), (1, 1), (0, 0), (0, 0), (0, 0)]);
        // a última volta deixa a vista na camada do jogador
        assert_eq!(v.layer, 0);
    }

    /// Meia-volta 12 → 11 no último tique da subida (contador do Jake 0) com a transição
    /// já parada (`+0x44 = 0`): o original entra em `0x4262c9`, que começa outra transição
    /// (`+0x4c`, `+0x44 = 1`, `0x426352…0x42637c`), como uma descida nova.
    #[test]
    fn half_turn_on_the_last_tick_starts_a_new_transition() {
        use State::{LayerDown as D, LayerUp as U};
        let mut v = ViewState::new(1);
        let got = walk(
            &mut v,
            &[
                U(6),
                U(5),
                U(4),
                U(3),
                U(2),
                U(1),
                U(0),
                D(0),
                D(1),
                D(2),
                D(3),
                D(4),
                D(5),
                State::Idle,
            ],
            &[1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1],
            3,
        );
        assert_eq!(
            got[6..],
            [
                (0, 0),
                (1, 2),
                (1, 3),
                (1, 4),
                (1, 5),
                (1, 6),
                (1, 0),
                (1, 0)
            ]
        );
        assert_eq!(v.focus_distance, 23);
        // com o contador do Jake ≠ 0, a meia-volta não começa outra
        let mut w = ViewState::new(0);
        w.follow(Some(U(2)), Some(D(2)), 0, 23);
        assert_eq!(w.counter, 0);
    }

    #[test]
    fn zoom_steps_towards_the_cell_distance() {
        let mut v = ViewState::new(0);
        v.tick(1, 30);
        v.tick(1, 30);
        assert_eq!(v.distance, 22);
        v.tick(1, 0);
        assert_eq!(v.distance, 21);
        v.distance = 1;
        v.tick(1, -5);
        assert_eq!(v.distance, MIN_DISTANCE);
    }

    /// O `+0x4c` guardado na entrada da transição é normalizado pela vista que desenha: com
    /// uma ou duas vistas e qualquer tamanho, o Jake fica no tamanho que a camada dele tinha
    /// nessa vista no começo (revisão da M16: o tique normalizava por 640×405 e uma vista).
    #[test]
    fn focus_distance_is_normalised_by_the_drawing_view() {
        for views in [1, 2] {
            for size in [(640, 405), (640, 202), (320, 405), (640, 480)] {
                let mut st = ViewState::new(1);
                st.distance = 26;
                let mut v = View::new(solo(1).screen, size, (0, 0), st);
                v.views = views;
                let want = v.layer_distance(1);
                v.state.follow(
                    Some(State::Idle),
                    Some(State::LayerDown(1)),
                    1,
                    st.raw_distance(1),
                );
                assert_eq!(v.focus_distance(), want, "{views} vistas, {size:?}");
            }
        }
    }
}
