use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::model::{Point2D, Room, Wall, WallSide};

/// Tolerance for merging vertices (in mm).
const MERGE_EPSILON: f64 = 5.0;

/// A vertex in the wall graph, representing a merged endpoint.
#[derive(Debug, Clone)]
pub struct GraphVertex {
    pub position: Point2D,
    /// Adjacent edges sorted by angle from this vertex.
    /// Each entry: (neighbor_vertex_index, wall_id, angle_radians)
    pub edges: Vec<(usize, Uuid, f64)>,
}

/// The planar wall graph: merged vertices and adjacency with sorted angles.
#[derive(Debug, Clone)]
pub struct WallGraph {
    pub vertices: Vec<GraphVertex>,
}

impl WallGraph {
    /// Build a planar graph from a list of walls.
    ///
    /// 1. Collect all wall endpoints.
    /// 2. Merge points within `MERGE_EPSILON` into unique vertices.
    /// 3. Create edges (bidirectional) for each wall.
    /// 4. Sort each vertex's adjacency list by outgoing angle.
    pub fn build(walls: &[Wall]) -> Self {
        if walls.is_empty() {
            return WallGraph {
                vertices: Vec::new(),
            };
        }

        // Step 1+2: Collect and merge vertices
        let mut positions: Vec<Point2D> = Vec::new();
        let mut point_to_vertex: HashMap<(Uuid, bool), usize> = HashMap::new(); // (wall_id, is_end) -> vertex index

        for wall in walls {
            let start_idx = find_or_insert_vertex(&mut positions, wall.start);
            let end_idx = find_or_insert_vertex(&mut positions, wall.end);
            point_to_vertex.insert((wall.id, false), start_idx);
            point_to_vertex.insert((wall.id, true), end_idx);
        }

        // Step 3: Build adjacency lists
        let mut adjacency: Vec<Vec<(usize, Uuid, f64)>> = vec![Vec::new(); positions.len()];

        for wall in walls {
            let start_idx = point_to_vertex[&(wall.id, false)];
            let end_idx = point_to_vertex[&(wall.id, true)];

            // Skip degenerate walls (zero-length after merging)
            if start_idx == end_idx {
                continue;
            }

            // Forward edge: start -> end
            let angle_fwd = angle_between(positions[start_idx], positions[end_idx]);
            adjacency[start_idx].push((end_idx, wall.id, angle_fwd));

            // Backward edge: end -> start
            let angle_bwd = angle_between(positions[end_idx], positions[start_idx]);
            adjacency[end_idx].push((start_idx, wall.id, angle_bwd));
        }

        // Step 4: Sort each vertex's edges by angle
        for adj in &mut adjacency {
            adj.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));
        }

        // Build the graph
        let vertices = positions
            .into_iter()
            .zip(adjacency)
            .map(|(pos, edges)| GraphVertex {
                position: pos,
                edges,
            })
            .collect();

        WallGraph { vertices }
    }

    /// Get the vertex index for a wall's start or end point.
    pub fn vertex_index_for_wall(&self, walls: &[Wall], wall_id: Uuid, is_end: bool) -> Option<usize> {
        let wall = walls.iter().find(|w| w.id == wall_id)?;
        let point = if is_end { wall.end } else { wall.start };
        self.vertices
            .iter()
            .position(|v| v.position.distance_to(point) < MERGE_EPSILON)
    }

    /// Find all minimal cycles (faces) in the planar graph.
    ///
    /// Uses the minimum-angle traversal: for each directed edge (u→v),
    /// find the next edge at v by picking the one immediately after the
    /// reverse direction (v→u) in the sorted adjacency list (next CCW turn).
    /// Follow this chain until returning to the start.
    ///
    /// Returns cycles as lists of `DirectedEdge`. The outer boundary
    /// (largest area) is excluded.
    pub fn find_minimal_cycles(&self) -> Vec<Vec<DirectedEdge>> {
        let mut visited: HashSet<(usize, usize)> = HashSet::new();
        let mut cycles: Vec<Vec<DirectedEdge>> = Vec::new();

        for u in 0..self.vertices.len() {
            for &(v, wall_id, _) in &self.vertices[u].edges {
                if visited.contains(&(u, v)) {
                    continue;
                }

                // Trace a cycle starting from directed edge u→v
                let mut cycle = Vec::new();
                let mut cur_from = u;
                let mut cur_to = v;
                let mut cur_wall = wall_id;
                let mut valid = true;

                loop {
                    if visited.contains(&(cur_from, cur_to)) {
                        valid = false;
                        break;
                    }
                    visited.insert((cur_from, cur_to));
                    cycle.push(DirectedEdge {
                        from: cur_from,
                        to: cur_to,
                        wall_id: cur_wall,
                    });

                    // At cur_to, find the next edge: the one just after
                    // the reverse edge (cur_to→cur_from) in CCW order.
                    match self.next_edge_ccw(cur_to, cur_from) {
                        Some((next_to, next_wall)) => {
                            cur_from = cur_to;
                            cur_to = next_to;
                            cur_wall = next_wall;
                        }
                        None => {
                            valid = false;
                            break;
                        }
                    }

                    // Cycle closed?
                    if cur_from == u && cur_to == v {
                        break;
                    }

                    // Safety: prevent infinite loops
                    if cycle.len() > self.vertices.len() * 2 {
                        valid = false;
                        break;
                    }
                }

                if valid && cycle.len() >= 3 {
                    cycles.push(cycle);
                }
            }
        }

        // Exclude the outer boundary (largest absolute area)
        if cycles.len() > 1 {
            let mut max_area = 0.0_f64;
            let mut max_idx = 0;
            for (i, cycle) in cycles.iter().enumerate() {
                let area = self.signed_area(cycle).abs();
                if area > max_area {
                    max_area = area;
                    max_idx = i;
                }
            }
            cycles.remove(max_idx);
        }

        cycles
    }

    /// Given that we arrived at vertex `at` from vertex `from_vertex`,
    /// find the next outgoing edge by choosing the one immediately after
    /// the reverse direction in the CCW-sorted adjacency list.
    ///
    /// The reverse edge (at→from_vertex) has some angle θ. We want the
    /// next edge in the sorted list after this reverse entry.
    fn next_edge_ccw(&self, at: usize, from_vertex: usize) -> Option<(usize, Uuid)> {
        let edges = &self.vertices[at].edges;
        if edges.is_empty() {
            return None;
        }

        // Find the index of the reverse edge (at→from_vertex) in the adjacency
        let reverse_idx = edges.iter().position(|&(nb, _, _)| nb == from_vertex)?;

        // The next edge in CCW order is the one just after the reverse,
        // wrapping around. This gives the smallest left turn.
        let next_idx = (reverse_idx + 1) % edges.len();
        let (next_to, next_wall, _) = edges[next_idx];
        Some((next_to, next_wall))
    }

    /// Compute the signed area of a cycle using the Shoelace formula.
    /// Positive = CCW winding, Negative = CW winding.
    pub fn signed_area(&self, cycle: &[DirectedEdge]) -> f64 {
        let mut area = 0.0;
        for edge in cycle {
            let p1 = self.vertices[edge.from].position;
            let p2 = self.vertices[edge.to].position;
            area += p1.x * p2.y - p2.x * p1.y;
        }
        area / 2.0
    }

    /// Detect rooms from the wall graph.
    ///
    /// Finds minimal cycles, determines wall sides, and returns Room structs.
    /// Room names are auto-generated as "Комната 1", "Комната 2", etc.
    pub fn detect_rooms(&self, walls: &[Wall]) -> Vec<Room> {
        let cycles = self.find_minimal_cycles();
        let mut rooms = Vec::new();

        for (i, cycle) in cycles.iter().enumerate() {
            let signed_area = self.signed_area(cycle);
            // CCW winding (positive area): room interior is to the LEFT of travel direction
            // CW winding (negative area): room interior is to the RIGHT of travel direction
            let interior_is_left = signed_area > 0.0;

            let mut wall_ids = Vec::new();
            let mut wall_sides = Vec::new();

            for edge in cycle {
                let wall_id = edge.wall_id;
                // Avoid duplicate wall IDs in the same room
                // (shouldn't happen in valid minimal cycles, but be safe)
                if wall_ids.contains(&wall_id) {
                    continue;
                }

                // Determine if this directed edge goes in the same direction as the wall
                let wall = match walls.iter().find(|w| w.id == wall_id) {
                    Some(w) => w,
                    None => continue,
                };

                let from_pos = self.vertices[edge.from].position;
                let forward = from_pos.distance_to(wall.start) < MERGE_EPSILON;

                // Wall side determination:
                // "Left" = left side when looking from wall.start to wall.end
                //
                // If edge is forward (same as wall direction):
                //   interior_is_left => Left side faces interior
                //   !interior_is_left => Right side faces interior
                // If edge is backward (opposite to wall direction):
                //   interior_is_left => Right side faces interior (left of travel = right of wall)
                //   !interior_is_left => Left side faces interior
                let side = match (forward, interior_is_left) {
                    (true, true) | (false, false) => WallSide::Left,
                    (true, false) | (false, true) => WallSide::Right,
                };

                wall_ids.push(wall_id);
                wall_sides.push(side);
            }

            if wall_ids.len() >= 3 {
                rooms.push(Room::new(
                    format!("Комната {}", i + 1),
                    wall_ids,
                    wall_sides,
                ));
            }
        }

        rooms
    }
}

