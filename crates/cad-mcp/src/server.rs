use std::sync::{Arc, Mutex};

use cad_core::*;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{ServerCapabilities, ServerInfo};
use rmcp::{schemars, tool, tool_handler, tool_router, ServerHandler};
use schemars::JsonSchema;
use serde::Deserialize;

/// gfp-cad MCP サーバー — セマンティック建築モデルを操作
#[derive(Clone)]
pub struct GfpCadMcpServer {
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
    state: Arc<Mutex<ServerState>>,
}

struct ServerState {
    building: Option<Building>,
}

impl GfpCadMcpServer {
    pub fn new() -> Self {
        let tool_router = Self::tool_router();
        Self {
            tool_router,
            state: Arc::new(Mutex::new(ServerState { building: None })),
        }
    }

    fn with_building<F, R>(&self, f: F) -> String
    where
        F: FnOnce(&Building) -> R,
        R: std::fmt::Display,
    {
        let state = self.state.lock().unwrap();
        match &state.building {
            Some(b) => f(b).to_string(),
            None => "Error: No building created. Use create_building first.".to_string(),
        }
    }

    fn with_building_mut<F>(&self, f: F) -> String
    where
        F: FnOnce(&mut Building) -> String,
    {
        let mut state = self.state.lock().unwrap();
        match &mut state.building {
            Some(b) => f(b),
            None => "Error: No building created. Use create_building first.".to_string(),
        }
    }
}

