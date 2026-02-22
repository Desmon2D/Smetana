/// Triangulate a simple polygon using ear-clipping.
/// Input: vertices in order (CCW or CW).
/// Output: list of triangle index triples [i, j, k] referencing input vertices.
pub fn triangulate(vertices: &[egui::Pos2]) -> Vec<[usize; 3]> {
    let n = vertices.len();
    if n < 3 {
        return Vec::new();
    }
    if n == 3 {
        return vec![[0, 1, 2]];
    }

    let mut result = Vec::new();

    // Build mutable index list
    let mut indices: Vec<usize> = (0..n).collect();

    // Determine winding: if signed area is negative (CW), reverse to make CCW
    let area = signed_area_from_indices(vertices, &indices);
    if area < 0.0 {
        indices.reverse();
    }

    let mut remaining = indices.len();
    let mut fail_count = 0;

    while remaining > 3 {
        let mut ear_found = false;

        for i in 0..remaining {
            let prev = if i == 0 { remaining - 1 } else { i - 1 };
            let next = (i + 1) % remaining;

            let a = vertices[indices[prev]];
            let b = vertices[indices[i]];
            let c = vertices[indices[next]];

            // Must be convex (positive cross product for CCW)
            if cross(a, b, c) <= 0.0 {
                continue;
            }

            // No other vertex must be inside this triangle
            let mut contains_other = false;
            for j in 0..remaining {
                if j == prev || j == i || j == next {
                    continue;
                }
                if point_in_triangle(vertices[indices[j]], a, b, c) {
                    contains_other = true;
                    break;
                }
            }

            if !contains_other {
                result.push([indices[prev], indices[i], indices[next]]);
                indices.remove(i);
                remaining -= 1;
                ear_found = true;
                fail_count = 0;
                break;
            }
        }

        if !ear_found {
            fail_count += 1;
            if fail_count > 1 {
                // Degenerate polygon — give up
                break;
            }
        }
    }

    // Add the final triangle
    if remaining == 3 {
        result.push([indices[0], indices[1], indices[2]]);
    }

    result
}

/// Signed area of a polygon given by indices into the vertex array.
/// Positive = CCW, Negative = CW.
fn signed_area_from_indices(vertices: &[egui::Pos2], indices: &[usize]) -> f32 {
    let mut area = 0.0_f32;
    let n = indices.len();
    for i in 0..n {
        let j = (i + 1) % n;
        let p1 = vertices[indices[i]];
        let p2 = vertices[indices[j]];
        area += p1.x * p2.y - p2.x * p1.y;
    }
    area / 2.0
}

/// 2D cross product of vectors (b-a) and (c-b).
/// Positive means left turn (convex for CCW winding).
fn cross(a: egui::Pos2, b: egui::Pos2, c: egui::Pos2) -> f32 {
    (b.x - a.x) * (c.y - b.y) - (b.y - a.y) * (c.x - b.x)
}

/// Check if point p is inside triangle (a, b, c) using barycentric coordinates.
fn point_in_triangle(p: egui::Pos2, a: egui::Pos2, b: egui::Pos2, c: egui::Pos2) -> bool {
    let d1 = sign(p, a, b);
    let d2 = sign(p, b, c);
    let d3 = sign(p, c, a);

    let has_neg = (d1 < 0.0) || (d2 < 0.0) || (d3 < 0.0);
    let has_pos = (d1 > 0.0) || (d2 > 0.0) || (d3 > 0.0);

    !(has_neg && has_pos)
}

fn sign(p1: egui::Pos2, p2: egui::Pos2, p3: egui::Pos2) -> f32 {
    (p1.x - p3.x) * (p2.y - p3.y) - (p2.x - p3.x) * (p1.y - p3.y)
}