/// Computed metrics for a room.
#[derive(Debug, Clone)]
pub struct RoomMetrics {
    /// Inner polygon vertices (wall centerlines offset inward by half-thickness)
    pub inner_polygon: Vec<Point2D>,
    /// Floor area in mm²
    pub area: f64,
    /// Inner perimeter in mm
    pub perimeter: f64,
}

/// Compute the inner polygon, area, and perimeter for a room.
///
/// For each wall in the room's contour, offset the wall centerline inward
/// by half-thickness on the room-facing side. Then intersect consecutive
/// offset lines to get the inner polygon vertices.
pub fn compute_room_metrics(room: &Room, walls: &[Wall]) -> Option<RoomMetrics> {
    if room.wall_ids.len() < 3 {
        return None;
    }

    // Build offset lines for each wall
    let mut offset_segments: Vec<(Point2D, Point2D)> = Vec::new();

    for (i, wall_id) in room.wall_ids.iter().enumerate() {
        let wall = walls.iter().find(|w| w.id == *wall_id)?;
        let side = room.wall_sides[i];
        let half_t = wall.thickness / 2.0;

        // Wall direction vector
        let dx = wall.end.x - wall.start.x;
        let dy = wall.end.y - wall.start.y;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1e-6 {
            return None;
        }

        // Unit normal: perpendicular to wall direction
        // Left normal (when looking start→end): (-dy, dx) / len
        // Right normal: (dy, -dx) / len
        let (nx, ny) = match side {
            WallSide::Left => (-dy / len, dx / len),
            WallSide::Right => (dy / len, -dx / len),
        };

        // Offset the wall centerline inward by half-thickness
        let offset_start = Point2D::new(wall.start.x + nx * half_t, wall.start.y + ny * half_t);
        let offset_end = Point2D::new(wall.end.x + nx * half_t, wall.end.y + ny * half_t);

        offset_segments.push((offset_start, offset_end));
    }

    // Intersect consecutive offset lines to get inner polygon vertices
    let n = offset_segments.len();
    let mut inner_polygon = Vec::with_capacity(n);

    for i in 0..n {
        let j = (i + 1) % n;
        let (a1, a2) = offset_segments[i];
        let (b1, b2) = offset_segments[j];

        match line_intersection(a1, a2, b1, b2) {
            Some(pt) => inner_polygon.push(pt),
            // If lines are parallel, use the endpoint of the first segment
            None => inner_polygon.push(offset_segments[i].1),
        }
    }

    // Compute area using Shoelace formula
    let mut area = 0.0;
    for i in 0..inner_polygon.len() {
        let j = (i + 1) % inner_polygon.len();
        let p1 = inner_polygon[i];
        let p2 = inner_polygon[j];
        area += p1.x * p2.y - p2.x * p1.y;
    }
    let area = (area / 2.0).abs();

    // Compute perimeter
    let mut perimeter = 0.0;
    for i in 0..inner_polygon.len() {
        let j = (i + 1) % inner_polygon.len();
        perimeter += inner_polygon[i].distance_to(inner_polygon[j]);
    }

    Some(RoomMetrics {
        inner_polygon,
        area,
        perimeter,
    })
}

