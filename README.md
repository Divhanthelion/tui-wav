# tui-wav

Play an audio file in the terminal and watch a live FFT spectrum.

```bash
cargo run --release -- /path/to/track.wav
```

Supports the usual suspects via [rodio](https://github.com/RustAudio/rodio) / Symphonia: **WAV, MP3, FLAC, OGG, M4A**.

## Controls

| Key | Action |
|-----|--------|
| `space` / `p` | Pause / resume |
| `q` / `Esc` | Quit |

## Install

```bash
cargo install --path .
tui-wav song.mp3
```

## How it works

1. Decode the file into PCM and start playback on the default output device.
2. Each frame, take a Hann-windowed slice around the playhead and run an FFT (`rustfft`).
3. Collapse bins into log-spaced bands, smooth + peak-hold, draw with [ratatui](https://ratatui.rs).

## License

MIT
