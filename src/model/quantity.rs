use crate::editor::room_metrics::compute_room_metrics;
use crate::model::{Opening, OpeningKind, Room, UnitType, Wall, WallSide};

/// Total opening area (mm²) for openings attached to a wall.
/// Uses the canonical `wall.openings` ID list.
pub fn opening_area_mm2(wall: &Wall, openings: &[Opening]) -> f64 {
    wall.openings
        .iter()
        .filter_map(|oid| openings.iter().find(|o| o.id == *oid))
        .map(|o| o.kind.height() * o.kind.width())
        .sum()
}

/// Quantity for a whole wall side.
pub fn wall_side_quantity(unit: UnitType, wall: &Wall, side: WallSide, openings: &[Opening]) -> f64 {
    match unit {
        UnitType::Piece => 1.0,
        UnitType::SquareMeter => {
            let side_data = match side {
                WallSide::Left => &wall.left_side,
                WallSide::Right => &wall.right_side,
            };
            let gross = side_data.gross_area();
            let open_area = opening_area_mm2(wall, openings);
            (gross - open_area) / 1_000_000.0
        }
        UnitType::LinearMeter => {
            let side_data = match side {
                WallSide::Left => &wall.left_side,
                WallSide::Right => &wall.right_side,
            };
            side_data.length / 1000.0
        }
    }
}

/// Quantity for one section of a wall side (falls back to whole-side if index out of range).
pub fn wall_section_quantity(
    unit: UnitType,
    wall: &Wall,
    side: WallSide,
    section_index: usize,
    openings: &[Opening],
) -> f64 {
    let side_data = match side {
        WallSide::Left => &wall.left_side,
        WallSide::Right => &wall.right_side,
    };

    if let Some(section) = side_data.sections.get(section_index) {
        match unit {
            UnitType::Piece => 1.0,
            UnitType::SquareMeter => section.gross_area() / 1_000_000.0,
            UnitType::LinearMeter => section.length / 1000.0,
        }
    } else {
        wall_side_quantity(unit, wall, side, openings)
    }
}

/// Quantity for an opening (door or window).
pub fn opening_quantity(unit: UnitType, opening: &Opening) -> f64 {
    match unit {
        UnitType::Piece => 1.0,
        UnitType::SquareMeter => match &opening.kind {
            OpeningKind::Door { height, width } => height * width / 1_000_000.0,
            OpeningKind::Window { height, width, reveal_width, .. } => {
                let reveal_perimeter = 2.0 * height + 2.0 * width;
                reveal_perimeter * reveal_width / 1_000_000.0
            }
        },
        UnitType::LinearMeter => match &opening.kind {
            OpeningKind::Door { height, width } => (2.0 * height + width) / 1000.0,
            OpeningKind::Window { height, width, .. } => {
                (2.0 * height + 2.0 * width) / 1000.0
            }
        },
    }
}

/// Quantity for a room.
pub fn room_quantity(unit: UnitType, room: &Room, walls: &[Wall]) -> f64 {
    match unit {
        UnitType::Piece => 1.0,
        UnitType::SquareMeter => {
            compute_room_metrics(room, walls).map_or(0.0, |m| m.area / 1_000_000.0)
        }
        UnitType::LinearMeter => {
            compute_room_metrics(room, walls).map_or(0.0, |m| m.perimeter / 1000.0)
        }
    }
}
