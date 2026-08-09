use mpd::Song;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use std::time::Duration;

use crate::ui::command::CommandBar;
use crate::vim::search::SearchState;

pub struct StatusBar {
    pub now_playing: Option<Song>,
    pub elapsed: Option<Duration>,
    pub total: Option<Duration>,
    message: Result<String, String>,
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            now_playing: None,
            elapsed: None,
            total: None,
            message: Ok(String::new()),
        }
    }

    pub fn set_now_playing(&mut self, song: Option<Song>, elapsed: Option<Duration>) {
        self.now_playing = song;
        self.elapsed = elapsed;
    }

    pub fn set_message(&mut self, msg: Result<String, String>) {
        self.message = msg;
    }

    pub fn draw(
        &mut self,
        f: &mut Frame,
        area: Rect,
        search_state: Option<(&SearchState, usize)>,
        command_bar: Option<&CommandBar>,
    ) {
        let has_song = self.now_playing.is_some();

        if has_song {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(1), Constraint::Length(1)])
                .split(area);
            self.draw_progress(f, chunks[0]);
            self.draw_status(f, chunks[1], search_state, command_bar);
        } else {
            self.draw_status(f, area, search_state, command_bar);
        }
    }

    fn draw_progress(&mut self, f: &mut Frame, area: Rect) {
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

    fn draw_status(
        &self,
        f: &mut Frame,
        area: Rect,
        search_state: Option<(&SearchState, usize)>,
        command_bar: Option<&CommandBar>,
    ) {
        // command bar takes priority
        if let Some(cmd) = command_bar {
            let mut spans = vec![
                Span::styled(":", Style::default().fg(Color::White)),
                Span::styled(cmd.input.clone(), Style::default().fg(Color::White)),
            ];
            if let Some(ghost) = cmd.ghost_text() {
                // strip the part already typed from the ghost
                let typed_query = cmd.input.splitn(2, ' ').nth(1).unwrap_or("");
                let ghost_remainder = if ghost
                    .to_lowercase()
                    .starts_with(&typed_query.to_lowercase())
                {
                    &ghost[typed_query.len()..]
                } else {
                    &ghost
                };
                spans.push(Span::styled(
                    ghost_remainder.to_string(),
                    Style::default().fg(Color::DarkGray),
                ));
            }
            f.render_widget(Paragraph::new(Line::from(spans)), area);
            return;
        }

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
        match &self.message {
            Ok(msg) => (msg.clone(), Style::default().fg(Color::White)),
            Err(msg) => (msg.clone(), Style::default().fg(Color::Red)),
        };
        if let Some(song) = &self.now_playing {
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
