use rustfft::num_complex::Complex;
use rustfft::FftPlanner;

const FFT_SIZE: usize = 2048;

/// Real-time spectrum analyzer with Hann window, log bands, and peak hold.
pub struct Spectrum {
    fft_size: usize,
    planner_buf: Vec<Complex<f32>>,
    window: Vec<f32>,
    fft: std::sync::Arc<dyn rustfft::Fft<f32>>,
    bands: usize,
    smoothed: Vec<f32>,
    peaks: Vec<f32>,
}

impl Spectrum {
    pub fn new(bands: usize) -> Self {
        let bands = bands.max(8);
        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(FFT_SIZE);
        let window = hann(FFT_SIZE);

        Self {
            fft_size: FFT_SIZE,
            planner_buf: vec![Complex::new(0.0, 0.0); FFT_SIZE],
            window,
            fft,
            bands,
            smoothed: vec![0.0; bands],
            peaks: vec![0.0; bands],
        }
    }

    pub fn resize(&mut self, bands: usize) {
        let bands = bands.max(8);
        if bands != self.bands {
            self.bands = bands;
            self.smoothed.resize(bands, 0.0);
            self.peaks.resize(bands, 0.0);
        }
    }

    /// Analyze mono PCM ending at `end_frame` (exclusive playhead).
    pub fn analyze(&mut self, mono: &[f32], end_frame: usize) -> (&[f32], &[f32]) {
        let start = end_frame.saturating_sub(self.fft_size);
        let available = end_frame.saturating_sub(start).min(mono.len().saturating_sub(start));

        for i in 0..self.fft_size {
            let sample = if i < available {
                mono[start + i] * self.window[i]
            } else {
                0.0
            };
            self.planner_buf[i] = Complex::new(sample, 0.0);
        }

        self.fft.process(&mut self.planner_buf);

        let nyquist_bins = self.fft_size / 2;
        let mut mags = vec![0.0f32; nyquist_bins];
        let norm = 1.0 / (self.fft_size as f32).sqrt();
        for (i, mag) in mags.iter_mut().enumerate() {
            *mag = self.planner_buf[i].norm() * norm;
        }

        let raw = log_bands(&mags, self.bands);

        for i in 0..self.bands {
            // Attack fast, release slow — feels alive without flicker.
            let target = raw[i];
            if target > self.smoothed[i] {
                self.smoothed[i] = self.smoothed[i] * 0.35 + target * 0.65;
            } else {
                self.smoothed[i] = self.smoothed[i] * 0.85 + target * 0.15;
            }

            if self.smoothed[i] > self.peaks[i] {
                self.peaks[i] = self.smoothed[i];
            } else {
                self.peaks[i] *= 0.97;
            }
        }

        (&self.smoothed, &self.peaks)
    }
}

fn hann(n: usize) -> Vec<f32> {
    (0..n)
        .map(|i| {
            let x = std::f32::consts::PI * 2.0 * i as f32 / (n as f32 - 1.0);
            0.5 - 0.5 * x.cos()
        })
        .collect()
}

/// Collapse linear FFT bins into log-spaced visual bands, dB-scaled to ~[0, 1].
fn log_bands(mags: &[f32], bands: usize) -> Vec<f32> {
    let n = mags.len();
    if n == 0 || bands == 0 {
        return vec![0.0; bands];
    }

    // Skip DC; map from ~20Hz-equivalent bin through Nyquist on a log curve.
    let min_bin = 1.0f32;
    let max_bin = (n - 1) as f32;

    let mut out = vec![0.0f32; bands];
    for b in 0..bands {
        let t0 = b as f32 / bands as f32;
        let t1 = (b + 1) as f32 / bands as f32;
        let start = (min_bin * (max_bin / min_bin).powf(t0)).floor() as usize;
        let end = (min_bin * (max_bin / min_bin).powf(t1)).ceil() as usize;
        let end = end.max(start + 1).min(n);

        let mut peak = 0.0f32;
        for v in &mags[start..end] {
            if *v > peak {
                peak = *v;
            }
        }

        // dB compress so quiet content still moves the bars.
        let db = 20.0 * (peak + 1e-9).log10();
        out[b] = ((db + 60.0) / 60.0).clamp(0.0, 1.0);
    }
    out
}
