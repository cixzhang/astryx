//! Sound effects for shells: WAV files decoded to samples, and a mixer of a
//! few voices that an audio output pulls from on its own thread.
//!
//! The app registers sounds and asks for them with
//! [`Command::PlaySound`](astryx_core::Command::PlaySound); the shell
//! forwards both to a [`Controller`], and its audio callback calls
//! [`Mixer::fill`].

use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

use astryx_core::SoundId;

/// Voices that play at once. A new sound beyond them replaces the oldest.
pub const VOICES: usize = 4;

/// Why a WAV file could not be read.
#[derive(Debug, PartialEq, Eq)]
pub struct WavError(pub &'static str);

impl std::fmt::Display for WavError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WAV: {}", self.0)
    }
}

impl std::error::Error for WavError {}

/// Mono samples from -1 to 1 at `rate` per second.
#[derive(Clone, Debug, PartialEq)]
pub struct Samples {
    pub rate: u32,
    pub data: Vec<f32>,
}

/// Reads a RIFF WAV file of 8- or 16-bit PCM or 32-bit float, mixing its
/// channels down to one.
pub fn decode_wav(bytes: &[u8]) -> Result<Samples, WavError> {
    let u16_at = |b: &[u8], i: usize| u16::from_le_bytes([b[i], b[i + 1]]);
    let u32_at = |b: &[u8], i: usize| u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(WavError("not a RIFF WAVE file"));
    }
    let mut format: Option<(u16, u16, u32, u16)> = None;
    let mut at = 12;
    while at + 8 <= bytes.len() {
        let id = &bytes[at..at + 4];
        let len = u32_at(bytes, at + 4) as usize;
        let body = at + 8;
        let end = body.checked_add(len).filter(|&e| e <= bytes.len());
        let Some(end) = end else {
            return Err(WavError("chunk runs past the end"));
        };
        if id == b"fmt " {
            if len < 16 {
                return Err(WavError("short fmt chunk"));
            }
            format = Some((
                u16_at(bytes, body),
                u16_at(bytes, body + 2),
                u32_at(bytes, body + 4),
                u16_at(bytes, body + 14),
            ));
        } else if id == b"data" {
            let Some((tag, channels, rate, bits)) = format else {
                return Err(WavError("data before fmt"));
            };
            if channels == 0 || rate == 0 {
                return Err(WavError("no channels or rate"));
            }
            let raw = &bytes[body..end];
            let one: Vec<f32> = match (tag, bits) {
                (1, 16) => raw
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|c| f32::from(i16::from_le_bytes(*c)) / 32768.0)
                    .collect(),
                (1, 8) => raw
                    .iter()
                    .map(|&b| (f32::from(b) - 128.0) / 128.0)
                    .collect(),
                (3, 32) => raw
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|c| f32::from_le_bytes(*c))
                    .collect(),
                _ => return Err(WavError("unsupported sample format")),
            };
            let n = usize::from(channels);
            let data = one
                .chunks_exact(n)
                .map(|frame| frame.iter().sum::<f32>() / n as f32)
                .collect();
            return Ok(Samples { rate, data });
        }
        // Chunks are padded to an even length.
        at = end + (len & 1);
    }
    Err(WavError("no data chunk"))
}

/// `samples` at `rate`, by linear interpolation.
pub fn resample(samples: &Samples, rate: u32) -> Vec<f32> {
    if samples.rate == rate || samples.data.is_empty() {
        return samples.data.clone();
    }
    let step = samples.rate as f64 / rate as f64;
    let n = (samples.data.len() as f64 / step).floor() as usize;
    (0..n)
        .map(|i| {
            let pos = i as f64 * step;
            let j = pos as usize;
            let f = (pos - j as f64) as f32;
            let a = samples.data[j];
            let b = *samples.data.get(j + 1).unwrap_or(&a);
            a + (b - a) * f
        })
        .collect()
}

enum Message {
    Set(SoundId, Option<Arc<[f32]>>),
    Play(SoundId),
    Mute(bool),
}

/// The shell's side: registers sounds, plays them, mutes. Cheap to call
/// from the UI thread; the mixer picks the messages up on its next fill.
#[derive(Clone)]
pub struct Controller {
    tx: Sender<Message>,
    rate: u32,
}

impl Controller {
    /// Decodes `wav` for the output rate and hands it to the mixer; `None`
    /// frees the sound.
    pub fn set_sound(&self, id: SoundId, wav: Option<&[u8]>) -> Result<(), WavError> {
        let samples = match wav {
            Some(bytes) => Some(resample(&decode_wav(bytes)?, self.rate).into()),
            None => None,
        };
        let _ = self.tx.send(Message::Set(id, samples));
        Ok(())
    }

