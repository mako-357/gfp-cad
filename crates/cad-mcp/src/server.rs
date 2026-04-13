use std::sync::Arc;
use tokio::sync::Mutex;

use cad_core::*;
use cad_db::CadDbClient;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{ServerCapabilities, ServerInfo};
use rmcp::{schemars, tool, tool_handler, tool_router, ServerHandler};
use schemars::JsonSchema;
use serde::Deserialize;
use surrealdb::RecordId;

#[derive(Clone)]
pub struct GfpCadMcpServer {
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
    state: Arc<Mutex<ServerState>>,
    db: Arc<Option<CadDbClient>>,
}

struct ServerState {
    building: Option<Building>,
    current_user: Option<cad_db::User>,
    current_workspace: Option<RecordId>,
    current_project: Option<RecordId>,
}

impl GfpCadMcpServer {
    pub fn new(db: Option<CadDbClient>) -> Self {
        let tool_router = Self::tool_router();
        Self {
            tool_router,
            state: Arc::new(Mutex::new(ServerState {
                building: None,
                current_user: None,
                current_workspace: None,
                current_project: None,
            })),
            db: Arc::new(db),
        }
    }
}

// --- 入力スキーマ ---

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateBuildingInput { pub name: String, pub usage: Option<String>, pub structure_type: Option<String> }
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetGridInput { pub x_axes: Vec<GridAxisInput>, pub y_axes: Vec<GridAxisInput> }
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GridAxisInput { pub name: String, pub position: f64 }
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddFloorInput { pub name: String, pub level: f64, pub height: f64, pub ceiling_height: Option<f64> }
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddWallInput { pub floor: String, pub x1: f64, pub y1: f64, pub x2: f64, pub y2: f64, pub thickness: f64, pub is_exterior: Option<bool>, pub material: Option<String>, pub height: Option<f64> }
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddOpeningInput { pub floor: String, pub wall_id: String, pub position: f64, pub width: f64, pub height: f64, pub sill_height: Option<f64>, pub kind: Option<String> }
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddRoomInput { pub floor: String, pub name: String, pub boundary: Vec<PointInput>, pub floor_finish: Option<String>, pub floor_heating: Option<bool> }
#[derive(Debug, Deserialize, JsonSchema)]
pub struct PointInput { pub x: f64, pub y: f64 }
#[derive(Debug, Deserialize, JsonSchema)]
pub struct EmptyInput {}
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExportDxfInput { pub path: String, pub encoding: Option<String> }
#[derive(Debug, Deserialize, JsonSchema)]
pub struct RenderAutocadInput { pub origin_x: Option<f64>, pub origin_y: Option<f64> }
#[derive(Debug, Deserialize, JsonSchema)]
pub struct LoginInput { pub provider: String, pub provider_user_id: String, pub email: String, pub name: Option<String> }
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SelectInput { pub id: String }
#[derive(Debug, Deserialize, JsonSchema)]
pub struct NameInput { pub name: String }

fn parse_material(s: &str) -> WallMaterial {
    match s.to_uppercase().as_str() { "RC" => WallMaterial::RC, "LGS" => WallMaterial::LGS, "WOOD" => WallMaterial::Wood, "ALC" => WallMaterial::ALC, "CB" => WallMaterial::CB, _ => WallMaterial::Other(s.to_string()) }
}
fn parse_opening_kind(s: &str) -> OpeningKind {
    match s.to_lowercase().as_str() { "door" | "single_door" => OpeningKind::SingleDoor, "double_door" => OpeningKind::DoubleDoor, "sliding_door" => OpeningKind::SlidingDoor, "window" | "sliding_window" => OpeningKind::SlidingWindow, "fixed_window" | "fix" => OpeningKind::FixedWindow, _ => OpeningKind::Other(s.to_string()) }
}
fn make_rid(table: &str, id: &str) -> RecordId {
    if id.contains(':') { let p: Vec<&str> = id.splitn(2, ':').collect(); RecordId::from((p[0].to_string(), p[1].to_string())) }
    else { RecordId::from((table.to_string(), id.to_string())) }
}

