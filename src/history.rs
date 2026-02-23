use crate::model::{Opening, OpeningKind, Point2D, Project, SideData, Wall, WallSide};

pub trait Command {
    fn execute(&mut self, project: &mut Project);
    fn undo(&mut self, project: &mut Project);
    fn description(&self) -> &str;
}

// --- Wall commands ---

pub struct AddWallCommand {
    pub wall: Wall,
    /// Junction created on another wall's side at the wall's END point.
    /// (target_wall_id, side, t)
    pub junction_target: Option<(uuid::Uuid, WallSide, f64)>,
    /// Junction created on another wall's side at the wall's START point.
    /// (target_wall_id, side, t)
    pub start_junction_target: Option<(uuid::Uuid, WallSide, f64)>,
}

impl Command for AddWallCommand {
    fn execute(&mut self, project: &mut Project) {
        // Add end-point junction to target wall
        if let Some((target_id, side, t)) = self.junction_target {
            if let Some(target) = project.walls.iter_mut().find(|w| w.id == target_id) {
                let side_data = match side {
                    WallSide::Left => &mut target.left_side,
                    WallSide::Right => &mut target.right_side,
                };
                side_data.add_junction(self.wall.id, t);
            }
        }
        // Add start-point junction to target wall
        if let Some((target_id, side, t)) = self.start_junction_target {
            if let Some(target) = project.walls.iter_mut().find(|w| w.id == target_id) {
                let side_data = match side {
                    WallSide::Left => &mut target.left_side,
                    WallSide::Right => &mut target.right_side,
                };
                side_data.add_junction(self.wall.id, t);
            }
        }
        project.walls.push(self.wall.clone());
    }

    fn undo(&mut self, project: &mut Project) {
        // Remove end-point junction from target wall
        if let Some((target_id, side, _)) = self.junction_target {
            if let Some(target) = project.walls.iter_mut().find(|w| w.id == target_id) {
                let side_data = match side {
                    WallSide::Left => &mut target.left_side,
                    WallSide::Right => &mut target.right_side,
                };
                side_data.remove_junction(self.wall.id);
            }
        }
        // Remove start-point junction from target wall
        if let Some((target_id, side, _)) = self.start_junction_target {
            if let Some(target) = project.walls.iter_mut().find(|w| w.id == target_id) {
                let side_data = match side {
                    WallSide::Left => &mut target.left_side,
                    WallSide::Right => &mut target.right_side,
                };
                side_data.remove_junction(self.wall.id);
            }
        }
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
    /// Junctions that other walls had referencing this wall.
    /// Stored so they can be restored on undo.
    /// (host_wall_id, side, t)
    removed_junctions: Vec<(uuid::Uuid, WallSide, f64)>,
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
        // Find junctions on other walls that reference this wall
        let mut removed_junctions = Vec::new();
        for other in &project.walls {
            if other.id == wall_id {
                continue;
            }
            for j in &other.left_side.junctions {
                if j.wall_id == wall_id {
                    removed_junctions.push((other.id, WallSide::Left, j.t));
                }
            }
            for j in &other.right_side.junctions {
                if j.wall_id == wall_id {
                    removed_junctions.push((other.id, WallSide::Right, j.t));
                }
            }
        }
        Some(Self { wall, openings, removed_junctions })
    }
}

impl Command for RemoveWallCommand {
    fn execute(&mut self, project: &mut Project) {
        // Detach openings (set fallback_position, clear wall_id) instead of removing
        let wall = &self.wall;
        let wall_len = wall.length();
        for o in &self.openings {
            if let Some(po) = project.openings.iter_mut().find(|po| po.id == o.id) {
                if wall_len > 0.0 {
                    let t = po.offset_along_wall / wall_len;
                    po.fallback_position = Some(Point2D::new(
                        wall.start.x + (wall.end.x - wall.start.x) * t,
                        wall.start.y + (wall.end.y - wall.start.y) * t,
                    ));
                }
                po.wall_id = None;
            }
        }
        // Remove junctions referencing this wall from other walls
        for other in &mut project.walls {
            other.left_side.remove_junction(self.wall.id);
            other.right_side.remove_junction(self.wall.id);
        }
        project.walls.retain(|w| w.id != self.wall.id);
    }

    fn undo(&mut self, project: &mut Project) {
        project.walls.push(self.wall.clone());
        // Re-attach openings to the restored wall
        for o in &self.openings {
            if let Some(po) = project.openings.iter_mut().find(|po| po.id == o.id) {
                po.wall_id = o.wall_id;
                po.fallback_position = None;
            }
        }
        // Restore junctions on other walls
        for &(host_id, side, t) in &self.removed_junctions {
            if let Some(host) = project.walls.iter_mut().find(|w| w.id == host_id) {
                let side_data = match side {
                    WallSide::Left => &mut host.left_side,
                    WallSide::Right => &mut host.right_side,
                };
                side_data.add_junction(self.wall.id, t);
            }
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
pub struct WallProps {
    pub thickness: f64,
    pub left_side: SideData,
    pub right_side: SideData,
}

impl ModifyWallCommand {
    pub fn new(wall_id: uuid::Uuid, old: WallProps, new: WallProps) -> Self {
        Self { wall_id, old, new }
    }

    fn apply(props: &WallProps, wall_id: uuid::Uuid, project: &mut Project) {
        if let Some(wall) = project.walls.iter_mut().find(|w| w.id == wall_id) {
            wall.thickness = props.thickness;
            wall.left_side = props.left_side.clone();
            wall.right_side = props.right_side.clone();
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
