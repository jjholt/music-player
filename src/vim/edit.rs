use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub trait VimEditable {
    fn insert_above(&mut self);
    fn insert_below(&mut self);
    fn delete_current(&mut self);
    fn delete_from_cursor(&mut self);
    fn move_item_up(&mut self);
    fn move_item_down(&mut self);
}

#[derive(Debug, Clone, PartialEq)]
pub enum EditAction {
    InsertAbove,
    InsertBelow,
    DeleteCurrent,
    DeleteFromCursor,
    MoveUp,
    MoveDown,
}

#[derive(Debug, Default, Clone)]
pub struct EditState {
    pub last_was_d: bool,
}

impl EditState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.last_was_d = false;
    }
}

/// Returns `Some(EditAction)` if an action was completed, `None` if the key was not consumed.
/// Note: a pending `d` press returns `None` until `dd` is completed.
pub fn handle_edit_key<T: VimEditable>(
    target: &mut T,
    state: &mut EditState,
    key: KeyEvent,
) -> Option<EditAction> {
    match (key.modifiers, key.code) {
        (KeyModifiers::NONE, KeyCode::Char('i')) => {
            state.reset();
            target.insert_above();
            Some(EditAction::InsertAbove)
        }
        (KeyModifiers::NONE, KeyCode::Char('a')) => {
            state.reset();
            target.insert_below();
            Some(EditAction::InsertBelow)
        }
        (KeyModifiers::NONE, KeyCode::Char('o')) => {
            state.reset();
            target.insert_below();
            Some(EditAction::InsertBelow)
        }
        (KeyModifiers::SHIFT, KeyCode::Char('O')) => {
            state.reset();
            target.insert_above();
            Some(EditAction::InsertAbove)
        }
        (KeyModifiers::NONE, KeyCode::Char('d')) => {
            if state.last_was_d {
                state.reset();
                target.delete_current();
                Some(EditAction::DeleteCurrent)
            } else {
                state.last_was_d = true;
                None
            }
        }
        (KeyModifiers::SHIFT, KeyCode::Char('D')) => {
            state.reset();
            target.delete_from_cursor();
            Some(EditAction::DeleteFromCursor)
        }
        (KeyModifiers::SHIFT, KeyCode::Char('J')) => {
            state.reset();
            target.move_item_down();
            Some(EditAction::MoveDown)
        }
        (KeyModifiers::SHIFT, KeyCode::Char('K')) => {
            state.reset();
            target.move_item_up();
            Some(EditAction::MoveUp)
        }
        _ => {
            state.last_was_d = false;
            None
        }
    }
}