// =====================================================================
// 入力スキーマ
// =====================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateBuildingInput {
    #[schemars(description = "建物名")]
    pub name: String,
    #[schemars(description = "用途（住宅、別荘、事務所等）")]
    pub usage: Option<String>,
    #[schemars(description = "構造種別（木造、RC、S造等）")]
    pub structure_type: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetGridInput {
    #[schemars(description = "X方向の通り芯 [{name, position},...] position は mm")]
    pub x_axes: Vec<GridAxisInput>,
    #[schemars(description = "Y方向の通り芯 [{name, position},...] position は mm")]
    pub y_axes: Vec<GridAxisInput>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GridAxisInput {
    pub name: String,
    pub position: f64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddFloorInput {
    #[schemars(description = "階名（1F, 2F, B1F 等）")]
    pub name: String,
    #[schemars(description = "FL レベル mm（GLからの高さ）")]
    pub level: f64,
    #[schemars(description = "階高 mm")]
    pub height: f64,
    #[schemars(description = "天井高 mm（省略時: 階高-300）")]
    pub ceiling_height: Option<f64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddWallInput {
    #[schemars(description = "階名")]
    pub floor: String,
    #[schemars(description = "壁芯の始点 X mm")]
    pub x1: f64,
    #[schemars(description = "壁芯の始点 Y mm")]
    pub y1: f64,
    #[schemars(description = "壁芯の終点 X mm")]
    pub x2: f64,
    #[schemars(description = "壁芯の終点 Y mm")]
    pub y2: f64,
    #[schemars(description = "壁厚 mm")]
    pub thickness: f64,
    #[schemars(description = "外壁かどうか")]
    pub is_exterior: Option<bool>,
    #[schemars(description = "材料（RC, LGS, Wood, ALC, CB）")]
    pub material: Option<String>,
    #[schemars(description = "壁高 mm（省略時は階高に従う）")]
    pub height: Option<f64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddOpeningInput {
    #[schemars(description = "階名")]
    pub floor: String,
    #[schemars(description = "壁 ID（add_wall の戻り値）")]
    pub wall_id: String,
    #[schemars(description = "壁の始点からの距離 mm")]
    pub position: f64,
    #[schemars(description = "開口幅 mm")]
    pub width: f64,
    #[schemars(description = "開口高 mm")]
    pub height: f64,
    #[schemars(description = "窓台高 / 床からの高さ mm（ドアは 0）")]
    pub sill_height: Option<f64>,
    #[schemars(description = "種別: door, sliding_door, window, sliding_window, fixed_window")]
    pub kind: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddRoomInput {
    #[schemars(description = "階名")]
    pub floor: String,
    #[schemars(description = "室名")]
    pub name: String,
    #[schemars(description = "境界ポリゴン [{x,y},...] mm、反時計回り")]
    pub boundary: Vec<PointInput>,
    #[schemars(description = "床仕上げ")]
    pub floor_finish: Option<String>,
    #[schemars(description = "床暖房あり")]
    pub floor_heating: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PointInput {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EmptyInput {}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExportDxfInput {
    #[schemars(description = "出力ファイルパス")]
    pub path: String,
    #[schemars(description = "エンコーディング: utf8 or shift_jis（省略時 utf8）")]
    pub encoding: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RenderAutocadInput {
    #[schemars(description = "描画の原点 X mm（省略時 0）")]
    pub origin_x: Option<f64>,
    #[schemars(description = "描画の原点 Y mm（省略時 0）")]
    pub origin_y: Option<f64>,
}

// =====================================================================
// ツール実装
// =====================================================================

fn parse_material(s: &str) -> WallMaterial {
    match s.to_uppercase().as_str() {
        "RC" => WallMaterial::RC,
        "LGS" => WallMaterial::LGS,
        "WOOD" => WallMaterial::Wood,
        "ALC" => WallMaterial::ALC,
        "CB" => WallMaterial::CB,
        _ => WallMaterial::Other(s.to_string()),
    }
}

fn parse_opening_kind(s: &str) -> OpeningKind {
    match s.to_lowercase().as_str() {
        "door" | "single_door" => OpeningKind::SingleDoor,
        "double_door" => OpeningKind::DoubleDoor,
        "sliding_door" => OpeningKind::SlidingDoor,
        "window" | "sliding_window" => OpeningKind::SlidingWindow,
        "fixed_window" | "fix" => OpeningKind::FixedWindow,
        "hung_window" => OpeningKind::HungWindow,
        "casement_window" | "casement" => OpeningKind::CasementWindow,
        _ => OpeningKind::Other(s.to_string()),
    }
}

#[tool_router]
impl GfpCadMcpServer {
    #[tool(name = "create_building", description = "新しい建物を作成")]
    async fn create_building(&self, Parameters(i): Parameters<CreateBuildingInput>) -> String {
        let mut bldg = Building::new(&i.name);
        if let Some(u) = i.usage {
            bldg.metadata.usage = Some(u);
        }
        if let Some(s) = i.structure_type {
            bldg.metadata.structure_type = Some(s);
        }
        let id = bldg.id.to_string();
        self.state.lock().unwrap().building = Some(bldg);
        format!("Building '{}' created (id: {})", i.name, id)
    }

    #[tool(name = "set_grid", description = "通り芯を設定")]
    async fn set_grid(&self, Parameters(i): Parameters<SetGridInput>) -> String {
        self.with_building_mut(|b| {
            b.grid.x_axes = i.x_axes.iter().map(|a| GridAxis::new(&a.name, a.position)).collect();
            b.grid.y_axes = i.y_axes.iter().map(|a| GridAxis::new(&a.name, a.position)).collect();
            format!(
                "Grid set: {} x-axes, {} y-axes",
                b.grid.x_axes.len(),
                b.grid.y_axes.len()
            )
        })
    }

    #[tool(name = "add_floor", description = "階を追加")]
    async fn add_floor(&self, Parameters(i): Parameters<AddFloorInput>) -> String {
        self.with_building_mut(|b| {
            let mut floor = Floor::new(&i.name, i.level, i.height);
            if let Some(ch) = i.ceiling_height {
                floor.ceiling_height = ch;
            }
            let id = floor.id.to_string();
            b.floors.push(floor);
            format!("Floor '{}' added (id: {})", i.name, id)
        })
    }

    #[tool(name = "add_wall", description = "壁を追加。壁 ID を返す")]
    async fn add_wall(&self, Parameters(i): Parameters<AddWallInput>) -> String {
        self.with_building_mut(|b| {
            let floor = b.floors.iter_mut().find(|f| f.name == i.floor);
            let Some(floor) = floor else {
                return format!("Error: Floor '{}' not found", i.floor);
            };
            let mut wall = Wall::new(
                Point2D::new(i.x1, i.y1),
                Point2D::new(i.x2, i.y2),
                i.thickness,
            );
            if let Some(ext) = i.is_exterior {
                wall.is_exterior = ext;
            }
            if let Some(m) = &i.material {
                wall.material = parse_material(m);
            }
            wall.height = i.height;
            let id = wall.id.to_string();
            floor.walls.push(wall);
            id
        })
    }

    #[tool(
        name = "add_opening",
        description = "開口部（ドア/窓）を追加"
    )]
    async fn add_opening(&self, Parameters(i): Parameters<AddOpeningInput>) -> String {
        self.with_building_mut(|b| {
            let floor = b.floors.iter_mut().find(|f| f.name == i.floor);
            let Some(floor) = floor else {
                return format!("Error: Floor '{}' not found", i.floor);
            };
            let wall_uuid = match uuid::Uuid::parse_str(&i.wall_id) {
                Ok(u) => u,
                Err(_) => return "Error: Invalid wall_id".to_string(),
            };
            let sill = i.sill_height.unwrap_or(0.0);
            let mut opening = if sill > 0.0 {
                Opening::window(wall_uuid, i.position, i.width, i.height, sill)
            } else {
                Opening::door(wall_uuid, i.position, i.width, i.height)
            };
            if let Some(k) = &i.kind {
                opening.kind = parse_opening_kind(k);
            }
            let id = opening.id.to_string();
            floor.openings.push(opening);
            id
        })
    }

    #[tool(name = "add_room", description = "部屋を追加")]
    async fn add_room(&self, Parameters(i): Parameters<AddRoomInput>) -> String {
        self.with_building_mut(|b| {
            let floor = b.floors.iter_mut().find(|f| f.name == i.floor);
            let Some(floor) = floor else {
                return format!("Error: Floor '{}' not found", i.floor);
            };
            let boundary: Vec<Point2D> = i.boundary.iter().map(|p| Point2D::new(p.x, p.y)).collect();
            let mut room = Room::new(&i.name, boundary);
            room.floor_finish = i.floor_finish;
            room.has_floor_heating = i.floor_heating.unwrap_or(false);
            let area = room.area();
            let id = room.id.to_string();
            floor.rooms.push(room);
            format!("{} ({:.1} sqm, id: {})", i.name, area, id)
        })
    }

    #[tool(
        name = "get_building_summary",
        description = "建物の概要を取得"
    )]
    async fn get_building_summary(&self, Parameters(_): Parameters<EmptyInput>) -> String {
        self.with_building(|b| {
            let mut out = format!("=== {} ===\n", b.name);
            if let Some(u) = &b.metadata.usage {
                out += &format!("用途: {u}\n");
            }

            out += &format!("\n通り芯:\n");
            for (name, span) in b.grid.x_spans() {
                out += &format!("  X: {name} = {:.0}mm ({:.2}m)\n", span, span / 1000.0);
            }
            for (name, span) in b.grid.y_spans() {
                out += &format!("  Y: {name} = {:.0}mm ({:.2}m)\n", span, span / 1000.0);
            }

            for floor in &b.floors {
                out += &format!(
                    "\n{} (FL+{:.0}, CH={:.0}):\n  壁:{} 開口:{} 部屋:{}\n",
                    floor.name,
                    floor.level,
                    floor.ceiling_height,
                    floor.walls.len(),
                    floor.openings.len(),
                    floor.rooms.len(),
                );
                for room in &floor.rooms {
                    out += &format!("    {} — {:.1}sqm\n", room.name, room.area());
                }
                out += &format!("  床面積: {:.1}sqm\n", floor.area());
            }
            out += &format!("\n延べ面積: {:.1}sqm", b.total_floor_area());
            out
        })
    }

    #[tool(
        name = "get_building_json",
        description = "建物データを JSON で取得"
    )]
    async fn get_building_json(&self, Parameters(_): Parameters<EmptyInput>) -> String {
        self.with_building(|b| {
            serde_json::to_string_pretty(b).unwrap_or_else(|e| format!("Error: {e}"))
        })
    }

    #[tool(name = "export_dxf", description = "建物を DXF ファイルに出力")]
    async fn export_dxf(&self, Parameters(i): Parameters<ExportDxfInput>) -> String {
        let state = self.state.lock().unwrap();
        let Some(b) = &state.building else {
            return "Error: No building".to_string();
        };
        let enc = match i.encoding.as_deref() {
            Some("shift_jis" | "sjis") => cad_dxf::DxfEncoding::ShiftJis,
            _ => cad_dxf::DxfEncoding::Utf8,
        };
        let exporter = cad_dxf::DxfExporter::new(enc);
        match std::fs::File::create(&i.path) {
            Ok(mut f) => match exporter.export(b, &mut f) {
                Ok(report) => format!("Exported to {}: {report}", i.path),
                Err(e) => format!("Error: {e}"),
            },
            Err(e) => format!("Error creating file: {e}"),
        }
    }

    #[tool(
        name = "render_autocad",
        description = "建物を AutoCAD に描画（autocad-mcp プラグイン必要）"
    )]
    async fn render_autocad(&self, Parameters(i): Parameters<RenderAutocadInput>) -> String {
        let state = self.state.lock().unwrap();
        let Some(b) = &state.building else {
            return "Error: No building".to_string();
        };
        let origin = Point2D::new(
            i.origin_x.unwrap_or(0.0),
            i.origin_y.unwrap_or(0.0),
        );
        let renderer = cad_acad::AcadRenderer::with_origin(origin);
        match renderer.render_building(b) {
            Ok(report) => format!("{report}"),
            Err(e) => format!("Error: {e}"),
        }
    }
}

#[tool_handler]
impl ServerHandler for GfpCadMcpServer {
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::new(ServerCapabilities::builder().enable_tools().build());
        info.instructions = Some(
            "gfp-cad MCP Server — Lightweight BIM。\
             建物モデルを作成し、AutoCAD/DXF に出力。\
             create_building → set_grid → add_floor → add_wall → add_opening → add_room → export_dxf / render_autocad"
                .into(),
        );
        info
    }
}
