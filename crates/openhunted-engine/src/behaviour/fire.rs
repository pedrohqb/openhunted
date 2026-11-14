// SPDX-License-Identifier: GPL-3.0-or-later
//! Fogo (tipo 42, `0x004100a2`/`0x0041028c`/`0x004104a3`; `docs/re/barril-fogo-cerca.md`):
//! cresce e míngua pelos `values`, queima 1 por tique quem cruza a caixa.

use super::{Ctx, Hazard, Out};
use crate::damage::Pending;
use crate::entity::{Behaviour, Entity};
use crate::physics::Rect;
use crate::triggers::satisfied;

/// Altura máxima padrão e de reserva.
pub const MAX_HEIGHT: i32 = 96;
pub const DEFAULT_HEIGHT: i32 = 32;
/// Linha 42 do `DAMAGE.TBL`: natureza fogo, 1 de dano direto.
pub const DAMAGE: i32 = 1;
/// `values[13]` (`+0x294`) ≥ 1000: o fogo já foi iniciado e os campos valem como estão.
const STARTED: i32 = 1000;

#[derive(Debug, Clone)]
pub struct FireState {
    pub height: i32,
    /// Ritmo (`+0x280`): > 0 cresce `r % 10` a cada `r / 10` tiques; < 0 míngua; 0 estável.
    pub rate: i32,
    /// Tiques até o próximo passo (`+0x284`).
    pub timer: i32,
    /// Âncora: centro x e base y (`+0x288/+0x28c`).
    pub anchor: (i32, i32),
    pub max: i32,
    /// Altura do quadro de referência do `OFI.ANM` (escala = altura / esta).
    pub frame_h: i32,
    pub visible: bool,
}

impl FireState {
    /// `values`: 0 crescer, 1 minguar, 2 espera no máximo, 3 máximo, 4 altura inicial,
    /// 10/11 âncora e 13 marca de iniciado. `pos`/`size` = posição e tamanho do sprite na
    /// fase (para a âncora e a altura de quem não foi iniciado).
    pub fn new(values: &[i32], pos: (i32, i32), size: (i32, i32), frame_h: i32) -> Self {
        let v = |i: usize| values.get(i).copied().unwrap_or(0);
        let started = v(13) >= STARTED;
        let h = if started { v(4) } else { size.1 };
        let height = if h <= 0 {
            DEFAULT_HEIGHT
        } else {
            h.min(MAX_HEIGHT)
        };
        let anchor = if started && (v(10) != 0 || v(11) != 0) {
            (v(10), v(11))
        } else {
            (pos.0 + size.0 / 2, pos.1 + size.1)
        };
        let rate = if v(0) != 0 { v(0) } else { -v(1) };
        Self {
            height,
            rate,
            timer: rate.abs() / 10,
            anchor,
            max: if v(3) > 0 { v(3) } else { MAX_HEIGHT },
            frame_h: frame_h.max(1),
            visible: true,
        }
    }

    /// Caixa (0, 0, altura·512/169, altura), posta pela âncora.
    pub fn rect(&self) -> Rect {
        let w = self.height * 512 / 169;
        Rect {
            x: self.anchor.0 - w / 2,
            y: self.anchor.1 - self.height,
            w,
            h: self.height,
        }
    }

    /// Crescer/minguar; `false` = apagou.
    fn grow(&mut self, values: &[i32]) -> bool {
        let v = |i: usize| values.get(i).copied().unwrap_or(0);
        if self.rate == 0 {
            return true;
        }
        self.timer -= 1;
        if self.timer > 0 {
            return true;
        }
        if self.rate > 0 {
            self.height += self.rate % 10;
            self.timer = self.rate / 10;
            if self.height > self.max {
                self.height = self.max;
                self.timer = v(2);
                self.rate = -v(1);
            }
        } else {
            self.height -= (-self.rate) % 10;
            self.timer = (-self.rate) / 10;
            if self.height <= 0 {
                return false;
            }
        }
        true
    }
}

