//! Real `<audio>` decode+playback — gated behind the `audio` feature (off
//! by default; see `Cargo.toml`, and `docs/NATIVE_RENDERING_PLAN.md`'s
//! "Phase 3" writeup for why: a genuinely new OS-audio-device dependency
//! tree, unlike the animated-image work, which only extended the `image`
//! crate already pulled in). Uses `rodio` (symphonia-backed decoders) for
//! both decode and output — MP3/AAC/WAV/Vorbis/FLAC ("core priority" set),
//! not Opus (not in symphonia's built-in codec set; a real, separate
//! follow-up).
//!
//! Scoped to what's actually built: real decode, real playback triggered
//! by the `autoplay` attribute, and `pause()`/`is_playing()` for whatever
//! native/JS control gets wired up later. *Not* built: `currentTime`/seek,
//! `volume`/`muted` element properties, a visual `controls` widget (a real
//! UI subsystem of its own, same category as `custom-widget`'s form
//! controls), and JS-facing `HTMLMediaElement` bindings (this engine's JS
//! DOM binding surface — `getElementById`/`textContent`/`style`/
//! `classList`/`addEventListener`/`createElement`, see
//! `docs/NATIVE_RENDERING_PLAN.md`'s Phase 3 JS section — has no media
//! element methods at all yet; `.play()`/`.pause()` from script is a
//! separate, real addition on top of this).

use std::cell::RefCell;
use std::io::Cursor;
use std::sync::{Arc, Mutex};

// The single OS audio output stream for whichever thread first plays
// something. `rodio::OutputStream` is deliberately `!Send`/`!Sync` (many
// platform audio APIs require the stream to stay on the thread that
// opened it), so — unlike everything else this module hands out — it can
// never live on a `Node`/inside the document tree, which needs to stay
// `Send + Sync` for `parallel-construct`'s rayon traversal (the same
// constraint that shaped `AnimatedImageData::current_frame` using
// `AtomicUsize` instead of `Cell` in the animated-image work above).
// `OutputStreamHandle`, by contrast, *is* `Send + Sync + Clone`, so
// that's what actually gets threaded through to create a `Sink` per
// `<audio>` element — this thread-local only ever hands out clones of it,
// never the stream itself.
thread_local! {
    static OUTPUT_STREAM: RefCell<Option<(rodio::OutputStream, rodio::OutputStreamHandle)>> =
        const { RefCell::new(None) };
}

/// Whether a real audio output device is available on this thread — for
/// tests elsewhere (`document.rs`'s `audio_resource_tests`) to skip a
/// device-dependent assertion the same way this module's own tests do,
/// rather than duplicating `OUTPUT_STREAM`-probing logic.
pub fn output_stream_handle_for_tests() -> Option<()> {
    output_stream_handle().map(|_| ())
}

fn output_stream_handle() -> Option<rodio::OutputStreamHandle> {
    OUTPUT_STREAM.with(|cell| {
        let mut slot = cell.borrow_mut();
        if slot.is_none() {
            // No audio output device (headless CI, a sandboxed test
            // environment, etc.) is a real, expected case — not fatal.
            // Every caller treats a `None` handle as "silently don't
            // play," matching how a real `<audio>` element degrades (an
            // `error` event a page *might* listen for, not a crash).
            *slot = rodio::OutputStream::try_default().ok();
        }
        slot.as_ref().map(|(_, handle)| handle.clone())
    })
}

/// Playback state for one `<audio>` element. Unlike `OutputStream` above,
/// `rodio::Sink` *is* `Send + Sync`, so this can live directly on the
/// node's `SpecialElementData` — no thread-local indirection needed here.
pub struct AudioPlayer {
    sink: Mutex<Option<rodio::Sink>>,
}

impl std::fmt::Debug for AudioPlayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioPlayer").finish_non_exhaustive()
    }
}

impl Default for AudioPlayer {
    fn default() -> Self {
        Self { sink: Mutex::new(None) }
    }
}

