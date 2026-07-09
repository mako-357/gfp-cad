//! The edited model, view/selection state, and a snapshot-based undo stack.

use std::path::PathBuf;

use cad_core::{Building, Floor};
use uuid::Uuid;

/// Max snapshots kept in each of the undo / redo stacks.
const HISTORY_LIMIT: usize = 64;

/// What the user currently has selected (by stable id within the active floor).
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Selection {
    #[default]
    None,
    Wall(Uuid),
    Opening(Uuid),
    Room(Uuid),
}

pub struct Document {
    pub building: Building,
    pub active_floor: usize,
    pub path: Option<PathBuf>,
    pub selection: Selection,
    pub dirty: bool,
    /// Bumped whenever geometry changes so the renderer rebuilds its buffers.
    pub revision: u64,
    undo: Vec<Building>,
    redo: Vec<Building>,
}

impl Document {
    pub fn new(building: Building) -> Self {
        Self {
            building,
            active_floor: 0,
            path: None,
            selection: Selection::None,
            dirty: false,
            revision: 1,
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    pub fn active_floor(&self) -> Option<&Floor> {
        self.building.floors.get(self.active_floor)
    }

    /// Run an edit as one undoable transaction: snapshot, mutate, bump revision.
    /// The closure returns any value (e.g. the new entity's id) for the caller.
    pub fn edit<R>(&mut self, f: impl FnOnce(&mut Building) -> R) -> R {
        let snapshot = self.building.clone();
        let result = f(&mut self.building);
        self.undo.push(snapshot);
        if self.undo.len() > HISTORY_LIMIT {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.dirty = true;
        self.revision += 1;
        result
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo(&mut self) -> bool {
        let Some(prev) = self.undo.pop() else {
            return false;
        };
        self.redo.push(std::mem::replace(&mut self.building, prev));
        self.dirty = true;
        self.revision += 1;
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(next) = self.redo.pop() else {
            return false;
        };
        self.undo.push(std::mem::replace(&mut self.building, next));
        self.dirty = true;
        self.revision += 1;
        true
    }

    /// World bbox `[min_x, min_y, max_x, max_y]` over grid, walls and rooms.
    pub fn bounds(&self) -> [f64; 4] {
        crate::scene::bounds::model_bounds(&self.building, self.active_floor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::sample;

    #[test]
    fn edit_undo_redo_restores_state() {
        let mut doc = Document::new(sample::building());
        let n0 = doc.building.floors[0].rooms.len();
        assert!(!doc.can_undo());

        // Edit: add a room.
        doc.edit(|b| {
            let f = &mut b.floors[0];
            f.rooms.push(cad_core::Room::new(
                "Test",
                cad_core::Point2D::new(0.0, 0.0),
            ));
        });
        assert_eq!(doc.building.floors[0].rooms.len(), n0 + 1);
        assert!(doc.dirty && doc.can_undo() && !doc.can_redo());

        // Undo restores the original count; redo re-applies.
        assert!(doc.undo());
        assert_eq!(doc.building.floors[0].rooms.len(), n0);
        assert!(doc.can_redo());
        assert!(doc.redo());
        assert_eq!(doc.building.floors[0].rooms.len(), n0 + 1);
    }

    #[test]
    fn a_new_edit_clears_the_redo_stack() {
        let mut doc = Document::new(sample::building());
        doc.edit(|b| b.name = "A".into());
        doc.undo();
        assert!(doc.can_redo());
        doc.edit(|b| b.name = "B".into());
        assert!(!doc.can_redo(), "a fresh edit must invalidate redo");
    }
}
