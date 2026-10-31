// SPDX-License-Identifier: GPL-3.0-or-later
//! Engine do OpenHunted: monta a cena de uma fase, avança as animações por tick e desenha
//! num framebuffer indexado. Não depende de SDL nem de relógio.

pub mod anim;
pub mod assets;
pub mod behaviour;
pub mod damage;
pub mod entity;
pub mod input;
pub mod jake;
pub mod objects;
pub mod physics;
pub mod render;
pub mod rng;
pub mod scene;
pub mod triggers;

pub use anim::{AnimPlayer, DrawOp, draw_ops};
pub use assets::Assets;
pub use behaviour::{Event, Outcome};
pub use entity::{Behaviour, Entity};
pub use jake::{Jake, JumpKind, State};
pub use physics::{Body, Rect, SUB, World, step};
pub use render::{
    Framebuffer, SCREEN_H, SCREEN_W, background_index, clamp_camera, display_palette, draw_text,
    measure_text,
};
pub use scene::{Layer, Marker, Scene, Sprite, TICK_MS};
