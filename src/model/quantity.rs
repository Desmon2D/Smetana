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

/// Opening area (mm²) overlapping a parametric section range [t_start, t_end] of a wall.
pub fn section_opening_area(wall: &Wall, openings: &[Opening], t_start: f64, t_end: f64) -> f64 {
    let wall_len = wall.length();
    if wall_len < 0.001 {
        return 0.0;
    }
    let sec_start_mm = t_start * wall_len;
    let sec_end_mm = t_end * wall_len;

    wall.openings
        .iter()
        .filter_map(|oid| openings.iter().find(|o| o.id == *oid))
        .map(|o| {
            let half_w = o.kind.width() / 2.0;
            let open_start = o.offset_along_wall - half_w;
            let open_end = o.offset_along_wall + half_w;
            let overlap = (open_end.min(sec_end_mm) - open_start.max(sec_start_mm)).max(0.0);
            o.kind.height() * overlap
        })
        .sum()
}

/// Net area (mm²) for a specific section of a wall side: gross minus overlapping openings.
pub fn section_net_area(wall: &Wall, side: WallSide, section_index: usize, openings: &[Opening]) -> f64 {
    let side_data = match side {
        WallSide::Left => &wall.left_side,
        WallSide::Right => &wall.right_side,
    };

    let section = match side_data.sections.get(section_index) {
        Some(s) => s,
        None => return 0.0,
    };

    // Build parametric boundaries from junctions
    let mut boundaries = Vec::with_capacity(side_data.junctions.len() + 2);
    boundaries.push(0.0);
    for j in &side_data.junctions {
        boundaries.push(j.t);
    }
    boundaries.push(1.0);

    let t_start = boundaries.get(section_index).copied().unwrap_or(0.0);
    let t_end = boundaries.get(section_index + 1).copied().unwrap_or(1.0);

    let open_area = section_opening_area(wall, openings, t_start, t_end);
    (section.gross_area() - open_area).max(0.0)
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
            let gross = side_data.computed_gross_area();
            let open_area = opening_area_mm2(wall, openings);
            ((gross - open_area) / 1_000_000.0).max(0.0)
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
            UnitType::SquareMeter => section_net_area(wall, side, section_index, openings) / 1_000_000.0,
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
            compute_room_metrics(room, walls).map_or(0.0, |m| m.net_area / 1_000_000.0)
        }
        UnitType::LinearMeter => {
            compute_room_metrics(room, walls).map_or(0.0, |m| m.perimeter / 1000.0)
        }
    }
}
