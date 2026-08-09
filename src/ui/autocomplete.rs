use mpd::Song;

fn song_tag<'a>(song: &'a Song, key: &str) -> &'a str {
    song.tags
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))
        .map(|(_, v)| v.as_str())
        .unwrap_or("Unknown")
}

pub fn format_song_suggestion(song: &Song) -> String {
    let artist = song.artist.as_deref().unwrap_or("Unknown");
    let album = song_tag(song, "Album");
    let title = song.title.as_deref().unwrap_or("Unknown");
    format!("{} - {}: {}", artist, album, title)
}

#[derive(Clone, Debug)]
pub struct Autocomplete {
    pub query: String,
    pub matches: Vec<Song>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub visible_count: usize,
}

impl Autocomplete {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            matches: vec![],
            selected: 0,
            scroll_offset: 0,
            visible_count: 5,
        }
    }

    pub fn update(&mut self, query: &str, all_songs: &[Song]) {
        self.query = query.to_string();
        let q = query.to_lowercase();
        self.matches = all_songs
            .iter()
            .filter(|s| {
                s.title.as_deref().unwrap_or("").to_lowercase().contains(&q)
                    || s.artist.as_deref().unwrap_or("").to_lowercase().contains(&q)
                    || song_tag(s, "Album").to_lowercase().contains(&q)
            })
            .cloned()
            .collect();
        self.selected = 0;
        self.scroll_offset = 0;
    }

    pub fn next(&mut self) {
        if self.matches.is_empty() {
            return;
        }
        self.selected = (self.selected + 1) % self.matches.len();
        self.sync_scroll();
    }

    pub fn prev(&mut self) {
        if self.matches.is_empty() {
            return;
        }
        self.selected = self
            .selected
            .checked_sub(1)
            .unwrap_or(self.matches.len() - 1);
        self.sync_scroll();
    }

    fn sync_scroll(&mut self) {
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        } else if self.selected >= self.scroll_offset + self.visible_count {
            self.scroll_offset = self.selected - self.visible_count + 1;
        }
    }

    pub fn current(&self) -> Option<&Song> {
        self.matches.get(self.selected)
    }

    pub fn has_matches(&self) -> bool {
        !self.matches.is_empty()
    }
}
