// SPDX-License-Identifier: GPL-3.0-or-later
//! Leitura dos formatos de dados do Hunter Hunted (Sierra/Dynamix, 1996).

pub mod anim;
pub mod bitmap;
pub mod border;
mod bytes;
pub mod chunk;
pub mod compress;
mod error;
pub mod font;
pub mod game_data;
pub mod hls;
pub mod interface;
pub mod level;
pub mod level_body;
mod lists;
pub mod palette;
mod riff;
pub mod save;
pub mod ske;
pub mod sound;
pub mod source;
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
pub mod text;
pub mod volume;

pub use anim::{AnimFrame, AnimLabel, Animation, Instruction};
pub use bitmap::{Bitmap, Frame};
pub use border::{Border, BorderBox};
pub use chunk::{Chunk, chunks, find_chunk};
pub use compress::{decompress, unpack_block};
pub use error::{FormatError, Result};
pub use font::{Font, Glyph};
pub use game_data::{GameData, GameVersion, open_data};
pub use hls::LevelArchive;
pub use interface::{Control, Interface};
pub use level::{Level, MissionInfo, parse_mission_list};
pub use level_body::{Cell, Decoration, LevelObject, Prop, parse_body};
pub use palette::Palette;
pub use save::{Hero, HeroList, SaveGame};
pub use ske::{Skeleton, SkeletonEntry};
pub use sound::Sound;
pub use source::{DataSource, DirSource, IsoSource, normalize_path};
pub use text::parse_text_list;
pub use volume::{Entry, ResourceArchive};
