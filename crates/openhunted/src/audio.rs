// SPDX-License-Identifier: GPL-3.0-or-later
//! Efeitos sonoros (`docs/re/som.md`): decodificação dos `SX_%04d.WAV`, mixer de 16 canais
//! com volume (0–100), pan (−128 … 128) e troca pelo canal de menor prioridade, e a saída
//! pela SDL3. O mixer não depende da SDL (testável sem placa de som); a SDL só puxa as
//! amostras misturadas por um `AudioStream` com callback.

use std::collections::HashMap;
use std::sync::Arc;

use openhunted_engine::sound::{self, Cue};
use openhunted_formats::{ResourceArchive, Sound};

/// Canais de mixagem (`0x480f14 = 16`).
pub const CHANNELS: usize = 16;
/// Taxa da saída (estéreo, 16 bits); a SDL converte para a do dispositivo.
pub const OUT_RATE: u32 = 22_050;

/// Som decodificado: amostras de 16 bits com sinal, intercaladas se estéreo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pcm {
    pub rate: u32,
    /// 1 (mono) ou 2 (estéreo).
    pub channels: u16,
    pub samples: Vec<i16>,
}

impl Pcm {
    /// Converte um WAV PCM de 8 bits (sem sinal) ou 16 bits (com sinal, LE), mono ou estéreo.
    pub fn decode(s: &Sound) -> Option<Pcm> {
        if !(1..=2).contains(&s.channels) || s.sample_rate == 0 {
            return None;
        }
        let samples = match s.bits_per_sample {
            8 => s.data.iter().map(|&b| (i16::from(b) - 128) << 8).collect(),
            16 => s
                .data
                .chunks_exact(2)
                .map(|c| i16::from_le_bytes([c[0], c[1]]))
                .collect(),
            _ => return None,
        };
        Some(Pcm {
            rate: s.sample_rate,
            channels: s.channels,
            samples,
        })
    }

    /// Número de quadros (amostras por canal).
    pub fn frames(&self) -> usize {
        self.samples.len() / usize::from(self.channels)
    }
}

/// Ganhos esquerdo e direito em 1/256 a partir do volume (0–100, linear) e do pan
/// (−128 … 128, balanço: o centro toca cheio nos dois lados e cada lado cai até 0 na ponta
/// oposta).
pub fn gains(volume: u8, pan: i16) -> (i32, i32) {
    let v = i32::from(volume.min(100)) * 256 / 100;
    let pan = i32::from(pan.clamp(-128, 128));
    let left = v * (128 - pan.max(0)) / 128;
    let right = v * (128 + pan.min(0)) / 128;
    (left, right)
}

/// Um som tocando num canal.
#[derive(Debug, Clone)]
struct Voice {
    id: u16,
    priority: u16,
    pcm: Arc<Pcm>,
    gain: (i32, i32),
    /// Posição em quadros da fonte, ponto fixo 16.16.
    pos: u64,
    /// Passo por quadro de saída (taxa da fonte / taxa da saída), 16.16.
    step: u64,
    /// Recomeça do início ao acabar (sinal 2 de `0x43c93d`; música dos menus).
    looping: bool,
}

/// Mixer de até 16 sons.
#[derive(Debug, Clone)]
pub struct Mixer {
    rate: u32,
    voices: Vec<Voice>,
    /// Soma por quadro (E, D) antes da saturação, reaproveitada entre chamadas.
    acc: Vec<(i32, i32)>,
    /// Volume mestre em 1/256 (`SfxVolume`/`MusicVolume`, `0x44e040`).
    master: i32,
}

impl Mixer {
    pub fn new(rate: u32) -> Self {
        Self {
            rate: rate.max(1),
            voices: Vec::with_capacity(CHANNELS),
            // 16 K quadros: cabe qualquer pedido do SDL sem alocar na thread de áudio
            acc: Vec::with_capacity(16 * 1024),
            master: 256,
        }
    }

    /// Volume mestre (0–100) aplicado a todos os sons.
    pub fn set_master(&mut self, volume: u8) {
        self.master = i32::from(volume.min(100)) * 256 / 100;
    }

