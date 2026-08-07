use std::fs::File;
use std::num::NonZero;
use std::path::Path;
use std::time::Duration;

use rodio::buffer::SamplesBuffer;
use rodio::source::Source;
use rodio::{Decoder, DeviceSinkBuilder, Player};

/// Loaded track plus a live rodio player.
pub struct TrackPlayer {
    pub title: String,
    /// Mono PCM in [-1, 1], one sample per frame.
    pub mono: Vec<f32>,
    pub sample_rate: u32,
    pub duration: Duration,
    _sink: rodio::MixerDeviceSink,
    player: Player,
}

impl TrackPlayer {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref();
        let title = path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());

        let file = File::open(path).map_err(|e| format!("open {}: {e}", path.display()))?;
        let decoder =
            Decoder::try_from(file).map_err(|e| format!("decode {}: {e}", path.display()))?;

        let sample_rate = decoder.sample_rate().get();
        let channels = decoder.channels().get() as usize;
        let interleaved: Vec<f32> = decoder.collect();

        if interleaved.is_empty() {
            return Err("file contains no audio samples".into());
        }
        if channels == 0 {
            return Err("invalid channel count".into());
        }

        let mono = downmix_mono(&interleaved, channels);
        let duration = Duration::from_secs_f64(mono.len() as f64 / sample_rate as f64);

        let mut sink = DeviceSinkBuilder::open_default_sink()
            .map_err(|e| format!("audio output: {e}"))?;
        sink.log_on_drop(false);
        let player = Player::connect_new(sink.mixer());

        let channels_nz = NonZero::new(channels as u16).ok_or("invalid channel count")?;
        let rate_nz = NonZero::new(sample_rate).ok_or("invalid sample rate")?;
        player.append(SamplesBuffer::new(channels_nz, rate_nz, interleaved));

        Ok(Self {
            title,
            mono,
            sample_rate,
            duration,
            _sink: sink,
            player,
        })
    }

    pub fn toggle_pause(&self) {
        if self.player.is_paused() {
            self.player.play();
        } else {
            self.player.pause();
        }
    }

    pub fn is_paused(&self) -> bool {
        self.player.is_paused()
    }

    pub fn is_finished(&self) -> bool {
        self.player.empty()
    }

    pub fn position(&self) -> Duration {
        self.player.get_pos().min(self.duration)
    }

    /// Frame index into [`Self::mono`] for the current playhead.
    pub fn frame_index(&self) -> usize {
        let frames = (self.position().as_secs_f64() * self.sample_rate as f64) as usize;
        frames.min(self.mono.len().saturating_sub(1))
    }
}

fn downmix_mono(interleaved: &[f32], channels: usize) -> Vec<f32> {
    interleaved
        .chunks_exact(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}
