// SPDX-License-Identifier: GPL-3.0-or-later
//! Grafite (tipo 6, `0x00414ea4`/`0x004150c0`; `docs/re/objetos-tutorial.md`): uma linha do
//! `GRAFFITI.RES` pintada com o `OGF.ANM` como fonte; sem colisão; visível com os gatilhos.

use super::{Ctx, Out};
use crate::entity::{Behaviour, Entity};
use openhunted_formats::Animation;

/// Altura padrão (`values[1]` 0 ou < 5).
pub const DEFAULT_HEIGHT: i32 = 18;
/// Quadro de referência da escala e da largura do espaço: o glifo `A`.
const REFERENCE: usize = b'A' as usize;

#[derive(Debug, Clone)]
pub struct GraffitiState {
    pub text: String,
    /// Glifos: deslocamento x (px da fase) e quadro do `OGF.ANM`.
    pub glyphs: Vec<(i32, usize)>,
    /// Escala (altura / altura do `A`).
    pub scale: (i32, i32),
    /// Largura total em px.
    pub width: i32,
    pub visible: bool,
}

/// Texto da linha `n` (contando de 1) do `GRAFFITI.RES` (id `17000 + n`).
pub fn line(lines: &[String], n: i32) -> String {
    usize::try_from(n)
        .ok()
        .and_then(|n| n.checked_sub(1))
        .and_then(|i| lines.get(i))
        .cloned()
        .unwrap_or_default()
}

impl GraffitiState {
    /// Dispõe o texto com a fonte: glifo = quadro `toupper(c)`; espaço = largura do `A`.
    pub fn new(text: &str, height: i32, font: &Animation) -> Self {
        let height = if height < 5 { DEFAULT_HEIGHT } else { height };
        let ref_h = font
            .frames
            .get(REFERENCE)
            .map_or(1, |f| i32::from(f.height).max(1));
        let ref_w = font.frames.get(REFERENCE).map_or(0, |f| i32::from(f.width));
        let sc = |v: i32| v * height / ref_h;
        let mut pen = 0;
        let mut glyphs = Vec::new();
        for c in text.chars().map(|c| c.to_ascii_uppercase()) {
            let code = c as usize;
            if c == ' ' {
                pen += sc(ref_w);
                continue;
            }
            let Some(f) = font.frames.get(code) else {
                continue;
            };
            glyphs.push((pen, code));
            pen += sc(i32::from(f.width));
        }
        Self {
            text: text.to_string(),
            glyphs,
            scale: (height, ref_h),
            width: pen,
            visible: true,
        }
    }
}

/// Visível com os gatilhos verdadeiros (ou sem gatilhos), escondido com falsos.
pub fn tick(e: &mut Entity, ctx: &Ctx, _out: &mut Out) {
    let visible = crate::triggers::satisfied(&e.triggers, ctx.on);
    if let Behaviour::Graffiti(st) = &mut e.behaviour {
        st.visible = visible;
    }
    e.on = visible;
}

#[cfg(test)]
mod tests {
    use super::*;
    use openhunted_formats::AnimFrame;

    fn font() -> Animation {
        let mut frames = vec![
            AnimFrame {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
                script_offset: 0,
                extra: 0,
            };
            256
        ];
        frames[b'A' as usize].width = 49;
        frames[b'A' as usize].height = 54;
        frames[b'1' as usize].width = 34;
        frames[b'C' as usize].width = 40;
        Animation {
            version: 1004,
            width: 0,
            height: 0,
            extra: [0; 3],
            table_a: vec![],
            frames,
            labels: vec![],
            script: vec![],
            bitmaps: vec![],
            palette: None,
        }
    }

    #[test]
    fn text_is_laid_out_glyph_by_glyph() {
        let lines: Vec<String> = ["RUN", "1", "car a"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(line(&lines, 2), "1");
        assert_eq!(line(&lines, 0), "");
        assert_eq!(line(&lines, 9), "");
        let g = GraffitiState::new(&line(&lines, 2), 18, &font());
        assert_eq!(g.width, 11, "o \"1\" da T_J_005 tem 11 px");
        assert_eq!(g.scale, (18, 54));
        let g = GraffitiState::new("car a", 0, &font());
        // C (40·18/54 = 13), A (16), R (largura 0 no teste), espaço (16), A
        assert_eq!(
            g.glyphs,
            vec![
                (0, b'C' as usize),
                (13, b'A' as usize),
                (29, b'R' as usize),
                (45, b'A' as usize)
            ]
        );
        assert_eq!(g.scale.0, DEFAULT_HEIGHT);
    }
}
