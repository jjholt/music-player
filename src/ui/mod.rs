pub mod autocomplete;
pub mod command;
pub mod library;
pub mod playlist;
pub mod statusbar;

use crossterm::event::KeyEvent;
use ratatui::Frame;
use std::time::Duration;

use crate::config::Config;
use crate::mpd::MpdClient;
use crate::ui::command::CommandBar;
use crate::ui::library::LibraryView;
use crate::ui::playlist::{PlaylistKeyResult, PlaylistView};
use crate::ui::statusbar::StatusBar;

#[derive(Debug, Clone, PartialEq)]
pub enum ActiveView {
    Playlist,
    Library,
}

pub enum Mode {
    Normal,
    Input,
}

pub trait Connection {}
impl Connection for MpdClient {}
pub struct NoConnection;
impl Connection for NoConnection {}

pub struct App<Mpd: Connection> {
    pub config: Config,
    pub mpd: Mpd,
    pub active_view: ActiveView,
    pub playlist: PlaylistView,
    pub library: LibraryView,
    pub status_bar: StatusBar,
    pub command_bar: CommandBar,
    pub db_updating: bool,
    current_song_id: Option<u32>,
    pub mode: Mode,
}

impl<T: Connection> App<T> {
    pub fn update_mode(&mut self) {
        self.mode = if self.command_bar.active
            || self.playlist.insert_state.is_some()
            || self.library.search.active
            || self.playlist.search.active
        {
            Mode::Input
        } else {
            Mode::Normal
        }
    }
}

impl App<NoConnection> {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            mpd: NoConnection,
            active_view: ActiveView::Playlist,
            playlist: PlaylistView::new(),
            library: LibraryView::new(),
            status_bar: StatusBar::new(),
            command_bar: CommandBar::new(),
            db_updating: false,
            current_song_id: None,
            mode: Mode::Normal,
        }
    }
    pub fn connect(self) -> anyhow::Result<App<MpdClient>> {
        let client = MpdClient::connect(&self.config)?;
        Ok(App {
            config: self.config,
            mpd: client,
            active_view: self.active_view,
            playlist: self.playlist,
            library: self.library,
            status_bar: self.status_bar,
            command_bar: self.command_bar,
            db_updating: self.db_updating,
            current_song_id: self.current_song_id,
            mode: self.mode,
        })
    }
}

impl App<MpdClient> {
    pub fn trigger_database_update(&mut self) {
        match self.mpd.update_database() {
            Ok(_) => {
                self.db_updating = true;
                self.status_bar
                    .set_message(Some("Updating database...".into()));
            }
            Err(e) => self
                .status_bar
                .set_message(Some(format!("Update error: {}", e))),
        }
    }

    pub fn load_library(&mut self) {
        match self.mpd.all_songs() {
            Ok(songs) => self.library.load_all_songs(songs),
            Err(e) => self
                .status_bar
                .set_message(Some(format!("Library load error: {}", e))),
        }
    }

    pub fn tick(&mut self) {
        let client = &mut self.mpd;
        match client.status() {
            Ok(status) => {
                if self.db_updating && status.updating_db.is_none() {
                    self.db_updating = false;
                    match client.all_songs() {
                        Ok(songs) => {
                            self.library.load_all_songs(songs);
                            self.status_bar
                                .set_message(Some("Database updated.".into()));
                        }
                        Err(e) => self
                            .status_bar
                            .set_message(Some(format!("Reload error: {}", e))),
                    }
                }
                self.status_bar.elapsed = status.elapsed;
                self.status_bar.total = status.duration;
                let new_id = status.song.map(|p| p.id.0);
                if new_id != self.current_song_id {
                    self.current_song_id = new_id;
                    match client.current_song() {
                        Ok(song) => self.status_bar.set_now_playing(song, status.elapsed),
                        Err(e) => self.status_bar.set_message(Some(format!("Error: {}", e))),
                    }
                }
            }
            Err(e) => self
                .status_bar
                .set_message(Some(format!("Status error: {}", e))),
        }
    }

    pub fn set_view(&mut self, view: ActiveView) {
        self.active_view = view;
        self.refresh_playlist();
    }

    // fn refresh_current_song(&mut self) {
    //     let client = &mut self.mpd;
    //     match client.status() {
    //         Ok(status) => {
    //             self.status_bar.elapsed = status.elapsed;
    //             self.status_bar.total = status.duration;
    //             self.current_song_id = status.song.map(|p| p.id.0);
    //             match client.current_song() {
    //                 Ok(song) => self.status_bar.set_now_playing(song, status.elapsed),
    //                 Err(e) => self.status_bar.set_message(Some(format!("Error: {}", e))),
    //             }
    //         }
    //         Err(e) => self.status_bar.set_message(Some(format!("Error: {}", e))),
    //     }
    // }

    fn refresh_playlist(&mut self) {
        let client = &mut self.mpd;
        match client.queue() {
            Ok(songs) => self.playlist.set_tracks(songs),
            Err(e) => self.status_bar.set_message(Some(format!("Error: {}", e))),
        }
    }

    pub fn seek_forward(&mut self) {
        let client = &mut self.mpd;
        let elapsed = self.status_bar.elapsed.unwrap_or(Duration::ZERO);
        let total = self.status_bar.total.unwrap_or(Duration::ZERO);
        let new_pos = (elapsed + Duration::from_secs(10)).min(total);
        if let Err(e) = client.seek(new_pos) {
            self.status_bar
                .set_message(Some(format!("Seek error: {}", e)));
        } else {
            self.status_bar.elapsed = Some(new_pos);
        }
    }