const NO_BLDG: &str = "Error: No building. Use create_building first.";
const NO_DB: &str = "Error: DB not connected.";
const NO_LOGIN: &str = "Error: Not logged in. Use login first.";

#[tool_router]
impl GfpCadMcpServer {
    // === 建物操作 ===

    #[tool(name = "create_building", description = "建物を作成")]
    async fn create_building(&self, Parameters(i): Parameters<CreateBuildingInput>) -> String {
        let mut bldg = Building::new(&i.name);
        bldg.metadata.usage = i.usage;
        bldg.metadata.structure_type = i.structure_type;
        let id = bldg.id.to_string();
        self.state.lock().await.building = Some(bldg);
        format!("Building '{}' created ({id})", i.name)
    }

    #[tool(name = "set_grid", description = "通り芯を設定")]
    async fn set_grid(&self, Parameters(i): Parameters<SetGridInput>) -> String {
        let mut s = self.state.lock().await;
        let Some(b) = &mut s.building else { return NO_BLDG.into() };
        b.grid.x_axes = i.x_axes.iter().map(|a| GridAxis::new(&a.name, a.position)).collect();
        b.grid.y_axes = i.y_axes.iter().map(|a| GridAxis::new(&a.name, a.position)).collect();
        format!("Grid: {} x, {} y", b.grid.x_axes.len(), b.grid.y_axes.len())
    }

    #[tool(name = "add_floor", description = "階を追加")]
    async fn add_floor(&self, Parameters(i): Parameters<AddFloorInput>) -> String {
        let mut s = self.state.lock().await;
        let Some(b) = &mut s.building else { return NO_BLDG.into() };
        let mut floor = Floor::new(&i.name, i.level, i.height);
        if let Some(ch) = i.ceiling_height { floor.ceiling_height = ch; }
        let id = floor.id.to_string();
        b.floors.push(floor);
        format!("Floor '{}' ({id})", i.name)
    }

    #[tool(name = "add_wall", description = "壁を追加（ID を返す）")]
    async fn add_wall(&self, Parameters(i): Parameters<AddWallInput>) -> String {
        let mut s = self.state.lock().await;
        let Some(b) = &mut s.building else { return NO_BLDG.into() };
        let Some(floor) = b.floors.iter_mut().find(|f| f.name == i.floor) else { return format!("Error: Floor '{}' not found", i.floor) };
        let mut wall = Wall::new(Point2D::new(i.x1, i.y1), Point2D::new(i.x2, i.y2), i.thickness);
        if let Some(ext) = i.is_exterior { wall.is_exterior = ext; }
        if let Some(m) = &i.material { wall.material = parse_material(m); }
        wall.height = i.height;
        let id = wall.id.to_string();
        floor.walls.push(wall);
        id
    }

    #[tool(name = "add_opening", description = "開口部を追加")]
    async fn add_opening(&self, Parameters(i): Parameters<AddOpeningInput>) -> String {
        let mut s = self.state.lock().await;
        let Some(b) = &mut s.building else { return NO_BLDG.into() };
        let Some(floor) = b.floors.iter_mut().find(|f| f.name == i.floor) else { return format!("Error: Floor '{}' not found", i.floor) };
        let Ok(wid) = uuid::Uuid::parse_str(&i.wall_id) else { return "Error: Invalid wall_id".into() };
        let sill = i.sill_height.unwrap_or(0.0);
        let mut op = if sill > 0.0 { Opening::window(wid, i.position, i.width, i.height, sill) } else { Opening::door(wid, i.position, i.width, i.height) };
        if let Some(k) = &i.kind { op.kind = parse_opening_kind(k); }
        let id = op.id.to_string();
        floor.openings.push(op);
        id
    }

