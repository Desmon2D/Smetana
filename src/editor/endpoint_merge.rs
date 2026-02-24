use glam::DVec2;
use uuid::Uuid;

use crate::model::Wall;

/// Merge wall endpoints that are within `epsilon` distance of each other.
///
/// Returns groups of (merged_position, list_of_(wall_id, is_end_point)).
/// Each group represents a junction where multiple wall endpoints meet.
pub(crate) fn merge_endpoints(walls: &[Wall], epsilon: f64) -> Vec<(DVec2, Vec<(Uuid, bool)>)> {
    let mut groups: Vec<(DVec2, Vec<(Uuid, bool)>)> = Vec::new();

    for wall in walls {
        for &is_end in &[false, true] {
            let pt = if is_end { wall.end } else { wall.start };
            let mut found = false;
            for (jpos, members) in &mut groups {
                if jpos.distance(pt) < epsilon {
                    members.push((wall.id, is_end));
                    found = true;
                    break;
                }
            }
            if !found {
                groups.push((pt, vec![(wall.id, is_end)]));
            }
        }
    }

    groups
}
