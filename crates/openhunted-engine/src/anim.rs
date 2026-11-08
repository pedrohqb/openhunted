// SPDX-License-Identifier: GPL-3.0-or-later
//! Execução mínima do roteiro das animações (opcodes 0, 1 e 2).

use openhunted_formats::Animation;

/// Desenho de um bitmap pedido por um quadro de animação (opcode 2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrawOp {
    pub bitmap: String,
    /// Quadro dentro do bitmap.
    pub frame: u16,
    /// Deslocamento em relação à origem da animação (quadro + argumento do opcode).
    pub x: i32,
    pub y: i32,
}

/// Instruções do quadro `frame`: do início do roteiro do quadro até o opcode 0 ou 1.
fn frame_script(anim: &Animation, frame: usize) -> &[openhunted_formats::Instruction] {
    let Some(f) = anim.frames.get(frame) else {
        return &[];
    };
    let Some(start) = anim.script.iter().position(|i| i.offset == f.script_offset) else {
        return &[];
    };
    let rest = &anim.script[start..];
    let end = rest
        .iter()
        .position(|i| i.op == 0 || i.op == 1)
        .map_or(rest.len(), |e| e + 1);
    &rest[..end]
}

/// Bitmaps desenhados pelo quadro `frame`. Opcodes além de 0, 1 e 2 são ignorados.
pub fn draw_ops(anim: &Animation, frame: usize) -> Vec<DrawOp> {
    let Some(f) = anim.frames.get(frame) else {
        return Vec::new();
    };
    frame_script(anim, frame)
        .iter()
        .filter(|i| i.op == 2 && i.args.len() >= 4)
        .filter_map(|i| {
            let tag = i.args[0] as u16;
            let name = anim.bitmaps.iter().find(|(id, _)| *id == tag)?.1.clone();
            Some(DrawOp {
                bitmap: name,
                frame: i.args[1] as u16,
                x: i32::from(f.x) + i32::from(i.args[2]),
                y: i32::from(f.y) + i32::from(i.args[3]),
            })
        })
        .collect()
}

/// Medida de referência da escala da explosão (`0x0040d20c`): o maior lado entre a maior
/// largura e a maior altura dos quadros do rótulo que começa em `start` — o quadro inicial
/// (`0x43585c` em 1:1) e os seguintes até antes do quadro em que começa outro rótulo ou do fim
/// da animação (`0x446fda` ≠ 0), cada um pelo registro do quadro (`0x44714b`). Fora da
/// animação, 0.
pub fn label_extent(anim: &Animation, start: usize) -> i32 {
    if start >= anim.frames.len() {
        return 0;
    }
    let starts_label = |f: usize| anim.labels.iter().any(|l| usize::from(l.frame) == f);
    let end = (start + 1..anim.frames.len())
        .find(|&f| starts_label(f))
        .unwrap_or(anim.frames.len());
    anim.frames[start..end]
        .iter()
        .map(|f| i32::from(f.width).max(i32::from(f.height)))
        .max()
        .unwrap_or(0)
}

/// Reprodução de uma sequência: do quadro inicial de um rótulo até o quadro que termina a
/// animação (opcode 1) ou até o início do rótulo seguinte, em laço.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimPlayer {
    start: usize,
    end: usize,
    frame: usize,
}

impl AnimPlayer {
    /// Começa no rótulo `id`; `None` se o rótulo não existe.
    pub fn at_label(anim: &Animation, id: u16) -> Option<Self> {
        let start = anim.labels.iter().find(|l| l.id == id)?.frame as usize;
        Some(Self::from_frame(anim, start))
    }

    /// Começa no quadro `start` (0 se não houver rótulo).
    pub fn from_frame(anim: &Animation, start: usize) -> Self {
        let next_label = anim
            .labels
            .iter()
            .map(|l| l.frame as usize)
            .filter(|&f| f > start)
            .min()
            .unwrap_or(anim.frames.len());
        let mut end = next_label.max(start + 1);
        for f in start..next_label {
            if frame_script(anim, f).iter().any(|i| i.op == 1) {
                end = f + 1;
                break;
            }
        }
        Self {
            start,
            end,
            frame: start,
        }
    }

    /// Quadro atual.
    pub fn frame(&self) -> usize {
        self.frame
    }

    /// Número de quadros da sequência.
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    /// Sequência vazia (nunca: tem ao menos um quadro).
    pub fn is_empty(&self) -> bool {
        self.end <= self.start
    }

    /// Está no último quadro da sequência.
    pub fn is_last(&self) -> bool {
        self.frame + 1 >= self.end
    }

    /// Vai ao quadro `i` da sequência (limitado ao último).
    pub fn seek(&mut self, i: usize) {
        self.frame = self.start + i.min(self.len().saturating_sub(1));
    }