    pub fn play(&self, id: SoundId) {
        let _ = self.tx.send(Message::Play(id));
    }

    /// Silences everything, including sounds already playing.
    pub fn set_muted(&self, muted: bool) {
        let _ = self.tx.send(Message::Mute(muted));
    }
}

/// The audio thread's side: mixes the playing voices into output buffers.
pub struct Mixer {
    rx: Receiver<Message>,
    sounds: HashMap<SoundId, Arc<[f32]>>,
    /// Playing sounds and how far each is, oldest first.
    voices: Vec<(Arc<[f32]>, usize)>,
    muted: bool,
}

impl Mixer {
    /// A mixer for an output of `rate` samples per second, and its controller.
    pub fn new(rate: u32) -> (Mixer, Controller) {
        let (tx, rx) = channel();
        (
            Mixer {
                rx,
                sounds: HashMap::new(),
                voices: Vec::new(),
                muted: false,
            },
            Controller { tx, rate },
        )
    }

    /// Fills `out`, `channels` interleaved samples per frame, with the
    /// voices mixed. Silence when nothing plays or the mixer is muted.
    pub fn fill(&mut self, out: &mut [f32], channels: usize) {
        while let Ok(m) = self.rx.try_recv() {
            match m {
                Message::Set(id, Some(s)) => {
                    self.sounds.insert(id, s);
                }
                Message::Set(id, None) => {
                    self.sounds.remove(&id);
                }
                Message::Play(id) => {
                    if let (false, Some(s)) = (self.muted, self.sounds.get(&id)) {
                        if self.voices.len() == VOICES {
                            self.voices.remove(0);
                        }
                        self.voices.push((s.clone(), 0));
                    }
                }
                Message::Mute(m) => {
                    self.muted = m;
                    if m {
                        self.voices.clear();
                    }
                }
            }
        }
        out.fill(0.0);
        let channels = channels.max(1);
        for (samples, pos) in &mut self.voices {
            for frame in out.chunks_exact_mut(channels) {
                let Some(&v) = samples.get(*pos) else { break };
                *pos += 1;
                for s in frame {
                    *s += v;
                }
            }
        }
        self.voices.retain(|(s, pos)| *pos < s.len());
        for s in out {
            *s = s.clamp(-1.0, 1.0);
        }
    }

    /// Voices playing now.
    pub fn playing(&self) -> usize {
        self.voices.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 16-bit WAV file of `samples` at `rate` with `channels` channels.
    fn wav(rate: u32, channels: u16, samples: &[i16]) -> Vec<u8> {
        let data: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
        let mut out = Vec::new();
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
        out.extend_from_slice(b"WAVEfmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&channels.to_le_bytes());
        out.extend_from_slice(&rate.to_le_bytes());
        out.extend_from_slice(&(rate * u32::from(channels) * 2).to_le_bytes());
        out.extend_from_slice(&(channels * 2).to_le_bytes());
        out.extend_from_slice(&16u16.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&data);
        out
    }

    #[test]
    fn wav_decodes_and_mixes_down_to_mono() {
        let s = decode_wav(&wav(22050, 2, &[16384, -16384, 8192, 8192])).unwrap();
        assert_eq!(s.rate, 22050);
        assert_eq!(s.data, [0.0, 0.25]);
        assert!(decode_wav(b"RIFF....WAVX").is_err());
        assert!(decode_wav(&wav(22050, 1, &[])[..30]).is_err());
    }

    #[test]
    fn resampling_keeps_duration() {
        let s = Samples {
            rate: 22050,
            data: vec![0.5; 2205],
        };
        let r = resample(&s, 44100);
        assert_eq!(r.len(), 4410);
        assert!(r.iter().all(|&v| (v - 0.5).abs() < 1e-6));
    }

    #[test]
    fn voices_mix_end_and_mute() {
        let (mut mixer, control) = Mixer::new(8000);
        control
            .set_sound(SoundId(1), Some(&wav(8000, 1, &[8192; 4])))
            .unwrap();
        control.play(SoundId(1));
        control.play(SoundId(1));
        let mut out = [0.0f32; 12];
        mixer.fill(&mut out, 2);
        // Two voices of 0.25 on both channels for four frames, then silence.
        assert_eq!(&out[..8], &[0.5; 8]);
        assert_eq!(&out[8..], &[0.0; 4]);
        assert_eq!(mixer.playing(), 0);
        control.set_muted(true);
        control.play(SoundId(1));
        mixer.fill(&mut out, 2);
        assert_eq!(out, [0.0; 12]);
        for _ in 0..6 {
            control.set_muted(false);
            control.play(SoundId(1));
        }
        mixer.fill(&mut out[..2], 2);
        assert_eq!(mixer.playing(), VOICES);
    }
}
