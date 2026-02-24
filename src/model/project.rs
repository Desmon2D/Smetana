use glam::DVec2;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{Edge, Opening, Point, Room, Wall};

// --- Label (preserved from old model, inlined here) ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    pub id: Uuid,
    pub text: String,
    pub position: DVec2,
    /// Display font size in points (default 14.0)
    pub font_size: f64,
    /// Rotation in radians (default 0.0)
    pub rotation: f64,
}

impl Label {
    pub fn new(text: String, position: DVec2) -> Self {
        Self {
            id: Uuid::new_v4(),
            text,
            position,
            font_size: 14.0,
            rotation: 0.0,
        }
    }
}

// --- ProjectDefaults ---

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectDefaults {
    /// Default height for new points (mm)
    pub point_height: f64,
    /// Door height (mm)
    pub door_height: f64,
    /// Door width (mm)
    pub door_width: f64,
    /// Window height (mm)
    pub window_height: f64,
    /// Window width (mm)
    pub window_width: f64,
    /// Window sill height (mm)
    pub window_sill_height: f64,
    /// Window reveal width (mm)
    pub window_reveal_width: f64,
}

impl Default for ProjectDefaults {
    fn default() -> Self {
        Self {
            point_height: 2700.0,
            door_height: 2100.0,
            door_width: 900.0,
            window_height: 1400.0,
            window_width: 1200.0,
            window_sill_height: 900.0,
            window_reveal_width: 250.0,
        }
    }
}

// --- Project ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub points: Vec<Point>,
    pub edges: Vec<Edge>,
    pub rooms: Vec<Room>,
    pub walls: Vec<Wall>,
    pub openings: Vec<Opening>,
    #[serde(default)]
    pub labels: Vec<Label>,
    #[serde(default)]
    pub defaults: ProjectDefaults,
}

