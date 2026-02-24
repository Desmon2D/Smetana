use glam::DVec2;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::Project;
use super::geometry::shoelace_area;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub id: Uuid,
    pub name: String,
    /// Outer contour: ordered point IDs forming a closed polygon.
    pub points: Vec<Uuid>,
    /// Cutouts (columns, shafts): each is an ordered list of point IDs.
    #[serde(default)]
    pub cutouts: Vec<Vec<Uuid>>,
}

impl Room {
    pub fn new(name: String, points: Vec<Uuid>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            points,
            cutouts: Vec::new(),
        }
    }

    /// Floor area in mm² (outer contour minus cutouts).
    pub fn floor_area(&self, project: &Project) -> f64 {
        let outer = self.contour_area(&self.points, project);
        let cutout_area: f64 = self
            .cutouts
            .iter()
            .map(|c| self.contour_area(c, project))
            .sum();
        (outer - cutout_area).max(0.0)
    }

    /// Perimeter in mm (sum of outer contour edge distances).
    pub fn perimeter(&self, project: &Project) -> f64 {
        let n = self.points.len();
        (0..n)
            .map(|i| {
                let j = (i + 1) % n;
                project
                    .find_edge(self.points[i], self.points[j])
                    .map(|e| e.distance(&project.points))
                    .unwrap_or_else(|| {
                        // Fallback: compute from coordinates
                        let a = project.point(self.points[i]);
                        let b = project.point(self.points[j]);
                        match (a, b) {
                            (Some(a), Some(b)) => a.position.distance(b.position),
                            _ => 0.0,
                        }
                    })
            })
            .sum()
    }

    fn contour_area(&self, contour: &[Uuid], project: &Project) -> f64 {
        let has_overrides = contour.windows(2).any(|w| {
            project
                .find_edge(w[0], w[1])
                .is_some_and(|e| e.distance_override.is_some() || e.angle_override.is_some())
        }) || (contour.len() >= 2
            && project
                .find_edge(*contour.last().unwrap(), contour[0])
                .is_some_and(|e| e.distance_override.is_some() || e.angle_override.is_some()));

        if has_overrides {
            self.area_from_measurements(contour, project)
        } else {
            self.area_from_coordinates(contour, project)
        }
    }

    fn area_from_coordinates(&self, contour: &[Uuid], project: &Project) -> f64 {
        shoelace_area(&project.resolve_positions(contour))
    }

    fn area_from_measurements(&self, contour: &[Uuid], project: &Project) -> f64 {
        let n = contour.len();
        if n < 3 {
            return 0.0;
        }

        let mut distances = Vec::with_capacity(n);
        let mut angles = Vec::with_capacity(n);

        for i in 0..n {
            let j = (i + 1) % n;
            let edge = match project.find_edge(contour[i], contour[j]) {
                Some(e) => e,
                None => return self.area_from_coordinates(contour, project),
            };
            distances.push(edge.distance(&project.points));

            // Angle at vertex j (between edge i->j and edge j->k)
            let k = (j + 1) % n;
            let next_edge = match project.find_edge(contour[j], contour[k]) {
                Some(e) => e,
                None => return self.area_from_coordinates(contour, project),
            };
            angles.push(edge.angle(next_edge, &project.points));
        }

        // Reconstruct polygon vertices from distances and angles
        let mut vertices = Vec::with_capacity(n);
        vertices.push(DVec2::ZERO);

        let mut cumulative_angle: f64 = 0.0;

        for i in 0..n - 1 {
            cumulative_angle += std::f64::consts::PI - angles[i].to_radians();
            let dir = DVec2::new(cumulative_angle.cos(), cumulative_angle.sin());
            let prev = *vertices.last().unwrap();
            vertices.push(prev + dir * distances[i]);
        }

        shoelace_area(&vertices)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Point;

    /// Build a project with 4 points forming a 2000×3000mm rectangle,
    /// edges for the contour, and a room referencing them.
    fn make_rect_project() -> (Project, Uuid) {
        let mut project = Project::new("test".to_string());

        let ids: Vec<Uuid> = (0..4).map(|_| Uuid::new_v4()).collect();
        let positions = [
            DVec2::new(0.0, 0.0),
            DVec2::new(2000.0, 0.0),
            DVec2::new(2000.0, 3000.0),
            DVec2::new(0.0, 3000.0),
        ];

        for (i, &id) in ids.iter().enumerate() {
            project.points.push(Point {
                id,
                position: positions[i],
                height: 2700.0,
            });
        }

        let point_ids: Vec<Uuid> = ids.clone();
        project.ensure_contour_edges(&point_ids);

        let room = Room::new("Test Room".to_string(), point_ids);
        let room_id = room.id;
        project.rooms.push(room);

        (project, room_id)
    }

    #[test]
    fn test_room_perimeter() {
        let (project, room_id) = make_rect_project();
        let room = project.room(room_id).unwrap();
        let perimeter = room.perimeter(&project);
        // 2×2000 + 2×3000 = 10000
        assert!(
            (perimeter - 10000.0).abs() < 0.01,
            "expected 10000, got {perimeter}"
        );
    }

    #[test]
    fn test_room_floor_area() {
        let (project, room_id) = make_rect_project();
        let room = project.room(room_id).unwrap();
        let area = room.floor_area(&project);
        // 2000×3000 = 6,000,000 mm²
        assert!(
            (area - 6_000_000.0).abs() < 0.01,
            "expected 6000000, got {area}"
        );
    }

    #[test]
    fn test_room_floor_area_with_cutout() {
        let (mut project, room_id) = make_rect_project();

        // Add a 500×500mm cutout inside the room
        let cutout_ids: Vec<Uuid> = (0..4).map(|_| Uuid::new_v4()).collect();
        let cutout_positions = [
            DVec2::new(500.0, 500.0),
            DVec2::new(1000.0, 500.0),
            DVec2::new(1000.0, 1000.0),
            DVec2::new(500.0, 1000.0),
        ];

        for (i, &id) in cutout_ids.iter().enumerate() {
            project.points.push(Point {
                id,
                position: cutout_positions[i],
                height: 2700.0,
            });
        }

        project.ensure_contour_edges(&cutout_ids);

        let room = project.rooms.iter_mut().find(|r| r.id == room_id).unwrap();
        room.cutouts.push(cutout_ids);

        let room = project.room(room_id).unwrap();
        let area = room.floor_area(&project);
        // 6,000,000 - 250,000 = 5,750,000 mm²
        assert!(
            (area - 5_750_000.0).abs() < 0.01,
            "expected 5750000, got {area}"
        );
    }
}
