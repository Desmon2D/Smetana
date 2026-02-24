pub mod canvas;
pub mod snap;

pub use canvas::Canvas;
pub use snap::{snap, snap_to_point};

use uuid::Uuid;

/// The currently active drawing/editing tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Select,
    Point,
    Room,
    Wall,
    Door,
    Window,
    Label,
}

/// What kind of object is currently selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selection {
    None,
    Point(Uuid),
    Edge(Uuid),
    Opening(Uuid),
    Wall(Uuid),
    Room(Uuid),
    Label(Uuid),
}

impl Selection {
    pub fn point(&self) -> Option<Uuid> {
        match self {
            Self::Point(id) => Some(*id),
            _ => None,
        }
    }
    pub fn edge(&self) -> Option<Uuid> {
        match self {
            Self::Edge(id) => Some(*id),
            _ => None,
        }
    }
    pub fn room(&self) -> Option<Uuid> {
        match self {
            Self::Room(id) => Some(*id),
            _ => None,
        }
    }
    pub fn wall(&self) -> Option<Uuid> {
        match self {
            Self::Wall(id) => Some(*id),
            _ => None,
        }
    }
    pub fn opening(&self) -> Option<Uuid> {
        match self {
            Self::Opening(id) => Some(*id),
            _ => None,
        }
    }
    pub fn label(&self) -> Option<Uuid> {
        match self {
            Self::Label(id) => Some(*id),
            _ => None,
        }
    }
}

/// Shared tool state for contour/polygon-based tools (Room, Wall, Door, Window).
#[derive(Default)]
pub struct ToolState {
    /// Points collected so far for the contour/polygon.
    pub points: Vec<Uuid>,
    /// Whether we are building a cutout (Room tool only).
    pub building_cutout: bool,
}

/// Visibility modes controlling which geometry layers are rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisibilityMode {
    /// Everything visible.
    All,
    /// Only points and edges (wireframe).
    Wireframe,
    /// Points and rooms (no wall fills).
    Rooms,
}

impl VisibilityMode {
    pub fn show_room_fills(&self) -> bool {
        matches!(self, Self::All | Self::Rooms)
    }

    pub fn show_wall_fills(&self) -> bool {
        matches!(self, Self::All)
    }

    pub fn show_opening_fills(&self) -> bool {
        matches!(self, Self::All)
    }
}

/// Editor state: active tool, selection, canvas viewport, and tool states.
pub struct EditorState {
    pub active_tool: Tool,
    pub selection: Selection,
    pub canvas: Canvas,
    pub tool_state: ToolState,
    pub visibility: VisibilityMode,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            active_tool: Tool::Select,
            selection: Selection::None,
            canvas: Canvas::default(),
            tool_state: ToolState::default(),
            visibility: VisibilityMode::All,
        }
    }
}
