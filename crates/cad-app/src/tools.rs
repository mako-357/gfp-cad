//! Editing tools: a small state machine over world-space pointer events.
//!
//! Tools mutate the model only through `Document::edit`, so every action is one
//! undo step. Live (in-progress) geometry is returned as a preview for the overlay.

use cad_core::{Building, Floor, Opening, Point2D, Room};

use crate::document::{Document, Selection};

/// Max distance (mm) from a click to a wall for `AddOpening` to attach a door.
/// ~half a metre — beyond this the click is treated as a miss (no edit).
const OPENING_ATTACH_TOL: f64 = 500.0;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Select,
    DrawWall,
    AddRoom,
    AddOpening,
}

impl Tool {
    pub fn label(self) -> &'static str {
        match self {
            Tool::Select => "選択",
            Tool::DrawWall => "壁",
            Tool::AddRoom => "部屋",
            Tool::AddOpening => "開口",
        }
    }
}

/// Per-tool in-progress state (world mm).
#[derive(Default)]
pub struct ToolState {
    wall_start: Option<[f64; 2]>,
    room_pts: Vec<[f64; 2]>,
}

/// A preview polyline/point the overlay can draw while a tool is mid-action.
pub enum Preview {
    None,
    Segment([f64; 2], [f64; 2]),
    Polyline(Vec<[f64; 2]>),
}

impl ToolState {
    pub fn cancel(&mut self) {
        self.wall_start = None;
        self.room_pts.clear();
    }

    pub fn is_active(&self) -> bool {
        self.wall_start.is_some() || !self.room_pts.is_empty()
    }

    /// Left-click at a snapped world point. Returns a new selection if something was created.
    pub fn on_click(&mut self, tool: Tool, doc: &mut Document, p: [f64; 2]) -> Option<Selection> {
        match tool {
            Tool::Select => None,
            Tool::DrawWall => self.click_wall(doc, p),
            Tool::AddRoom => self.click_room(doc, p),
            Tool::AddOpening => self.click_opening(doc, p),
        }
    }

    /// Enter / double-click: finalize the current multi-point action (rooms).
    pub fn on_commit(&mut self, tool: Tool, doc: &mut Document) -> Option<Selection> {
        if tool == Tool::AddRoom {
            return self.finish_room(doc);
        }
        None
    }

    pub fn preview(&self, tool: Tool, cursor: [f64; 2]) -> Preview {
        match tool {
            Tool::DrawWall => match self.wall_start {
                Some(s) => Preview::Segment(s, cursor),
                None => Preview::None,
            },
            Tool::AddRoom if !self.room_pts.is_empty() => {
                let mut pts = self.room_pts.clone();
                pts.push(cursor);
                Preview::Polyline(pts)
            }
            _ => Preview::None,
        }
    }

    fn click_wall(&mut self, doc: &mut Document, p: [f64; 2]) -> Option<Selection> {
        match self.wall_start.take() {
            None => {
                self.wall_start = Some(p);
                None
            }
            Some(start) => {
                if dist(start, p) < 1.0 {
                    self.wall_start = Some(p); // ignore zero-length; keep drawing
                    return None;
                }
                // Chain: the endpoint becomes the next segment's start.
                self.wall_start = Some(p);
                let floor_idx = doc.active_floor;
                let id = doc.edit(|b| add_wall(b, floor_idx, start, p));
                id.map(Selection::Wall)
            }
        }
    }

    fn click_room(&mut self, doc: &mut Document, p: [f64; 2]) -> Option<Selection> {
        // Click near the first vertex closes and commits the polygon.
        if self.room_pts.len() >= 3 && dist(self.room_pts[0], p) < 300.0 {
            return self.finish_room(doc);
        }
        self.room_pts.push(p);
        None
    }

    fn finish_room(&mut self, doc: &mut Document) -> Option<Selection> {
        if self.room_pts.len() < 3 {
            self.room_pts.clear();
            return None;
        }
        let pts = std::mem::take(&mut self.room_pts);
        let floor_idx = doc.active_floor;
        let id = doc.edit(|b| add_room(b, floor_idx, pts));
        id.map(Selection::Room)
    }

    fn click_opening(&mut self, doc: &mut Document, p: [f64; 2]) -> Option<Selection> {
        // Find the nearest wall on the active floor and place a door at the
        // projection — but only if the click is actually near a wall.
        let floor_idx = doc.active_floor;
        let hit = doc
            .building
            .floors
            .get(floor_idx)
            .and_then(|f| nearest_wall(f, p, OPENING_ATTACH_TOL));
        let (wall_id, position) = hit?;
        let id = doc.edit(|b| {
            let f = b.floors.get_mut(floor_idx)?;
            let o = Opening::door(wall_id, position, 800.0, 2000.0);
            let id = o.id;
            f.openings.push(o);
            Some(id)
        });
        id.map(Selection::Opening)
    }
}

fn add_wall(b: &mut Building, floor_idx: usize, a: [f64; 2], c: [f64; 2]) -> Option<uuid::Uuid> {
    let f = b.floors.get_mut(floor_idx)?;
    // Floor::add_wall が端点をノードに解決（一致する既存端点があれば接合）。
    Some(f.add_wall(Point2D::new(a[0], a[1]), Point2D::new(c[0], c[1]), 100.0))
}

fn add_room(b: &mut Building, floor_idx: usize, pts: Vec<[f64; 2]>) -> Option<uuid::Uuid> {
    let f = b.floors.get_mut(floor_idx)?;
    let boundary: Vec<Point2D> = pts.iter().map(|p| Point2D::new(p[0], p[1])).collect();
    let n = f.rooms.len() + 1;
    let room = Room::new(format!("Room {n}"), boundary);
    let id = room.id;
    f.rooms.push(room);
    Some(id)
}

/// Nearest wall to `p` on `floor` within `max_dist` (mm), returning
/// (wall id, projected position along centerline mm). `None` if nothing is close.
fn nearest_wall(floor: &Floor, p: [f64; 2], max_dist: f64) -> Option<(uuid::Uuid, f64)> {
    let mut best: Option<(uuid::Uuid, f64, f64)> = None; // (id, pos, dist)
    for w in &floor.walls {
        let Some((a, b)) = floor.wall_endpoints(w) else {
            continue;
        };
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let len2 = dx * dx + dy * dy;
        if len2 < 1e-6 {
            continue;
        }
        let t = ((p[0] - a.x) * dx + (p[1] - a.y) * dy) / len2;
        let tc = t.clamp(0.0, 1.0);
        let proj = [a.x + dx * tc, a.y + dy * tc];
        let d = dist(p, proj);
        let pos = tc * len2.sqrt();
        if d <= max_dist && best.is_none_or(|(_, _, bd)| d < bd) {
            best = Some((w.id, pos, d));
        }
    }
    best.map(|(id, pos, _)| (id, pos))
}

fn dist(a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}
