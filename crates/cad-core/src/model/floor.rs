use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{Node, NodeId, Opening, Room, Wall};
use crate::Point2D;

/// 端点ノードのマージ許容（mm）。この距離内の座標は同じノードとみなす。
const NODE_MERGE_TOL: f64 = 1.0;

/// 階
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Floor {
    pub id: Uuid,
    /// 階名（"1F", "2F", "B1F" 等）
    pub name: String,
    /// FL レベル (mm) — GL からの高さ
    pub level: f64,
    /// 階高 (mm)
    pub height: f64,
    /// 天井高 (mm)
    pub ceiling_height: f64,
    /// 壁グラフのノード（接合点。座標の唯一の在処）
    #[serde(default)]
    pub nodes: Vec<Node>,
    /// 壁（端点はノードを参照）
    pub walls: Vec<Wall>,
    /// 開口部（ドア・窓）
    pub openings: Vec<Opening>,
    /// 部屋
    pub rooms: Vec<Room>,
}

impl Floor {
    pub fn new(name: impl Into<String>, level: f64, height: f64) -> Self {
        Self {
            id: Uuid::now_v7(),
            name: name.into(),
            level,
            height,
            ceiling_height: height - 300.0, // デフォルト: 階高 - 300mm
            nodes: Vec::new(),
            walls: Vec::new(),
            openings: Vec::new(),
            rooms: Vec::new(),
        }
    }

    /// 床面積の合計 (sqm)
    pub fn area(&self) -> f64 {
        self.rooms.iter().map(|r| r.area()).sum()
    }

    // === ノードグラフ ===

    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn node_point(&self, id: NodeId) -> Option<Point2D> {
        self.node(id).map(|n| n.point)
    }

    /// 座標に対応するノード ID を返す。既存ノードが許容内にあれば再利用し（＝接合）、
    /// 無ければ新規作成する。これにより端点を共有する壁は同じノードを指す。
    pub fn add_node(&mut self, point: Point2D) -> NodeId {
        if let Some(n) = self.nodes.iter().find(|n| {
            (n.point.x - point.x).abs() < NODE_MERGE_TOL
                && (n.point.y - point.y).abs() < NODE_MERGE_TOL
        }) {
            return n.id;
        }
        let node = Node::new(point);
        let id = node.id;
        self.nodes.push(node);
        id
    }

    /// 壁の両端の座標を解決する（どちらかのノードが欠けていれば `None`）。
    pub fn wall_endpoints(&self, wall: &Wall) -> Option<(Point2D, Point2D)> {
        Some((self.node_point(wall.start)?, self.node_point(wall.end)?))
    }

    /// 壁面の4隅（`cad_core::face_quad` の共有定義をノード解決して適用）。
    pub fn wall_face_quad(&self, wall: &Wall) -> Option<[Point2D; 4]> {
        let (a, b) = self.wall_endpoints(wall)?;
        crate::face_quad(a, b, wall.thickness)
    }

    /// 壁芯の長さ (mm)。
    pub fn wall_length(&self, wall: &Wall) -> Option<f64> {
        let (a, b) = self.wall_endpoints(wall)?;
        Some(a.distance_to(&b))
    }

    /// 座標から壁を追加する。端点は `add_node` でマージ解決され、既存の端点と一致すれば
    /// 自動的に接合する。壁 ID を返す（プロパティ設定は `walls` 経由で行う）。
    pub fn add_wall(&mut self, a: Point2D, b: Point2D, thickness: f64) -> Uuid {
        let start = self.add_node(a);
        let end = self.add_node(b);
        let wall = Wall::new(start, end, thickness);
        let id = wall.id;
        self.walls.push(wall);
        id
    }

    /// `add_wall` と同じだが、追加した壁への `&mut` を返す（材料・外壁フラグ等を続けて
    /// 設定できる。id は `.id` で取れる）。
    pub fn add_wall_mut(&mut self, a: Point2D, b: Point2D, thickness: f64) -> &mut Wall {
        let id = self.add_wall(a, b, thickness);
        self.walls
            .iter_mut()
            .find(|w| w.id == id)
            .expect("just added")
    }
}
