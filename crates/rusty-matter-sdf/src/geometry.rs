use rusty_matter_model::Vec3;

use crate::{MeshSdfSignMode, SdfError};

#[derive(Clone, Copy, Debug)]
pub(crate) struct Triangle {
    a: Vec3,
    b: Vec3,
    c: Vec3,
    normal: Vec3,
}

impl Triangle {
    pub(crate) fn new(a: Vec3, b: Vec3, c: Vec3) -> Result<Self, SdfError> {
        let normal = (b - a).cross(c - a);
        let length = normal.length();
        if length <= f32::EPSILON {
            return Err(SdfError::DegenerateTriangle);
        }
        Ok(Self {
            a,
            b,
            c,
            normal: normal / length,
        })
    }
}

pub(crate) fn nearest_signed_distance(
    point: Vec3,
    triangles: &[Triangle],
    sign_mode: MeshSdfSignMode,
) -> Result<f32, SdfError> {
    let mut best: Option<(f32, Vec3, Vec3)> = None;
    for triangle in triangles {
        let closest = closest_point_on_triangle(point, triangle.a, triangle.b, triangle.c);
        let distance_squared = point.distance_squared(closest);
        if best.as_ref().map_or(true, |(best_distance, _, _)| {
            distance_squared < *best_distance
        }) {
            best = Some((distance_squared, closest, triangle.normal));
        }
    }

    let Some((distance_squared, closest, normal)) = best else {
        return Err(SdfError::DegenerateTriangle);
    };
    let distance = distance_squared.sqrt();
    let signed = match sign_mode {
        MeshSdfSignMode::UnsignedOnly => distance,
        MeshSdfSignMode::TriangleNormal => {
            if (point - closest).dot(normal) < 0.0 {
                -distance
            } else {
                distance
            }
        }
    };
    Ok(signed)
}

fn closest_point_on_triangle(point: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    let ab = b - a;
    let ac = c - a;
    let ap = point - a;
    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }

    let bp = point - b;
    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }

    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        return a + ab * v;
    }

    let cp = point - c;
    let d5 = ab.dot(cp);
    let d6 = ac.dot(cp);
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }

    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        return a + ac * w;
    }

    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return b + (c - b) * w;
    }

    let denominator = 1.0 / (va + vb + vc);
    let v = vb * denominator;
    let w = vc * denominator;
    a + ab * v + ac * w
}
