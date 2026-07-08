//! 壁グラフの平面トポロジ — 壁が囲む「面(face)」を導出する（DCEL / ハーフエッジ）。
//!
//! 部屋は独立ポリゴンではなく、壁グラフの**有界面**として導出する（Revit の Room 相当）。
//! 導出時に **planarize**（壁のスパン上に載る他ノードで壁を分割）してから面トレースするので、
//! T字接合（端点が別壁の途中に載る）も、保存モデルを変えずに正しく連結扱いになる。

use std::collections::HashMap;

use crate::{Node, NodeId, Point2D, Wall};

/// 壁グラフの1つの有界面（部屋候補）。`nodes`/`polygon` は外周ループ、`holes` は内部に
/// 含まれる非連結ループ（浮いたクローゼット等の穴）、`area` は**穴を差し引いた正味面積**(mm²)。
#[derive(Debug, Clone)]
pub struct Face {
    pub nodes: Vec<NodeId>,
    pub polygon: Vec<Point2D>,
    pub holes: Vec<Vec<Point2D>>,
    pub area: f64,
}

/// 点がセグメント a-b の**内部**（端点除く）に `tol` 以内で載っているか。載っていれば
/// パラメータ t∈(0,1) を返す。
fn point_on_segment(p: Point2D, a: Point2D, b: Point2D, tol: f64) -> Option<f64> {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len2 = dx * dx + dy * dy;
    if len2 < 1e-9 {
        return None;
    }
    let t = ((p.x - a.x) * dx + (p.y - a.y) * dy) / len2;
    if !(1e-9..=(1.0 - 1e-9)).contains(&t) {
        return None; // 端点付近は除外
    }
    let proj = Point2D::new(a.x + dx * t, a.y + dy * t);
    if (p.x - proj.x).hypot(p.y - proj.y) <= tol {
        Some(t)
    } else {
        None
    }
}

/// ポリゴンの符号付き面積（shoelace、mm²）。正 = CCW。
fn signed_area(poly: &[Point2D]) -> f64 {
    let n = poly.len();
    if n < 3 {
        return 0.0;
    }
    let mut s = 0.0;
    for i in 0..n {
        let a = poly[i];
        let b = poly[(i + 1) % n];
        s += a.x * b.y - b.x * a.y;
    }
    s / 2.0
}

/// 2線分 a1-a2, b1-b2 が**内部で真に交差**するか（端点共有・接触は除く）。
pub fn segments_cross(a1: Point2D, a2: Point2D, b1: Point2D, b2: Point2D) -> bool {
    let cross =
        |o: Point2D, p: Point2D, q: Point2D| (p.x - o.x) * (q.y - o.y) - (p.y - o.y) * (q.x - o.x);
    let d1 = cross(b1, b2, a1);
    let d2 = cross(b1, b2, a2);
    let d3 = cross(a1, a2, b1);
    let d4 = cross(a1, a2, b2);
    // 端点接触（いずれかが 0）は交差扱いしない（共有端点・T字は planarize の担当）。
    const E: f64 = 1e-6;
    if d1.abs() < E || d2.abs() < E || d3.abs() < E || d4.abs() < E {
        return false;
    }
    (d1 > 0.0) != (d2 > 0.0) && (d3 > 0.0) != (d4 > 0.0)
}

/// 点がセグメント a-b の内部に `tol` 以内で載っているか（端点除外）。validate 用に公開。
pub fn point_near_segment(p: Point2D, a: Point2D, b: Point2D, tol: f64) -> bool {
    point_on_segment(p, a, b, tol).is_some()
}

