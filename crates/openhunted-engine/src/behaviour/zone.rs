// SPDX-License-Identifier: GPL-3.0-or-later
//! Zona de proximidade (tipo 73, `0x0040e14e`).

#[derive(Debug, Clone, Default)]
pub struct ZoneState {
    pub timer: i32,
}
