use anyhow::Result;
use cad_core::*;
use serde_json::json;

use crate::bridge;

/// gfp-cad モデルを AutoCAD に描画するレンダラー
pub struct AcadRenderer {
    /// 描画の原点オフセット (mm)
    pub origin: Point2D,
}

impl AcadRenderer {
    pub fn new() -> Self {
        Self {
            origin: Point2D::new(0.0, 0.0),
        }
    }

    pub fn with_origin(origin: Point2D) -> Self {
        Self { origin }
    }

    /// 接続チェック
    pub fn is_connected(&self) -> bool {
        bridge::is_connected()
    }

    /// Building 全体を描画
    pub fn render_building(&self, bldg: &Building) -> Result<RenderReport> {
        let mut report = RenderReport::default();

        // レイヤー作成
        self.setup_layers()?;
        report.layers_created = 6;

        // 通り芯（建物で1回だけ描画）
        report.grid_lines += self.render_grid(&bldg.grid)?;

        // 各階を描画
        for floor in &bldg.floors {
            let floor_report = self.render_floor(floor)?;
            report.walls += floor_report.walls;
            report.openings += floor_report.openings;
            report.rooms += floor_report.rooms;
            report.texts += floor_report.texts;
        }

        // タイトル
        self.render_title(bldg)?;
        report.texts += 1;

        Ok(report)
    }

    fn setup_layers(&self) -> Result<()> {
        let layers = [
            ("GFP_GRID", 8),     // 通り芯: グレー
            ("GFP_WALL", 7),     // 壁: 白
            ("GFP_WALL_EXT", 1), // 外壁: 赤
            ("GFP_OPENING", 3),  // 開口: 緑
            ("GFP_ROOM", 4),     // 部屋: シアン
            ("GFP_DIM", 2),      // 寸法: 黄
        ];
        for (name, color) in layers {
            bridge::send("create_layer", json!({"name": name, "color": color}))?;
        }
        Ok(())
    }

    fn render_grid(&self, grid: &GridSystem) -> Result<usize> {
        let ox = self.origin.x;
        let oy = self.origin.y;
        let mut count = 0;

        // 建物の範囲
        let x_max = grid
            .x_axes
            .iter()
            .map(|a| a.position)
            .fold(0.0f64, f64::max);
        let y_max = grid
            .y_axes
            .iter()
            .map(|a| a.position)
            .fold(0.0f64, f64::max);
        let margin = 2000.0;

        // X 通り芯（縦線）
        for axis in &grid.x_axes {
            let x = ox + axis.position;
            bridge::send(
                "draw_line",
                json!({
                    "x1": x, "y1": oy - margin,
                    "x2": x, "y2": oy + y_max + margin,
                    "layer": "GFP_GRID"
                }),
            )?;
            // ラベル（上）
            bridge::send(
                "draw_circle",
                json!({
                    "cx": x, "cy": oy + y_max + margin + 300.0,
                    "radius": 250.0, "layer": "GFP_GRID"
                }),
            )?;
            bridge::send(
                "draw_text",
                json!({
                    "x": x - 60.0, "y": oy + y_max + margin + 220.0,
                    "text": axis.name, "height": 150.0, "layer": "GFP_GRID"
                }),
            )?;
            // ラベル（下）
            bridge::send(
                "draw_circle",
                json!({
                    "cx": x, "cy": oy - margin - 300.0,
                    "radius": 250.0, "layer": "GFP_GRID"
                }),
            )?;
            bridge::send(
                "draw_text",
                json!({
                    "x": x - 60.0, "y": oy - margin - 380.0,
                    "text": axis.name, "height": 150.0, "layer": "GFP_GRID"
                }),
            )?;
            count += 1;
        }

        // Y 通り芯（横線）
        for axis in &grid.y_axes {
            let y = oy + axis.position;
            bridge::send(
                "draw_line",
                json!({
                    "x1": ox - margin, "y1": y,
                    "x2": ox + x_max + margin, "y2": y,
                    "layer": "GFP_GRID"
                }),
            )?;
            // ラベル（左）
            bridge::send(
                "draw_circle",
                json!({
                    "cx": ox - margin - 300.0, "cy": y,
                    "radius": 250.0, "layer": "GFP_GRID"
                }),
            )?;
            bridge::send(
                "draw_text",
                json!({
                    "x": ox - margin - 360.0, "y": y - 80.0,
                    "text": axis.name, "height": 150.0, "layer": "GFP_GRID"
                }),
            )?;
            count += 1;
        }

        // スパン寸法
        let dim_y = oy - margin - 1500.0;
        let sorted_x: Vec<_> = {
            let mut v: Vec<_> = grid.x_axes.iter().collect();
            v.sort_by(|a, b| a.position.partial_cmp(&b.position).unwrap());
            v
        };
        for w in sorted_x.windows(2) {
            let x1 = ox + w[0].position;
            let x2 = ox + w[1].position;
            let span = (w[1].position - w[0].position).abs();
            bridge::send(
                "draw_line",
                json!({
                    "x1": x1, "y1": dim_y, "x2": x2, "y2": dim_y, "layer": "GFP_DIM"
                }),
            )?;
            bridge::send(
                "draw_text",
                json!({
                    "x": (x1 + x2) / 2.0 - 200.0, "y": dim_y + 50.0,
                    "text": format!("{:.0}", span), "height": 100.0, "layer": "GFP_DIM"
                }),
            )?;
        }

        Ok(count)
    }