    pub fn seek_backward(&mut self) {
        let client = &mut self.mpd;
        let elapsed = self.status_bar.elapsed.unwrap_or(Duration::ZERO);
        let new_pos = elapsed.saturating_sub(Duration::from_secs(10));
        if let Err(e) = client.seek(new_pos) {
            self.status_bar
                .set_message(Some(format!("Seek error: {}", e)));
        } else {
            self.status_bar.elapsed = Some(new_pos);
        }
    }

    pub fn toggle_pause(&mut self) {
        let client = &mut self.mpd;
        if let Err(e) = client.toggle_pause() {
            self.status_bar.set_message(Some(format!("Error: {}", e)));
        }
    }

    pub fn next(&mut self) {
        let client = &mut self.mpd;
        if let Err(e) = client.next() {
            self.status_bar.set_message(Some(format!("Error: {}", e)));
        }
    }

    pub fn prev(&mut self) {
        let client = &mut self.mpd;
        if let Err(e) = client.prev() {
            self.status_bar.set_message(Some(format!("Error: {}", e)));
        }
    }

    fn append_and_play(&mut self, songs: Vec<mpd::Song>) {
        if songs.is_empty() {
            return;
        }
        let client = &mut self.mpd;
        match client.queue_len() {
            Ok(pos) => {
                for song in &songs {
                    if let Err(e) = client.append_song(song) {
                        self.status_bar.set_message(Some(format!("Error: {}", e)));
                        return;
                    }
                }
                if let Err(e) = client.play_at(pos) {
                    self.status_bar.set_message(Some(format!("Error: {}", e)));
                }
                self.refresh_playlist();
            }
            Err(e) => self.status_bar.set_message(Some(format!("Error: {}", e))),
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        use crossterm::event::{KeyCode, KeyModifiers};

        // command bar takes priority
        if self.command_bar.active {
            match (key.modifiers, key.code) {
                (KeyModifiers::NONE, KeyCode::Esc) => {
                    self.command_bar.close();
                }
                (KeyModifiers::NONE, KeyCode::Enter) => {
                    let songs = self.command_bar.songs_to_add(&self.library.all_songs);
                    if songs.is_empty() {
                        self.status_bar.set_message(Some("Invalid song.".into()));
                    } else {
                        self.command_bar.commit_history();
                        self.command_bar.close();
                        self.append_and_play(songs);
                    }
                }
                (KeyModifiers::NONE, KeyCode::Tab) => {
                    self.command_bar.next_match();
                }
                (KeyModifiers::SHIFT, KeyCode::BackTab) => {
                    self.command_bar.prev_match();
                }
                (KeyModifiers::NONE, KeyCode::Up) => {
                    self.command_bar.history_prev();
                    let all = self.library.all_songs.clone();
                    self.command_bar.update_matches(&all);
                }
                (KeyModifiers::NONE, KeyCode::Down) => {
                    self.command_bar.history_next();
                    let all = self.library.all_songs.clone();
                    self.command_bar.update_matches(&all);
                }
                (KeyModifiers::NONE, KeyCode::Backspace) => {
                    self.command_bar.pop();
                    let all = self.library.all_songs.clone();
                    self.command_bar.update_matches(&all);
                }
                (KeyModifiers::NONE, KeyCode::Char(c)) => {
                    self.command_bar.push(c);
                    let all = self.library.all_songs.clone();
                    self.command_bar.update_matches(&all);
                }
                _ => {}
            }
            return;
        }

        // open command bar
        if let (KeyModifiers::NONE, KeyCode::Char(':')) = (key.modifiers, key.code) {
            self.command_bar.open();
            return;
        }

        match self.active_view {
            ActiveView::Playlist => {
                let all_songs = self.library.all_songs.clone();
                let result = self.playlist.handle_key(key, &mut self.mpd, &all_songs);
                match result {
                    PlaylistKeyResult::Status(msg) => self.status_bar.set_message(Some(msg)),
                    PlaylistKeyResult::AppendAndPlay(songs) => self.append_and_play(songs),
                    PlaylistKeyResult::None => {}
                }
            }
            ActiveView::Library => {
                let selection = self.library.handle_key(key);
                if let Some(msg) = selection.status {
                    self.status_bar.set_message(Some(msg));
                }
                if !selection.songs_to_add.is_empty() {
                    self.append_and_play(selection.songs_to_add);
                }
            }
        }
        self.update_mode();
    }

    pub fn draw(&mut self, f: &mut Frame) {
        use ratatui::layout::{Constraint, Direction, Layout};

        let has_song = self.status_bar.now_playing.is_some();
        let status_height = if has_song { 2 } else { 1 };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(status_height)])
            .split(f.area());

        match self.active_view {
            ActiveView::Playlist => self.playlist.draw(f, chunks[0]),
            ActiveView::Library => self.library.draw(f, chunks[0]),
        }

        let search_state = match self.active_view {
            ActiveView::Playlist => {
                Some((&self.playlist.search, self.playlist.search_matches.len()))
            }
            ActiveView::Library => Some((&self.library.search, self.library.search_matches.len())),
        };

        self.status_bar.draw(
            f,
            chunks[1],
            search_state,
            if self.command_bar.active {
                Some(&self.command_bar)
            } else {
                None
            },
        );
    }
}