pub fn tick(e: &mut Entity, ctx: &Ctx, out: &mut Out) {
    let visible = satisfied(&e.triggers, ctx.on);
    let values = e.values.clone();
    let Behaviour::Fire(st) = &mut e.behaviour else {
        return;
    };
    st.visible = visible;
    e.on = visible;
    if !visible {
        return;
    }
    if !st.grow(&values) {
        e.alive = false;
        return;
    }
    let r = st.rect();
    let (h, fh) = (st.height, st.frame_h);
    e.x = r.x;
    e.y = r.y;
    e.boxes = vec![Rect {
        x: 0,
        y: 0,
        w: r.w,
        h: r.h,
    }];
    if let Some(sp) = e.sprite.as_mut() {
        sp.scale = (h, fh);
    }
    let burn = Pending {
        amount: DAMAGE,
        fire: true,
        ..Default::default()
    };
    // `0x4104a3`: todo objeto presente na caixa (os dois jogadores inclusive)
    let hit: Vec<Option<usize>> = ctx
        .active_in(e.layer)
        .filter(|(_, j)| j.overlaps(&r))
        .map(|(k, _)| k)
        .collect();
    for k in hit {
        out.hurt_player(k, &burn);
    }
    out.hazards.push(Hazard {
        rect: r,
        layer: e.layer,
        damage: burn,
        source: None,
        from: 0,
    });
}

/// Alcance do fogo deixado pelo barril (`0x40fb58(&pos, camada, 384)`, `0x0040e082`).
pub const BARREL_RANGE: i32 = 384;

/// Uma célula vista por `floor_fires` (`0x4317db`): bits de `célula+0x04` (2 = fechada
/// embaixo) e as caixas (tipo 2) do BRD atual, relativas à célula e já espelhadas.
pub struct FloorCell {
    pub bits: i32,
    pub boxes: Vec<Rect>,
}

/// Um fogo do chão: `values` (com a âncora em 10/11 e `values[13] = 1000`), espelho e se
/// fica atrás (`+0x1b4` 175, senão 55).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FloorFire {
    pub values: Vec<i32>,
    pub mirror: bool,
    pub behind: bool,
}

/// Os fogos que o barril com `values[0] ≠ 0` deixa ao explodir (`0x0040fb58`): acha o chão na
/// célula (`y` = 116 na célula, ou o topo de uma caixa larga na metade de baixo), estende-se
/// até `range/2` para cada lado até uma parede (caixa alta que cruza o meio da linha de
/// células) ou o fim das células, e cobre as células fechadas embaixo com fogos de até 256 px
/// (largura > 512 → o próximo começa 128 px adiante; > 384 → no meio do que sobra; > 256 → os
/// últimos 256). Cada fogo: cresce 5 por tique (`+0x260 = 5`) até `largura·169/512`, fica
/// `200 + rand % 20` tiques e míngua 1 px a cada `rand & 3` tiques; base 6 px abaixo do chão
/// ou 2 acima, por sorteio. O gerador é consumido na ordem do original.
pub fn floor_fires(
    at: (i32, i32),
    range: i32,
    level_w: i32,
    cell: &dyn Fn(i32, i32) -> Option<FloorCell>,
    rng: &mut crate::rng::Rng,
) -> Vec<FloorFire> {
    let mut fires = Vec::new();
    if range < 16 {
        return fires;
    }
    let (x, y) = at;
    let half = range / 2;
    let tall_across_middle = |b: &Rect| b.w < b.h && b.y <= 64 && b.y + b.h >= 64;
    // o chão
    let mut fy = y - y % 128 + 116;
    if let Some(c) = cell(x / 128, y / 128) {
        for b in &c.boxes {
            if b.y > 64 && b.w > b.h {
                fy += b.y - fy % 128;
            }
        }
    }
    let row = fy / 128;
    let boxes = |cx: i32| cell(cx, row).filter(|c| !c.boxes.is_empty());
    // para a esquerda
    let mut l = x;
    'left: while x - l < half {
        let Some(c) = boxes(l / 128) else { break };
        for b in &c.boxes {
            if l % 128 >= b.x + b.w && tall_across_middle(b) {
                l += b.x + b.w - l % 128;
                break 'left;
            }
        }
        l -= l % 128 + 1;
        if l < 0 {
            break;
        }
    }
    if half < x - l {
        l = x - half;
    }
    // para a direita
    let mut r = x;
    'right: while r - x < half {
        let Some(c) = boxes(r / 128) else { break };
        for b in &c.boxes {
            if r % 128 <= b.x && tall_across_middle(b) {
                r += b.x - r % 128;
                break 'right;
            }
        }
        r += 128 - r % 128;
        if r >= level_w {
            break;
        }
    }
    if r - x > half {
        r = x + half;
    }
    // trechos de células fechadas embaixo
    while r > l {
        let seg = l;
        while r > l {
            match cell(l / 128, row) {
                Some(c) if c.bits & 2 != 0 => {
                    let m = l % 128;
                    l += if m != 0 { 128 - m } else { 128 };
                    l = l.min(r);
                }
                _ => {
                    l = (l - l % 128).max(seg);
                    break;
                }
            }
        }
        if l == seg {
            l += 128 - l % 128;
            continue;
        }
        let w = l - seg;
        let fw = w.min(256);
        if w > 512 {
            l = seg + 128;
        } else if w > 384 {
            l = seg + (w - 256) / 2;
        } else if w > 256 {
            l -= 256;
        }
        let behind = rng.next_u16() & 1 != 0;
        let shrink = (rng.next_u16() & 3) as i32 * 10 + 1;
        let max = fw * 169 / 512;
        let stay = 200 + (rng.next_u16() % 20) as i32;
        let anchor = (seg + fw / 2, fy + if behind { 6 } else { -2 });
        let mirror = rng.next_u16() & 1 != 0;
        let mut values = vec![0; 14];
        values[0] = 5;
        values[1] = shrink;
        values[2] = stay;
        values[3] = max;
        values[4] = 2;
        values[10] = anchor.0;
        values[11] = anchor.1;
        values[13] = STARTED;
        fires.push(FloorFire {
            values,
            mirror,
            behind,
        });
    }
    fires
}

