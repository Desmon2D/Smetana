/// Triangulate a simple polygon using earcutr (earcut algorithm).
/// Input: vertices in order (CCW or CW).
/// Output: list of triangle index triples [i, j, k] referencing input vertices.
pub fn triangulate(vertices: &[egui::Pos2]) -> Vec<[usize; 3]> {
    if vertices.len() < 3 {
        return Vec::new();
    }
    let coords: Vec<f32> = vertices.iter().flat_map(|p| [p.x, p.y]).collect();
    let indices = earcutr::earcut(&coords, &[], 2).unwrap_or_default();
    indices.chunks(3).map(|c| [c[0], c[1], c[2]]).collect()
}
