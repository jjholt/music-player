use crossterm::event::KeyEvent;
use ratatui::Frame;
use std::time::Duration;

use crate::config::Config;
use crate::mpd::MpdClient;
use crate::ui::browse::{self, BrowseView};
use crate::ui::command::{self, CommandBar};
use crate::ui::library::{self, LibraryView};
use crate::ui::playlist::{self, PlaylistView};
use crate::ui::statusbar::StatusBar;

#[derive(Debug, Clone, PartialEq)]
pub enum ActiveView {
    Playlist,
    Library,
    Browse,
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
    pub browse: BrowseView,
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
            || self.browse.is_editing()
            || self.browse.search.active
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
            browse: BrowseView::new(),
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
            browse: self.browse,
            status_bar: self.status_bar,
            command_bar: self.command_bar,
            db_updating: self.db_updating,
            current_song_id: self.current_song_id,
            mode: self.mode,
        })
    }
}

impl App<MpdClient> {
    pub fn trigger_database_update(&mut self) -> &mut Self {
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
        self
    }

    pub fn load_library(&mut self) -> &mut Self {
        match self.mpd.all_songs() {
            Ok(songs) => self.library.load_all_songs(songs),
            Err(e) => self
                .status_bar
                .set_message(Some(format!("Library load error: {}", e))),
        }
        self
    }

    pub fn tick(&mut self) -> &mut Self {
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
        self
    }

    pub fn set_view(&mut self, view: ActiveView) -> &mut Self {
        self.active_view = view;
        self.refresh_playlist();
        self
    }

    fn refresh_playlist(&mut self) -> &mut Self {
        let client = &mut self.mpd;
        match client.queue() {
            Ok(songs) => self.playlist.set_tracks(songs),
            Err(e) => self.status_bar.set_message(Some(format!("Error: {}", e))),
        }
        self
    }

    pub fn seek_forward(&mut self) -> &mut Self {
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
        self
    }

    pub fn seek_backward(&mut self) -> &mut Self {
        let client = &mut self.mpd;
        let elapsed = self.status_bar.elapsed.unwrap_or(Duration::ZERO);
        let new_pos = elapsed.saturating_sub(Duration::from_secs(10));
        if let Err(e) = client.seek(new_pos) {
            self.status_bar
                .set_message(Some(format!("Seek error: {}", e)));
        } else {
            self.status_bar.elapsed = Some(new_pos);
        }
        self
    }

    pub fn toggle_pause(&mut self) -> &mut Self {
        let client = &mut self.mpd;
        if let Err(e) = client.toggle_pause() {
            self.status_bar.set_message(Some(format!("Error: {}", e)));
        }
        self
    }

    pub fn next(&mut self) -> &mut Self {
        let client = &mut self.mpd;
        if let Err(e) = client.next() {
            self.status_bar.set_message(Some(format!("Error: {}", e)));
        }
        self
    }

    pub fn prev(&mut self) -> &mut Self {
        let client = &mut self.mpd;
        if let Err(e) = client.prev() {
            self.status_bar.set_message(Some(format!("Error: {}", e)));
        }
        self
    }

    pub fn append(&mut self, songs: Vec<mpd::Song>) -> &mut Self {
        let client = &mut self.mpd;
        for song in songs {
            if let Err(e) = client.append_song(&song) {
                self.status_bar.set_message(Some(format!("Error: {}", e)));
            }
        }
        self.refresh_playlist();
        self
    }

    pub fn append_and_play(&mut self, songs: Vec<mpd::Song>) -> &mut Self {
        if songs.is_empty() {
            return self;
        }
        let _ = self
            .mpd
            .queue_len()
            .map(|pos| self.append(songs).mpd.play_at(pos))
            .map_err(|e| self.status_bar.set_message(Some(format!("Error: {}", e))));
        self
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> &mut Self {
        if self.command_bar.active {
            command::handle_key(self, key);
            self.update_mode();
            return self;
        }

        match self.active_view {
            ActiveView::Playlist => playlist::handle_key(self, key),
            ActiveView::Library => library::handle_key(self, key),
            ActiveView::Browse => browse::handle_key(self, key),
        }

        self.update_mode();
        self
    }

    pub fn draw(&mut self, f: &mut Frame) -> &mut Self {
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
            ActiveView::Browse => self.browse.draw(f, chunks[0]),
        }

        let search_state = match self.active_view {
            ActiveView::Playlist => {
                Some((&self.playlist.search, self.playlist.search_matches.len()))
            }
            ActiveView::Library => Some((&self.library.search, self.library.search_matches.len())),
            ActiveView::Browse => Some((&self.browse.search, self.browse.search_matches.len())),
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
        self
    }
}
