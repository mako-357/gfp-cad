use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{Node, NodeId, Opening, Room, Wall};
use crate::Point2D;

/// 端点ノードのマージ許容（mm、Euclidean）。この距離内の座標は同じノードとみなして
/// 接合する。接合は座標近接ヒューリスティックなので、**確実に接合させたい入力**（GUI 作図・
/// DXF インポート等）は事前にグリッド/既存ノードへスナップすること（GUI は snap 実装済み）。
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
    /// 壁グラフのノード（接合点。座標の唯一の在処）。
    ///
    /// `#[serde(default)]` は「nodes フィールドが無い JSON」を空 Vec で受けるためだけのもの。
    /// **ba6c9a6 以前の Building とは非互換**（Wall.start/end が Point2D→NodeId に型変更された
    /// ため、旧保存データのロードには移行が必要。後方互換ではない）。
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

    /// 床面積の合計 (sqm) — 各部屋の導出面積の総和。
    pub fn area(&self) -> f64 {
        self.rooms.iter().filter_map(|r| self.room_area(r)).sum()
    }

    // === 面（部屋）の導出 ===

    /// 壁グラフの有界面（部屋候補）を導出する。外部の非有界面（符号付き面積が負）は除く。
    pub fn faces(&self) -> Vec<crate::Face> {
        crate::topology::faces(&self.nodes, &self.walls, NODE_MERGE_TOL)
            .into_iter()
            .filter(|f| f.area > 1.0) // 正の有界面のみ（外部=負・退化=0 を除外）
            .collect()
    }

    /// 点を含む最小の有界面（入れ子時は内側）。
    pub fn face_at(&self, p: Point2D) -> Option<crate::Face> {
        self.faces()
            .into_iter()
            .filter(|f| crate::topology::point_in_polygon(p, &f.polygon))
            .min_by(|a, b| a.area.total_cmp(&b.area))
    }

    /// 部屋の境界ポリゴン（シードを含む面）。未囲い（外部に落ちた）なら `None`。
    pub fn room_boundary(&self, room: &Room) -> Option<Vec<Point2D>> {
        self.face_at(room.seed).map(|f| f.polygon)
    }

    /// 部屋の床面積 (sqm)。
    pub fn room_area(&self, room: &Room) -> Option<f64> {
        self.face_at(room.seed).map(|f| f.area.abs() / 1_000_000.0)
    }

    /// 部屋の周長 (mm)。
    pub fn room_perimeter(&self, room: &Room) -> Option<f64> {
        let poly = self.room_boundary(room)?;
        let n = poly.len();
        Some(
            (0..n)
                .map(|i| poly[i].distance_to(&poly[(i + 1) % n]))
                .sum(),
        )
    }

    // === ノードグラフ ===

    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn node_point(&self, id: NodeId) -> Option<Point2D> {
        self.node(id).map(|n| n.point)
    }

    /// 座標に対応するノード ID を返す。許容(`NODE_MERGE_TOL`, Euclidean)内で**最も近い**
    /// 既存ノードがあれば再利用し（＝接合）、無ければ新規作成する。これにより端点を共有する
    /// 壁は同じノードを指す。
    ///
    /// TODO: 壁数が数千規模の一括インポートでは線形探索が O(n²) になる。その経路を作る際は
    /// 空間ハッシュ（セルサイズ ≒ NODE_MERGE_TOL のグリッドバケット）へ。
    pub fn add_node(&mut self, point: Point2D) -> NodeId {
        let nearest = self
            .nodes
            .iter()
            .map(|n| (n.id, n.point.distance_to(&point)))
            .filter(|(_, d)| *d < NODE_MERGE_TOL)
            .min_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((id, _)) = nearest {
            return id;
        }
        let node = Node::new(point);
        let id = node.id;
        self.nodes.push(node);
        id
    }

    /// モデル整合を検証し、問題（dangling / orphan / 開口の親壁欠落）の一覧を返す。
    /// 空なら健全。load 後・export 前に呼び、silent なデータ欠落を検出するために使う。
    pub fn validate(&self) -> Vec<String> {
        use std::collections::HashSet;
        let mut issues = Vec::new();
        let node_ids: HashSet<NodeId> = self.nodes.iter().map(|n| n.id).collect();
        let mut referenced: HashSet<NodeId> = HashSet::new();
        for w in &self.walls {
            for (which, id) in [("start", w.start), ("end", w.end)] {
                if !node_ids.contains(&id) {
                    issues.push(format!(
                        "floor {:?}: 壁 {} の {} ノード {id} が存在しない (dangling)",
                        self.name, w.id, which
                    ));
                }
                referenced.insert(id);
            }
        }
        for n in &self.nodes {
            if !referenced.contains(&n.id) {
                issues.push(format!(
                    "floor {:?}: ノード {} をどの壁も参照していない (orphan)",
                    self.name, n.id
                ));
            }
        }
        let wall_ids: HashSet<Uuid> = self.walls.iter().map(|w| w.id).collect();
        for o in &self.openings {
            if !wall_ids.contains(&o.wall_id) {
                issues.push(format!(
                    "floor {:?}: 開口 {} の親壁 {} が存在しない",
                    self.name, o.id, o.wall_id
                ));
            }
        }
        issues
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
