use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use mpd::Song;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
};

use crate::vim::{
    motion::{MotionAction, MotionState, VimNavigable, handle_motion_key},
    search::{SearchState, VimSearchable, handle_search_input, handle_search_normal},
};

#[derive(Debug, Clone, PartialEq)]
pub enum LibraryColumn {
    Artists,
    Albums,
    Songs,
}

#[derive(Debug, Default)]
pub struct Selection {
    pub status: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub song: Option<Song>,
    pub songs_to_add: Vec<Song>,
}

pub struct LibraryView {
    pub all_songs: Vec<Song>,

    pub artists: Vec<String>,
    pub albums: Vec<String>,
    pub songs: Vec<Song>,

    pub artist_cursor: usize,
    pub album_cursor: usize,
    pub song_cursor: usize,

    pub focused_column: LibraryColumn,

    pub artist_motion: MotionState,
    pub album_motion: MotionState,
    pub song_motion: MotionState,

    pub search: SearchState,
    pub search_matches: Vec<usize>,
    pub search_match_idx: usize,

    artist_state: ListState,
    album_state: ListState,
    song_state: ListState,
}

impl LibraryView {
    pub fn new() -> Self {
        Self {
            all_songs: vec![],
            artists: vec![],
            albums: vec![],
            songs: vec![],
            artist_cursor: 0,
            album_cursor: 0,
            song_cursor: 0,
            focused_column: LibraryColumn::Artists,
            artist_motion: MotionState::new(),
            album_motion: MotionState::new(),
            song_motion: MotionState::new(),
            search: SearchState::new(),
            search_matches: vec![],
            search_match_idx: 0,
            artist_state: ListState::default(),
            album_state: ListState::default(),
            song_state: ListState::default(),
        }
    }

    pub fn load_all_songs(&mut self, songs: Vec<Song>) {
        self.all_songs = songs;
        self.artists = self.collect_artists();
        self.artist_cursor = 0;
        self.artist_state.select(Some(0));
        self.refresh_albums();
        self.refresh_songs();
    }

    fn collect_artists(&self) -> Vec<String> {
        self.all_songs
            .iter()
            .filter_map(|s| s.artist.clone())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    fn refresh_albums(&mut self) {
        let artist = self
            .artists
            .get(self.artist_cursor)
            .cloned()
            .unwrap_or_default();
        self.albums = self
            .all_songs
            .iter()
            .filter(|s| s.artist.as_deref().unwrap_or("") == artist)
            .filter_map(|s| song_tag_owned(s, "Album"))
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        self.album_cursor = 0;
        self.album_state.select(Some(0));
    }

    fn refresh_songs(&mut self) {
        let artist = self
            .artists
            .get(self.artist_cursor)
            .cloned()
            .unwrap_or_default();
        let album = self
            .albums
            .get(self.album_cursor)
            .cloned()
            .unwrap_or_default();
        self.songs = self
            .all_songs
            .iter()
            .filter(|s| {
                s.artist.as_deref().unwrap_or("") == artist && song_tag(s, "Album") == album
            })
            .cloned()
            .collect();
        self.song_cursor = 0;
        self.song_state.select(Some(0));
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Selection {
        if self.search.active {
            let mut search = self.search.clone();
            handle_search_input(self, &mut search, key);
            self.search = search;
            return Selection::default();
        }

        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Char('h')) => {
                self.focus_left();
                return Selection::default();
            }
            (KeyModifiers::NONE, KeyCode::Char('l')) => {
                return self.focus_right();
            }
            (KeyModifiers::NONE, KeyCode::Enter) => return self.select_current(),
            _ => {}
        }

        let mut search = self.search.clone();
        if handle_search_normal(self, &mut search, key) {
            self.search = search;
            return Selection::default();
        }

        match self.focused_column {
            LibraryColumn::Artists => {
                let mut motion = self.artist_motion.clone();
                let mut proxy = ArtistProxy(self);
                if let Some(action) = handle_motion_key(&mut proxy, &mut motion, key) {
                    proxy.0.artist_motion = motion;
                    match action {
                        MotionAction::MoveDown(_)
                        | MotionAction::MoveUp(_)
                        | MotionAction::GoToTop
                        | MotionAction::GoToBottom => {
                            proxy.0.refresh_albums();
                            proxy.0.refresh_songs();
                        }
                        MotionAction::Select => {
                            let artist = proxy.0.artists.get(proxy.0.artist_cursor).cloned();
                            return Selection {
                                artist,
                                ..Default::default()
                            };
                        }
                        _ => {}
                    }
                }
            }
            LibraryColumn::Albums => {
                let mut motion = self.album_motion.clone();
                let mut proxy = AlbumProxy(self);
                if let Some(action) = handle_motion_key(&mut proxy, &mut motion, key) {
                    proxy.0.album_motion = motion;
                    match action {
                        MotionAction::MoveDown(_)
                        | MotionAction::MoveUp(_)
                        | MotionAction::GoToTop
                        | MotionAction::GoToBottom => {
                            proxy.0.refresh_songs();
                        }
                        MotionAction::Select => {
                            let artist = proxy.0.artists.get(proxy.0.artist_cursor).cloned();
                            let album = proxy.0.albums.get(proxy.0.album_cursor).cloned();
                            return Selection {
                                artist,
                                album,
                                ..Default::default()
                            };
                        }
                        _ => {}
                    }
                }
            }
            LibraryColumn::Songs => {
                let mut motion = self.song_motion.clone();
                let mut proxy = SongProxy(self);
                if let Some(action) = handle_motion_key(&mut proxy, &mut motion, key) {
                    proxy.0.song_motion = motion;
                    if action == MotionAction::Select {
                        let song = proxy.0.songs.get(proxy.0.song_cursor).cloned();
                        return Selection {
                            song,
                            ..Default::default()
                        };
                    }
                }
            }
        }

        Selection::default()
    }