    /// Começa um som em laço (como `play`).
    pub fn play_looped(&mut self, id: u16, pcm: Arc<Pcm>, priority: u16, volume: u8) -> bool {
        if !self.play(id, pcm, priority, volume, 0) {
            return false;
        }
        if let Some(v) = self.voices.iter_mut().rev().find(|v| v.id == id) {
            v.looping = true;
        }
        true
    }

    /// Para todas as instâncias do som `id` (`0x44ded8`).
    pub fn stop(&mut self, id: u16) {
        self.voices.retain(|v| v.id != id);
    }

    /// O som `id` está tocando.
    pub fn is_playing(&self, id: u16) -> bool {
        self.voices.iter().any(|v| v.id == id)
    }

    /// Sons tocando agora.
    #[cfg(test)]
    pub fn active(&self) -> usize {
        self.voices.len()
    }

    /// Ids tocando agora, na ordem dos canais.
    #[cfg(test)]
    pub fn playing(&self) -> Vec<u16> {
        self.voices.iter().map(|v| v.id).collect()
    }

    /// Começa um som (`0x0043c93d`). Com os 16 canais ocupados, toma o canal de menor
    /// prioridade abaixo da nova (ou de mesma prioridade tocando o mesmo som); sem candidato,
    /// não toca e devolve `false`.
    pub fn play(&mut self, id: u16, pcm: Arc<Pcm>, priority: u16, volume: u8, pan: i16) -> bool {
        if pcm.frames() == 0 {
            return false;
        }
        let voice = Voice {
            id,
            priority,
            step: (u64::from(pcm.rate) << 16) / u64::from(self.rate),
            pcm,
            gain: gains(volume, pan),
            pos: 0,
            looping: false,
        };
        if self.voices.len() < CHANNELS {
            self.voices.push(voice);
            return true;
        }
        let victim = self
            .voices
            .iter()
            .enumerate()
            .filter(|(_, v)| v.priority < priority || (v.priority == priority && v.id == id))
            .min_by_key(|(_, v)| v.priority)
            .map(|(i, _)| i);
        match victim {
            Some(i) => {
                self.voices[i] = voice;
                true
            }
            None => false,
        }
    }

    /// Mistura os sons em `out` (estéreo intercalado, E/D), substituindo o conteúdo; satura
    /// em 16 bits. Os sons que acabam saem dos canais.
    pub fn mix(&mut self, out: &mut [i16]) {
        let acc = &mut self.acc;
        acc.clear();
        acc.resize(out.len() / 2, (0, 0));
        let master = self.master;
        for v in &mut self.voices {
            let ch = usize::from(v.pcm.channels);
            let total = v.pcm.frames() as u64;
            let (gl, gr) = ((v.gain.0 * master) >> 8, (v.gain.1 * master) >> 8);
            for a in acc.iter_mut() {
                let mut f = v.pos >> 16;
                if f >= total {
                    if !v.looping {
                        break;
                    }
                    v.pos -= total << 16;
                    f = v.pos >> 16;
                }
                let i = f as usize * ch;
                let l = i32::from(v.pcm.samples[i]);
                let r = i32::from(v.pcm.samples[i + ch - 1]);
                a.0 += (l * gl) >> 8;
                a.1 += (r * gr) >> 8;
                v.pos += v.step;
            }
        }
        self.voices
            .retain(|v| v.looping || (v.pos >> 16) < v.pcm.frames() as u64);
        for (o, a) in out.chunks_exact_mut(2).zip(acc.iter()) {
            o[0] = a.0.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16;
            o[1] = a.1.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16;
        }
        if out.len() % 2 == 1 {
            out[out.len() - 1] = 0;
        }
    }
}

/// Destino dos sons resolvidos (o mixer, a SDL ou um registro nos testes).
pub trait SoundOut {
    fn play(&mut self, id: u16, pcm: Arc<Pcm>, priority: u16, volume: u8, pan: i16);
    /// Som em laço (música dos menus).
    fn play_looped(&mut self, id: u16, pcm: Arc<Pcm>, priority: u16, volume: u8) {
        self.play(id, pcm, priority, volume, 0);
    }
    fn stop(&mut self, _id: u16) {}
    fn is_playing(&mut self, _id: u16) -> bool {
        false
    }
    /// Volume mestre (0–100).
    fn set_master(&mut self, _volume: u8) {}
}

