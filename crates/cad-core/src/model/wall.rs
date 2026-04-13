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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WallMaterial {
    /// RC (鉄筋コンクリート)
    RC,
    /// 軽量鉄骨 (LGS)
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

impl Default for WallMaterial {
    fn default() -> Self {
        Self::LGS
    }
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
}
