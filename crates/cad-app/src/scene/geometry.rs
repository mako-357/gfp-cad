//! Model -> tessellated GPU geometry. Pure (no winit/wgpu state beyond Vertex).
//!
//! Draw order (painter, no depth test): grid, room fills, walls, openings.

use cad_core::{Building, Floor, Opening, Wall};
use lyon::math::point;
use lyon::path::Path;
use lyon::tessellation::{BuffersBuilder, FillOptions, FillTessellator, FillVertex, VertexBuffers};

use crate::config;
use crate::document::Selection;
use crate::gpu::Vertex;

type Geo = VertexBuffers<Vertex, u32>;

/// Highlight geometry for the current selection (drawn over the model, world space).
pub fn build_overlay(floor: &Floor, sel: Selection) -> Geo {
    let mut g: Geo = VertexBuffers::new();
    match sel {
        Selection::Wall(id) => {
            if let Some(w) = floor.walls.iter().find(|w| w.id == id) {
                wall_rect(&mut g, w, floor, config::SEL);
            }
        }
        Selection::Opening(id) => {
            if let Some(o) = floor.openings.iter().find(|o| o.id == id) {
                opening_rect(&mut g, floor, o, config::SEL);
            }
        }
        Selection::Room(id) => {
            if let Some(r) = floor.rooms.iter().find(|r| r.id == id)
                && let Some(face) = floor.face_at(r.seed)
            {
                fill_face(&mut g, &face, config::SEL); // 穴も切り抜く（塗りと一致）
            }
        }
        Selection::Node(id) => {
            if let Some(n) = floor.nodes.iter().find(|n| n.id == id) {
                // Larger accent square is drawn by `push_node_handles`; here just
                // ensure a visible marker even if handles are off.
                push_square(&mut g, [n.point.x, n.point.y], 60.0, config::NODE_SEL);
            }
        }
        Selection::None => {}
    }
    g
}

/// Grabbable node handles for Select mode: a small square per graph node,
/// with `sel` (if a Node) drawn larger in the accent colour. `hw` is the world
/// half-size (caller passes a screen-constant size via camera scale).
pub fn push_node_handles(g: &mut Geo, floor: &Floor, hw: f64, sel: Selection) {
    let selected = match sel {
        Selection::Node(id) => Some(id),
        _ => None,
    };
    for n in &floor.nodes {
        if Some(n.id) == selected {
            push_square(g, [n.point.x, n.point.y], hw * 1.8, config::NODE_SEL);
        } else {
            push_square(g, [n.point.x, n.point.y], hw, config::NODE);
        }
    }
}

/// A square of half-size `hw` (world mm) centred at `c`.
fn push_square(g: &mut Geo, c: [f64; 2], hw: f64, color: u32) {
    push_quad(
        g,
        [
            [c[0] - hw, c[1] - hw],
            [c[0] + hw, c[1] - hw],
            [c[0] + hw, c[1] + hw],
            [c[0] - hw, c[1] + hw],
        ],
        color,
    );
}

pub fn build(building: &Building, floor_idx: usize) -> Geo {
    let mut g: Geo = VertexBuffers::new();

    let bbox = padded_bounds(building, floor_idx);
    grid(&mut g, building, bbox);

    if let Some(floor) = building.floors.get(floor_idx) {
        // 部屋の面は壁グラフから **1回だけ** 導出（room 毎に再構築しない）。未囲いなら塗らない。
        for (_room, face) in floor.rooms_faces() {
            if let Some(f) = face {
                fill_face(&mut g, &f, config::ROOM_FILL);
            }
        }
        // Interior walls first, then the (brighter) exterior shell on top: where an
        // interior wall shares an L-corner with an exterior wall, both mitre-extend
        // and overlap — drawing the exterior last keeps the shell colour on top.
        for wall in floor.walls.iter().filter(|w| !w.is_exterior) {
            wall_body(&mut g, wall, floor);
        }
        for wall in floor.walls.iter().filter(|w| w.is_exterior) {
            wall_body(&mut g, wall, floor);
        }
        for opening in &floor.openings {
            opening_mark(&mut g, floor, opening);
        }
    }
    g
}

fn grid(g: &mut Geo, building: &Building, bbox: [f64; 4]) {
    let hw = config::GRID_HALF_W;
    for a in &building.grid.x_axes {
        push_line(
            g,
            [a.position, bbox[1]],
            [a.position, bbox[3]],
            hw,
            config::GRID,
        );
    }
    for a in &building.grid.y_axes {
        push_line(
            g,
            [bbox[0], a.position],
            [bbox[2], a.position],
            hw,
            config::GRID,
        );
    }
}

fn wall_body(g: &mut Geo, wall: &Wall, floor: &Floor) {
    let color = if wall.is_exterior {
        config::WALL_EXT
    } else {
        config::WALL_INT
    };
    wall_rect(g, wall, floor, color);
}

