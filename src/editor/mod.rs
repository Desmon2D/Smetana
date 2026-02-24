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

/// State for the Room tool: collecting points for a contour.
#[derive(Default)]
pub struct RoomToolState {
    /// Points collected so far for the room contour.
    pub points: Vec<Uuid>,
    /// Whether we are building a cutout (after room is created).
    pub building_cutout: bool,
}

/// State for polygon-based tools (Wall, Door, Window): collecting points.
#[derive(Default)]
pub struct PolygonToolState {
    /// Points collected so far for the polygon.
    pub points: Vec<Uuid>,
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
    pub room_tool: RoomToolState,
    pub polygon_tool: PolygonToolState,
    pub visibility: VisibilityMode,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            active_tool: Tool::Select,
            selection: Selection::None,
            canvas: Canvas::default(),
            room_tool: RoomToolState::default(),
            polygon_tool: PolygonToolState::default(),
            visibility: VisibilityMode::All,
        }
    }
}
