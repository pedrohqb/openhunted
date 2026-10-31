// SPDX-License-Identifier: GPL-3.0-or-later
//! Leitura dos formatos de dados do Hunter Hunted (Sierra/Dynamix, 1996).

mod error;
pub mod source;
#[cfg(feature = "test-support")]
pub mod test_support;

pub use error::{FormatError, Result};
pub use source::{DataSource, DirSource, IsoSource, normalize_path};