/// Push a wall's face rectangle in `color`. An end is extended by half-thickness
/// along the centreline **only where another wall shares that end node** (an
/// L-corner) so orthogonal corners mitre cleanly; a free end keeps its exact
/// length (no stub), and a T-junction end (which meets another wall's *span*,
/// not its node) is not extended (so it can't poke through the shell).
/// Render-only — the exact face (`face_quad`) is what DXF/AutoCAD use.
fn wall_rect(g: &mut Geo, wall: &Wall, floor: &Floor, color: u32) {
    let Some((a, b)) = floor.wall_endpoints(wall) else {
        return;
    };
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 0.1 {
        return;
    }
    let t = wall.thickness / 2.0;
    let (ux, uy) = (dx / len, dy / len); // along centreline
    let (nx, ny) = (-uy * t, ux * t); // perpendicular, half-thickness
    let ext_s = if shares_node(floor, wall, wall.start) {
        t
    } else {
        0.0
    };
    let ext_e = if shares_node(floor, wall, wall.end) {
        t
    } else {
        0.0
    };
    let sx = a.x - ux * ext_s;
    let sy = a.y - uy * ext_s;
    let ex = b.x + ux * ext_e;
    let ey = b.y + uy * ext_e;
    push_quad(
        g,
        [
            [sx + nx, sy + ny],
            [ex + nx, ey + ny],
            [ex - nx, ey - ny],
            [sx - nx, sy - ny],
        ],
        color,
    );
}

/// True if any *other* wall shares the node `node` as one of its endpoints
/// (exact, via `NodeId`) — i.e. an L-corner.
fn shares_node(floor: &Floor, wall: &Wall, node: cad_core::NodeId) -> bool {
    floor
        .walls
        .iter()
        .any(|w| w.id != wall.id && (w.start == node || w.end == node))
}

fn opening_mark(g: &mut Geo, floor: &Floor, opening: &Opening) {
    opening_rect(g, floor, opening, config::OPENING);
}

fn opening_rect(g: &mut Geo, floor: &Floor, opening: &Opening, color: u32) {
    let Some(wall) = floor.walls.iter().find(|w| w.id == opening.wall_id) else {
        return;
    };
    let Some((a, b)) = floor.wall_endpoints(wall) else {
        return;
    };
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len = (dx * dx + dy * dy).sqrt();
    if len < 0.1 {
        return;
    }
    let (ux, uy) = (dx / len, dy / len);
    let (nx, ny) = (-uy, ux);
    let cx = a.x + ux * opening.position;
    let cy = a.y + uy * opening.position;
    let hw = opening.width / 2.0;
    let t = wall.thickness / 2.0 + 10.0; // slightly proud of the wall
    push_quad(
        g,
        [
            [cx - ux * hw + nx * t, cy - uy * hw + ny * t],
            [cx + ux * hw + nx * t, cy + uy * hw + ny * t],
            [cx + ux * hw - nx * t, cy + uy * hw - ny * t],
            [cx - ux * hw - nx * t, cy - uy * hw - ny * t],
        ],
        color,
    );
}

/// Fill a derived face (outer ring minus holes) via lyon. Even-odd fill rule cuts
/// the holes (non-connected inner loops, e.g. a floating closet).
fn fill_face(g: &mut Geo, face: &cad_core::Face, color: u32) {
    let mut builder = Path::builder();
    add_ring(&mut builder, &face.polygon);
    for hole in &face.holes {
        add_ring(&mut builder, hole);
    }
    fill_built(g, builder.build(), color);
}

/// Add one closed ring to a path builder (skips rings with < 3 points).
fn add_ring(builder: &mut lyon::path::path::Builder, ring: &[cad_core::Point2D]) {
    if ring.len() < 3 {
        return;
    }
    builder.begin(point(ring[0].x as f32, ring[0].y as f32));
    for p in &ring[1..] {
        builder.line_to(point(p.x as f32, p.y as f32));
    }
    builder.end(true);
}

fn fill_built(g: &mut Geo, path: Path, color: u32) {
    let mut tess = FillTessellator::new();
    let _ = tess.tessellate_path(
        &path,
        &FillOptions::tolerance(1.0), // lyon default fill rule is even-odd → holes cut
        &mut BuffersBuilder::new(g, |v: FillVertex| Vertex {
            pos: v.position().to_array(),
            color,
        }),
    );
}

/// Append a world-space polyline (open) as thin quads — used for tool previews.
pub fn push_polyline(g: &mut Geo, pts: &[[f64; 2]], hw: f64, color: u32) {
    for seg in pts.windows(2) {
        push_line(g, seg[0], seg[1], hw, color);
    }
}

/// A line segment as a rectangle of half-width `hw` (world mm).
fn push_line(g: &mut Geo, a: [f64; 2], b: [f64; 2], hw: f64, color: u32) {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-6 {
        return;
    }
    let (nx, ny) = (-dy / len * hw, dx / len * hw);
    push_quad(
        g,
        [
            [a[0] + nx, a[1] + ny],
            [b[0] + nx, b[1] + ny],
            [b[0] - nx, b[1] - ny],
            [a[0] - nx, a[1] - ny],
        ],
        color,
    );
}

/// Push a convex quad (CCW or CW) as two triangles.
fn push_quad(g: &mut Geo, corners: [[f64; 2]; 4], color: u32) {
    let base = g.vertices.len() as u32;
    for c in corners {
        g.vertices.push(Vertex {
            pos: [c[0] as f32, c[1] as f32],
            color,
        });
    }
    g.indices
        .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

/// Model bounds with a margin so grid lines extend a little past the plan.
fn padded_bounds(building: &Building, floor_idx: usize) -> [f64; 4] {
    let b = super::bounds::model_bounds(building, floor_idx);
    let pad = 1000.0;
    [b[0] - pad, b[1] - pad, b[2] + pad, b[3] + pad]
}