/// 点がポリゴン内部か（even-odd）。
pub fn point_in_polygon(p: Point2D, poly: &[Point2D]) -> bool {
    if poly.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let (xi, yi) = (poly[i].x, poly[i].y);
        let (xj, yj) = (poly[j].x, poly[j].y);
        if (yi > p.y) != (yj > p.y) && p.x < (xj - xi) * (p.y - yi) / (yj - yi) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// 壁グラフの全ての面を導出する。`tol` は planarize（スパン上ノード判定）の許容。
///
/// 手順:
/// 1. **planarize**: 各壁を、そのスパン上に載る他ノードで細分してハーフエッジ集合を作る。
/// 2. 各ノードで出辺を角度ソート。
/// 3. `next(u→v)` = v の角度環で twin(v→u) の時計回り隣を採る。
/// 4. next をたどって面をトレース。
pub fn faces(nodes: &[Node], walls: &[Wall], tol: f64) -> Vec<Face> {
    let pt: HashMap<NodeId, Point2D> = nodes.iter().map(|n| (n.id, n.point)).collect();

    // --- 1. planarize → ハーフエッジ (from, to)。twin = index ^ 1 ---
    let mut he: Vec<(NodeId, NodeId)> = Vec::new();
    for w in walls {
        let (Some(&a), Some(&b)) = (pt.get(&w.start), pt.get(&w.end)) else {
            continue;
        };
        if w.start == w.end {
            continue;
        }
        // 壁のスパン上に載る他ノードを t 順に集めて細分。
        let mut on: Vec<(f64, NodeId)> = vec![(0.0, w.start), (1.0, w.end)];
        for n in nodes {
            if n.id == w.start || n.id == w.end {
                continue;
            }
            if let Some(t) = point_on_segment(n.point, a, b, tol) {
                on.push((t, n.id));
            }
        }
        on.sort_by(|x, y| x.0.total_cmp(&y.0));
        on.dedup_by_key(|(_, id)| *id);
        for pair in on.windows(2) {
            let (u, v) = (pair[0].1, pair[1].1);
            // 同一 id / 同一座標（zero-length）は角度環を汚すので除外。
            if u != v && pt[&u].distance_to(&pt[&v]) > 1e-6 {
                he.push((u, v));
                he.push((v, u));
            }
        }
    }
    if he.is_empty() {
        return Vec::new();
    }

    // --- 2. 各ノードの出辺を角度ソート ---
    let angle = |i: usize| -> f64 {
        let (from, to) = he[i];
        let (a, b) = (pt[&from], pt[&to]);
        (b.y - a.y).atan2(b.x - a.x)
    };
    let mut out: HashMap<NodeId, Vec<usize>> = HashMap::new();
    for (i, (from, _)) in he.iter().enumerate() {
        out.entry(*from).or_default().push(i);
    }
    for ring in out.values_mut() {
        ring.sort_by(|&i, &j| angle(i).total_cmp(&angle(j)));
    }
    let mut pos_in_ring: HashMap<usize, usize> = HashMap::new();
    for ring in out.values() {
        for (k, &i) in ring.iter().enumerate() {
            pos_in_ring.insert(i, k);
        }
    }

    // next(h): h=(u→v)。twin=h^1=(v→u)。v の角度環で twin の時計回り隣（k-1）を採る。
    let next = |h: usize| -> usize {
        let twin = h ^ 1;
        let node = he[twin].0; // = v
        let ring = &out[&node];
        let k = pos_in_ring[&twin];
        let deg = ring.len();
        ring[(k + deg - 1) % deg]
    };

    // --- 3. 全サイクルのトレース ---
    struct Cycle {
        nodes: Vec<NodeId>,
        polygon: Vec<Point2D>,
        area: f64, // 符号付き
    }
    let mut visited = vec![false; he.len()];
    let mut cycles: Vec<Cycle> = Vec::new();
    for start in 0..he.len() {
        if visited[start] {
            continue;
        }
        let mut loop_nodes = Vec::new();
        let mut h = start;
        loop {
            visited[h] = true;
            loop_nodes.push(he[h].0);
            h = next(h);
            if h == start || visited[h] {
                break;
            }
        }
        let polygon: Vec<Point2D> = loop_nodes.iter().map(|id| pt[id]).collect();
        let area = signed_area(&polygon);
        cycles.push(Cycle {
            nodes: loop_nodes,
            polygon,
            area,
        });
    }

    // --- 4. 有界面 + 穴の関連付け ---
    // 有界面 = CCW(正)。CW(負)サイクルは (a) 各連結成分の外周を CW で見た**外側境界**と、
    // (b) 非連結の内側ループ（浮いたクローゼット等）の外周 = **穴** から成る。
    //
    // 各負サイクルを「代表点を含み、かつ **面積が |穴| より真に大きい**（＝真に囲む親）
    // 最小の正面」へ穴として割り当てて差し引く。該当する親が無ければ、それは連結成分の
    // 外側境界（非有界）なので捨てる。この規則は非連結成分が複数あっても（各成分の外周は
    // より大きい親を持たず捨てられる）、多階層ネストでも（各穴が1つ上の親に付く）正しい。
    const EPS: f64 = 1.0;
    let mut faces: Vec<Face> = cycles
        .iter()
        .filter(|c| c.area > EPS)
        .map(|c| Face {
            nodes: c.nodes.clone(),
            polygon: c.polygon.clone(),
            holes: Vec::new(),
            area: c.area,
        })
        .collect();
    for c in cycles.iter().filter(|c| c.area < -EPS) {
        let hole_area = c.area.abs();
        let rep = interior_point(&c.polygon);
        if let Some(idx) = faces
            .iter()
            .enumerate()
            .filter(|(_, f)| {
                f.area > hole_area + EPS // 真に囲む親（|穴| より大きい面）だけ
                    && f.polygon.len() >= 3
                    && point_in_polygon(rep, &f.polygon)
            })
            .min_by(|a, b| a.1.area.total_cmp(&b.1.area))
            .map(|(i, _)| i)
        {
            faces[idx].area -= hole_area;
            faces[idx].holes.push(c.polygon.clone());
        }
        // 親が無ければ連結成分の外周（非有界）→ 捨てる。
    }
    faces
}

/// 単純多角形の**内部が保証された1点**。頂点重心が内部ならそれを、外（凹形状）なら
/// 重心の y を通る水平走査線で最初の内部スパンの中点を返す（穴の親判定の代表点に使う）。
fn interior_point(poly: &[Point2D]) -> Point2D {
    let n = poly.len();
    let c = {
        let (sx, sy) = poly
            .iter()
            .fold((0.0, 0.0), |(sx, sy), p| (sx + p.x, sy + p.y));
        Point2D::new(sx / n.max(1) as f64, sy / n.max(1) as f64)
    };
    if n < 3 || point_in_polygon(c, poly) {
        return c;
    }
    let y = c.y;
    let mut xs: Vec<f64> = Vec::new();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        if (a.y > y) != (b.y > y) {
            xs.push(a.x + (y - a.y) / (b.y - a.y) * (b.x - a.x));
        }
    }
    xs.sort_by(f64::total_cmp);
    if xs.len() >= 2 {
        Point2D::new((xs[0] + xs[1]) / 2.0, y)
    } else {
        c
    }
}

