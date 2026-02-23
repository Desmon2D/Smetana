pub mod canvas;
pub mod opening_tool;
pub mod room_detection;
pub mod room_metrics;
pub mod snap;
pub mod triangulation;
pub mod wall_joints;
mod wall_joints_miter;
pub mod wall_tool;

pub use canvas::Canvas;
pub use opening_tool::OpeningTool;
pub use room_detection::WallGraph;
pub use snap::{SnapResult, SnapType, snap};
pub use wall_tool::{WallTool, WallToolState};

use std::collections::HashMap;
use glam::DVec2;
use uuid::Uuid;

/// The currently active drawing/editing tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorTool {
    Select,
    Wall,
    Door,
    Window,
    Label,
}

/// What kind of object is currently selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selection {
    None,
    Wall(Uuid),
    Opening(Uuid),
    Room(Uuid),
    Label(Uuid),
}

/// Editor state: active tool, selection, and canvas viewport.
pub struct EditorState {
    pub active_tool: EditorTool,
    pub selection: Selection,
    pub canvas: Canvas,
    pub wall_tool: WallTool,
    pub opening_tool: OpeningTool,
    /// Transient: world position for openings with wall_id=None (orphaned or dragged off).
    pub orphan_positions: HashMap<Uuid, DVec2>,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            active_tool: EditorTool::Select,
            selection: Selection::None,
            canvas: Canvas::default(),
            wall_tool: WallTool::default(),
            opening_tool: OpeningTool::default(),
            orphan_positions: HashMap::new(),
        }
    }
}
