mod audio;
mod fft;
mod ui;

use std::env;
use std::process;

use audio::TrackPlayer;
use ui::{Action, App};

fn main() {
    if let Err(err) = run() {
        eprintln!("tui-wav: {err}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let path = args.next().ok_or_else(usage)?;

    let player = TrackPlayer::open(&path)?;
    let mut app = App::new().map_err(|e| format!("terminal: {e}"))?;

    loop {
        app.draw(&player).map_err(|e| format!("draw: {e}"))?;

        match app.poll_action().map_err(|e| format!("input: {e}"))? {
            Action::Quit => break,
            Action::TogglePause => player.toggle_pause(),
            Action::None => {}
        }

        if player.is_finished() {
            // Keep the final frame up until the user quits.
            // (Still redraw so resize stays correct.)
        }
    }

    Ok(())
}

fn usage() -> String {
    "usage: tui-wav <audio-file>\n\n\
     play a wav/mp3/flac/ogg/m4a file and render a live FFT spectrum in the terminal.\n\
     keys: space pause/resume · q quit"
        .into()
}
