use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::Point2D;

/// 部屋
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub id: Uuid,
    /// 室名
    pub name: String,
    /// 部屋の輪郭（閉じたポリゴン、反時計回り）(mm)
    pub boundary: Vec<Point2D>,
    /// 壁 ID の一覧（この部屋を囲む壁）
    pub wall_ids: Vec<Uuid>,
    /// 天井高 (mm) — None の場合は階の天井高に従う
    pub ceiling_height: Option<f64>,
    /// 床仕上げ
    pub floor_finish: Option<String>,
    /// 壁仕上げ
    pub wall_finish: Option<String>,
    /// 天井仕上げ
    pub ceiling_finish: Option<String>,
    /// 床暖房
    pub has_floor_heating: bool,
}

impl Room {
    pub fn new(name: impl Into<String>, boundary: Vec<Point2D>) -> Self {
        Self {
            id: Uuid::now_v7(),
            name: name.into(),
            boundary,
            wall_ids: Vec::new(),
            ceiling_height: None,
            floor_finish: None,
            wall_finish: None,
            ceiling_finish: None,
            has_floor_heating: false,
        }
    }

    /// 床面積 (sqm) — Shoelace formula
    pub fn area(&self) -> f64 {
        let n = self.boundary.len();
        if n < 3 {
            return 0.0;
        }
        let mut sum = 0.0;
        for i in 0..n {
            let j = (i + 1) % n;
            sum += self.boundary[i].x * self.boundary[j].y;
            sum -= self.boundary[j].x * self.boundary[i].y;
        }
        (sum.abs() / 2.0) / 1_000_000.0 // mm² → sqm
    }

    /// 周長 (mm)
    pub fn perimeter(&self) -> f64 {
        let n = self.boundary.len();
        (0..n)
            .map(|i| self.boundary[i].distance_to(&self.boundary[(i + 1) % n]))
            .sum()
    }
}
