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

    /// 床面積の合計 (sqm) — 各部屋の導出面積の総和。**同一面に複数シードが落ちても
    /// 面は1回だけ計上**（仕切り削除後の二重計上を防ぐ）。
    pub fn area(&self) -> f64 {
        use std::collections::HashSet;
        let faces = self.faces();
        let mut seen: HashSet<Vec<NodeId>> = HashSet::new();
        let mut total = 0.0;
        for r in &self.rooms {
            if let Some(f) = resolve_seed(&faces, r.seed)
                && seen.insert(face_key(&f))
            {
                total += f.area / 1_000_000.0;
            }
        }
        total
    }

    // === 面（部屋）の導出 ===

    /// 壁グラフの有界面（部屋候補、穴を差し引いた正味面積）を導出する。
    pub fn faces(&self) -> Vec<crate::Face> {
        crate::topology::faces(&self.nodes, &self.walls, NODE_MERGE_TOL)
    }

    /// 点を含む最小の有界面（入れ子時は内側）。
    pub fn face_at(&self, p: Point2D) -> Option<crate::Face> {
        resolve_seed(&self.faces(), p)
    }

    /// 全部屋を **1回の面導出**で解決する（room ごとに faces() を再構築しない）。
    /// 描画・ラベル・面積の hot path はこれを使う。
    pub fn rooms_faces(&self) -> Vec<(&Room, Option<crate::Face>)> {
        let faces = self.faces();
        self.rooms
            .iter()
            .map(|r| (r, resolve_seed(&faces, r.seed)))
            .collect()
    }

    /// 部屋の境界ポリゴン（シードを含む面の外周）。未囲いなら `None`。
    pub fn room_boundary(&self, room: &Room) -> Option<Vec<Point2D>> {
        self.face_at(room.seed).map(|f| f.polygon)
    }

    /// 部屋の床面積 (sqm)。穴を差し引いた正味。
    pub fn room_area(&self, room: &Room) -> Option<f64> {
        self.face_at(room.seed).map(|f| f.area / 1_000_000.0)
    }

    /// 部屋の周長 (mm、外周）。面を1回だけ解決する。
    pub fn room_perimeter(&self, room: &Room) -> Option<f64> {
        let poly = self.face_at(room.seed)?.polygon;
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

    /// ノードを移動する（座標を差し替えるだけ）。そのノードを端点に持つ壁は同じ NodeId を
    /// 参照しているので**自動的に追従**し、部屋も面の再導出で追従する。存在しない id は無視。
    /// マージ（別ノードへの吸着・接合）はしない。
    pub fn move_node(&mut self, id: NodeId, point: Point2D) {
        if let Some(n) = self.nodes.iter_mut().find(|n| n.id == id) {
            n.point = point;
        }
    }

    /// ノード `id` を端点に持つ壁の本数（インスペクタ表示・degree 判定用）。
    pub fn walls_at_node(&self, id: NodeId) -> usize {
        self.walls
            .iter()
            .filter(|w| w.start == id || w.end == id)
            .count()
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

        // 未囲い部屋 + 同一面への複数シード。
        let faces = self.faces();
        let mut claims: std::collections::HashMap<Vec<NodeId>, Vec<String>> = Default::default();
        for r in &self.rooms {
            match resolve_seed(&faces, r.seed) {
                None => issues.push(format!(
                    "floor {:?}: 部屋 {:?} のシードが閉領域に無い (未囲い)",
                    self.name, r.name
                )),
                Some(f) => claims.entry(face_key(&f)).or_default().push(r.name.clone()),
            }
        }
        for names in claims.values().filter(|v| v.len() > 1) {
            issues.push(format!(
                "floor {:?}: 同一の面に複数の部屋シード {names:?}（面積が二重計上される）",
                self.name
            ));
        }

        // ノード無しで内部交差する壁対。
        for i in 0..self.walls.len() {
            for j in (i + 1)..self.walls.len() {
                let (w1, w2) = (&self.walls[i], &self.walls[j]);
                if w1.start == w2.start
                    || w1.start == w2.end
                    || w1.end == w2.start
                    || w1.end == w2.end
                {
                    continue;
                }
                if let (Some((a1, a2)), Some((b1, b2))) =
                    (self.wall_endpoints(w1), self.wall_endpoints(w2))
                    && crate::topology::segments_cross(a1, a2, b1, b2)
                {
                    issues.push(format!(
                        "floor {:?}: 壁 {} と {} がノード無しで交差（面が壊れる）",
                        self.name, w1.id, w2.id
                    ));
                }
            }
        }

        // 接合し損ね: **宙ぶらり(degree 1)** の端点が別壁スパンに許容超〜10倍で近接
        // （planarize で繋がらない失敗 T字）。共有コーナー（意図的な二重壁・空洞壁の端）は
        // degree≥2 なので誤検出しない。
        let degree = |id: NodeId| -> usize {
            self.walls
                .iter()
                .filter(|w| w.start == id || w.end == id)
                .count()
        };
        for w in &self.walls {
            for ep in [w.start, w.end] {
                if degree(ep) != 1 {
                    continue; // 宙ぶらりの端だけを対象に
                }
                let Some(p) = self.node_point(ep) else {
                    continue;
                };
                for other in &self.walls {
                    if other.id == w.id || other.start == ep || other.end == ep {
                        continue;
                    }
                    if let Some((a, b)) = self.wall_endpoints(other)
                        && !crate::topology::point_near_segment(p, a, b, NODE_MERGE_TOL)
                        && crate::topology::point_near_segment(p, a, b, NODE_MERGE_TOL * 10.0)
                    {
                        issues.push(format!(
                            "floor {:?}: 壁 {} の端点が壁 {} のスパンに近接だが未接合（許容外）",
                            self.name, w.id, other.id
                        ));
                    }
                }
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

/// 面の一意キー = ソート済みノード集合。隣接面はノードを共有するので単一ノードでは
/// 一意にならない（共有された最小ノードで別々の面が衝突し面積を取りこぼす）。
fn face_key(f: &crate::Face) -> Vec<NodeId> {
    let mut k = f.nodes.clone();
    k.sort();
    k
}

/// 点を含む最小の有界面を（事前計算済みの faces から）選ぶ。入れ子時は内側を採る。
/// 穴の内部は「含まない」（穴には別の面が張られているか、無ければ未囲い扱い）。
fn resolve_seed(faces: &[crate::Face], seed: Point2D) -> Option<crate::Face> {
    faces
        .iter()
        .filter(|f| crate::topology::face_contains(f, seed))
        .min_by(|a, b| a.area.total_cmp(&b.area))
        .cloned()
}