/// A entidade de um fogo do chão (sem sprite; a cena põe o `OFI.ANM`).
pub fn floor_fire_entity(f: &FloorFire, layer: i32, frame_h: i32) -> Entity {
    let st = FireState::new(&f.values, (0, 0), (0, 0), frame_h);
    let r = st.rect();
    Entity {
        kind: crate::entity::FIRE,
        x: r.x,
        y: r.y,
        // `0x10` (`0x0040ff6f`) e os da inicialização (`0x28`, sem `0x80`)
        flags: 0x8003d,
        layer,
        id: 0,
        values: f.values.clone(),
        triggers: Vec::new(),
        objective: 0,
        health: 0,
        armor: 0,
        boxes: vec![Rect {
            x: 0,
            y: 0,
            w: r.w,
            h: r.h,
        }],
        solid_shapes: false,
        on: true,
        alive: true,
        sprite: None,
        behaviour: Behaviour::Fire(st),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviour::test_support::{ctx, ctx_with_jake, logic};
    use crate::entity::FIRE;

    /// Os `values` do fogo da `T_J_005`.
    const T_J_005: [i32; 14] = [19, 0, 999, 32, 38, 308, 999, 0, 19, 1, 1164, 381, 38, 1000];

    fn fire(values: &[i32]) -> Entity {
        let mut f = logic(FIRE, 1107, 343, values);
        f.flags = 0x8002d;
        f.behaviour = Behaviour::Fire(FireState::new(values, (1107, 343), (115, 38), 63));
        f
    }

    #[test]
    fn tutorial_fire_settles_at_32_px_and_burns_one_per_tick() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(1150, 334);
        let c = ctx(&j, &w, &on);
        let mut f = fire(&T_J_005);
        let mut out = Out::default();
        tick(&mut f, &c, &mut out);
        let Behaviour::Fire(st) = &f.behaviour else {
            panic!()
        };
        assert_eq!(st.height, 32);
        assert_eq!(
            st.rect(),
            Rect {
                x: 1116,
                y: 349,
                w: 96,
                h: 32
            }
        );
        assert_eq!(out.jake_damage.amount, 1);
        assert!(out.jake_damage.fire);
        assert_eq!(out.hazards.len(), 1);
        for _ in 0..50 {
            tick(&mut f, &c, &mut out);
        }
        let Behaviour::Fire(st) = &f.behaviour else {
            panic!()
        };
        assert_eq!(st.height, 32, "values[1] = 0: estável");
        assert_eq!(out.jake_damage.amount, 51);
    }

    #[test]
    fn fire_grows_shrinks_and_hides_on_false_triggers() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(0, 0);
        let c = ctx(&j, &w, &on);
        // cresce 5 px a cada 2 tiques de 10 até 20, espera 3, míngua 4 por tique
        let vals = [25, 14, 3, 20, 10, 0, 0, 0, 0, 0, 100, 200, 0, 1000];
        let mut f = fire(&vals);
        let mut hs = vec![];
        for _ in 0..16 {
            if !f.alive {
                break;
            }
            tick(&mut f, &c, &mut Out::default());
            if let Behaviour::Fire(st) = &f.behaviour {
                hs.push(st.height);
            }
        }
        assert_eq!(hs[..6], [10, 15, 15, 20, 20, 20]);
        assert!(!f.alive, "apagou: {hs:?}");
        let off = |_: i32| Some(false);
        let c = ctx(&j, &w, &off);
        let mut f = fire(&T_J_005);
        f.triggers = vec![(3, Some(11))];
        let mut out = Out::default();
        tick(&mut f, &c, &mut out);
        assert!(!f.on && out.hazards.is_empty());
    }

    /// Células de 128 px numa linha (y 0): chão em 116 (sem caixa larga), paredes onde a
    /// lista diz, fechadas embaixo onde `floor`.
    fn row_cells(
        floor: &'static [bool],
        walls: &'static [(i32, Rect)],
    ) -> impl Fn(i32, i32) -> Option<FloorCell> {
        move |cx, cy| {
            if cy != 0 || cx < 0 || cx as usize >= floor.len() {
                return None;
            }
            let mut boxes = vec![Rect {
                x: 0,
                y: 120,
                w: 128,
                h: 8,
            }];
            boxes.extend(walls.iter().filter(|(c, _)| *c == cx).map(|(_, b)| *b));
            Some(FloorCell {
                bits: if floor[cx as usize] { 2 } else { 0 },
                boxes,
            })
        }
    }

    #[test]
    fn barrel_fire_covers_the_floor_up_to_walls_and_range() {
        let mut rng = crate::rng::Rng::new(7);
        // chão largo (caixa 128 × 8 em y 120 > 64): o chão é y = 120; sem paredes, ±192
        let cells = row_cells(&[true; 8], &[]);
        let f = floor_fires((500, 40), BARREL_RANGE, 1024, &cells, &mut rng);
        let spans: Vec<(i32, i32)> = f
            .iter()
            .map(|f| {
                let w = f.values[3] * 512 / 169;
                (f.values[10], w)
            })
            .collect();
        // de 308 a 692 (384 px, que não passa de 384): 256 a partir de 308 e, como passa de
        // 256, o próximo trecho são os últimos 256 (436…692), sobrepostos
        assert_eq!(f.len(), 2, "{spans:?}");
        assert_eq!(f[0].values[10], 308 + 128);
        assert_eq!(f[0].values[3], 256 * 169 / 512);
        assert_eq!(f[1].values[10], 436 + 128);
        assert_eq!(f[1].values[3], 256 * 169 / 512);
        for x in &f {
            assert!(
                x.values[11] == 126 || x.values[11] == 118,
                "base 120 + 6 ou − 2"
            );
            assert_eq!((x.values[0], x.values[4], x.values[13]), (5, 2, 1000));
            assert!(matches!(x.values[1], 1 | 11 | 21 | 31));
            assert!((200..220).contains(&x.values[2]));
        }
        // parede alta no meio da célula 2 (x 300…310): o fogo começa em 310
        let cells = row_cells(
            &[true; 8],
            &[(
                2,
                Rect {
                    x: 44,
                    y: 0,
                    w: 10,
                    h: 128,
                },
            )],
        );
        let f = floor_fires((400, 40), BARREL_RANGE, 1024, &cells, &mut rng);
        assert_eq!(f[0].values[10], 310 + 128);
        // célula 4 aberta embaixo: o trecho para nela e recomeça depois
        let cells = row_cells(&[true, true, true, true, false, true, true, true], &[]);
        let f = floor_fires((500, 40), BARREL_RANGE, 1024, &cells, &mut rng);
        let centres: Vec<i32> = f.iter().map(|f| f.values[10]).collect();
        assert_eq!(centres, vec![308 + 102, 640 + 26], "308…512 e 640…692");
    }

    #[test]
    fn floor_fire_grows_stays_and_dies() {
        let f = FloorFire {
            values: vec![5, 1, 3, 20, 2, 0, 0, 0, 0, 0, 300, 120, 0, 1000],
            mirror: false,
            behind: true,
        };
        let mut e = floor_fire_entity(&f, 0, 63);
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(5000, 0);
        let c = ctx(&j, &w, &on);
        let mut heights = vec![];
        while e.alive && heights.len() < 100 {
            tick(&mut e, &c, &mut Out::default());
            if let Behaviour::Fire(s) = &e.behaviour {
                heights.push(s.height);
            }
        }
        assert_eq!(
            &heights[..5],
            &[7, 12, 17, 20, 20],
            "+5 por tique até o máximo"
        );
        assert!(!e.alive, "minguou até apagar");
        let b = e.bounds().unwrap();
        assert_eq!(b.y + b.h, 120, "base na âncora");
    }
}