    #[tool(name = "add_room", description = "部屋を追加")]
    async fn add_room(&self, Parameters(i): Parameters<AddRoomInput>) -> String {
        let mut s = self.state.lock().await;
        let Some(b) = &mut s.building else { return NO_BLDG.into() };
        let Some(floor) = b.floors.iter_mut().find(|f| f.name == i.floor) else { return format!("Error: Floor '{}' not found", i.floor) };
        let boundary: Vec<Point2D> = i.boundary.iter().map(|p| Point2D::new(p.x, p.y)).collect();
        let mut room = Room::new(&i.name, boundary);
        room.floor_finish = i.floor_finish;
        room.has_floor_heating = i.floor_heating.unwrap_or(false);
        let area = room.area();
        floor.rooms.push(room);
        format!("{} ({area:.1} sqm)", i.name)
    }

    #[tool(name = "get_building_summary", description = "建物の概要")]
    async fn get_building_summary(&self, Parameters(_): Parameters<EmptyInput>) -> String {
        let s = self.state.lock().await;
        let Some(b) = &s.building else { return NO_BLDG.into() };
        let mut out = format!("=== {} ===\n", b.name);
        for floor in &b.floors {
            out += &format!("{}: 壁{} 開口{} 部屋{}\n", floor.name, floor.walls.len(), floor.openings.len(), floor.rooms.len());
            for room in &floor.rooms { out += &format!("  {} — {:.1}sqm\n", room.name, room.area()); }
        }
        out += &format!("延べ面積: {:.1}sqm", b.total_floor_area());
        out
    }

    #[tool(name = "get_building_json", description = "建物 JSON")]
    async fn get_building_json(&self, Parameters(_): Parameters<EmptyInput>) -> String {
        let s = self.state.lock().await;
        let Some(b) = &s.building else { return NO_BLDG.into() };
        serde_json::to_string_pretty(b).unwrap_or_else(|e| format!("Error: {e}"))
    }

