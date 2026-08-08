use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use mpd::Song;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Row, Table, TableState},
    Frame,
};

use crate::mpd::MpdClient;
use crate::vim::{
    edit::{handle_edit_key, EditAction, EditState, VimEditable},
    motion::{handle_motion_key, MotionAction, MotionState, VimNavigable},
    search::{handle_search_input, handle_search_normal, SearchState, VimSearchable},
};

pub struct PlaylistView {
    pub tracks: Vec<Song>,
    pub cursor: usize,
    pub motion: MotionState,
    pub edit: EditState,
    pub search: SearchState,
    pub search_matches: Vec<usize>,
    pub search_match_idx: usize,
    table_state: TableState,
}

impl PlaylistView {
    pub fn new() -> Self {
        Self {
            tracks: vec![],
            cursor: 0,
            motion: MotionState::new(),
            edit: EditState::new(),
            search: SearchState::new(),
            search_matches: vec![],
            search_match_idx: 0,
            table_state: TableState::default(),
        }
    }

    pub fn set_tracks(&mut self, tracks: Vec<Song>) {
        self.tracks = tracks;
        self.sync_cursor();
    }

    fn sync_cursor(&mut self) {
        self.table_state.select(Some(self.cursor));
    }

    pub fn handle_key(
        &mut self,
        key: KeyEvent,
        mpd: Option<&mut MpdClient>,
    ) -> Option<String> {
        if self.search.active {
            let mut search = self.search.clone();
            handle_search_input(self, &mut search, key);
            self.search = search;
            return None;
        }

        // c — clear playlist
        if let (KeyModifiers::NONE, KeyCode::Char('c')) = (key.modifiers, key.code) {
            self.tracks.clear();
            self.cursor = 0;
            self.sync_cursor();
            if let Some(client) = mpd {
                if let Err(e) = client.clear_queue() {
                    return Some(format!("Error: {}", e));
                }
            }
            return None;
        }

        let mut motion = self.motion.clone();
        if let Some(action) = handle_motion_key(self, &mut motion, key) {
            self.motion = motion;
            if action == MotionAction::Select {
                if let Some(client) = mpd {
                    if let Err(e) = client.play_at(self.cursor as u32) {
                        return Some(format!("Error: {}", e));
                    }
                }
            }
            return None;
        }

        let mut search = self.search.clone();
        if handle_search_normal(self, &mut search, key) {
            self.search = search;
            return None;
        }

        let mut edit = self.edit.clone();
        if let Some(action) = handle_edit_key(self, &mut edit, key) {
            self.edit = edit;
            if let Some(client) = mpd {
                let result = match action {
                    EditAction::DeleteCurrent => client.delete_at(self.cursor as u32),
                    EditAction::DeleteFromCursor => {
                        let pos = self.cursor as u32;
                        let count = self.tracks.len() as u32 + 1;
                        let mut err = Ok(());
                        for _ in pos..pos + count {
                            if let Err(e) = client.delete_at(pos) {
                                err = Err(e);
                                break;
                            }
                        }
                        err
                    }
                    EditAction::MoveUp => {
                        client.swap_tracks(self.cursor as u32, self.cursor as u32 + 1)
                    }
                    EditAction::MoveDown => {
                        client.swap_tracks(self.cursor as u32, self.cursor as u32 - 1)
                    }
                    EditAction::InsertAbove | EditAction::InsertBelow => Ok(()),
                };
                if let Err(e) = result {
                    return Some(format!("Error: {}", e));
                }
            }
            return None;
        }

        None
    }

