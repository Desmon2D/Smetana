use std::collections::HashSet;
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

        // Step 1+2: Collect and merge vertices (including junction points)
        let mut positions: Vec<Point2D> = Vec::new();

        // For each wall, collect all vertices along its length:
        // start, junction points (sorted by t), end.
        // wall_vertices[i] = list of vertex indices for wall i, in order from start to end.
        let mut wall_vertices: Vec<(Uuid, Vec<usize>)> = Vec::new();

        for wall in walls {
            let start_idx = find_or_insert_vertex(&mut positions, wall.start);

            // Collect all junction t values from both sides
            let mut junction_ts: Vec<f64> = Vec::new();
            for j in &wall.left_side.junctions {
                junction_ts.push(j.t);
            }
            for j in &wall.right_side.junctions {
                if !junction_ts.iter().any(|&existing| (existing - j.t).abs() < 0.001) {
                    junction_ts.push(j.t);
                }
            }
            junction_ts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

            let mut verts = vec![start_idx];

            // Add intermediate vertices at junction points
            for &t in &junction_ts {
                let pt = Point2D::new(
                    wall.start.x + (wall.end.x - wall.start.x) * t,
                    wall.start.y + (wall.end.y - wall.start.y) * t,
                );
                let idx = find_or_insert_vertex(&mut positions, pt);
                verts.push(idx);
            }

            let end_idx = find_or_insert_vertex(&mut positions, wall.end);
            verts.push(end_idx);

            wall_vertices.push((wall.id, verts));
        }

        // Step 2b: Merge T-junction vertices.
        // Wall endpoints at T-junctions are on the host wall's EDGE (offset by
        // half-thickness), while junction split vertices are on the CENTERLINE.
        // These positions differ by ~half-thickness (e.g. 100mm for a 200mm wall)
        // which exceeds MERGE_EPSILON. Force-merge them so the graph is connected.
        for host_wall in walls {
            for junction in host_wall
                .left_side
                .junctions
                .iter()
                .chain(host_wall.right_side.junctions.iter())
            {
                // Junction vertex on the host wall's centerline
                let junc_pt = Point2D::new(
                    host_wall.start.x
                        + (host_wall.end.x - host_wall.start.x) * junction.t,
                    host_wall.start.y
                        + (host_wall.end.y - host_wall.start.y) * junction.t,
                );
                let junc_idx =
                    match positions.iter().position(|p| p.distance_to(junc_pt) < MERGE_EPSILON) {
                        Some(idx) => idx,
                        None => continue,
                    };

                // Find the connecting wall's endpoint closest to the junction
                let conn_wall = match walls.iter().find(|w| w.id == junction.wall_id) {
                    Some(w) => w,
                    None => continue,
                };
                let start_dist = conn_wall.start.distance_to(junc_pt);
                let end_dist = conn_wall.end.distance_to(junc_pt);
                let conn_endpoint = if start_dist < end_dist {
                    conn_wall.start
                } else {
                    conn_wall.end
                };
                let conn_idx =
                    match positions.iter().position(|p| p.distance_to(conn_endpoint) < MERGE_EPSILON)
                    {
                        Some(idx) => idx,
                        None => continue,
                    };

                if conn_idx == junc_idx {
                    continue;
                }

                // Redirect all references from conn_idx to junc_idx
                for (_, verts) in &mut wall_vertices {
                    for v in verts.iter_mut() {
                        if *v == conn_idx {
                            *v = junc_idx;
                        }
                    }
                }
            }
        }

        // Step 3: Build adjacency lists — each wall segment becomes edges
        let mut adjacency: Vec<Vec<(usize, Uuid, f64)>> = vec![Vec::new(); positions.len()];

        for (wall_id, verts) in &wall_vertices {
            for i in 0..verts.len() - 1 {
                let from = verts[i];
                let to = verts[i + 1];

                if from == to {
                    continue;
                }

                // Forward edge
                let angle_fwd = angle_between(positions[from], positions[to]);
                adjacency[from].push((to, *wall_id, angle_fwd));

                // Backward edge
                let angle_bwd = angle_between(positions[to], positions[from]);
                adjacency[to].push((from, *wall_id, angle_bwd));
            }
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
