//! World-space hit-testing. Priority: openings > walls > rooms.

use cad_core::{Building, Floor, Opening, Point2D};

use crate::document::Selection;

/// Snap `world` to the nearest grid-axis intersection / line / wall endpoint
/// within `tol` (world mm). Returns the snapped point (or the input if none).
pub fn snap(building: &Building, floor: Option<&Floor>, world: [f64; 2], tol: f64) -> [f64; 2] {
    let xs: Vec<f64> = building.grid.x_axes.iter().map(|a| a.position).collect();
    let ys: Vec<f64> = building.grid.y_axes.iter().map(|a| a.position).collect();

    let nearest = |vals: &[f64], v: f64| -> Option<f64> {
        vals.iter()
            .copied()
            .filter(|g| (g - v).abs() <= tol)
            .min_by(|a, b| (a - v).abs().total_cmp(&(b - v).abs()))
    };

    // Grid snap (per-axis). Track the resulting 2D distance so a wall endpoint
    // only wins when it is genuinely closer than the grid snap.
    let mut out = world;
    if let Some(gx) = nearest(&xs, world[0]) {
        out[0] = gx;
    }
    if let Some(gy) = nearest(&ys, world[1]) {
        out[1] = gy;
    }
    let mut best = (out[0] - world[0]).hypot(out[1] - world[1]);

    // 壁端点＝グラフのノードへスナップ。
    if let Some(f) = floor {
        for n in &f.nodes {
            let d = (n.point.x - world[0]).hypot(n.point.y - world[1]);
            if d <= tol && d < best {
                best = d;
                out = [n.point.x, n.point.y];
            }
        }
    }
    out
}

/// Pick the top-most entity at `world` (mm). `tol` is an extra world-space
/// padding (mm) so thin entities are easy to click.
pub fn pick(floor: &Floor, world: [f64; 2], tol: f64) -> Selection {
    for o in &floor.openings {
        if let Some(w) = floor.walls.iter().find(|w| w.id == o.wall_id)
            && let Some((a, b)) = floor.wall_endpoints(w)
            && hits_opening([a.x, a.y], [b.x, b.y], w.thickness, o, world, tol)
        {
            return Selection::Opening(o.id);
        }
    }
    for w in &floor.walls {
        if let Some((a, b)) = floor.wall_endpoints(w)
            && hits_wall([a.x, a.y], [b.x, b.y], w.thickness, world, tol)
        {
            return Selection::Wall(w.id);
        }
    }
    // 面は1回だけ導出。クリックを含む**最小**面の部屋を選ぶ（穴の内側の部屋が外側に勝つ）。
    // 穴の内部はその面に含めない（塗り・面積と一致、判定は cad_core と共有）。
    let wp = Point2D::new(world[0], world[1]);
    let mut best: Option<(Selection, f64)> = None;
    for (r, face) in floor.rooms_faces() {
        if let Some(f) = face
            && cad_core::topology::face_contains(&f, wp)
            && best.as_ref().is_none_or(|(_, a)| f.area < *a)
        {
            best = Some((Selection::Room(r.id), f.area));
        }
    }
    if let Some((sel, _)) = best {
        return sel;
    }
    Selection::None
}

fn hits_wall(a: [f64; 2], b: [f64; 2], thickness: f64, p: [f64; 2], tol: f64) -> bool {
    let (d, t) = dist_to_segment(p, a, b);
    (0.0..=1.0).contains(&t) && d <= thickness / 2.0 + tol
}

fn hits_opening(
    a: [f64; 2],
    b: [f64; 2],
    thickness: f64,
    o: &Opening,
    p: [f64; 2],
    tol: f64,
) -> bool {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let len = (dx * dx + dy * dy).sqrt();
    if len < 0.1 {
        return false;
    }
    let (ux, uy) = (dx / len, dy / len);
    // Projection along the wall centerline, and perpendicular distance.
    let rx = p[0] - a[0];
    let ry = p[1] - a[1];
    let along = rx * ux + ry * uy;
    let perp = (rx * -uy + ry * ux).abs();
    (along - o.position).abs() <= o.width / 2.0 + tol && perp <= thickness / 2.0 + tol
}

/// Distance from `p` to segment `a`-`b`, plus the clamped projection parameter t in [0,1].
fn dist_to_segment(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> (f64, f64) {
    let (abx, aby) = (b[0] - a[0], b[1] - a[1]);
    let len2 = abx * abx + aby * aby;
    if len2 < 1e-9 {
        return (hypot(p[0] - a[0], p[1] - a[1]), 0.0);
    }
    let t = ((p[0] - a[0]) * abx + (p[1] - a[1]) * aby) / len2;
    let tc = t.clamp(0.0, 1.0);
    let proj = [a[0] + abx * tc, a[1] + aby * tc];
    (hypot(p[0] - proj[0], p[1] - proj[1]), t)
}

fn hypot(x: f64, y: f64) -> f64 {
    (x * x + y * y).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::sample;

    #[test]
    fn picks_the_right_entity() {
        let b = sample::building();
        let f = &b.floors[0];

        // Inside LDK, away from any wall.
        assert!(matches!(
            pick(f, [2500.0, 4000.0], 50.0),
            Selection::Room(_)
        ));
        // On the south wall centerline, clear of the window (which spans ~1700..3300).
        assert!(matches!(pick(f, [8000.0, 0.0], 50.0), Selection::Wall(_)));
        // On the south wall at the window position -> opening wins over wall.
        assert!(matches!(
            pick(f, [2500.0, 0.0], 50.0),
            Selection::Opening(_)
        ));
        // Far outside everything.
        assert!(matches!(pick(f, [50000.0, 50000.0], 50.0), Selection::None));
    }
}
