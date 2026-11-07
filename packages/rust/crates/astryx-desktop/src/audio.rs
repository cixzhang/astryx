//! Sound effects through the default output device.

use astryx_audio::{Controller, Mixer};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleFormat;

/// The open output stream and the controller that feeds it. Dropping it
/// stops the sound.
pub struct Audio {
    _stream: cpal::Stream,
    pub control: Controller,
}

impl Audio {
    pub fn open() -> Result<Audio, String> {
        let device = cpal::default_host()
            .default_output_device()
            .ok_or("no output device")?;
        let config = device.default_output_config().map_err(|e| e.to_string())?;
        let channels = usize::from(config.channels());
        let (mut mixer, control) = Mixer::new(config.sample_rate().0);
        let err = |e| eprintln!("astryx-desktop: audio: {e}");
        let stream = match config.sample_format() {
            SampleFormat::F32 => device.build_output_stream(
                &config.into(),
                move |out: &mut [f32], _| mixer.fill(out, channels),
                err,
                None,
            ),
            SampleFormat::I16 => {
                let mut scratch = Vec::new();
                device.build_output_stream(
                    &config.into(),
                    move |out: &mut [i16], _| {
                        scratch.resize(out.len(), 0.0);
                        mixer.fill(&mut scratch, channels);
                        for (o, s) in out.iter_mut().zip(&scratch) {
                            *o = (s * 32767.0) as i16;
                        }
                    },
                    err,
                    None,
                )
            }
            other => return Err(format!("unsupported sample format {other:?}")),
        }
        .map_err(|e| e.to_string())?;
        stream.play().map_err(|e| e.to_string())?;
        Ok(Audio {
            _stream: stream,
            control,
        })
    }
}
