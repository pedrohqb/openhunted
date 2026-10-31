// SPDX-License-Identifier: GPL-3.0-or-later
//! Leitura dos formatos de dados do Hunter Hunted (Sierra/Dynamix, 1996).

mod bytes;
mod error;
pub mod game_data;
pub mod source;
#[cfg(feature = "test-support")]
pub mod test_support;
pub mod volume;

pub use error::{FormatError, Result};
pub use game_data::{GameData, GameVersion, open_data};
pub use source::{DataSource, DirSource, IsoSource, normalize_path};
pub use volume::{Entry, ResourceArchive};