impl Project {
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            points: Vec::new(),
            edges: Vec::new(),
            rooms: Vec::new(),
            walls: Vec::new(),
            openings: Vec::new(),
            labels: Vec::new(),
            defaults: ProjectDefaults::default(),
        }
    }

    // --- Lookup by ID ---

    pub fn point(&self, id: Uuid) -> Option<&Point> {
        self.points.iter().find(|p| p.id == id)
    }

    pub fn point_mut(&mut self, id: Uuid) -> Option<&mut Point> {
        self.points.iter_mut().find(|p| p.id == id)
    }

    pub fn edge(&self, id: Uuid) -> Option<&Edge> {
        self.edges.iter().find(|e| e.id == id)
    }

    pub fn edge_mut(&mut self, id: Uuid) -> Option<&mut Edge> {
        self.edges.iter_mut().find(|e| e.id == id)
    }

    pub fn room(&self, id: Uuid) -> Option<&Room> {
        self.rooms.iter().find(|r| r.id == id)
    }

    pub fn wall(&self, id: Uuid) -> Option<&Wall> {
        self.walls.iter().find(|w| w.id == id)
    }

    pub fn opening(&self, id: Uuid) -> Option<&Opening> {
        self.openings.iter().find(|o| o.id == id)
    }

    pub fn opening_mut(&mut self, id: Uuid) -> Option<&mut Opening> {
        self.openings.iter_mut().find(|o| o.id == id)
    }

    /// Resolve a list of point IDs to their world-space positions.
    pub fn resolve_positions(&self, ids: &[Uuid]) -> Vec<DVec2> {
        ids.iter()
            .filter_map(|id| self.point(*id).map(|p| p.position))
            .collect()
    }

    // --- Edge lookup (direction-agnostic) ---

    pub fn find_edge(&self, a: Uuid, b: Uuid) -> Option<&Edge> {
        self.edges
            .iter()
            .find(|e| (e.point_a == a && e.point_b == b) || (e.point_a == b && e.point_b == a))
    }

    #[allow(dead_code)]
    pub fn find_edge_mut(&mut self, a: Uuid, b: Uuid) -> Option<&mut Edge> {
        self.edges
            .iter_mut()
            .find(|e| (e.point_a == a && e.point_b == b) || (e.point_a == b && e.point_b == a))
    }

    /// Ensure an edge exists between two points. Returns the edge ID.
    /// If an edge already exists (in either direction), returns its ID.
    pub fn ensure_edge(&mut self, point_a: Uuid, point_b: Uuid) -> Uuid {
        if let Some(edge) = self.find_edge(point_a, point_b) {
            return edge.id;
        }
        let edge = Edge::new(point_a, point_b);
        let id = edge.id;
        self.edges.push(edge);
        id
    }

    /// Ensure all edges exist for a closed contour of points.
    pub fn ensure_contour_edges(&mut self, points: &[Uuid]) {
        for i in 0..points.len() {
            let j = (i + 1) % points.len();
            self.ensure_edge(points[i], points[j]);
        }
    }

    // --- Mutation (cascade delete) ---

    /// Remove a point and cascade-delete all edges, rooms, walls, and openings
    /// that reference it.
    pub fn remove_point(&mut self, id: Uuid) {
        self.edges.retain(|e| e.point_a != id && e.point_b != id);
        self.rooms
            .retain(|r| !r.points.contains(&id) && !r.cutouts.iter().any(|c| c.contains(&id)));
        self.walls.retain(|w| !w.points.contains(&id));
        self.openings.retain(|o| !o.points.contains(&id));
        self.points.retain(|p| p.id != id);
    }

    /// Remove a room by ID.
    pub fn remove_room(&mut self, id: Uuid) {
        self.rooms.retain(|r| r.id != id);
    }

    /// Remove a wall by ID.
    pub fn remove_wall(&mut self, id: Uuid) {
        self.walls.retain(|w| w.id != id);
    }

    /// Remove an opening by ID.
    pub fn remove_opening(&mut self, id: Uuid) {
        self.openings.retain(|o| o.id != id);
    }

    /// Remove a label by ID.
    pub fn remove_label(&mut self, id: Uuid) {
        self.labels.retain(|l| l.id != id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{OpeningKind, Room, Wall};

    #[test]
    fn test_ensure_edge_dedup() {
        let mut project = Project::new("test".to_string());
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        project
            .points
            .push(Point::new(DVec2::new(0.0, 0.0), 2700.0));
        project
            .points
            .push(Point::new(DVec2::new(1000.0, 0.0), 2700.0));
        // Override IDs for lookup
        project.points[0].id = a;
        project.points[1].id = b;

        let id1 = project.ensure_edge(a, b);
        let id2 = project.ensure_edge(a, b);
        let id3 = project.ensure_edge(b, a); // reversed direction
        assert_eq!(id1, id2);
        assert_eq!(id1, id3);
        assert_eq!(project.edges.len(), 1);
    }

    #[test]
    fn test_remove_point_cascades() {
        let mut project = Project::new("test".to_string());

        // Create 4 points forming a square
        let ids: Vec<Uuid> = (0..4).map(|_| Uuid::new_v4()).collect();
        let positions = [
            DVec2::new(0.0, 0.0),
            DVec2::new(1000.0, 0.0),
            DVec2::new(1000.0, 1000.0),
            DVec2::new(0.0, 1000.0),
        ];
        for (i, &id) in ids.iter().enumerate() {
            project.points.push(Point {
                id,
                position: positions[i],
                height: 2700.0,
            });
        }

        // Create edges for the contour
        project.ensure_contour_edges(&ids);
        assert_eq!(project.edges.len(), 4);

        // Create a room, wall, and opening using all 4 points
        let room = Room::new("Room".to_string(), ids.clone());
        project.rooms.push(room);

        let wall = Wall::new(ids.clone());
        project.walls.push(wall);

        let opening = Opening::new(
            ids.clone(),
            OpeningKind::Door {
                height: 2100.0,
                width: 900.0,
            },
        );
        project.openings.push(opening);

        assert_eq!(project.rooms.len(), 1);
        assert_eq!(project.walls.len(), 1);
        assert_eq!(project.openings.len(), 1);

        // Remove one point — should cascade-delete everything referencing it
        project.remove_point(ids[0]);

        assert_eq!(project.points.len(), 3);
        // Edges with point 0: 0-1 and 3-0 should be removed, leaving 1-2 and 2-3
        assert_eq!(project.edges.len(), 2);
        // Room, wall, opening all referenced point 0
        assert_eq!(project.rooms.len(), 0);
        assert_eq!(project.walls.len(), 0);
        assert_eq!(project.openings.len(), 0);
    }

    #[test]
    fn test_remove_room_only() {
        let mut project = Project::new("test".to_string());
        let ids: Vec<Uuid> = (0..3).map(|_| Uuid::new_v4()).collect();
        for &id in &ids {
            project.points.push(Point::new(DVec2::ZERO, 2700.0));
            project.points.last_mut().unwrap().id = id;
        }
        let room = Room::new("R".to_string(), ids.clone());
        let room_id = room.id;
        project.rooms.push(room);

        project.remove_room(room_id);
        assert_eq!(project.rooms.len(), 0);
        // Points should NOT be removed
        assert_eq!(project.points.len(), 3);
    }
}