    fn focus_left(&mut self) {
        self.focused_column = match self.focused_column {
            LibraryColumn::Albums => LibraryColumn::Artists,
            LibraryColumn::Songs => LibraryColumn::Albums,
            LibraryColumn::Artists => LibraryColumn::Artists,
        };
    }

    fn focus_right(&mut self) -> Selection {
        match self.focused_column {
            LibraryColumn::Artists => {
                self.focused_column = LibraryColumn::Albums;
            }
            LibraryColumn::Albums => {
                self.focused_column = LibraryColumn::Songs;
            }
            LibraryColumn::Songs => {}
        }
        Selection::default()
    }

    fn select_current(&mut self) -> Selection {
        match self.focused_column {
            LibraryColumn::Artists => {
                let songs: Vec<Song> = self
                    .all_songs
                    .iter()
                    .filter(|s| {
                        s.artist.as_deref().unwrap_or("")
                            == self
                                .artists
                                .get(self.artist_cursor)
                                .map(|s| s.as_str())
                                .unwrap_or("")
                    })
                    .cloned()
                    .collect();
                Selection {
                    songs_to_add: songs,
                    ..Default::default()
                }
            }
            LibraryColumn::Albums => {
                let artist = self
                    .artists
                    .get(self.artist_cursor)
                    .map(|s| s.as_str())
                    .unwrap_or("");
                let album = self
                    .albums
                    .get(self.album_cursor)
                    .map(|s| s.as_str())
                    .unwrap_or("");
                let songs = self
                    .all_songs
                    .iter()
                    .filter(|s| {
                        s.artist.as_deref().unwrap_or("") == artist && song_tag(s, "Album") == album
                    })
                    .cloned()
                    .collect();
                Selection {
                    songs_to_add: songs,
                    ..Default::default()
                }
            }
            LibraryColumn::Songs => {
                let song = self.songs.get(self.song_cursor).cloned();
                Selection {
                    songs_to_add: song.into_iter().collect(),
                    ..Default::default()
                }
            }
        }
    }

    pub fn draw(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(33),
                Constraint::Percentage(34),
            ])
            .split(area);

        let focused_style = Style::default().bg(Color::DarkGray);
        let normal_style = Style::default();

        let artist_items: Vec<ListItem> = self
            .artists
            .iter()
            .enumerate()
            .map(|(i, a)| {
                let item = ListItem::new(a.as_str());
                if self.search_matches.contains(&i) {
                    item.style(Style::default().fg(Color::Yellow))
                } else {
                    item
                }
            })
            .collect();
        let artist_list = List::new(artist_items)
            .block(Block::default().borders(Borders::ALL).title("Artists"))
            .highlight_style(if self.focused_column == LibraryColumn::Artists {
                focused_style
            } else {
                normal_style
            });
        f.render_stateful_widget(artist_list, chunks[0], &mut self.artist_state);

        let album_items: Vec<ListItem> = self
            .albums
            .iter()
            .map(|a| ListItem::new(a.as_str()))
            .collect();
        let album_list = List::new(album_items)
            .block(Block::default().borders(Borders::ALL).title("Albums"))
            .highlight_style(if self.focused_column == LibraryColumn::Albums {
                focused_style
            } else {
                normal_style
            });
        f.render_stateful_widget(album_list, chunks[1], &mut self.album_state);

        let song_items: Vec<ListItem> = self
            .songs
            .iter()
            .map(|s| {
                let title = s.title.as_deref().unwrap_or("Unknown");
                let time = s
                    .duration
                    .map(|d| format!("{}:{:02}", d.as_secs() / 60, d.as_secs() % 60))
                    .unwrap_or_else(|| "-".into());
                ListItem::new(format!("{} [{}]", title, time))
            })
            .collect();
        let song_list = List::new(song_items)
            .block(Block::default().borders(Borders::ALL).title("Songs"))
            .highlight_style(if self.focused_column == LibraryColumn::Songs {
                focused_style
            } else {
                normal_style
            });
        f.render_stateful_widget(song_list, chunks[2], &mut self.song_state);
    }
}