    /// Avança um quadro sem voltar ao início; devolve `true` se já estava no fim.
    pub fn tick_once(&mut self) -> bool {
        if self.is_last() {
            return true;
        }
        self.frame += 1;
        false
    }

    /// Volta um quadro sem passar do início; devolve `true` se já estava no início.
    pub fn tick_back(&mut self) -> bool {
        if self.frame <= self.start {
            return true;
        }
        self.frame -= 1;
        false
    }

    /// Avança um quadro, voltando ao início da sequência no fim.
    pub fn tick(&mut self, _anim: &Animation) {
        self.frame = if self.frame + 1 >= self.end {
            self.start
        } else {
            self.frame + 1
        };
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use openhunted_formats::{AnimFrame, AnimLabel, Animation, Instruction};

    #[test]
    fn tick_once_stops_at_end() {
        let anim = Animation {
            version: 1004,
            width: 4,
            height: 4,
            extra: [0; 3],
            table_a: vec![],
            frames: vec![frame(0), frame(0), frame(0)],
            labels: vec![],
            script: vec![],
            bitmaps: vec![],
            palette: None,
        };
        let mut p = AnimPlayer::from_frame(&anim, 0);
        assert!(!p.tick_once());
        assert!(!p.tick_once());
        assert!(p.is_last());
        assert!(p.tick_once());
        assert_eq!(p.frame(), 2);
        assert!(!p.tick_back());
        assert!(!p.tick_back());
        assert!(p.tick_back());
        assert_eq!(p.frame(), 0);
    }

    fn frame(off: u16) -> AnimFrame {
        AnimFrame {
            x: -1,
            y: 2,
            width: 4,
            height: 4,
            script_offset: off,
            extra: 0,
        }
    }
    fn ins(offset: u16, op: u16, args: &[i16]) -> Instruction {
        Instruction {
            offset,
            op,
            args: args.to_vec(),
        }
    }

    /// Rótulo 1: quadros 0–1 (o 1 termina com opcode 1). Rótulo 2: quadro 2.
    pub(crate) fn anim() -> Animation {
        Animation {
            version: 1004,
            width: 4,
            height: 4,
            extra: [0; 3],
            table_a: vec![],
            frames: vec![frame(0), frame(7), frame(15)],
            labels: vec![
                AnimLabel {
                    frame: 0,
                    id: 1,
                    name: None,
                },
                AnimLabel {
                    frame: 2,
                    id: 2,
                    name: None,
                },
            ],
            script: vec![
                ins(0, 2, &[7, 0, 3, 4, 0]),
                ins(6, 0, &[]),
                ins(7, 2, &[7, 1, 0, 0, 0]),
                ins(13, 0, &[]),
                ins(14, 1, &[]),
                ins(15, 2, &[7, 2, 0, 0, 0]),
                ins(21, 0, &[]),
            ],
            bitmaps: vec![(7, "x.bmp".into())],
            palette: None,
        }
    }

    #[test]
    fn draw_ops_follow_frame_script() {
        let a = anim();
        let ops = draw_ops(&a, 0);
        assert_eq!(
            ops,
            vec![DrawOp {
                bitmap: "x.bmp".into(),
                frame: 0,
                x: 2,
                y: 6
            }]
        );
    }

    /// A medida vai do quadro `start` até antes do próximo rótulo (ou o fim), pelo maior lado
    /// entre a maior largura e a maior altura: o primeiro quadro de 1 px não manda.
    #[test]
    fn label_extent_takes_the_largest_side_of_the_label() {
        let sized = |w: i16, h: i16| AnimFrame {
            width: w,
            height: h,
            ..frame(0)
        };
        let mut a = anim();
        a.frames = vec![sized(1, 1), sized(5, 3), sized(2, 9), sized(100, 40)];
        a.labels[1].frame = 3;
        assert_eq!(label_extent(&a, 0), 9);
        assert_eq!(label_extent(&a, 1), 9);
        assert_eq!(label_extent(&a, 3), 100);
        assert_eq!(label_extent(&a, 7), 0);
    }

    #[test]
    fn player_loops_within_label_sequence() {
        let a = anim();
        let mut p = AnimPlayer::at_label(&a, 1).unwrap();
        assert_eq!(p.frame(), 0);
        p.tick(&a);
        assert_eq!(p.frame(), 1);
        p.tick(&a); // o quadro 1 termina a animação: volta ao início do rótulo
        assert_eq!(p.frame(), 0);
        let mut q = AnimPlayer::at_label(&a, 2).unwrap();
        q.tick(&a); // sequência de um quadro só
        assert_eq!(q.frame(), 2);
        assert!(AnimPlayer::at_label(&a, 99).is_none());
    }
}
