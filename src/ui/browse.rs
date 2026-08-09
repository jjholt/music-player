use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use mpd::Song;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

use crate::{
    mpd::MpdClient,
    ui::App,
    vim::{
        motion::{MotionAction, MotionState, VimNavigable, handle_motion_key},
        search::{SearchState, VimSearchable, handle_search_input, handle_search_normal},
    },
};

#[derive(Debug, Clone, PartialEq)]
pub enum BrowseField {
    Any,
    Artist,
    Album,
    Song,
    Genre,
    Filename,
    Date,
}

impl BrowseField {
    pub fn label(&self) -> &'static str {
        match self {
            BrowseField::Any => "Any",
            BrowseField::Artist => "Artist",
            BrowseField::Album => "Album",
            BrowseField::Song => "Song",
            BrowseField::Genre => "Genre",
            BrowseField::Filename => "Filename",
            BrowseField::Date => "Date",
        }
    }

    pub fn all() -> &'static [BrowseField] {
        &[
            BrowseField::Any,
            BrowseField::Artist,
            BrowseField::Album,
            BrowseField::Song,
            BrowseField::Genre,
            BrowseField::Filename,
            BrowseField::Date,
        ]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BrowseFocus {
    Field(usize),
    Search,
    Reset,
    Results,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BrowseEditMode {
    Normal,
    Editing,
}

pub struct BrowseView {
    pub fields: Vec<(BrowseField, String)>,
    pub focus: BrowseFocus,
    pub edit_mode: BrowseEditMode,
    pub results: Vec<Song>,
    pub search: SearchState,
    pub search_matches: Vec<usize>,
    pub search_match_idx: usize,
    pub motion: MotionState,
    list_state: ListState,
}

impl BrowseView {
    pub fn new() -> Self {
        Self {
            fields: BrowseField::all()
                .iter()
                .map(|f| (f.clone(), String::new()))
                .collect(),
            focus: BrowseFocus::Field(0),
            edit_mode: BrowseEditMode::Normal,
            results: vec![],
            search: SearchState::new(),
            search_matches: vec![],
            search_match_idx: 0,
            motion: MotionState::new(),
            list_state: ListState::default(),
        }
    }

    pub fn is_editing(&self) -> bool {
        self.edit_mode == BrowseEditMode::Editing
    }

    fn field_count(&self) -> usize {
        self.fields.len()
    }

    fn focus_next(&mut self) {
        self.focus = match &self.focus {
            BrowseFocus::Field(i) => {
                if *i + 1 < self.field_count() {
                    BrowseFocus::Field(i + 1)
                } else {
                    BrowseFocus::Search
                }
            }
            BrowseFocus::Search => BrowseFocus::Reset,
            BrowseFocus::Reset => BrowseFocus::Field(0),
            BrowseFocus::Results => BrowseFocus::Results,
        };
    }

    fn focus_prev(&mut self) {
        self.focus = match &self.focus {
            BrowseFocus::Field(i) => {
                if *i > 0 {
                    BrowseFocus::Field(i - 1)
                } else {
                    BrowseFocus::Reset
                }
            }
            BrowseFocus::Search => BrowseFocus::Field(self.field_count() - 1),
            BrowseFocus::Reset => BrowseFocus::Search,
            BrowseFocus::Results => BrowseFocus::Results,
        };
    }

    pub fn run_search(&mut self, all_songs: &[Song]) {
        self.results = all_songs
            .iter()
            .filter(|s| self.matches_filters(s))
            .cloned()
            .collect();
        self.list_state.select(Some(0));
        self.search_matches.clear();
    }

    pub fn reset(&mut self) {
        for (_, value) in &mut self.fields {
            value.clear();
        }
        self.results.clear();
        self.list_state.select(None);
        self.search_matches.clear();
    }

    fn matches_filters(&self, song: &Song) -> bool {
        for (field, value) in &self.fields {
            if value.is_empty() {
                continue;
            }
            let v = value.to_lowercase();
            let matches = match field {
                BrowseField::Any => {
                    song.title
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&v)
                        || song
                            .artist
                            .as_deref()
                            .unwrap_or("")
                            .to_lowercase()
                            .contains(&v)
                        || song_tag(song, "Album").to_lowercase().contains(&v)
                        || song_tag(song, "Genre").to_lowercase().contains(&v)
                        || song.file.to_lowercase().contains(&v)
                        || song_tag(song, "Date").to_lowercase().contains(&v)
                }
                BrowseField::Artist => song
                    .artist
                    .as_deref()
                    .unwrap_or("")
                    .to_lowercase()
                    .contains(&v),
                BrowseField::Album => song_tag(song, "Album").to_lowercase().contains(&v),
                BrowseField::Song => song
                    .title
                    .as_deref()
                    .unwrap_or("")
                    .to_lowercase()
                    .contains(&v),
                BrowseField::Genre => song_tag(song, "Genre").to_lowercase().contains(&v),
                BrowseField::Filename => song.file.to_lowercase().contains(&v),
                BrowseField::Date => song_tag(song, "Date").to_lowercase().contains(&v),
            };
            if !matches {
                return false;
            }
        }
        true
    }

    pub fn handle_key(&mut self, key: KeyEvent, all_songs: &[Song]) -> BrowseResult {
        // editing a field
        if self.edit_mode == BrowseEditMode::Editing {
            if let BrowseFocus::Field(i) = self.focus {
                match (key.modifiers, key.code) {
                    (KeyModifiers::NONE, KeyCode::Esc) => {
                        self.edit_mode = BrowseEditMode::Normal;
                        self.run_search(all_songs);
                    }
                    (KeyModifiers::NONE, KeyCode::Enter) => {
                        self.edit_mode = BrowseEditMode::Normal;
                        self.run_search(all_songs);
                        if i + 1 < self.field_count() {
                            self.focus = BrowseFocus::Field(i + 1);
                        } else {
                            self.focus = BrowseFocus::Results;
                            self.list_state.select(Some(0));
                        }
                    }
                    (KeyModifiers::NONE, KeyCode::Backspace) => {
                        self.fields[i].1.pop();
                        self.run_search(all_songs);
                    }
                    (KeyModifiers::NONE, KeyCode::Char(c)) => {
                        self.fields[i].1.push(c);
                        self.run_search(all_songs);
                    }
                    _ => {}
                }
            }
            return BrowseResult::None;
        }

        // results list navigation
        if self.focus == BrowseFocus::Results {
            if self.search.active {
                let mut search = self.search.clone();
                handle_search_input(self, &mut search, key);
                self.search = search;
                return BrowseResult::None;
            }

            match (key.modifiers, key.code) {
                (KeyModifiers::NONE, KeyCode::Esc) => {
                    self.focus = BrowseFocus::Search;
                    return BrowseResult::None;
                }
                _ => {}
            }

            let mut search = self.search.clone();
            if handle_search_normal(self, &mut search, key) {
                self.search = search;
                return BrowseResult::None;
            }

            let mut motion = self.motion.clone();
            if let Some(action) = handle_motion_key(self, &mut motion, key) {
                self.motion = motion;
                if action == MotionAction::Select {
                    let song = self
                        .results
                        .get(self.list_state.selected().unwrap_or(0))
                        .cloned();
                    if let Some(s) = song {
                        return BrowseResult::AppendAndPlay(vec![s]);
                    }
                }
            }
            return BrowseResult::None;
        }

        // normal mode — field/button navigation
        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Tab) => self.focus_next(),
            (KeyModifiers::SHIFT, KeyCode::BackTab) => self.focus_prev(),
            (KeyModifiers::NONE, KeyCode::Char('h')) => self.focus_prev(),
            (KeyModifiers::NONE, KeyCode::Char('l')) => self.focus_next(),
            (KeyModifiers::NONE, KeyCode::Char('i')) | (KeyModifiers::NONE, KeyCode::Char('a')) => {
                if let BrowseFocus::Field(_) = self.focus {
                    self.edit_mode = BrowseEditMode::Editing;
                }
            }
            (KeyModifiers::NONE, KeyCode::Enter) => match self.focus {
                BrowseFocus::Search => {
                    self.run_search(all_songs);
                    self.focus = BrowseFocus::Results;
                    self.list_state.select(Some(0));
                }
                BrowseFocus::Reset => {
                    self.reset();
                }
                BrowseFocus::Results => {
                    let song = self
                        .results
                        .get(self.list_state.selected().unwrap_or(0))
                        .cloned();
                    if let Some(s) = song {
                        return BrowseResult::AppendAndPlay(vec![s]);
                    }
                }
                BrowseFocus::Field(_) => {
                    self.edit_mode = BrowseEditMode::Editing;
                }
            },
            _ => {}
        }

        BrowseResult::None
    }

    pub fn draw(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        self.draw_header(f, chunks[0]);
        self.draw_results(f, chunks[1]);
    }

    fn draw_header(&self, f: &mut Frame, area: Rect) {
        let total_fields = self.fields.len();
        let button_count = 2usize; // Search, Reset
        let total = total_fields + button_count;

        let constraints: Vec<Constraint> = (0..total)
            .map(|_| Constraint::Ratio(1, total as u32))
            .collect();

        let header_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(constraints)
            .split(area);

        for (i, (field, value)) in self.fields.iter().enumerate() {
            let focused = self.focus == BrowseFocus::Field(i);
            let editing = focused && self.edit_mode == BrowseEditMode::Editing;

            let display = if editing {
                format!("{}_", value)
            } else {
                value.clone()
            };

            let style = if focused {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };

            let block = Block::default()
                .borders(Borders::ALL)
                .title(field.label())
                .border_style(if focused {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                });

            let paragraph = Paragraph::new(display).block(block).style(style);
            f.render_widget(paragraph, header_chunks[i]);
        }

        // Search button
        let search_focused = self.focus == BrowseFocus::Search;
        let search_block = Block::default()
            .borders(Borders::ALL)
            .border_style(if search_focused {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            });
        let search_btn = Paragraph::new("Search")
            .block(search_block)
            .style(if search_focused {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            });
        f.render_widget(search_btn, header_chunks[total_fields]);

        // Reset button
        let reset_focused = self.focus == BrowseFocus::Reset;
        let reset_block = Block::default()
            .borders(Borders::ALL)
            .border_style(if reset_focused {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            });
        let reset_btn = Paragraph::new("Reset")
            .block(reset_block)
            .style(if reset_focused {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            });
        f.render_widget(reset_btn, header_chunks[total_fields + 1]);
    }

    fn draw_results(&mut self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .results
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let artist = s.artist.as_deref().unwrap_or("Unknown");
                let title = s.title.as_deref().unwrap_or("Unknown");
                let album = song_tag(s, "Album");
                let time = s
                    .duration
                    .map(|d| format!("{}:{:02}", d.as_secs() / 60, d.as_secs() % 60))
                    .unwrap_or_else(|| "-".into());
                let text = format!("{} — {} — {} [{}]", artist, title, album, time);
                let item = ListItem::new(text);
                if self.search_matches.contains(&i) {
                    item.style(Style::default().fg(Color::Yellow))
                } else {
                    item
                }
            })
            .collect();

        let focused = self.focus == BrowseFocus::Results;
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Results ({})", self.results.len()))
                    .border_style(if focused {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    }),
            )
            .highlight_style(Style::default().bg(Color::DarkGray));

        f.render_stateful_widget(list, area, &mut self.list_state);
    }
}