    fn render_floor(&self, floor: &Floor) -> Result<RenderReport> {
        let mut report = RenderReport::default();
        let ox = self.origin.x;
        let oy = self.origin.y;

        // 壁を描画
        for wall in &floor.walls {
            let layer = if wall.is_exterior {
                "GFP_WALL_EXT"
            } else {
                "GFP_WALL"
            };

            let s = Point2D::new(ox + wall.start.x, oy + wall.start.y);
            let e = Point2D::new(ox + wall.end.x, oy + wall.end.y);
            let t = wall.thickness / 2.0;

            // 壁の方向ベクトル
            let dx = e.x - s.x;
            let dy = e.y - s.y;
            let len = (dx * dx + dy * dy).sqrt();
            if len < 0.1 {
                continue;
            }
            let nx = -dy / len * t; // 法線方向
            let ny = dx / len * t;

            // 壁を閉じたポリラインで描画（壁芯から両側に厚みを振る）
            let pts = vec![
                vec![s.x + nx, s.y + ny],
                vec![e.x + nx, e.y + ny],
                vec![e.x - nx, e.y - ny],
                vec![s.x - nx, s.y - ny],
            ];
            bridge::send(
                "draw_polyline",
                json!({
                    "points": pts, "closed": true, "layer": layer
                }),
            )?;
            report.walls += 1;
        }

        // 開口部を描画（壁上に矩形で表示）
        for opening in &floor.openings {
            // 開口部の壁を探す
            if let Some(wall) = floor.walls.iter().find(|w| w.id == opening.wall_id) {
                let s = Point2D::new(ox + wall.start.x, oy + wall.start.y);
                let e = Point2D::new(ox + wall.end.x, oy + wall.end.y);
                let dx = e.x - s.x;
                let dy = e.y - s.y;
                let len = (dx * dx + dy * dy).sqrt();
                if len < 0.1 {
                    continue;
                }

                // 壁に沿った方向
                let ux = dx / len;
                let uy = dy / len;
                // 開口の中心位置
                let cx = s.x + ux * opening.position;
                let cy = s.y + uy * opening.position;
                let hw = opening.width / 2.0;

                // 平面図上の開口表示（壁を横切る線）
                let nx = -uy * wall.thickness;
                let ny = ux * wall.thickness;

                bridge::send(
                    "draw_line",
                    json!({
                        "x1": cx - ux * hw + nx, "y1": cy - uy * hw + ny,
                        "x2": cx + ux * hw + nx, "y2": cy + uy * hw + ny,
                        "layer": "GFP_OPENING"
                    }),
                )?;
                bridge::send(
                    "draw_line",
                    json!({
                        "x1": cx - ux * hw - nx, "y1": cy - uy * hw - ny,
                        "x2": cx + ux * hw - nx, "y2": cy + uy * hw - ny,
                        "layer": "GFP_OPENING"
                    }),
                )?;

                report.openings += 1;
            }
        }

        // 部屋名ラベル
        for room in &floor.rooms {
            if room.boundary.len() < 3 {
                continue;
            }
            // 重心
            let cx: f64 =
                room.boundary.iter().map(|p| p.x).sum::<f64>() / room.boundary.len() as f64;
            let cy: f64 =
                room.boundary.iter().map(|p| p.y).sum::<f64>() / room.boundary.len() as f64;

            bridge::send(
                "draw_text",
                json!({
                    "x": ox + cx - 500.0, "y": oy + cy + 100.0,
                    "text": &room.name, "height": 150.0, "layer": "GFP_ROOM"
                }),
            )?;
            bridge::send(
                "draw_text",
                json!({
                    "x": ox + cx - 300.0, "y": oy + cy - 200.0,
                    "text": format!("{:.1}sqm", room.area()), "height": 100.0, "layer": "GFP_ROOM"
                }),
            )?;
            report.rooms += 1;
            report.texts += 2;
        }

        Ok(report)
    }

    fn render_title(&self, bldg: &Building) -> Result<()> {
        let x_max = bldg
            .grid
            .x_axes
            .iter()
            .map(|a| a.position)
            .fold(0.0f64, f64::max);
        let y_max = bldg
            .grid
            .y_axes
            .iter()
            .map(|a| a.position)
            .fold(0.0f64, f64::max);

        bridge::send(
            "draw_text",
            json!({
                "x": self.origin.x + x_max / 2.0 - 2000.0,
                "y": self.origin.y + y_max + 4000.0,
                "text": format!("{} — 1F Plan (gfp-cad)", bldg.name),
                "height": 250.0, "layer": "GFP_DIM"
            }),
        )?;
        Ok(())
    }
}

impl Default for AcadRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Default)]
pub struct RenderReport {
    pub layers_created: usize,
    pub grid_lines: usize,
    pub walls: usize,
    pub openings: usize,
    pub rooms: usize,
    pub texts: usize,
}

impl std::fmt::Display for RenderReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Rendered: {} layers, {} grid lines, {} walls, {} openings, {} rooms, {} texts",
            self.layers_created, self.grid_lines, self.walls, self.openings, self.rooms, self.texts
        )
    }
}
