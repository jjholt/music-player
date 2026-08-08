use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub trait VimNavigable {
    fn move_down(&mut self, n: usize);
    fn move_up(&mut self, n: usize);
    fn go_to_top(&mut self);
    fn go_to_bottom(&mut self);
    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MotionAction {
    MoveDown(usize),
    MoveUp(usize),
    GoToTop,
    HalfPageUp,
    HalfPageDown,
    GoToBottom,
    Select,
    PendingG,
}

#[derive(Debug, Default, Clone)]
pub struct MotionState {
    pub pending_count: Option<usize>,
    pub last_was_g: bool,
}

impl MotionState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_digit(&mut self, c: char) {
        let digit = c.to_digit(10).unwrap() as usize;
        self.pending_count = Some(self.pending_count.unwrap_or(0) * 10 + digit);
    }

    pub fn take_count(&mut self) -> usize {
        self.pending_count.take().unwrap_or(1)
    }

    pub fn reset(&mut self) {
        self.pending_count = None;
        self.last_was_g = false;
    }
}

/// Returns `Some(MotionAction)` if an action was completed or is pending,
/// `None` if the key was not consumed.
pub fn handle_motion_key<T: VimNavigable>(
    target: &mut T,
    state: &mut MotionState,
    key: KeyEvent,
) -> Option<MotionAction> {
    match (key.modifiers, key.code) {
        (KeyModifiers::NONE, KeyCode::Char(c)) if c.is_ascii_digit() && c != '0' => {
            state.push_digit(c);
            None
        }
        (KeyModifiers::NONE, KeyCode::Char('j')) => {
            let n = state.take_count();
            target.move_down(n);
            state.last_was_g = false;
            Some(MotionAction::MoveDown(n))
        }
        (KeyModifiers::NONE, KeyCode::Char('k')) => {
            let n = state.take_count();
            target.move_up(n);
            state.last_was_g = false;
            Some(MotionAction::MoveUp(n))
        }
        (KeyModifiers::SHIFT, KeyCode::Char('G')) => {
            state.reset();
            target.go_to_bottom();
            Some(MotionAction::GoToBottom)
        }
        (KeyModifiers::NONE, KeyCode::Char('g')) => {
            if state.last_was_g {
                state.reset();
                target.go_to_top();
                Some(MotionAction::GoToTop)
            } else {
                state.last_was_g = true;
                Some(MotionAction::PendingG)
            }
        }
        (KeyModifiers::NONE, KeyCode::Enter) => {
            state.reset();
            Some(MotionAction::Select)
        }
        (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
            state.reset();
            target.move_up(10);
            Some(MotionAction::HalfPageUp)
        }
        (KeyModifiers::CONTROL, KeyCode::Char('d')) => {
            state.reset();
            target.move_down(10);
            Some(MotionAction::HalfPageDown)
        }
        _ => {
            state.last_was_g = false;
            None
        }
    }
}
