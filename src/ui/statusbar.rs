use mpd::Song;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use std::time::Duration;

use crate::vim::search::SearchState;

pub struct StatusBar {
    pub now_playing: Option<Song>,
    pub elapsed: Option<Duration>,
    pub total: Option<Duration>,
    message: Option<String>,
    pub progress_bar_area: Option<Rect>,
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            now_playing: None,
            elapsed: None,
            total: None,
            message: None,
            progress_bar_area: None,
        }
    }

    pub fn set_now_playing(&mut self, song: Option<Song>, elapsed: Option<Duration>) {
        self.now_playing = song;
        self.elapsed = elapsed;
        self.message = None;
    }

    pub fn set_message(&mut self, msg: Option<String>) {
        self.message = msg;
    }

    pub fn draw(&mut self, f: &mut Frame, area: Rect, search_state: Option<(&SearchState, usize)>) {
        let has_song = self.now_playing.is_some();

        if has_song {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(1), Constraint::Length(1)])
                .split(area);

            self.draw_progress(f, chunks[0]);
            self.draw_status(f, chunks[1], search_state);
        } else {
            self.progress_bar_area = None;
            self.draw_status(f, area, search_state);
        }
    }

    fn draw_progress(&mut self, f: &mut Frame, area: Rect) {
        self.progress_bar_area = Some(area);

        let elapsed = self.elapsed.unwrap_or(Duration::ZERO);
        let total = self.total.unwrap_or(Duration::from_secs(1));

        let elapsed_str = format_duration(elapsed);
        let total_str = format_duration(total);
        let time_str = format!("[{}/{}]", elapsed_str, total_str);

        let time_width = time_str.len() as u16;
        let bar_width = area.width.saturating_sub(time_width + 1) as usize;

        let progress = if total.as_secs() > 0 {
            (elapsed.as_secs_f64() / total.as_secs_f64()).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let filled = (progress * bar_width as f64) as usize;
        let bar = if bar_width > 0 {
            let mut s = "=".repeat(filled.saturating_sub(1));
            if filled > 0 {
                s.push('>');
            }
            let remaining = bar_width.saturating_sub(s.len());
            s.push_str(&" ".repeat(remaining));
            s
        } else {
            String::new()
        };

        let text = format!("{} {}", bar, time_str);
        let paragraph = Paragraph::new(Line::from(Span::styled(
            text,
            Style::default().fg(Color::Green),
        )));
        f.render_widget(paragraph, area);
    }

    fn draw_status(&self, f: &mut Frame, area: Rect, search_state: Option<(&SearchState, usize)>) {
        let (text, style) = if let Some((search, match_count)) = search_state {
            if search.active {
                (
                    format!(
                        "/{}_ ({} match{})",
                        search.query,
                        match_count,
                        if match_count == 1 { "" } else { "es" }
                    ),
                    Style::default().fg(Color::Yellow),
                )
            } else {
                self.normal_content()
            }
        } else {
            self.normal_content()
        };

        let paragraph = Paragraph::new(Line::from(Span::styled(text, style)));
        f.render_widget(paragraph, area);
    }

    fn normal_content(&self) -> (String, Style) {
        if let Some(ref msg) = self.message {
            (msg.clone(), Style::default().fg(Color::Red))
        } else if let Some(ref song) = self.now_playing {
            (format_now_playing(song), Style::default().fg(Color::Cyan))
        } else {
            (
                "No track playing.".to_string(),
                Style::default().fg(Color::DarkGray),
            )
        }
    }
}

fn format_now_playing(song: &Song) -> String {
    let artist = song.artist.as_deref().unwrap_or("Unknown");
    let title = song.title.as_deref().unwrap_or("Unknown");
    let album = song_tag(song, "Album");
    let year = song_tag(song, "Date");
    format!("Playing: {} | {} {} | {}", artist, album, year, title)
}

fn format_duration(d: Duration) -> String {
    let secs = d.as_secs();
    format!("{}:{:02}", secs / 60, secs % 60)
}

fn song_tag<'a>(song: &'a Song, key: &str) -> &'a str {
    song.tags
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))
        .map(|(_, v)| v.as_str())
        .unwrap_or("Unknown")
}
