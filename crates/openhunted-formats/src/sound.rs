// SPDX-License-Identifier: GPL-3.0-or-later
//! Sons RIFF `WAVE` PCM.

use crate::bytes::{u16_at, u32_at};
use crate::riff::{require, riff_chunks};
use crate::{FormatError, Result};

/// Som PCM.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sound {
    pub sample_rate: u32,
    pub channels: u16,
    pub bits_per_sample: u16,
    pub data: Vec<u8>,
}

impl Sound {
    /// Lê um RIFF `WAVE` PCM (formato 1).
    pub fn parse(data: &[u8]) -> Result<Self> {
        const WHAT: &str = "som WAV";
        let chunks = riff_chunks(data, b"WAVE", WHAT)?;
        let fmt = require(&chunks, b"fmt ", WHAT)?;
        let format = u16_at(fmt, 0, WHAT)?;
        if format != 1 {
            return Err(FormatError::Unsupported {
                what: WHAT,
                detail: format!("formato {format} (só PCM)"),
            });
        }
        Ok(Sound {
            channels: u16_at(fmt, 2, WHAT)?,
            sample_rate: u32_at(fmt, 4, WHAT)?,
            bits_per_sample: u16_at(fmt, 14, WHAT)?,
            data: require(&chunks, b"data", WHAT)?.to_vec(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synth_wav(format: u16) -> Vec<u8> {
        let mut fmt = Vec::new();
        fmt.extend_from_slice(&format.to_le_bytes());
        fmt.extend_from_slice(&1u16.to_le_bytes());
        fmt.extend_from_slice(&11025u32.to_le_bytes());
        fmt.extend_from_slice(&11025u32.to_le_bytes());
        fmt.extend_from_slice(&1u16.to_le_bytes());
        fmt.extend_from_slice(&8u16.to_le_bytes());
        let mut body = b"WAVEfmt ".to_vec();
        body.extend_from_slice(&(fmt.len() as u32).to_le_bytes());
        body.extend(fmt);
        body.extend_from_slice(b"data");
        body.extend_from_slice(&3u32.to_le_bytes());
        body.extend_from_slice(&[0x80, 0x81, 0x7f]);
        let mut v = b"RIFF".to_vec();
        v.extend_from_slice(&(body.len() as u32).to_le_bytes());
        v.extend(body);
        v
    }

    #[test]
    fn wav_parses_pcm() {
        let s = Sound::parse(&synth_wav(1)).unwrap();
        assert_eq!(
            (s.sample_rate, s.channels, s.bits_per_sample),
            (11025, 1, 8)
        );
        assert_eq!(s.data, vec![0x80, 0x81, 0x7f]);
    }

    #[test]
    fn wav_rejects_non_pcm() {
        assert!(matches!(
            Sound::parse(&synth_wav(2)),
            Err(crate::FormatError::Unsupported { .. })
        ));
    }

    #[test]
    fn truncated_wav_never_panics() {
        let d = synth_wav(1);
        for n in 0..d.len() {
            assert!(Sound::parse(&d[..n]).is_err());
        }
    }
}
