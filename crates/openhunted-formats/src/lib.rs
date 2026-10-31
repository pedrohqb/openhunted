// SPDX-License-Identifier: GPL-3.0-or-later
//! Leitura dos formatos de dados do Hunter Hunted (Sierra/Dynamix, 1996).

pub mod anim;
pub mod bitmap;
mod bytes;
pub mod chunk;
pub mod compress;
mod error;
pub mod game_data;
mod lists;
pub mod palette;
mod riff;
pub mod sound;
pub mod source;
#[cfg(feature = "test-support")]
pub mod test_support;
pub mod text;
pub mod volume;

pub use anim::{AnimFrame, AnimLabel, Animation, Instruction};
pub use bitmap::{Bitmap, Frame};
pub use chunk::{Chunk, chunks, find_chunk};
pub use compress::{decompress, unpack_block};
pub use error::{FormatError, Result};
pub use game_data::{GameData, GameVersion, open_data};
pub use palette::Palette;
pub use sound::Sound;
pub use source::{DataSource, DirSource, IsoSource, normalize_path};
pub use text::parse_text_list;
pub use volume::{Entry, ResourceArchive};
