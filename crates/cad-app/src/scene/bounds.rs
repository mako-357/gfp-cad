//! Single source of truth for the model's world bounding box.
//!
//! Combines the grid extent (via `GridSystem::bounds`) with the active floor's
//! walls and rooms. Used by camera fit, geometry, and labels so the three never
//! drift.

use cad_core::Building;

/// World bbox `[min_x, min_y, max_x, max_y]` (mm). Falls back to a default box
/// when the building is empty.
pub fn model_bounds(building: &Building, floor_idx: usize) -> [f64; 4] {
    let mut b = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
    let mut acc = |x: f64, y: f64| {
        b[0] = b[0].min(x);
        b[1] = b[1].min(y);
        b[2] = b[2].max(x);
        b[3] = b[3].max(y);
    };

    if let Some(g) = building.grid.bounds() {
        acc(g[0], g[1]);
        acc(g[2], g[3]);
    }
    if let Some(floor) = building.floors.get(floor_idx) {
        // 実際に描画される点（壁が参照する端点）から算出。孤児ノードは bbox に含めない。
        for w in &floor.walls {
            if let Some((a, b)) = floor.wall_endpoints(w) {
                acc(a.x, a.y);
                acc(b.x, b.y);
            }
        }
        // 部屋境界は壁から導出されるので壁で既にカバーされる。念のためシード点も含める。
        for r in &floor.rooms {
            acc(r.seed.x, r.seed.y);
        }
    }

    if b[0] > b[2] {
        return [0.0, 0.0, 10000.0, 10000.0];
    }
    b
}
