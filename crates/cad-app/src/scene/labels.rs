//! Label specs (world position + text). The GPU text buffers are built in `app`
//! from these via `Text::make_label`; positions are projected to screen each frame.

use cad_core::Building;

use crate::config;

pub struct LabelSpec {
    pub world: [f64; 2],
    pub text: String,
    pub size_px: f32,
    /// OpenType weight (100..900) — selects the bundled Gen Interface JP face.
    pub weight: u16,
    pub color: (u8, u8, u8),
}

pub fn build(building: &Building, floor_idx: usize) -> Vec<LabelSpec> {
    let mut out = Vec::new();
    let bbox = super::bounds::model_bounds(building, floor_idx);

    // Building title (heavy weight) above the plan.
    out.push(LabelSpec {
        world: [(bbox[0] + bbox[2]) / 2.0, bbox[3] + 1600.0],
        text: building.name.clone(),
        size_px: 22.0,
        weight: 800, // ExtraBold
        color: config::LABEL_ROOM,
    });

    // Grid axis names: x-axes along the top, y-axes down the left. Light weight.
    for a in &building.grid.x_axes {
        out.push(LabelSpec {
            world: [a.position, bbox[3] + 600.0],
            text: a.name.clone(),
            size_px: 14.0,
            weight: 300, // Light
            color: config::LABEL_GRID,
        });
    }
    for a in &building.grid.y_axes {
        out.push(LabelSpec {
            world: [bbox[0] - 900.0, a.position],
            text: a.name.clone(),
            size_px: 14.0,
            weight: 300,
            color: config::LABEL_GRID,
        });
    }

    // Room name (SemiBold) + derived area (Regular) at the room seed. 未囲いはスキップ。
    // 面は1回だけ導出（room 毎に再構築しない）。
    if let Some(floor) = building.floors.get(floor_idx) {
        for (room, face) in floor.rooms_faces() {
            let Some(area) = face.map(|f| f.area / 1_000_000.0) else {
                continue;
            };
            let (cx, cy) = (room.seed.x, room.seed.y);
            out.push(LabelSpec {
                world: [cx, cy + 300.0],
                text: room.name.clone(),
                size_px: 17.0,
                weight: 600, // SemiBold
                color: config::LABEL_ROOM,
            });
            out.push(LabelSpec {
                world: [cx, cy - 350.0],
                text: format!("{area:.1}㎡"),
                size_px: 13.0,
                weight: 400, // Regular
                color: config::LABEL_GRID,
            });
        }
    }
    out
}
