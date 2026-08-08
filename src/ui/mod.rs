pub mod library;
pub mod playlist;
pub mod statusbar;

use crossterm::event::KeyEvent;
use ratatui::Frame;
use std::time::Duration;

use crate::config::Config;
use crate::mpd::MpdClient;
use crate::ui::library::LibraryView;
use crate::ui::playlist::PlaylistView;
use crate::ui::statusbar::StatusBar;

#[derive(Debug, Clone, PartialEq)]
pub enum ActiveView {
    Playlist,
    Library,
}

pub struct App {
    pub config: Config,
    pub mpd: Option<MpdClient>,
    pub active_view: ActiveView,
    pub playlist: PlaylistView,
    pub library: LibraryView,
    pub status_bar: StatusBar,
    pub db_updating: bool,
    current_song_id: Option<u32>,
}

impl App {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            mpd: None,
            active_view: ActiveView::Playlist,
            playlist: PlaylistView::new(),
            library: LibraryView::new(),
            status_bar: StatusBar::new(),
            db_updating: false,
            current_song_id: None,
        }
    }

    pub fn connect_mpd(&mut self) {
        match MpdClient::connect(&self.config) {
            Ok(mut client) => {
                match client.all_songs() {
                    Ok(songs) => self.library.load_all_songs(songs),
                    Err(e) => self
                        .status_bar
                        .set_message(Some(format!("Library load error: {}", e))),
                }
                self.mpd = Some(client);
                self.refresh_current_song();
                self.refresh_playlist();
            }
            Err(e) => {
                self.status_bar
                    .set_message(Some(format!("MPD connection error: {}", e)));
            }
        }
    }

    pub fn toggle_pause(&mut self) {
        if let Some(ref mut client) = self.mpd {
            if let Err(e) = client.toggle_pause() {
                self.status_bar.set_message(Some(format!("Error: {}", e)));
            }
        }
    }

    pub fn trigger_database_update(&mut self) {
        if let Some(ref mut client) = self.mpd {
            match client.update_database() {
                Ok(_) => {
                    self.db_updating = true;
                    self.status_bar
                        .set_message(Some("Updating database...".into()));
                }
                Err(e) => self
                    .status_bar
                    .set_message(Some(format!("Update error: {}", e))),
            }
        } else {
            self.status_bar
                .set_message(Some("Not connected to MPD.".into()));
        }
    }

    pub fn tick(&mut self) {
        if let Some(ref mut client) = self.mpd {
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
    }

    pub fn set_view(&mut self, view: ActiveView) {
        self.active_view = view;
        self.refresh_playlist();
    }

    fn refresh_current_song(&mut self) {
        if let Some(ref mut client) = self.mpd {
            match client.status() {
                Ok(status) => {
                    self.status_bar.elapsed = status.elapsed;
                    self.status_bar.total = status.duration;
                    self.current_song_id = status.song.map(|p| p.id.0);
                    match client.current_song() {
                        Ok(song) => self.status_bar.set_now_playing(song, status.elapsed),
                        Err(e) => self.status_bar.set_message(Some(format!("Error: {}", e))),
                    }
                }
                Err(e) => self.status_bar.set_message(Some(format!("Error: {}", e))),
            }
        }
    }

    fn refresh_playlist(&mut self) {
        if let Some(ref mut client) = self.mpd {
            match client.queue() {
                Ok(songs) => self.playlist.set_tracks(songs),
                Err(e) => self.status_bar.set_message(Some(format!("Error: {}", e))),
            }
        }
    }

    pub fn seek_forward(&mut self) {
        if let Some(ref mut client) = self.mpd {
            let elapsed = self.status_bar.elapsed.unwrap_or(Duration::ZERO);
            let total = self.status_bar.total.unwrap_or(Duration::ZERO);
            let new_pos = (elapsed + Duration::from_secs(10)).min(total);
            if let Err(e) = client.seek(new_pos) {
                self.status_bar
                    .set_message(Some(format!("Seek error: {}", e)));
            }
        }
    }

    pub fn seek_backward(&mut self) {
        if let Some(ref mut client) = self.mpd {
            let elapsed = self.status_bar.elapsed.unwrap_or(Duration::ZERO);
            let new_pos = elapsed.saturating_sub(Duration::from_secs(10));
            if let Err(e) = client.seek(new_pos) {
                self.status_bar
                    .set_message(Some(format!("Seek error: {}", e)));
            }
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        match self.active_view {
            ActiveView::Playlist => {
                let status = self.playlist.handle_key(key, self.mpd.as_mut());
                if let Some(msg) = status {
                    self.status_bar.set_message(Some(msg));
                }
            }
            ActiveView::Library => {
                let selection = self.library.handle_key(key);
                if let Some(msg) = selection.status {
                    self.status_bar.set_message(Some(msg));
                }
                if !selection.songs_to_add.is_empty() {
                    if let Some(ref mut client) = self.mpd {
                        match client.queue_len() {
                            Ok(pos) => {
                                for song in &selection.songs_to_add {
                                    if let Err(e) = client.append_song(song) {
                                        self.status_bar.set_message(Some(format!("Error: {}", e)));
                                        break;
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
                }
            }
        }
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

        self.status_bar.draw(f, chunks[1], search_state);
    }
}

fn format_duration(d: Duration) -> String {
    let secs = d.as_secs();
    format!("{}:{:02}", secs / 60, secs % 60)
}