impl SoundOut for Mixer {
    fn play(&mut self, id: u16, pcm: Arc<Pcm>, priority: u16, volume: u8, pan: i16) {
        Mixer::play(self, id, pcm, priority, volume, pan);
    }
    fn play_looped(&mut self, id: u16, pcm: Arc<Pcm>, priority: u16, volume: u8) {
        Mixer::play_looped(self, id, pcm, priority, volume);
    }
    fn stop(&mut self, id: u16) {
        Mixer::stop(self, id);
    }
    fn is_playing(&mut self, id: u16) -> bool {
        Mixer::is_playing(self, id)
    }
    fn set_master(&mut self, volume: u8) {
        Mixer::set_master(self, volume);
    }
}

/// Efeitos da cena: resolve os pedidos de um tique pela vista e manda ao destino, com os
/// sons decodificados uma vez e guardados por id.
pub struct Sfx {
    cache: HashMap<u16, Option<Arc<Pcm>>>,
    out: Box<dyn SoundOut>,
}

impl Sfx {
    pub fn new(out: Box<dyn SoundOut>) -> Self {
        Self {
            cache: HashMap::new(),
            out,
        }
    }

    /// Som `id` decodificado do arquivo (`None` se não é efeito, falta ou não decodifica;
    /// avisa uma vez).
    pub fn pcm(&mut self, archive: &ResourceArchive, id: u16) -> Option<Arc<Pcm>> {
        self.cache
            .entry(id)
            .or_insert_with(|| {
                let name = sound::resource_name(id)?;
                let pcm = archive
                    .get(&name)
                    .ok()
                    .and_then(|d| Sound::parse(d).ok())
                    .and_then(|s| Pcm::decode(&s));
                if pcm.is_none() {
                    eprintln!("aviso: som {name} ausente ou ilegível");
                }
                pcm.map(Arc::new)
            })
            .clone()
    }

    /// Volume mestre dos efeitos (0–100; abaixo de 10 o chamador já passa 0).
    pub fn set_volume(&mut self, volume: u8) {
        self.out.set_master(volume);
    }

    /// Toca os sons de um tique ouvidos pela vista centrada em `listener` na camada `layer`.
    pub fn tick(
        &mut self,
        archive: &ResourceArchive,
        cues: &[Cue],
        listener: (i32, i32),
        layer: i32,
    ) {
        for p in sound::resolve(cues, listener, layer) {
            if let Some(pcm) = self.pcm(archive, p.id) {
                self.out.play(p.id, pcm, p.priority, p.volume, p.pan);
            }
        }
    }
}

/// Saída pela SDL3: um `AudioStream` do dispositivo padrão que puxa as amostras do mixer.
pub struct SdlOut {
    stream: sdl3::audio::AudioStreamWithCallback<Feed>,
    _device: sdl3::audio::AudioDevice,
}

/// Lado do callback: o mixer e o buffer reaproveitado.
pub struct Feed {
    mixer: Mixer,
    buf: Vec<i16>,
}

impl sdl3::audio::AudioCallback<i16> for Feed {
    fn callback(&mut self, stream: &mut sdl3::audio::AudioStream, requested: i32) {
        // `requested` conta amostras (os dois canais): arredonda para quadros inteiros
        let n = (requested.max(0) as usize).div_ceil(2) * 2;
        self.buf.resize(n, 0); // já reservado em `open`: não aloca no callback
        self.mixer.mix(&mut self.buf);
        let _ = stream.put_data_i16(&self.buf);
    }
}

impl Drop for SdlOut {
    /// O sdl3 0.20 libera o callback antes de destruir o fluxo; pausar e tomar o lock espera
    /// um callback em curso e evita que a thread de áudio toque memória liberada.
    fn drop(&mut self) {
        let _ = self.stream.pause();
        drop(self.stream.lock());
    }
}

