// SPDX-License-Identifier: GPL-3.0-or-later
//! Engine do OpenHunted: monta a cena de uma fase, avança as animações por tick e desenha
//! num framebuffer indexado. Não depende de SDL nem de relógio.

pub mod anim;
pub mod assets;
pub mod render;
pub mod scene;

pub use anim::{AnimPlayer, DrawOp, draw_ops};
pub use assets::Assets;
pub use render::{Framebuffer, SCREEN_H, SCREEN_W, background_index, clamp_camera};
pub use scene::{Layer, Marker, Scene, Sprite, TICK_MS};