/// Intersect two infinite lines defined by points (a1,a2) and (b1,b2).
/// Returns None if lines are parallel.
fn line_intersection(a1: Point2D, a2: Point2D, b1: Point2D, b2: Point2D) -> Option<Point2D> {
    let d1x = a2.x - a1.x;
    let d1y = a2.y - a1.y;
    let d2x = b2.x - b1.x;
    let d2y = b2.y - b1.y;

    let denom = d1x * d2y - d1y * d2x;
    if denom.abs() < 1e-10 {
        return None;
    }

    let t = ((b1.x - a1.x) * d2y - (b1.y - a1.y) * d2x) / denom;
    Some(Point2D::new(a1.x + t * d1x, a1.y + t * d1y))
}

/// A directed edge in the wall graph.
#[derive(Debug, Clone)]
pub struct DirectedEdge {
    pub from: usize,
    pub to: usize,
    pub wall_id: Uuid,
}

/// Compute the angle in radians from point `from` to point `to`.
/// Returns a value in [-π, π].
fn angle_between(from: Point2D, to: Point2D) -> f64 {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    dy.atan2(dx)
}

/// Find an existing vertex within MERGE_EPSILON, or insert a new one.
/// Returns the vertex index.
fn find_or_insert_vertex(positions: &mut Vec<Point2D>, point: Point2D) -> usize {
    for (i, existing) in positions.iter().enumerate() {
        if existing.distance_to(point) < MERGE_EPSILON {
            return i;
        }
    }
    positions.push(point);
    positions.len() - 1
}