struct ArtistProxy<'a>(&'a mut LibraryView);
impl<'a> VimNavigable for ArtistProxy<'a> {
    fn move_down(&mut self, n: usize) {
        self.0.artist_cursor =
            (self.0.artist_cursor + n).min(self.0.artists.len().saturating_sub(1));
        self.0.artist_state.select(Some(self.0.artist_cursor));
    }
    fn move_up(&mut self, n: usize) {
        self.0.artist_cursor = self.0.artist_cursor.saturating_sub(n);
        self.0.artist_state.select(Some(self.0.artist_cursor));
    }
    fn go_to_top(&mut self) {
        self.0.artist_cursor = 0;
        self.0.artist_state.select(Some(0));
    }
    fn go_to_bottom(&mut self) {
        self.0.artist_cursor = self.0.artists.len().saturating_sub(1);
        self.0.artist_state.select(Some(self.0.artist_cursor));
    }
    fn len(&self) -> usize {
        self.0.artists.len()
    }
}

struct AlbumProxy<'a>(&'a mut LibraryView);
impl<'a> VimNavigable for AlbumProxy<'a> {
    fn move_down(&mut self, n: usize) {
        self.0.album_cursor = (self.0.album_cursor + n).min(self.0.albums.len().saturating_sub(1));
        self.0.album_state.select(Some(self.0.album_cursor));
    }
    fn move_up(&mut self, n: usize) {
        self.0.album_cursor = self.0.album_cursor.saturating_sub(n);
        self.0.album_state.select(Some(self.0.album_cursor));
    }
    fn go_to_top(&mut self) {
        self.0.album_cursor = 0;
        self.0.album_state.select(Some(0));
    }
    fn go_to_bottom(&mut self) {
        self.0.album_cursor = self.0.albums.len().saturating_sub(1);
        self.0.album_state.select(Some(self.0.album_cursor));
    }
    fn len(&self) -> usize {
        self.0.albums.len()
    }
}

struct SongProxy<'a>(&'a mut LibraryView);
impl<'a> VimNavigable for SongProxy<'a> {
    fn move_down(&mut self, n: usize) {
        self.0.song_cursor = (self.0.song_cursor + n).min(self.0.songs.len().saturating_sub(1));
        self.0.song_state.select(Some(self.0.song_cursor));
    }
    fn move_up(&mut self, n: usize) {
        self.0.song_cursor = self.0.song_cursor.saturating_sub(n);
        self.0.song_state.select(Some(self.0.song_cursor));
    }
    fn go_to_top(&mut self) {
        self.0.song_cursor = 0;
        self.0.song_state.select(Some(0));
    }
    fn go_to_bottom(&mut self) {
        self.0.song_cursor = self.0.songs.len().saturating_sub(1);
        self.0.song_state.select(Some(self.0.song_cursor));
    }
    fn len(&self) -> usize {
        self.0.songs.len()
    }
}

impl VimSearchable for LibraryView {
    fn search(&mut self, query: &str) {
        self.search_matches = self
            .artists
            .iter()
            .enumerate()
            .filter(|(_, a)| a.to_lowercase().contains(&query.to_lowercase()))
            .map(|(i, _)| i)
            .collect();
        self.search_match_idx = 0;
        if let Some(&first) = self.search_matches.first() {
            self.artist_cursor = first;
            self.artist_state.select(Some(first));
            self.refresh_albums();
            self.refresh_songs();
        }
    }
    fn next_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        self.search_match_idx = (self.search_match_idx + 1) % self.search_matches.len();
        self.artist_cursor = self.search_matches[self.search_match_idx];
        self.artist_state.select(Some(self.artist_cursor));
        self.refresh_albums();
        self.refresh_songs();
    }
    fn prev_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        self.search_match_idx = self
            .search_match_idx
            .checked_sub(1)
            .unwrap_or(self.search_matches.len() - 1);
        self.artist_cursor = self.search_matches[self.search_match_idx];
        self.artist_state.select(Some(self.artist_cursor));
        self.refresh_albums();
        self.refresh_songs();
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

fn song_tag_owned(song: &Song, key: &str) -> Option<String> {
    song.tags
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))
        .map(|(_, v)| v.clone())
}