/// 点が面の内部か（外周に入り、かつどの穴にも入っていない）。
pub fn face_contains(face: &Face, p: Point2D) -> bool {
    point_in_polygon(p, &face.polygon) && !face.holes.iter().any(|h| point_in_polygon(p, h))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Wall;

    #[test]
    fn rectangle_split_by_partition_yields_two_rooms() {
        // 10000×8000 の矩形を X=5000 の仕切りで2分割。仕切り端(ps/pn)は外壁のスパン上
        // （＝ T字接合）→ planarize で外壁が分割され、面が2つに割れる。
        let sw = Node::new(Point2D::new(0.0, 0.0));
        let se = Node::new(Point2D::new(10000.0, 0.0));
        let nw = Node::new(Point2D::new(0.0, 8000.0));
        let ne = Node::new(Point2D::new(10000.0, 8000.0));
        let ps = Node::new(Point2D::new(5000.0, 0.0));
        let pn = Node::new(Point2D::new(5000.0, 8000.0));
        let walls = vec![
            Wall::new(sw.id, se.id, 150.0),
            Wall::new(nw.id, ne.id, 150.0),
            Wall::new(sw.id, nw.id, 150.0),
            Wall::new(se.id, ne.id, 150.0),
            Wall::new(ps.id, pn.id, 100.0),
        ];
        let nodes = vec![sw, se, nw, ne, ps, pn];

        let bounded: Vec<Face> = faces(&nodes, &walls, 1.0)
            .into_iter()
            .filter(|f| f.area > 1.0)
            .collect();
        assert_eq!(bounded.len(), 2, "矩形+仕切り → 2部屋のはず");
        for f in &bounded {
            let sqm = f.area / 1_000_000.0;
            assert!((sqm - 40.0).abs() < 0.1, "各室 5000×8000=40sqm, got {sqm}");
        }

        // シード点で部屋を引けること（左室 / 右室）。
        let left = faces(&nodes, &walls, 1.0)
            .into_iter()
            .filter(|f| f.area > 1.0)
            .find(|f| point_in_polygon(Point2D::new(2500.0, 4000.0), &f.polygon));
        assert!(left.is_some(), "左室のシードが面に入る");
    }
}
