use crossterm::event::KeyEvent;
use mpd::Song;

use crate::{mpd::MpdClient, ui::App};

fn song_tag<'a>(song: &'a Song, key: &str) -> &'a str {
    song.tags
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))
        .map(|(_, v)| v.as_str())
        .unwrap_or("Unknown")
}

pub fn format_album_suggestion(song: &Song) -> String {
    let artist = song.artist.as_deref().unwrap_or("Unknown");
    let album = song_tag(song, "Album");
    format!("{}: {}", artist, album)
}

/// A single registered command.
pub struct Command {
    pub name: &'static str,
    pub description: &'static str,
}

/// All registered commands. Add new entries here to extend.
pub const COMMANDS: &[Command] = &[
    Command {
        name: "a",
        description: "Search albums: :a <query>",
    },
    Command {
        name: "s",
        description: "Search songs: :s <query>",
    },
];

#[derive(Debug, Clone, PartialEq)]
pub enum CommandMode {
    Album,
    Song,
}

pub struct CommandBar {
    pub active: bool,
    pub input: String,
    pub mode: Option<CommandMode>,
    pub matches: Vec<Song>,
    pub selected: usize,
    pub history: Vec<String>,
    pub history_idx: Option<usize>,
}

impl CommandBar {
    pub fn new() -> Self {
        Self {
            active: false,
            input: String::new(),
            mode: None,
            matches: vec![],
            selected: 0,
            history: vec![],
            history_idx: None,
        }
    }

    pub fn open(&mut self) {
        self.active = true;
        self.input.clear();
        self.mode = None;
        self.matches.clear();
        self.selected = 0;
        self.history_idx = None;
    }

    pub fn close(&mut self) {
        self.active = false;
        self.input.clear();
        self.mode = None;
        self.matches.clear();
        self.selected = 0;
        self.history_idx = None;
    }

    pub fn push(&mut self, c: char) {
        self.input.push(c);
        self.history_idx = None;
    }

    pub fn pop(&mut self) {
        self.input.pop();
        self.history_idx = None;
    }

    pub fn commit_history(&mut self) {
        if !self.input.is_empty() {
            self.history.push(self.input.clone());
            if self.history.len() > 10 {
                self.history.remove(0);
            }
        }
    }

    pub fn history_prev(&mut self) {
        if self.history.is_empty() {
            return;
        }
        let idx = match self.history_idx {
            None => self.history.len() - 1,
            Some(i) => i.saturating_sub(1),
        };
        self.history_idx = Some(idx);
        self.input = self.history[idx].clone();
    }

    pub fn history_next(&mut self) {
        match self.history_idx {
            None => {}
            Some(i) => {
                if i + 1 < self.history.len() {
                    self.history_idx = Some(i + 1);
                    self.input = self.history[i + 1].clone();
                } else {
                    self.history_idx = None;
                    self.input.clear();
                }
            }
        }
    }

    /// Parses input and updates matches from all_songs.
    pub fn update_matches(&mut self, all_songs: &[Song]) {
        let parts: Vec<&str> = self.input.splitn(2, ' ').collect();
        if parts.len() < 2 {
            self.matches.clear();
            self.mode = None;
            return;
        }
        let cmd = parts[0];
        let query = parts[1].to_lowercase();

        match cmd {
            "a" => {
                self.mode = Some(CommandMode::Album);
                // collect unique (artist, album) pairs
                let mut seen = std::collections::BTreeSet::new();
                self.matches = all_songs
                    .iter()
                    .filter(|s| {
                        let album = song_tag(s, "Album").to_lowercase();
                        album.contains(&query)
                    })
                    .filter(|s| {
                        let key = format!(
                            "{}::{}",
                            s.artist.as_deref().unwrap_or(""),
                            song_tag(s, "Album")
                        );
                        seen.insert(key)
                    })
                    .cloned()
                    .collect();
            }
            "s" => {
                self.mode = Some(CommandMode::Song);
                self.matches = all_songs
                    .iter()
                    .filter(|s| {
                        s.title
                            .as_deref()
                            .unwrap_or("")
                            .to_lowercase()
                            .contains(&query)
                    })
                    .cloned()
                    .collect();
            }
            _ => {
                self.matches.clear();
                self.mode = None;
            }
        }
        self.selected = 0;
    }

    pub fn next_match(&mut self) {
        if self.matches.is_empty() {
            return;
        }
        self.selected = (self.selected + 1) % self.matches.len();
    }

    pub fn prev_match(&mut self) {
        if self.matches.is_empty() {
            return;
        }
        self.selected = self
            .selected
            .checked_sub(1)
            .unwrap_or(self.matches.len() - 1);
    }

    pub fn current_match(&self) -> Option<&Song> {
        self.matches.get(self.selected)
    }

    pub fn ghost_text(&self) -> Option<String> {
        let song = self.current_match()?;
        match self.mode {
            Some(CommandMode::Album) => Some(format_album_suggestion(song)),
            Some(CommandMode::Song) => {
                let title = song.title.as_deref().unwrap_or("Unknown");
                Some(title.to_string())
            }
            None => None,
        }
    }

    pub fn songs_to_add(&self, all_songs: &[Song]) -> Vec<Song> {
        match self.mode {
            Some(CommandMode::Album) => {
                if let Some(song) = self.current_match() {
                    let artist = song.artist.as_deref().unwrap_or("");
                    let album = song_tag(song, "Album");
                    all_songs
                        .iter()
                        .filter(|s| {
                            s.artist.as_deref().unwrap_or("") == artist
                                && song_tag(s, "Album") == album
                        })
                        .cloned()
                        .collect()
                } else {
                    vec![]
                }
            }
            Some(CommandMode::Song) => self.current_match().cloned().into_iter().collect(),
            None => vec![],
        }
    }
}
pub fn handle_key(client: &mut App<MpdClient>, key: KeyEvent) {
    use crossterm::event::{KeyCode, KeyModifiers};
    match (key.modifiers, key.code) {
        (KeyModifiers::NONE, KeyCode::Esc) => {
            client.command_bar.close();
        }
        (KeyModifiers::NONE, KeyCode::Enter) => {
            let songs = client.command_bar.songs_to_add(&client.library.all_songs);
            if songs.is_empty() {
                client.status_bar.set_message(Some("Invalid song.".into()));
            } else {
                client.command_bar.commit_history();
                client.command_bar.close();
                client.append_and_play(songs);
            }
        }
        (KeyModifiers::NONE, KeyCode::Tab) => {
            client.command_bar.next_match();
        }
        (KeyModifiers::SHIFT, KeyCode::BackTab) => {
            client.command_bar.prev_match();
        }
        (KeyModifiers::NONE, KeyCode::Up) => {
            client.command_bar.history_prev();
            let all = client.library.all_songs.clone();
            client.command_bar.update_matches(&all);
        }
        (KeyModifiers::NONE, KeyCode::Down) => {
            client.command_bar.history_next();
            let all = client.library.all_songs.clone();
            client.command_bar.update_matches(&all);
        }
        (KeyModifiers::NONE, KeyCode::Backspace) => {
            client.command_bar.pop();
            let all = client.library.all_songs.clone();
            client.command_bar.update_matches(&all);
        }
        (KeyModifiers::NONE, KeyCode::Char(c)) => {
            client.command_bar.push(c);
            let all = client.library.all_songs.clone();
            client.command_bar.update_matches(&all);
        }
        _ => {}
    }
}
