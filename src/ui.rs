use std::io::{self, Stdout};
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, Paragraph};
use ratatui::{Frame, Terminal};

use crate::audio::TrackPlayer;
use crate::fft::Spectrum;

pub struct App {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    spectrum: Spectrum,
}

pub enum Action {
    Quit,
    TogglePause,
    None,
}

impl App {
    pub fn new() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self {
            terminal,
            spectrum: Spectrum::new(64),
        })
    }

    pub fn draw(&mut self, player: &TrackPlayer) -> io::Result<()> {
        let mono = &player.mono;
        let frame_idx = player.frame_index();
        let paused = player.is_paused();
        let finished = player.is_finished();
        let pos = player.position();
        let dur = player.duration;
        let title = player.title.clone();

        let width = self.terminal.size()?.width as usize;
        let bands = ((width.saturating_sub(4)) / 2).clamp(24, 96);
        self.spectrum.resize(bands);
        let (levels, peaks) = self.spectrum.analyze(mono, frame_idx.max(1));
        let levels = levels.to_vec();
        let peaks = peaks.to_vec();

        self.terminal.draw(|f| {
            render(f, &title, &levels, &peaks, pos, dur, paused, finished);
        })?;
        Ok(())
    }

    pub fn poll_action(&self) -> io::Result<Action> {
        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    return Ok(match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => Action::Quit,
                        KeyCode::Char(' ') | KeyCode::Char('p') => Action::TogglePause,
                        _ => Action::None,
                    });
                }
            }
        }
        Ok(Action::None)
    }
}

impl Drop for App {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

fn render(
    f: &mut Frame,
    title: &str,
    levels: &[f32],
    peaks: &[f32],
    pos: Duration,
    dur: Duration,
    paused: bool,
    finished: bool,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(f.area());

    let status = if finished {
        ("FINISHED", Color::DarkGray)
    } else if paused {
        ("PAUSED", Color::Yellow)
    } else {
        ("PLAYING", Color::Green)
    };

    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            " tui-wav ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(title, Style::default().fg(Color::White)),
        Span::raw("  "),
        Span::styled(
            status.0,
            Style::default()
                .fg(status.1)
                .add_modifier(Modifier::BOLD),
        ),
    ]))
    .block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(header, chunks[0]);

    render_spectrum(f, chunks[1], levels, peaks);

    let ratio = if dur.as_secs_f64() > 0.0 {
        (pos.as_secs_f64() / dur.as_secs_f64()).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("progress"))
        .gauge_style(Style::default().fg(Color::Cyan).bg(Color::Black))
        .ratio(ratio)
        .label(format!("{} / {}", fmt_time(pos), fmt_time(dur)));
    f.render_widget(gauge, chunks[2]);

    let help = Paragraph::new(Line::from(vec![
        Span::styled("space", Style::default().fg(Color::Cyan)),
        Span::raw(" pause   "),
        Span::styled("q", Style::default().fg(Color::Cyan)),
        Span::raw(" quit"),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(help, chunks[3]);
}

fn render_spectrum(f: &mut Frame, area: Rect, levels: &[f32], peaks: &[f32]) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("spectrum")
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 || levels.is_empty() {
        return;
    }

    let height = inner.height as usize;
    let mut lines: Vec<Line> = Vec::with_capacity(height);

    for row in 0..height {
        let mut spans = Vec::with_capacity(levels.len());
        // row 0 is the top of the chart
        let threshold = 1.0 - (row as f32 + 1.0) / height as f32;
        let half = 1.0 - (row as f32 + 0.5) / height as f32;

        for (i, &level) in levels.iter().enumerate() {
            let peak = peaks.get(i).copied().unwrap_or(0.0);
            let color = band_color(i, levels.len());

            let ch = if peak >= threshold && peak < threshold + 1.0 / height as f32 && peak > level
            {
                Span::styled("•", Style::default().fg(Color::White))
            } else if level >= threshold {
                Span::styled("█", Style::default().fg(color))
            } else if level >= half {
                Span::styled("▄", Style::default().fg(color))
            } else {
                Span::raw(" ")
            };
            spans.push(ch);
            // slight gap for readability on wide terminals
            if inner.width as usize >= levels.len() * 2 {
                spans.push(Span::raw(" "));
            }
        }
        lines.push(Line::from(spans));
    }

    f.render_widget(Paragraph::new(lines), inner);
}

fn band_color(index: usize, total: usize) -> Color {
    let t = index as f32 / total.max(1) as f32;
    // low → cyan, mid → green, high → yellow (classic analyzer, not purple haze)
    if t < 0.33 {
        Color::Cyan
    } else if t < 0.66 {
        Color::Green
    } else {
        Color::Yellow
    }
}

fn fmt_time(d: Duration) -> String {
    let secs = d.as_secs();
    format!("{:02}:{:02}", secs / 60, secs % 60)
}