impl AudioPlayer {
    /// Decode `bytes` and start playing immediately. Silently no-ops
    /// (doesn't return an `Err`/panic) if there's no audio output device,
    /// or `bytes` doesn't decode as a supported format — see this
    /// module's doc comment on why that's the deliberately-chosen failure
    /// mode here, matching a real browser's own graceful degradation.
    pub fn play(&self, bytes: Arc<Vec<u8>>) {
        let Some(handle) = output_stream_handle() else { return };
        // `Arc<Vec<u8>>` doesn't itself implement `AsRef<[u8]>` (only
        // `AsRef<Vec<u8>>`, which isn't what `Cursor`/`Read` need) — one
        // real copy out of the `Arc`, not avoidable without a custom
        // `Read` wrapper that isn't worth it for a decode that happens
        // once per `play()` call.
        let Ok(source) = rodio::Decoder::new(Cursor::new((*bytes).clone())) else { return };
        let Ok(sink) = rodio::Sink::try_new(&handle) else { return };
        sink.append(source);
        sink.play();
        *self.sink.lock().unwrap() = Some(sink);
    }

    pub fn pause(&self) {
        if let Some(sink) = self.sink.lock().unwrap().as_ref() {
            sink.pause();
        }
    }

    pub fn resume(&self) {
        if let Some(sink) = self.sink.lock().unwrap().as_ref() {
            sink.play();
        }
    }

    /// `false` once nothing has ever played, was explicitly paused, or
    /// the decoded audio finished (`rodio::Sink::empty()` — nothing left
    /// queued once a non-looping source finishes).
    pub fn is_playing(&self) -> bool {
        self.sink
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|s| !s.is_paused() && !s.empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal valid PCM16 mono WAV file, hand-built rather than pulling
    /// in an encoder crate just for this — the format is simple enough
    /// (a 44-byte RIFF/WAVE header followed by raw PCM samples) that
    /// writing it directly is less code than a dependency would be.
    /// `duration_ms` of near-silence (a low-amplitude sine-ish ramp, not
    /// literal zeros, so it's not trivially optimized away by anything
    /// downstream that might special-case true silence).
    fn make_test_wav(duration_ms: u32) -> Vec<u8> {
        let sample_rate: u32 = 8_000;
        let num_samples = (sample_rate * duration_ms / 1000) as usize;
        let samples: Vec<i16> = (0..num_samples).map(|i| ((i % 100) as i16) * 100).collect();

        let data_len = (samples.len() * 2) as u32;
        let mut wav = Vec::with_capacity(44 + data_len as usize);
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36 + data_len).to_le_bytes());
        wav.extend_from_slice(b"WAVE");
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk size
        wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
        wav.extend_from_slice(&1u16.to_le_bytes()); // mono
        wav.extend_from_slice(&sample_rate.to_le_bytes());
        wav.extend_from_slice(&(sample_rate * 2).to_le_bytes()); // byte rate
        wav.extend_from_slice(&2u16.to_le_bytes()); // block align
        wav.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data_len.to_le_bytes());
        for s in samples {
            wav.extend_from_slice(&s.to_le_bytes());
        }
        wav
    }

    #[test]
    fn a_fresh_player_reports_not_playing() {
        let player = AudioPlayer::default();
        assert!(!player.is_playing());
    }

    #[test]
    fn pause_and_resume_on_an_empty_player_do_not_panic() {
        let player = AudioPlayer::default();
        player.pause();
        player.resume();
        assert!(!player.is_playing());
    }

    #[test]
    fn play_with_undecodable_bytes_does_not_panic_and_stays_not_playing() {
        let player = AudioPlayer::default();
        player.play(Arc::new(b"not any kind of audio file".to_vec()));
        assert!(!player.is_playing());
    }

    #[test]
    fn play_with_a_real_wav_starts_playback_when_a_device_is_available() {
        let player = AudioPlayer::default();
        player.play(Arc::new(make_test_wav(200)));

        // A real audio output device isn't guaranteed to exist in every
        // environment this runs in (sandboxed/headless CI) — treated the
        // same way the existing font-metrics tests already skip when a
        // needed real resource (a usable system font) isn't present,
        // rather than failing on something this test can't control.
        if output_stream_handle().is_none() {
            eprintln!("skipping playing assertion: no audio output device available");
            return;
        }
        assert!(player.is_playing(), "a valid WAV should start playing when a device is available");

        player.pause();
        assert!(!player.is_playing());
        player.resume();
        assert!(player.is_playing());
    }
}
