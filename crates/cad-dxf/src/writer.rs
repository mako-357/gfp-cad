use cad_core::*;
use std::io::Write;

/// DXF 文字エンコーディング
#[derive(Debug, Clone, Copy)]
pub enum DxfEncoding {
    /// UTF-8（AutoCAD 2007+ 向け）
    Utf8,
    /// Shift-JIS（Jw_cad 向け）
    ShiftJis,
}

/// gfp-cad モデルを DXF ファイルに出力
pub struct DxfExporter {
    pub encoding: DxfEncoding,
}

impl DxfExporter {
    pub fn new(encoding: DxfEncoding) -> Self {
        Self { encoding }
    }

    /// Jw_cad 向け（Shift-JIS）
    pub fn for_jwcad() -> Self {
        Self::new(DxfEncoding::ShiftJis)
    }

    /// AutoCAD 向け（UTF-8）
    pub fn for_autocad() -> Self {
        Self::new(DxfEncoding::Utf8)
    }

    /// Building を DXF に出力
    pub fn export<W: Write>(&self, bldg: &Building, out: &mut W) -> Result<ExportReport, Box<dyn std::error::Error>> {
        let mut dxf = DxfWriter::new(out, self.encoding);
        let mut report = ExportReport::default();

        // HEADER
        dxf.section("HEADER")?;
        dxf.pair(9, "$ACADVER")?; dxf.pair(1, "AC1009")?; // R12
        dxf.pair(9, "$INSUNITS")?; dxf.pair(70, "4")?;    // mm
        if matches!(self.encoding, DxfEncoding::Utf8) {
            dxf.pair(9, "$DWGCODEPAGE")?; dxf.pair(3, "UTF-8")?;
        } else {
            dxf.pair(9, "$DWGCODEPAGE")?; dxf.pair(3, "ANSI_932")?; // Shift-JIS
        }
        dxf.endsec()?;

        // TABLES - レイヤー定義
        dxf.section("TABLES")?;
        dxf.pair(0, "TABLE")?; dxf.pair(2, "LAYER")?; dxf.pair(70, "10")?;

        let layers = [
            ("0", 7),
            ("GFP_GRID", 8),
            ("GFP_WALL", 7),
            ("GFP_WALL_EXT", 1),
            ("GFP_OPENING", 3),
            ("GFP_ROOM", 4),
            ("GFP_DIM", 2),
        ];
        for (name, color) in &layers {
            dxf.pair(0, "LAYER")?;
            dxf.pair(2, name)?;
            dxf.pair(70, "0")?;
            dxf.pair(62, &color.to_string())?;
            dxf.pair(6, "CONTINUOUS")?;
            report.layers += 1;
        }
        dxf.pair(0, "ENDTAB")?;
        dxf.endsec()?;

        // ENTITIES
        dxf.section("ENTITIES")?;

        // 通り芯
        let grid = &bldg.grid;
        let x_max = grid.x_axes.iter().map(|a| a.position).fold(0.0f64, f64::max);
        let y_max = grid.y_axes.iter().map(|a| a.position).fold(0.0f64, f64::max);
        let margin = 2000.0;

        for axis in &grid.x_axes {
            dxf.line(axis.position, -margin, axis.position, y_max + margin, "GFP_GRID")?;
            dxf.text(axis.position - 60.0, y_max + margin + 300.0, 150.0, &axis.name, "GFP_GRID")?;
            dxf.text(axis.position - 60.0, -margin - 500.0, 150.0, &axis.name, "GFP_GRID")?;
            report.entities += 3;
        }
        for axis in &grid.y_axes {
            dxf.line(-margin, axis.position, x_max + margin, axis.position, "GFP_GRID")?;
            dxf.text(-margin - 500.0, axis.position - 80.0, 150.0, &axis.name, "GFP_GRID")?;
            report.entities += 2;
        }

        // 各階
        for floor in &bldg.floors {
            // 壁
            for wall in &floor.walls {
                let layer = if wall.is_exterior { "GFP_WALL_EXT" } else { "GFP_WALL" };
                let t = wall.thickness / 2.0;
                let dx = wall.end.x - wall.start.x;
                let dy = wall.end.y - wall.start.y;
                let len = (dx * dx + dy * dy).sqrt();
                if len < 0.1 { continue; }
                let nx = -dy / len * t;
                let ny = dx / len * t;

                dxf.lwpolyline(&[
                    (wall.start.x + nx, wall.start.y + ny),
                    (wall.end.x + nx, wall.end.y + ny),
                    (wall.end.x - nx, wall.end.y - ny),
                    (wall.start.x - nx, wall.start.y - ny),
                ], true, layer)?;
                report.entities += 1;
            }

            // 開口
            for opening in &floor.openings {
                if let Some(wall) = floor.walls.iter().find(|w| w.id == opening.wall_id) {
                    let dx = wall.end.x - wall.start.x;
                    let dy = wall.end.y - wall.start.y;
                    let len = (dx * dx + dy * dy).sqrt();
                    if len < 0.1 { continue; }
                    let ux = dx / len;
                    let uy = dy / len;
                    let cx = wall.start.x + ux * opening.position;
                    let cy = wall.start.y + uy * opening.position;
                    let hw = opening.width / 2.0;
                    let nx = -uy * wall.thickness;
                    let ny = ux * wall.thickness;

                    dxf.line(cx - ux * hw + nx, cy - uy * hw + ny,
                             cx + ux * hw + nx, cy + uy * hw + ny, "GFP_OPENING")?;
                    dxf.line(cx - ux * hw - nx, cy - uy * hw - ny,
                             cx + ux * hw - nx, cy + uy * hw - ny, "GFP_OPENING")?;
                    report.entities += 2;
                }
            }

            // 部屋名
            for room in &floor.rooms {
                if room.boundary.len() < 3 { continue; }
                let cx: f64 = room.boundary.iter().map(|p| p.x).sum::<f64>() / room.boundary.len() as f64;
                let cy: f64 = room.boundary.iter().map(|p| p.y).sum::<f64>() / room.boundary.len() as f64;

                dxf.text(cx - 500.0, cy + 100.0, 150.0, &room.name, "GFP_ROOM")?;
                dxf.text(cx - 300.0, cy - 200.0, 100.0, &format!("{:.1}sqm", room.area()), "GFP_ROOM")?;
                report.entities += 2;
            }
        }

        // タイトル
        dxf.text(x_max / 2.0 - 2000.0, y_max + 4000.0, 250.0,
                 &format!("{} — 1F Plan", bldg.name), "GFP_DIM")?;
        report.entities += 1;

        dxf.endsec()?;

        // EOF
        dxf.pair(0, "EOF")?;

        Ok(report)
    }
}

