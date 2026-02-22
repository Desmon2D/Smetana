use uuid::Uuid;

/// State for the opening (door/window) placement tool.
///
/// Tracks the wall the cursor is currently hovering over
/// and the computed offset along that wall, used for preview rendering.
pub struct OpeningTool {
    /// Wall ID the cursor is currently over (for preview).
    pub hover_wall_id: Option<Uuid>,
    /// Offset along the hovered wall in mm (for preview).
    pub hover_offset: f64,
}

impl Default for OpeningTool {
    fn default() -> Self {
        Self {
            hover_wall_id: None,
            hover_offset: 0.0,
        }
    }
}
