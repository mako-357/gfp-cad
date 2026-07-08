use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::Point2D;

/// 壁
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wall {
    pub id: Uuid,
    /// 壁芯の始点 (mm)
    pub start: Point2D,
    /// 壁芯の終点 (mm)
    pub end: Point2D,
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
    pub fn new(start: Point2D, end: Point2D, thickness: f64) -> Self {
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

    /// 壁芯の長さ (mm)
    pub fn length(&self) -> f64 {
        self.start.distance_to(&self.end)
    }

    /// 壁面積 (sqm) — 片面
    pub fn area(&self, floor_height: f64) -> f64 {
        let h = self.height.unwrap_or(floor_height);
        self.length() * h / 1_000_000.0
    }

    /// 壁芯を厚み分オフセットした面の4隅 (start±n, end±n, mm)。
    ///
    /// これが「壁の面とは何か」の唯一の定義。描画(cad-app / cad-acad)と
    /// DXF 出力(cad-dxf)はすべてこれを共有し、法線式の重複を避ける。
    /// 退化した壁(長さ≈0)では `None`。
    pub fn face_quad(&self) -> Option<[Point2D; 4]> {
        let dx = self.end.x - self.start.x;
        let dy = self.end.y - self.start.y;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 0.1 {
            return None;
        }
        let t = self.thickness / 2.0;
        let nx = -dy / len * t;
        let ny = dx / len * t;
        Some([
            Point2D::new(self.start.x + nx, self.start.y + ny),
            Point2D::new(self.end.x + nx, self.end.y + ny),
            Point2D::new(self.end.x - nx, self.end.y - ny),
            Point2D::new(self.start.x - nx, self.start.y - ny),
        ])
    }
}
