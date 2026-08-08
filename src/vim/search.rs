use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub trait VimSearchable {
    fn search(&mut self, query: &str);
    fn next_match(&mut self);
    fn prev_match(&mut self);
    fn current_query(&self) -> &str;
    fn match_count(&self) -> usize;
}

#[derive(Debug, Default, Clone)]
pub struct SearchState {
    pub active: bool,
    pub query: String,
}

impl SearchState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self) {
        self.active = true;
        self.query.clear();
    }

    pub fn close(&mut self) {
        self.active = false;
    }

    pub fn push(&mut self, c: char) {
        self.query.push(c);
    }

    pub fn pop(&mut self) {
        self.query.pop();
    }
}

/// Processes a key event while the search bar is open.
/// Calls `target.search()` on every change.
/// Returns `true` if the key was consumed.
pub fn handle_search_input<T: VimSearchable>(
    target: &mut T,
    state: &mut SearchState,
    key: KeyEvent,
) -> bool {
    match key.code {
        KeyCode::Esc | KeyCode::Enter => {
            state.close();
            true
        }
        KeyCode::Backspace => {
            state.pop();
            target.search(&state.query.clone());
            true
        }
        KeyCode::Char(c) => {
            state.push(c);
            target.search(&state.query.clone());
            true
        }
        _ => false,
    }
}

/// Processes `/`, `n`, `N` in normal mode.
/// Returns `true` if the key was consumed.
pub fn handle_search_normal<T: VimSearchable>(
    target: &mut T,
    state: &mut SearchState,
    key: KeyEvent,
) -> bool {
    match (key.modifiers, key.code) {
        // / — open search
        (KeyModifiers::NONE, KeyCode::Char('/')) => {
            state.open();
            true
        }

        // n — next match
        (KeyModifiers::NONE, KeyCode::Char('n')) => {
            target.next_match();
            true
        }

        // N — prev match
        (KeyModifiers::SHIFT, KeyCode::Char('N')) => {
            target.prev_match();
            true
        }

        _ => false,
    }
}