// --- DXF 低レベルライター ---

struct DxfWriter<'a, W: Write> {
    out: &'a mut W,
    encoding: DxfEncoding,
}

impl<'a, W: Write> DxfWriter<'a, W> {
    fn new(out: &'a mut W, encoding: DxfEncoding) -> Self {
        Self { out, encoding }
    }

    fn encode(&self, s: &str) -> Vec<u8> {
        match self.encoding {
            DxfEncoding::Utf8 => s.as_bytes().to_vec(),
            DxfEncoding::ShiftJis => {
                let (encoded, _, _) = encoding_rs::SHIFT_JIS.encode(s);
                encoded.to_vec()
            }
        }
    }

    fn pair(&mut self, code: i32, value: &str) -> Result<(), Box<dyn std::error::Error>> {
        write!(self.out, "{:>3}\r\n", code)?;
        let encoded = self.encode(value);
        self.out.write_all(&encoded)?;
        write!(self.out, "\r\n")?;
        Ok(())
    }

    fn section(&mut self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.pair(0, "SECTION")?;
        self.pair(2, name)
    }

    fn endsec(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.pair(0, "ENDSEC")
    }

    fn line(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, layer: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.pair(0, "LINE")?;
        self.pair(8, layer)?;
        self.pair(10, &format!("{:.3}", x1))?;
        self.pair(20, &format!("{:.3}", y1))?;
        self.pair(30, "0.0")?;
        self.pair(11, &format!("{:.3}", x2))?;
        self.pair(21, &format!("{:.3}", y2))?;
        self.pair(31, "0.0")?;
        Ok(())
    }

    fn lwpolyline(&mut self, pts: &[(f64, f64)], closed: bool, layer: &str) -> Result<(), Box<dyn std::error::Error>> {
        // DXF R12 では POLYLINE + VERTEX + SEQEND
        self.pair(0, "POLYLINE")?;
        self.pair(8, layer)?;
        self.pair(66, "1")?;
        let flag = if closed { "1" } else { "0" };
        self.pair(70, flag)?;

        for &(x, y) in pts {
            self.pair(0, "VERTEX")?;
            self.pair(8, layer)?;
            self.pair(10, &format!("{:.3}", x))?;
            self.pair(20, &format!("{:.3}", y))?;
            self.pair(30, "0.0")?;
        }

        self.pair(0, "SEQEND")?;
        self.pair(8, layer)?;
        Ok(())
    }

    fn text(&mut self, x: f64, y: f64, height: f64, content: &str, layer: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.pair(0, "TEXT")?;
        self.pair(8, layer)?;
        self.pair(10, &format!("{:.3}", x))?;
        self.pair(20, &format!("{:.3}", y))?;
        self.pair(30, "0.0")?;
        self.pair(40, &format!("{:.1}", height))?;
        self.pair(1, content)?;
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct ExportReport {
    pub layers: usize,
    pub entities: usize,
}

impl std::fmt::Display for ExportReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Exported: {} layers, {} entities", self.layers, self.entities)
    }
}
