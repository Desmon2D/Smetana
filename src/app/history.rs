use crate::model::Project;
use std::collections::VecDeque;

pub struct History {
    undo_stack: VecDeque<Project>,
    redo_stack: VecDeque<Project>,
    /// Monotonically increasing counter, bumped on every snapshot/undo/redo.
    pub version: u64,
    max_entries: usize,
}

impl History {
    pub fn new() -> Self {
        Self {
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
            version: 0,
            max_entries: 100,
        }
    }

    /// Save current project state before a mutation.
    pub fn snapshot(&mut self, project: &Project) {
        self.undo_stack.push_back(project.clone());
        if self.undo_stack.len() > self.max_entries {
            self.undo_stack.pop_front();
        }
        self.redo_stack.clear();
        self.version += 1;
    }

    pub fn undo(&mut self, project: &mut Project) -> bool {
        if let Some(prev) = self.undo_stack.pop_back() {
            self.redo_stack.push_back(project.clone());
            *project = prev;
            self.version += 1;
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self, project: &mut Project) -> bool {
        if let Some(next) = self.redo_stack.pop_back() {
            self.undo_stack.push_back(project.clone());
            *project = next;
            self.version += 1;
            true
        } else {
            false
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Bump version without storing a snapshot. For non-undoable state changes.
    pub fn mark_dirty(&mut self) {
        self.version += 1;
    }
}