    #[tool(name = "export_dxf", description = "DXF に出力")]
    async fn export_dxf(&self, Parameters(i): Parameters<ExportDxfInput>) -> String {
        let s = self.state.lock().await;
        let Some(b) = &s.building else { return NO_BLDG.into() };
        let enc = match i.encoding.as_deref() { Some("shift_jis" | "sjis") => cad_dxf::DxfEncoding::ShiftJis, _ => cad_dxf::DxfEncoding::Utf8 };
        match std::fs::File::create(&i.path) {
            Ok(mut f) => match cad_dxf::DxfExporter::new(enc).export(b, &mut f) { Ok(r) => format!("Exported: {r}"), Err(e) => format!("Error: {e}") },
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(name = "render_autocad", description = "AutoCAD に描画")]
    async fn render_autocad(&self, Parameters(i): Parameters<RenderAutocadInput>) -> String {
        let s = self.state.lock().await;
        let Some(b) = &s.building else { return NO_BLDG.into() };
        let origin = Point2D::new(i.origin_x.unwrap_or(0.0), i.origin_y.unwrap_or(0.0));
        match cad_acad::AcadRenderer::with_origin(origin).render_building(b) { Ok(r) => format!("{r}"), Err(e) => format!("Error: {e}") }
    }

    // === クラウド ===

    #[tool(name = "login", description = "Auth0 ログイン")]
    async fn login(&self, Parameters(i): Parameters<LoginInput>) -> String {
        let Some(db) = self.db.as_ref() else { return NO_DB.into() };
        match db.upsert_user_by_auth(&i.provider, &i.provider_user_id, &i.email, i.name.as_deref(), None).await {
            Ok(user) => {
                let name = user.name.clone().unwrap_or_default();
                let id = user.id.as_ref().map(|r| r.to_string()).unwrap_or_default();
                self.state.lock().await.current_user = Some(user);
                format!("Logged in: {name} ({id})")
            }
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(name = "list_workspaces", description = "ワークスペース一覧")]
    async fn list_workspaces(&self, Parameters(_): Parameters<EmptyInput>) -> String {
        let Some(db) = self.db.as_ref() else { return NO_DB.into() };
        let user = { let s = self.state.lock().await; s.current_user.clone() };
        let Some(user) = user else { return NO_LOGIN.into() };
        match db.list_workspaces(&user).await {
            Ok(ws) => { let mut o = format!("{} workspaces:\n", ws.len()); for w in &ws { o += &format!("  {} ({})\n", w.name, w.id.as_ref().map(|r| r.to_string()).unwrap_or_default()); } o }
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(name = "select_workspace", description = "ワークスペース選択")]
    async fn select_workspace(&self, Parameters(i): Parameters<SelectInput>) -> String {
        self.state.lock().await.current_workspace = Some(make_rid("workspaces", &i.id));
        format!("Workspace: {}", i.id)
    }

    #[tool(name = "create_project", description = "プロジェクト作成")]
    async fn create_project(&self, Parameters(i): Parameters<NameInput>) -> String {
        let Some(db) = self.db.as_ref() else { return NO_DB.into() };
        let (user, ws_id) = { let s = self.state.lock().await; (s.current_user.clone(), s.current_workspace.clone()) };
        let (Some(user), Some(ws_id)) = (user, ws_id) else { return "Error: Login and select workspace".into() };
        match db.create_project(&ws_id, &i.name, &user).await {
            Ok(p) => { let id = p.id.clone(); self.state.lock().await.current_project = id.clone(); format!("Project '{}' ({})", i.name, id.map(|r| r.to_string()).unwrap_or_default()) }
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(name = "list_projects", description = "プロジェクト一覧")]
    async fn list_projects(&self, Parameters(_): Parameters<EmptyInput>) -> String {
        let Some(db) = self.db.as_ref() else { return NO_DB.into() };
        let ws_id = { self.state.lock().await.current_workspace.clone() };
        let Some(ws_id) = ws_id else { return "Error: Select workspace".into() };
        match db.list_projects(&ws_id).await {
            Ok(ps) => { let mut o = format!("{} projects:\n", ps.len()); for p in &ps { o += &format!("  {} ({})\n", p.name, p.id.as_ref().map(|r| r.to_string()).unwrap_or_default()); } o }
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(name = "save_building", description = "建物をクラウドに保存")]
    async fn save_building(&self, Parameters(_): Parameters<EmptyInput>) -> String {
        let Some(db) = self.db.as_ref() else { return NO_DB.into() };
        let (building, user, proj_id) = {
            let s = self.state.lock().await;
            (s.building.clone(), s.current_user.clone(), s.current_project.clone())
        };
        let (Some(building), Some(user), Some(proj_id)) = (building, user, proj_id) else { return "Error: Need building + login + project".into() };
        match db.save_building(&proj_id, &building, &user).await {
            Ok(r) => format!("Saved '{}' ({})", building.name, r.id.map(|r| r.to_string()).unwrap_or_default()),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(name = "load_building", description = "クラウドから建物を読込")]
    async fn load_building(&self, Parameters(i): Parameters<SelectInput>) -> String {
        let Some(db) = self.db.as_ref() else { return NO_DB.into() };
        let rid = make_rid("buildings", &i.id);
        match db.load_building(&rid).await {
            Ok(b) => { let name = b.name.clone(); self.state.lock().await.building = Some(b); format!("Loaded '{name}'") }
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(name = "list_buildings", description = "建物一覧")]
    async fn list_buildings(&self, Parameters(_): Parameters<EmptyInput>) -> String {
        let Some(db) = self.db.as_ref() else { return NO_DB.into() };
        let proj_id = { self.state.lock().await.current_project.clone() };
        let Some(proj_id) = proj_id else { return "Error: Select project".into() };
        match db.list_buildings(&proj_id).await {
            Ok(bs) => { let mut o = format!("{} buildings:\n", bs.len()); for b in &bs { o += &format!("  {} ({})\n", b.name, b.id.as_ref().map(|r| r.to_string()).unwrap_or_default()); } o }
            Err(e) => format!("Error: {e}"),
        }
    }
}

#[tool_handler]
impl ServerHandler for GfpCadMcpServer {
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::new(ServerCapabilities::builder().enable_tools().build());
        info.instructions = Some("gfp-cad MCP — Lightweight BIM + Cloud. create_building → add_floor/wall/room → save_building / export_dxf / render_autocad".into());
        info
    }
}