impl SdlOut {
    /// Abre o dispositivo de saída padrão (estéreo, 16 bits, 22 050 Hz) e começa a tocar.
    pub fn open(sdl: &sdl3::Sdl) -> Result<Self, String> {
        use sdl3::audio::{AudioFormat, AudioSpec};
        let audio = sdl.audio().map_err(|e| e.to_string())?;
        let device = audio.default_playback_device();
        let spec = AudioSpec::new(Some(OUT_RATE as i32), Some(2), Some(AudioFormat::s16_sys()));
        let feed = Feed {
            mixer: Mixer::new(OUT_RATE),
            buf: Vec::with_capacity(32 * 1024),
        };
        let stream = device
            .open_playback_stream_with_callback::<Feed, i16>(&spec, feed)
            .map_err(|e| e.to_string())?;
        stream.resume().map_err(|e| e.to_string())?;
        Ok(Self {
            stream,
            _device: device,
        })
    }
}

impl SoundOut for SdlOut {
    fn play(&mut self, id: u16, pcm: Arc<Pcm>, priority: u16, volume: u8, pan: i16) {
        if let Some(mut feed) = self.stream.lock() {
            feed.mixer.play(id, pcm, priority, volume, pan);
        }
    }
    fn play_looped(&mut self, id: u16, pcm: Arc<Pcm>, priority: u16, volume: u8) {
        if let Some(mut feed) = self.stream.lock() {
            feed.mixer.play_looped(id, pcm, priority, volume);
        }
    }
    fn stop(&mut self, id: u16) {
        if let Some(mut feed) = self.stream.lock() {
            feed.mixer.stop(id);
        }
    }
    fn is_playing(&mut self, id: u16) -> bool {
        self.stream
            .lock()
            .is_some_and(|feed| feed.mixer.is_playing(id))
    }
    fn set_master(&mut self, volume: u8) {
        if let Some(mut feed) = self.stream.lock() {
            feed.mixer.set_master(volume);
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    fn pcm(rate: u32, samples: &[i16]) -> Arc<Pcm> {
        Arc::new(Pcm {
            rate,
            channels: 1,
            samples: samples.to_vec(),
        })
    }

    fn wav(bits: u16, channels: u16, data: &[u8]) -> Sound {
        Sound {
            sample_rate: 11025,
            channels,
            bits_per_sample: bits,
            data: data.to_vec(),
        }
    }

    #[test]
    fn decodes_8_and_16_bit_pcm() {
        let p = Pcm::decode(&wav(8, 1, &[0x80, 0xff, 0x00, 0x81])).unwrap();
        assert_eq!(p.samples, vec![0, 32512, -32768, 256]);
        assert_eq!((p.rate, p.frames()), (11025, 4));
        let p = Pcm::decode(&wav(
            16,
            2,
            &[0x01, 0x00, 0xff, 0xff, 0x00, 0x80, 0xff, 0x7f],
        ))
        .unwrap();
        assert_eq!(p.samples, vec![1, -1, i16::MIN, i16::MAX]);
        assert_eq!(p.frames(), 2);
        assert!(Pcm::decode(&wav(24, 1, &[0; 3])).is_none());
        assert!(Pcm::decode(&wav(8, 3, &[0; 3])).is_none());
    }

    #[test]
    fn gains_follow_volume_and_pan() {
        assert_eq!(gains(100, 0), (256, 256));
        assert_eq!(gains(50, 0), (128, 128));
        assert_eq!(gains(100, 128), (0, 256), "pan 128: só a direita");
        assert_eq!(gains(100, -128), (256, 0));
        assert_eq!(gains(100, 64), (128, 256));
        assert_eq!(gains(0, 0), (0, 0));
        assert_eq!(gains(200, 0), (256, 256), "acima de 100 satura");
    }

    #[test]
    fn mixes_one_voice_at_full_volume_then_silence() {
        let mut m = Mixer::new(11025);
        assert!(m.play(1, pcm(11025, &[1000, -2000, 3000]), 0, 100, 0));
        let mut out = [7i16; 8];
        m.mix(&mut out);
        assert_eq!(out, [1000, 1000, -2000, -2000, 3000, 3000, 0, 0]);
        assert_eq!(m.active(), 0, "acabou e saiu do canal");
        m.mix(&mut out);
        assert_eq!(out, [0; 8]);
    }

    #[test]
    fn volume_pan_and_stereo_sources() {
        let mut m = Mixer::new(11025);
        m.play(1, pcm(11025, &[1000]), 0, 50, 128);
        let mut out = [0i16; 2];
        m.mix(&mut out);
        assert_eq!(out, [0, 500]);
        let st = Arc::new(Pcm {
            rate: 11025,
            channels: 2,
            samples: vec![100, -100],
        });
        m.play(2, st, 0, 100, 0);
        m.mix(&mut out);
        assert_eq!(out, [100, -100], "estéreo: cada lado com o seu canal");
    }

    #[test]
    fn mixing_sums_and_saturates() {
        let mut m = Mixer::new(11025);
        m.play(1, pcm(11025, &[20_000, -20_000, 100]), 0, 100, 0);
        m.play(2, pcm(11025, &[20_000, -20_000, 200]), 0, 100, 0);
        let mut out = [0i16; 6];
        m.mix(&mut out);
        assert_eq!(out, [i16::MAX, i16::MAX, i16::MIN, i16::MIN, 300, 300]);
    }

    #[test]
    fn resamples_to_the_output_rate() {
        let mut m = Mixer::new(22050);
        m.play(1, pcm(11025, &[10, 20]), 0, 100, 0);
        let mut out = [0i16; 10];
        m.mix(&mut out);
        assert_eq!(out, [10, 10, 10, 10, 20, 20, 20, 20, 0, 0]);
        // continua de onde parou entre chamadas
        m.play(2, pcm(44100, &[1, 2, 3, 4]), 0, 100, 0);
        let mut out = [0i16; 2];
        m.mix(&mut out);
        assert_eq!(out, [1, 1]);
        m.mix(&mut out);
        assert_eq!(out, [3, 3]);
    }

    #[test]
    fn sixteen_channels_and_eviction_by_priority() {
        let mut m = Mixer::new(11025);
        let long = pcm(11025, &[1; 100]);
        for i in 0..16 {
            assert!(m.play(100 + i, long.clone(), 5 + i, 100, 0));
        }
        assert_eq!(m.active(), CHANNELS);
        // prioridade menor ou igual (outro som): não toca
        assert!(!m.play(900, long.clone(), 5, 100, 0));
        assert!(!m.play(901, long.clone(), 0, 100, 0));
        // prioridade maior: toma o canal de menor prioridade (o 100, prioridade 5)
        assert!(m.play(902, long.clone(), 10, 100, 0));
        assert_eq!(m.active(), CHANNELS);
        assert_eq!(m.playing()[0], 902);
        // mesma prioridade, mesmo som: recomeça no canal dele
        assert!(m.play(101, long.clone(), 6, 100, 0));
        assert_eq!(m.playing()[1], 101);
        assert_eq!(m.playing().iter().filter(|&&i| i == 101).count(), 1);
        // som vazio não ocupa canal
        assert!(!Mixer::new(11025).play(1, pcm(11025, &[]), 0, 100, 0));
    }

    #[test]
    fn looping_voice_wraps_until_stopped() {
        let mut m = Mixer::new(11025);
        assert!(m.play_looped(3000, pcm(11025, &[1, 2, 3]), 5000, 100));
        let mut out = [0i16; 14];
        m.mix(&mut out);
        assert_eq!(out, [1, 1, 2, 2, 3, 3, 1, 1, 2, 2, 3, 3, 1, 1]);
        assert!(m.is_playing(3000), "o laço não sai do canal");
        m.mix(&mut out[..4]);
        assert_eq!(out[..4], [2, 2, 3, 3], "continua de onde parou");
        m.stop(3000);
        assert!(!m.is_playing(3000));
        assert_eq!(m.active(), 0);
    }

    #[test]
    fn master_volume_scales_everything() {
        let mut m = Mixer::new(11025);
        m.set_master(50);
        m.play(1, pcm(11025, &[1000]), 0, 100, 0);
        let mut out = [0i16; 2];
        m.mix(&mut out);
        assert_eq!(out, [500, 500]);
        m.set_master(0);
        m.play(1, pcm(11025, &[1000]), 0, 100, 0);
        m.mix(&mut out);
        assert_eq!(out, [0, 0]);
    }

    /// Um som tocado: id, prioridade, volume e pan.
    pub(crate) type Played = (u16, u16, u8, i16);

    /// Destino que só registra.
    #[derive(Default, Clone)]
    pub(crate) struct Log(pub std::rc::Rc<std::cell::RefCell<Vec<Played>>>);

    impl SoundOut for Log {
        fn play(&mut self, id: u16, _: Arc<Pcm>, priority: u16, volume: u8, pan: i16) {
            self.0.borrow_mut().push((id, priority, volume, pan));
        }
    }

    pub(crate) fn wav_bytes() -> Vec<u8> {
        let mut fmt = Vec::new();
        fmt.extend_from_slice(&1u16.to_le_bytes());
        fmt.extend_from_slice(&1u16.to_le_bytes());
        fmt.extend_from_slice(&11025u32.to_le_bytes());
        fmt.extend_from_slice(&11025u32.to_le_bytes());
        fmt.extend_from_slice(&1u16.to_le_bytes());
        fmt.extend_from_slice(&8u16.to_le_bytes());
        let mut body = b"WAVEfmt ".to_vec();
        body.extend_from_slice(&(fmt.len() as u32).to_le_bytes());
        body.extend(fmt);
        body.extend_from_slice(b"data");
        body.extend_from_slice(&2u32.to_le_bytes());
        body.extend_from_slice(&[0x90, 0x70]);
        let mut v = b"RIFF".to_vec();
        v.extend_from_slice(&(body.len() as u32).to_le_bytes());
        v.extend(body);
        v
    }

    #[test]
    fn sfx_resolves_cues_and_decodes_once() {
        use openhunted_formats::test_support::mkarchive;
        let wav = wav_bytes();
        let (map, vols) = mkarchive(&[("RESOURCE.001", &[("SX_3040.WAV", &wav)])]);
        let ar = ResourceArchive::from_map(&map, |_| Ok(vols[0].1.clone())).unwrap();
        let log = Log::default();
        let mut sfx = Sfx::new(Box::new(log.clone()));
        let cues = [
            Cue::at(3040, 9, (1384, 500), 0),
            Cue::ui(3040, 200),
            Cue::ui(3999, 0), // ausente: ignorado
            Cue::ui(4000, 0), // não é efeito
        ];
        sfx.tick(&ar, &cues, (1000, 500), 0);
        assert_eq!(
            *log.0.borrow(),
            vec![(3040, 200, 100, 0), (3040, 9, 50, 64)]
        );
        let a = sfx.pcm(&ar, 3040).unwrap();
        let b = sfx.pcm(&ar, 3040).unwrap();
        assert!(Arc::ptr_eq(&a, &b), "decodificado uma vez");
        assert_eq!(a.samples, vec![0x10 << 8, -(0x10 << 8)]);
    }

    /// Os efeitos do jogo decodificam (8 bits mono, 11 025 Hz no geral) e o tiro mistura.
    #[test]
    fn real_sounds_decode_and_mix() {
        let Some(dir) = std::env::var_os("OPENHUNTED_DATA") else {
            eprintln!("OPENHUNTED_DATA não definida: teste pulado");
            return;
        };
        let gd = openhunted_formats::open_data(std::path::Path::new(&dir)).unwrap();
        let ar = gd.resources().unwrap();
        let mut sfx = Sfx::new(Box::new(Log::default()));
        for id in [
            3040u16, 3046, 3020, 3021, 3080, 3090, 3135, 3772, 3774, 5057,
        ] {
            let p = sfx.pcm(&ar, id).unwrap_or_else(|| panic!("SX_{id}.WAV"));
            assert!(p.frames() > 100, "{id}");
        }
        let shot = sfx.pcm(&ar, 3040).unwrap();
        assert_eq!((shot.rate, shot.channels), (11025, 1));
        let mut m = Mixer::new(OUT_RATE);
        m.play(3040, shot.clone(), 9, 100, 0);
        let mut out = vec![0i16; 2 * OUT_RATE as usize];
        m.mix(&mut out);
        assert!(out.iter().any(|&s| s.abs() > 1000), "o tiro tem som");
        assert_eq!(m.active(), 0, "e acaba em menos de 1 s");
    }
}
