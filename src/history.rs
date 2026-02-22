use crate::model::{Opening, OpeningKind, Project, Wall};

pub trait Command {
    fn execute(&mut self, project: &mut Project);
    fn undo(&mut self, project: &mut Project);
    fn description(&self) -> &str;
}

// --- Wall commands ---

pub struct AddWallCommand {
    pub wall: Wall,
}

impl Command for AddWallCommand {
    fn execute(&mut self, project: &mut Project) {
        project.walls.push(self.wall.clone());
    }

    fn undo(&mut self, project: &mut Project) {
        project.walls.retain(|w| w.id != self.wall.id);
    }

    fn description(&self) -> &str {
        "Добавить стену"
    }
}

pub struct RemoveWallCommand {
    wall: Wall,
    /// Openings that were attached to this wall (removed together).
    openings: Vec<Opening>,
}

impl RemoveWallCommand {
    pub fn new(wall_id: uuid::Uuid, project: &Project) -> Option<Self> {
        let wall = project.walls.iter().find(|w| w.id == wall_id)?.clone();
        let openings: Vec<Opening> = project
            .openings
            .iter()
            .filter(|o| o.wall_id == Some(wall_id))
            .cloned()
            .collect();
        Some(Self { wall, openings })
    }
}

impl Command for RemoveWallCommand {
    fn execute(&mut self, project: &mut Project) {
        // Remove attached openings
        for o in &self.openings {
            project.openings.retain(|po| po.id != o.id);
        }
        project.walls.retain(|w| w.id != self.wall.id);
    }

    fn undo(&mut self, project: &mut Project) {
        project.walls.push(self.wall.clone());
        for o in &self.openings {
            project.openings.push(o.clone());
        }
    }

    fn description(&self) -> &str {
        "Удалить стену"
    }
}

pub struct ModifyWallCommand {
    wall_id: uuid::Uuid,
    old: WallProps,
    new: WallProps,
}

#[derive(Clone)]
struct WallProps {
    thickness: f64,
    height_start: f64,
    height_end: f64,
}

impl ModifyWallCommand {
    pub fn from_values(
        wall_id: uuid::Uuid,
        old_thickness: f64, old_height_start: f64, old_height_end: f64,
        new_thickness: f64, new_height_start: f64, new_height_end: f64,
    ) -> Self {
        Self {
            wall_id,
            old: WallProps { thickness: old_thickness, height_start: old_height_start, height_end: old_height_end },
            new: WallProps { thickness: new_thickness, height_start: new_height_start, height_end: new_height_end },
        }
    }

    fn apply(props: &WallProps, wall_id: uuid::Uuid, project: &mut Project) {
        if let Some(wall) = project.walls.iter_mut().find(|w| w.id == wall_id) {
            wall.thickness = props.thickness;
            wall.height_start = props.height_start;
            wall.height_end = props.height_end;
        }
    }
}

impl Command for ModifyWallCommand {
    fn execute(&mut self, project: &mut Project) {
        Self::apply(&self.new, self.wall_id, project);
    }

    fn undo(&mut self, project: &mut Project) {
        Self::apply(&self.old, self.wall_id, project);
    }

    fn description(&self) -> &str {
        "Изменить стену"
    }
}

// --- Opening commands ---

pub struct AddOpeningCommand {
    pub opening: Opening,
}

impl Command for AddOpeningCommand {
    fn execute(&mut self, project: &mut Project) {
        if let Some(wid) = self.opening.wall_id {
            if let Some(wall) = project.walls.iter_mut().find(|w| w.id == wid) {
                if !wall.openings.contains(&self.opening.id) {
                    wall.openings.push(self.opening.id);
                }
            }
        }
        project.openings.push(self.opening.clone());
    }

    fn undo(&mut self, project: &mut Project) {
        if let Some(wid) = self.opening.wall_id {
            if let Some(wall) = project.walls.iter_mut().find(|w| w.id == wid) {
                wall.openings.retain(|id| *id != self.opening.id);
            }
        }
        project.openings.retain(|o| o.id != self.opening.id);
    }

