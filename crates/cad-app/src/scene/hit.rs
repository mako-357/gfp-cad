//! World-space hit-testing. Priority: openings > walls > rooms.

use cad_core::{Building, Floor, Opening, Wall};

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

    if let Some(f) = floor {
        for w in &f.walls {
            for e in [[w.start.x, w.start.y], [w.end.x, w.end.y]] {
                let d = (e[0] - world[0]).hypot(e[1] - world[1]);
                if d <= tol && d < best {
                    best = d;
                    out = e;
                }
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
            && hits_opening(w, o, world, tol)
        {
            return Selection::Opening(o.id);
        }
    }
    for w in &floor.walls {
        if hits_wall(w, world, tol) {
            return Selection::Wall(w.id);
        }
    }
    for r in &floor.rooms {
        if point_in_polygon(world, &r.boundary) {
            return Selection::Room(r.id);
        }
    }
    Selection::None
}

fn hits_wall(wall: &Wall, p: [f64; 2], tol: f64) -> bool {
    let (d, t) = dist_to_segment(p, [wall.start.x, wall.start.y], [wall.end.x, wall.end.y]);
    (0.0..=1.0).contains(&t) && d <= wall.thickness / 2.0 + tol
}

fn hits_opening(wall: &Wall, o: &Opening, p: [f64; 2], tol: f64) -> bool {
    let (dx, dy) = (wall.end.x - wall.start.x, wall.end.y - wall.start.y);
    let len = (dx * dx + dy * dy).sqrt();
    if len < 0.1 {
        return false;
    }
    let (ux, uy) = (dx / len, dy / len);
    // Projection along the wall centerline, and perpendicular distance.
    let rx = p[0] - wall.start.x;
    let ry = p[1] - wall.start.y;
    let along = rx * ux + ry * uy;
    let perp = (rx * -uy + ry * ux).abs();
    (along - o.position).abs() <= o.width / 2.0 + tol && perp <= wall.thickness / 2.0 + tol
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

fn point_in_polygon(p: [f64; 2], poly: &[cad_core::Point2D]) -> bool {
    if poly.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let (xi, yi) = (poly[i].x, poly[i].y);
        let (xj, yj) = (poly[j].x, poly[j].y);
        if (yi > p[1]) != (yj > p[1]) && p[0] < (xj - xi) * (p[1] - yi) / (yj - yi) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
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