pub enum BrowseResult {
    None,
    AppendAndPlay(Vec<Song>),
    Append(Vec<Song>),
}

impl VimNavigable for BrowseView {
    fn move_down(&mut self, n: usize) {
        let len = self.results.len();
        if len == 0 {
            return;
        }
        let cur = self.list_state.selected().unwrap_or(0);
        let next = (cur + n).min(len - 1);
        self.list_state.select(Some(next));
    }
    fn move_up(&mut self, n: usize) {
        let cur = self.list_state.selected().unwrap_or(0);
        self.list_state.select(Some(cur.saturating_sub(n)));
    }
    fn go_to_top(&mut self) {
        self.list_state.select(Some(0));
    }
    fn go_to_bottom(&mut self) {
        let len = self.results.len();
        self.list_state.select(Some(len.saturating_sub(1)));
    }
    fn len(&self) -> usize {
        self.results.len()
    }
}

impl VimSearchable for BrowseView {
    fn search(&mut self, query: &str) {
        let q = query.to_lowercase();
        self.search_matches = self
            .results
            .iter()
            .enumerate()
            .filter(|(_, s)| {
                s.title.as_deref().unwrap_or("").to_lowercase().contains(&q)
                    || s.artist
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&q)
            })
            .map(|(i, _)| i)
            .collect();
        self.search_match_idx = 0;
        if let Some(&first) = self.search_matches.first() {
            self.list_state.select(Some(first));
        }
    }
    fn next_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        self.search_match_idx = (self.search_match_idx + 1) % self.search_matches.len();
        self.list_state
            .select(Some(self.search_matches[self.search_match_idx]));
    }
    fn prev_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        self.search_match_idx = self
            .search_match_idx
            .checked_sub(1)
            .unwrap_or(self.search_matches.len() - 1);
        self.list_state
            .select(Some(self.search_matches[self.search_match_idx]));
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
        .unwrap_or("")
}

pub fn handle_key(app: &mut App<MpdClient>, key: KeyEvent) {
    use crossterm::event::{KeyCode, KeyModifiers};

    if let (KeyModifiers::NONE, KeyCode::Char(':')) = (key.modifiers, key.code) {
        app.command_bar.open();
        return;
    }

    let all_songs = app.library.all_songs.clone();
    match app.browse.handle_key(key, &all_songs) {
        BrowseResult::Append(songs) => {
            app.append(songs);
        },
        BrowseResult::AppendAndPlay(songs) => {
            app.append_and_play(songs);
        },
        BrowseResult::None => {}
    }
}