    fn description(&self) -> &str {
        "Добавить проём"
    }
}

pub struct RemoveOpeningCommand {
    opening: Opening,
}

impl RemoveOpeningCommand {
    pub fn new(opening_id: uuid::Uuid, project: &Project) -> Option<Self> {
        let opening = project.openings.iter().find(|o| o.id == opening_id)?.clone();
        Some(Self { opening })
    }
}

impl Command for RemoveOpeningCommand {
    fn execute(&mut self, project: &mut Project) {
        if let Some(wid) = self.opening.wall_id {
            if let Some(wall) = project.walls.iter_mut().find(|w| w.id == wid) {
                wall.openings.retain(|id| *id != self.opening.id);
            }
        }
        project.openings.retain(|o| o.id != self.opening.id);
    }

    fn undo(&mut self, project: &mut Project) {
        if let Some(wid) = self.opening.wall_id {
            if let Some(wall) = project.walls.iter_mut().find(|w| w.id == wid) {
                if !wall.openings.contains(&self.opening.id) {
                    wall.openings.push(self.opening.id);
                }
            }
        }
        project.openings.push(self.opening.clone());
    }

    fn description(&self) -> &str {
        "Удалить проём"
    }
}

pub struct ModifyOpeningCommand {
    opening_id: uuid::Uuid,
    old_kind: OpeningKind,
    new_kind: OpeningKind,
}

impl ModifyOpeningCommand {
    pub fn from_values(opening_id: uuid::Uuid, old_kind: OpeningKind, new_kind: OpeningKind) -> Self {
        Self { opening_id, old_kind, new_kind }
    }

    fn apply_kind(kind: &OpeningKind, opening_id: uuid::Uuid, project: &mut Project) {
        if let Some(opening) = project.openings.iter_mut().find(|o| o.id == opening_id) {
            opening.kind = kind.clone();
        }
    }
}

impl Command for ModifyOpeningCommand {
    fn execute(&mut self, project: &mut Project) {
        Self::apply_kind(&self.new_kind, self.opening_id, project);
    }

    fn undo(&mut self, project: &mut Project) {
        Self::apply_kind(&self.old_kind, self.opening_id, project);
    }

    fn description(&self) -> &str {
        "Изменить проём"
    }
}

pub struct History {
    undo_stack: Vec<Box<dyn Command>>,
    redo_stack: Vec<Box<dyn Command>>,
    /// Monotonically increasing counter, bumped on every push/undo/redo.
    pub version: u64,
}

impl History {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            version: 0,
        }
    }

    /// Execute a command and push it onto the undo stack. Clears the redo stack.
    pub fn push(&mut self, mut cmd: Box<dyn Command>, project: &mut Project) {
        cmd.execute(project);
        self.undo_stack.push(cmd);
        self.redo_stack.clear();
        self.version += 1;
    }

    /// Push a command whose effect has already been applied (e.g., by DragValue mutation).
    /// The command is placed on the undo stack without calling execute().
    pub fn push_already_applied(&mut self, cmd: Box<dyn Command>) {
        self.undo_stack.push(cmd);
        self.redo_stack.clear();
        self.version += 1;
    }

    /// Undo the last command. Returns true if something was undone.
    pub fn undo(&mut self, project: &mut Project) -> bool {
        if let Some(mut cmd) = self.undo_stack.pop() {
            cmd.undo(project);
            self.redo_stack.push(cmd);
            self.version += 1;
            true
        } else {
            false
        }
    }

    /// Redo the last undone command. Returns true if something was redone.
    pub fn redo(&mut self, project: &mut Project) -> bool {
        if let Some(mut cmd) = self.redo_stack.pop() {
            cmd.execute(project);
            self.undo_stack.push(cmd);
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
}
