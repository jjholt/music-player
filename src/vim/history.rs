use std::collections::VecDeque;

/// A single undoable snapshot of settings state.
/// Taken after every confirmed action.
#[derive(Debug, Clone)]
pub struct Snapshot<T: Clone> {
    pub state: T,
}

pub struct History<T: Clone> {
    /// Past states — undo pops from here
    undo_stack: VecDeque<T>,
    /// Future states — redo pops from here
    redo_stack: VecDeque<T>,
    cap: usize,
}

impl<T: Clone> History<T> {
    pub fn new(cap: usize) -> Self {
        Self {
            undo_stack: VecDeque::with_capacity(cap),
            redo_stack: VecDeque::with_capacity(cap),
            cap,
        }
    }

    /// Pushes a snapshot onto the undo stack.
    /// Clears the redo stack — a new action invalidates redo history.
    pub fn push(&mut self, state: T) {
        if self.undo_stack.len() == self.cap {
            self.undo_stack.pop_front();
        }
        self.undo_stack.push_back(state);
        self.redo_stack.clear();
    }

    /// Pops the last snapshot for undo.
    /// Pushes the current state onto the redo stack before returning.
    pub fn undo(&mut self, current: T) -> Option<T> {
        let past = self.undo_stack.pop_back()?;
        if self.redo_stack.len() == self.cap {
            self.redo_stack.pop_front();
        }
        self.redo_stack.push_back(current);
        Some(past)
    }

    /// Pops the last snapshot for redo.
    /// Pushes the current state onto the undo stack before returning.
    pub fn redo(&mut self, current: T) -> Option<T> {
        let future = self.redo_stack.pop_back()?;
        if self.undo_stack.len() == self.cap {
            self.undo_stack.pop_front();
        }
        self.undo_stack.push_back(current);
        Some(future)
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }
}
