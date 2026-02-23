use glam::DVec2;
use crate::editor::snap::SnapResult;

/// State machine for the wall drawing tool.
#[derive(Debug, Clone)]
pub enum WallToolState {
    /// Waiting for the user to click the first point.
    Idle,
    /// First point placed; waiting for second click to complete the wall.
    Drawing { start: DVec2 },
}

/// Wall drawing tool: two-click wall creation with chaining.
pub struct WallTool {
    pub state: WallToolState,
    /// Current snapped cursor position (updated every frame for preview).
    pub preview_end: Option<DVec2>,
    /// The very first point of the current chain (for contour closing detection).
    pub chain_start: Option<DVec2>,
    /// The last snap result (for determining T-junction attachment on click).
    pub last_snap: Option<SnapResult>,
    /// Snap result from the first click (start point). Stored separately so that
    /// only the second click's snap produces a junction_target.
    pub start_snap: Option<SnapResult>,
    /// Snap result from the very first click of the chain (preserved across chain
    /// continuations so the closing wall can register a T-junction at chain_start).
    pub chain_start_snap: Option<SnapResult>,
}

impl Default for WallTool {
    fn default() -> Self {
        Self {
            state: WallToolState::Idle,
            preview_end: None,
            chain_start: None,
            last_snap: None,
            start_snap: None,
            chain_start_snap: None,
        }
    }
}

impl WallTool {
    /// Reset the tool to Idle state, clearing chain.
    pub fn reset(&mut self) {
        self.state = WallToolState::Idle;
        self.preview_end = None;
        self.chain_start = None;
        self.last_snap = None;
        self.start_snap = None;
        self.chain_start_snap = None;
    }

    /// Continue chaining from the given endpoint.
    pub fn chain_from(&mut self, point: DVec2) {
        self.state = WallToolState::Drawing { start: point };
        // chain_start stays as the original starting vertex
    }
}
