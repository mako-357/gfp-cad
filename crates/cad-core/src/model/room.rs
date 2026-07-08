use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::Point2D;

/// 部屋。
///
/// 境界は**持たない** —— `seed`（部屋内部の1点）を壁グラフの面に紐づけ、境界・面積は
/// `Floor::room_boundary` / `Floor::room_area` で**導出**する（Revit の Room 相当）。
/// これにより壁を動かすと部屋も追従し、壁と部屋が desync しない。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub id: Uuid,
    /// 室名
    pub name: String,
    /// 部屋内部の1点 (mm)。この点を含む壁グラフの面が部屋の領域になる。
    pub seed: Point2D,
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
    /// 内部の1点（シード）を指定して部屋を作る。境界・面積は `Floor` 経由で導出する。
    pub fn new(name: impl Into<String>, seed: Point2D) -> Self {
        Self {
            id: Uuid::now_v7(),
            name: name.into(),
            seed,
            ceiling_height: None,
            floor_finish: None,
            wall_finish: None,
            ceiling_finish: None,
            has_floor_heating: false,
        }
    }
}