    pub fn draw(&mut self, f: &mut Frame, area: Rect) {
        let rows: Vec<Row> = self
            .tracks
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let artist = s.artist.as_deref().unwrap_or("Unknown");
                let title = s.title.as_deref().unwrap_or("Unknown");
                let track = song_tag(s, "Track");
                let album = song_tag(s, "Album");
                let time = s
                    .duration
                    .map(|d| format!("{}:{:02}", d.as_secs() / 60, d.as_secs() % 60))
                    .unwrap_or_else(|| "-".into());
                let row = Row::new(vec![
                    artist.to_string(),
                    track.to_string(),
                    title.to_string(),
                    album.to_string(),
                    time,
                ]);
                if self.search_matches.contains(&i) {
                    row.style(Style::default().fg(Color::Yellow))
                } else {
                    row
                }
            })
            .collect();

        let table = Table::new(
            rows,
            [
                ratatui::layout::Constraint::Percentage(20),
                ratatui::layout::Constraint::Percentage(5),
                ratatui::layout::Constraint::Percentage(35),
                ratatui::layout::Constraint::Percentage(30),
                ratatui::layout::Constraint::Percentage(10),
            ],
        )
        .header(
            Row::new(vec!["Artist", "Track", "Title", "Album", "Time"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(Block::default().borders(Borders::ALL).title("Playlist"))
        .highlight_style(Style::default().bg(Color::DarkGray));

        f.render_stateful_widget(table, area, &mut self.table_state);
    }
}

impl VimNavigable for PlaylistView {
    fn move_down(&mut self, n: usize) {
        self.cursor = (self.cursor + n).min(self.tracks.len().saturating_sub(1));
        self.sync_cursor();
    }
    fn move_up(&mut self, n: usize) {
        self.cursor = self.cursor.saturating_sub(n);
        self.sync_cursor();
    }
    fn go_to_top(&mut self) {
        self.cursor = 0;
        self.sync_cursor();
    }
    fn go_to_bottom(&mut self) {
        self.cursor = self.tracks.len().saturating_sub(1);
        self.sync_cursor();
    }
    fn len(&self) -> usize {
        self.tracks.len()
    }
}

impl VimEditable for PlaylistView {
    fn insert_above(&mut self) {}
    fn insert_below(&mut self) {}
    fn delete_current(&mut self) {
        self.tracks.remove(self.cursor);
        self.cursor = self.cursor.min(self.tracks.len().saturating_sub(1));
        self.sync_cursor();
    }
    fn delete_from_cursor(&mut self) {
        self.tracks.truncate(self.cursor);
        self.cursor = self.cursor.min(self.tracks.len().saturating_sub(1));
        self.sync_cursor();
    }
    fn move_item_up(&mut self) {
        if self.cursor > 0 {
            self.tracks.swap(self.cursor, self.cursor - 1);
            self.cursor -= 1;
            self.sync_cursor();
        }
    }
    fn move_item_down(&mut self) {
        if self.cursor + 1 < self.tracks.len() {
            self.tracks.swap(self.cursor, self.cursor + 1);
            self.cursor += 1;
            self.sync_cursor();
        }
    }
}

impl VimSearchable for PlaylistView {
    fn search(&mut self, query: &str) {
        self.search_matches = self
            .tracks
            .iter()
            .enumerate()
            .filter(|(_, s)| {
                s.title.as_deref().unwrap_or("").to_lowercase().contains(&query.to_lowercase())
                    || s.artist.as_deref().unwrap_or("").to_lowercase().contains(&query.to_lowercase())
            })
            .map(|(i, _)| i)
            .collect();
        self.search_match_idx = 0;
        if let Some(&first) = self.search_matches.first() {
            self.cursor = first;
            self.sync_cursor();
        }
    }
    fn next_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        self.search_match_idx = (self.search_match_idx + 1) % self.search_matches.len();
        self.cursor = self.search_matches[self.search_match_idx];
        self.sync_cursor();
    }
    fn prev_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        self.search_match_idx = self
            .search_match_idx
            .checked_sub(1)
            .unwrap_or(self.search_matches.len() - 1);
        self.cursor = self.search_matches[self.search_match_idx];
        self.sync_cursor();
    }
    fn current_query(&self) -> &str {
        &self.search.query
    }
    fn match_count(&self) -> usize {
        self.search_matches.len()
    }
}

fn song_tag<'a>(song: &'a Song, key: &str) -> &'a str {
    song.tags
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))
        .map(|(_, v)| v.as_str())
        .unwrap_or("Unknown")
}
