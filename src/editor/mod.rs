pub mod canvas;
pub mod opening_tool;
pub mod room_detection;
pub mod snap;
pub mod wall_tool;

pub use canvas::Canvas;
pub use opening_tool::OpeningTool;
pub use room_detection::WallGraph;
pub use snap::{SnapResult, SnapType, snap};
pub use wall_tool::{WallTool, WallToolState};

use uuid::Uuid;

/// The currently active drawing/editing tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorTool {
    Select,
    Wall,
    Door,
    Window,
}

/// What kind of object is currently selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selection {
    None,
    Wall(Uuid),
    Opening(Uuid),
    Room(Uuid),
}

/// Editor state: active tool, selection, and canvas viewport.
pub struct EditorState {
    pub active_tool: EditorTool,
    pub selection: Selection,
    pub canvas: Canvas,
    pub wall_tool: WallTool,
    pub opening_tool: OpeningTool,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            active_tool: EditorTool::Select,
            selection: Selection::None,
            canvas: Canvas::default(),
            wall_tool: WallTool::default(),
            opening_tool: OpeningTool::default(),
        }
    }
}
