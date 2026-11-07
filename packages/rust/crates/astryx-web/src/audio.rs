//! Sound effects through WebAudio. Browsers let a page start audio only
//! after a person interacts with it, so the context opens on the first
//! sound, which an input asks for.

use std::collections::HashMap;

use astryx_core::SoundId;
use web_sys::{AudioBuffer, AudioContext};

#[derive(Default)]
pub struct Audio {
    context: Option<AudioContext>,
    /// Decoded sounds, made into buffers once a context exists.
    sounds: HashMap<SoundId, astryx_audio::Samples>,
    buffers: HashMap<SoundId, AudioBuffer>,
}

impl Audio {
    /// Adds or replaces a sound from WAV bytes; `None` frees it.
    pub fn set_sound(&mut self, id: SoundId, wav: Option<&[u8]>) {
        self.buffers.remove(&id);
        match wav.map(astryx_audio::decode_wav) {
            Some(Ok(samples)) => {
                self.sounds.insert(id, samples);
            }
            Some(Err(e)) => {
                web_sys::console::warn_1(&format!("astryx: sound {id:?}: {e}").into());
                self.sounds.remove(&id);
            }
            None => {
                self.sounds.remove(&id);
            }
        }
    }

    pub fn play(&mut self, id: SoundId) {
        if self.context.is_none() {
            self.context = AudioContext::new().ok();
        }
        let Some(context) = &self.context else { return };
        if !self.buffers.contains_key(&id) {
            let Some(s) = self.sounds.get(&id) else {
                return;
            };
            let Ok(buffer) = context.create_buffer(1, s.data.len().max(1) as u32, s.rate as f32)
            else {
                return;
            };
            let _ = buffer.copy_to_channel(&s.data, 0);
            self.buffers.insert(id, buffer);
        }
        let Ok(source) = context.create_buffer_source() else {
            return;
        };
        source.set_buffer(self.buffers.get(&id));
        if source
            .connect_with_audio_node(&context.destination())
            .is_ok()
        {
            let _ = source.start();
        }
    }
}
