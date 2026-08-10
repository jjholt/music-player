use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use mpd::Song;
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{
        Block, Borders, Clear, List, ListItem, Row, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Table, TableState,
    },
};

use crate::ui::autocomplete::Autocomplete;
use crate::vim::{
    edit::{EditAction, EditState, VimEditable, handle_edit_key},
    motion::{MotionAction, MotionState, VimNavigable, handle_motion_key},
    search::{SearchState, VimSearchable, handle_search_input, handle_search_normal},
};
use crate::{mpd::MpdClient, ui::App};

#[derive(Debug, Clone, PartialEq)]
pub enum InsertPosition {
    Above,
    Below,
}

#[derive(Debug, Clone)]
pub struct InsertState {
    pub position: InsertPosition,
    pub input: String,
    pub autocomplete: Autocomplete,
}

pub enum PlaylistKeyResult {
    None,
    Status(String),
    AppendAndPlay(Vec<Song>),
    Append(Vec<Song>),
}

pub struct PlaylistView {
    pub tracks: Vec<Song>,
    pub cursor: usize,
    pub motion: MotionState,
    pub edit: EditState,
    pub search: SearchState,
    pub search_matches: Vec<usize>,
    pub search_match_idx: usize,
    pub insert_state: Option<InsertState>,
    pub current_song_id: Option<u32>,
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
            insert_state: None,
            table_state: TableState::default(),
            current_song_id: None,
        }
    }

    pub fn set_current_song_id(&mut self, id: Option<u32>) {
        self.current_song_id = id;
    }

    pub fn set_tracks(&mut self, tracks: Vec<Song>) {
        self.tracks = tracks;
        self.sync_cursor();
    }

    fn sync_cursor(&mut self) {
        self.table_state.select(Some(self.cursor));
    }

    pub fn actions(
        &mut self,
        key: KeyEvent,
        mpd: &mut MpdClient,
        all_songs: &[Song],
    ) -> PlaylistKeyResult {
        // insert mode
        if let Some(ref mut insert) = self.insert_state {
            match (key.modifiers, key.code) {
                (KeyModifiers::NONE, KeyCode::Esc) => {
                    self.insert_state = None;
                    return PlaylistKeyResult::None;
                }
                (KeyModifiers::NONE, KeyCode::Enter) => {
                    if insert.autocomplete.has_matches() {
                        if let Some(song) = insert.autocomplete.current().cloned() {
                            // let insert_pos = match insert.position {
                            //     InsertPosition::Above => self.cursor,
                            //     InsertPosition::Below => self.cursor + 1,
                            // };
                            self.insert_state = None;
                            return PlaylistKeyResult::AppendAndPlay(vec![song]);
                        }
                    }
                    return PlaylistKeyResult::Status("Invalid song.".into());
                }
                (KeyModifiers::NONE, KeyCode::Tab) => {
                    insert.autocomplete.next();
                    return PlaylistKeyResult::None;
                }
                (KeyModifiers::SHIFT, KeyCode::BackTab) => {
                    insert.autocomplete.prev();
                    return PlaylistKeyResult::None;
                }
                (KeyModifiers::NONE, KeyCode::Backspace) => {
                    insert.input.pop();
                    let q = insert.input.clone();
                    insert.autocomplete.update(&q, all_songs);
                    return PlaylistKeyResult::None;
                }
                (KeyModifiers::NONE, KeyCode::Char(c)) => {
                    insert.input.push(c);
                    let q = insert.input.clone();
                    insert.autocomplete.update(&q, all_songs);
                    return PlaylistKeyResult::None;
                }
                _ => return PlaylistKeyResult::None,
            }
        }

        if self.search.active {
            let mut search = self.search.clone();
            handle_search_input(self, &mut search, key);
            self.search = search;
            return PlaylistKeyResult::None;
        }

        if let (KeyModifiers::NONE, KeyCode::Char('c')) = (key.modifiers, key.code) {
            self.tracks.clear();
            self.cursor = 0;
            self.sync_cursor();
            if let Err(e) = mpd.clear_queue() {
                return PlaylistKeyResult::Status(format!("Error: {}", e));
            }
            return PlaylistKeyResult::None;
        }

        let mut motion = self.motion.clone();
        if let Some(action) = handle_motion_key(self, &mut motion, key) {
            self.motion = motion;
            if action == MotionAction::Select {
                if let Err(e) = mpd.play_at(self.cursor as u32) {
                    return PlaylistKeyResult::Status(format!("Error: {}", e));
                }
            }
            return PlaylistKeyResult::None;
        }

        let mut search = self.search.clone();
        if handle_search_normal(self, &mut search, key) {
            self.search = search;
            return PlaylistKeyResult::None;
        }

        let mut edit = self.edit.clone();
        if let Some(action) = handle_edit_key(self, &mut edit, key) {
            self.edit = edit;
            match action {
                EditAction::InsertAbove => {
                    self.insert_state = Some(InsertState {
                        position: InsertPosition::Above,
                        input: String::new(),
                        autocomplete: Autocomplete::new(),
                    });
                    return PlaylistKeyResult::None;
                }
                EditAction::InsertBelow => {
                    self.insert_state = Some(InsertState {
                        position: InsertPosition::Below,
                        input: String::new(),
                        autocomplete: Autocomplete::new(),
                    });
                    return PlaylistKeyResult::None;
                }
                _ => {}
            }
            let result = match action {
                EditAction::DeleteCurrent => mpd.delete_at(self.cursor as u32),
                EditAction::DeleteFromCursor => {
                    let pos = self.cursor as u32;
                    let count = self.tracks.len() as u32 + 1;
                    let mut err = Ok(());
                    for _ in pos..pos + count {
                        if let Err(e) = mpd.delete_at(pos) {
                            err = Err(e);
                            break;
                        }
                    }
                    err
                }
                EditAction::MoveUp => mpd.swap_tracks(self.cursor as u32, self.cursor as u32 + 1),
                EditAction::MoveDown => mpd.swap_tracks(self.cursor as u32, self.cursor as u32 - 1),
                EditAction::InsertAbove | EditAction::InsertBelow => Ok(()),
            };
            if let Err(e) = result {
                return PlaylistKeyResult::Status(format!("Error: {}", e));
            }
            return PlaylistKeyResult::None;
        }

        PlaylistKeyResult::None
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
                let is_playing = s.place.map(|p| p.id.0) == self.current_song_id
                    && self.current_song_id.is_some();
                let row = Row::new(vec![
                    artist.to_string(),
                    track.to_string(),
                    title.to_string(),
                    album.to_string(),
                    time,
                ]);

                if is_playing {
                    row.style(Style::default().bold())
                } else if self.search_matches.contains(&i) {
                    row.style(Style::default().fg(Color::Yellow))
                } else {
                    row
                }
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Percentage(20),
                Constraint::Percentage(5),
                Constraint::Percentage(35),
                Constraint::Percentage(30),
                Constraint::Percentage(10),
            ],
        )
        .header(
            Row::new(vec!["Artist", "Track", "Title", "Album", "Time"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(Block::default().borders(Borders::ALL).title("Playlist"))
        .row_highlight_style(Style::default().bg(Color::Rgb(40, 44, 52)));

        f.render_stateful_widget(table, area, &mut self.table_state);

        // draw autocomplete popup if in insert mode
        if let Some(ref insert) = self.insert_state {
            if !insert.input.is_empty() && insert.autocomplete.has_matches() {
                self.draw_autocomplete_popup(f, area, insert);
            }
        }
    }

    fn draw_autocomplete_popup(&self, f: &mut Frame, area: Rect, insert: &InsertState) {
        let visible = insert
            .autocomplete
            .visible_count
            .min(insert.autocomplete.matches.len());
        let popup_height = visible as u16 + 2; // +2 for border
        let popup_width = (area.width / 2).max(40);
        let popup_x = area.x + 2;

        // anchor above the cursor row
        let cursor_row = area.y + self.cursor as u16 + 1; // +1 for header
        let popup_y = cursor_row.saturating_sub(popup_height);

        let popup_area = Rect {
            x: popup_x,
            y: popup_y,
            width: popup_width,
            height: popup_height,
        };

        f.render_widget(Clear, popup_area);

        let items: Vec<ListItem> = insert
            .autocomplete
            .matches
            .iter()
            .skip(insert.autocomplete.scroll_offset)
            .take(visible)
            .enumerate()
            .map(|(i, s)| {
                let text = crate::ui::autocomplete::format_song_suggestion(s);
                let actual_idx = i + insert.autocomplete.scroll_offset;
                if actual_idx == insert.autocomplete.selected {
                    ListItem::new(text).style(Style::default().bg(Color::Rgb(40, 44, 52)))
                } else {
                    ListItem::new(text)
                }
            })
            .collect();

        let has_more = insert.autocomplete.matches.len() > visible;

        let list =
            List::new(items).block(Block::default().borders(Borders::ALL).title(if has_more {
                format!(
                    "Suggestions ({}/{})",
                    insert.autocomplete.selected + 1,
                    insert.autocomplete.matches.len()
                )
            } else {
                "Suggestions".into()
            }));

        f.render_widget(list, popup_area);

        if has_more {
            let mut scrollbar_state = ScrollbarState::new(insert.autocomplete.matches.len())
                .position(insert.autocomplete.scroll_offset);
            f.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight),
                popup_area,
                &mut scrollbar_state,
            );
        }
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
                s.title
                    .as_deref()
                    .unwrap_or("")
                    .to_lowercase()
                    .contains(&query.to_lowercase())
                    || s.artist
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&query.to_lowercase())
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

pub fn handle_key(app: &mut App<MpdClient>, key: KeyEvent) {
    use crossterm::event::{KeyCode, KeyModifiers};

    if let (KeyModifiers::NONE, KeyCode::Char(':')) = (key.modifiers, key.code) {
        app.command_bar.open();
        return;
    }

    let all_songs = app.library.all_songs.clone();
    let result = app.playlist.actions(key, &mut app.mpd, &all_songs);
    match result {
        PlaylistKeyResult::Status(msg) => {
            app.status_bar.set_message(Ok(msg));
        }
        PlaylistKeyResult::Append(songs) => {
            app.append(&songs);
        }
        PlaylistKeyResult::AppendAndPlay(songs) => {
            app.append_and_play(&songs);
        }
        PlaylistKeyResult::None => {}
    }
}
