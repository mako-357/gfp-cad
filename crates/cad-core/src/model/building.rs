use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{Floor, GridSystem};

/// 建物全体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Building {
    pub id: Uuid,
    pub name: String,
    /// 通り芯システム
    pub grid: GridSystem,
    /// 階の一覧（下から順）
    pub floors: Vec<Floor>,
    /// 建物全体のメタデータ
    pub metadata: BuildingMetadata,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BuildingMetadata {
    /// 建築面積 (sqm) — 自動計算可能
    pub building_area: Option<f64>,
    /// 延べ面積 (sqm) — 自動計算可能
    pub total_floor_area: Option<f64>,
    /// 用途
    pub usage: Option<String>,
    /// 構造種別
    pub structure_type: Option<String>,
}

impl Building {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::now_v7(),
            name: name.into(),
            grid: GridSystem::default(),
            floors: Vec::new(),
            metadata: BuildingMetadata::default(),
        }
    }

    /// 階を追加
    pub fn add_floor(&mut self, floor: Floor) {
        self.floors.push(floor);
    }

    /// 延べ面積を計算
    pub fn total_floor_area(&self) -> f64 {
        self.floors.iter().map(|f| f.area()).sum()
    }
}
