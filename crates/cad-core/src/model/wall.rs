use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{NodeId, Point2D};

/// 壁。端点は `NodeId` でノードを参照する（座標は持たない）。
///
/// 2 枚の壁が同じ端点ノードを共有する = 接合していて構造的に離れない。
/// 座標が必要な処理（描画・出力・当たり判定）は `Floor` のノード解決経由で行う。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wall {
    pub id: Uuid,
    /// 壁芯の始点ノード
    pub start: NodeId,
    /// 壁芯の終点ノード
    pub end: NodeId,
    /// 壁厚 (mm)
    pub thickness: f64,
    /// 壁高 (mm) — None の場合は階高に従う
    pub height: Option<f64>,
    /// 壁材料
    pub material: WallMaterial,
    /// 仕上げ（内側）
    pub finish_interior: Option<String>,
    /// 仕上げ（外側）
    pub finish_exterior: Option<String>,
    /// 外壁かどうか
    pub is_exterior: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum WallMaterial {
    /// RC (鉄筋コンクリート)
    RC,
    /// 軽量鉄骨 (LGS)
    #[default]
    LGS,
    /// 木造
    Wood,
    /// ALC
    ALC,
    /// CB (コンクリートブロック)
    CB,
    /// その他
    Other(String),
}

impl Wall {
    /// 端点ノードを指定して壁を作る。座標からの生成は `Floor::add_wall` を使う。
    pub fn new(start: NodeId, end: NodeId, thickness: f64) -> Self {
        Self {
            id: Uuid::now_v7(),
            start,
            end,
            thickness,
            height: None,
            material: WallMaterial::default(),
            finish_interior: None,
            finish_exterior: None,
            is_exterior: false,
        }
    }
}

/// 壁芯 `a`→`b` を厚み/2 でオフセットした面の4隅（start±n, end±n, mm）。
///
/// 「壁の面とは何か」の唯一の定義。描画(cad-app / cad-acad)と DXF 出力(cad-dxf)は
/// すべてこれを共有する。退化した壁（長さ≈0）では `None`。
pub fn face_quad(a: Point2D, b: Point2D, thickness: f64) -> Option<[Point2D; 4]> {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 0.1 {
        return None;
    }
    let t = thickness / 2.0;
    let nx = -dy / len * t;
    let ny = dx / len * t;
    Some([
        Point2D::new(a.x + nx, a.y + ny),
        Point2D::new(b.x + nx, b.y + ny),
        Point2D::new(b.x - nx, b.y - ny),
        Point2D::new(a.x - nx, a.y - ny),
    ])
}
