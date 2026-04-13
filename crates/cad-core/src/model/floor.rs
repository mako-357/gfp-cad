use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{Opening, Room, Wall};

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
    /// 壁
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
            walls: Vec::new(),
            openings: Vec::new(),
            rooms: Vec::new(),
        }
    }

    /// 床面積の合計 (sqm)
    pub fn area(&self) -> f64 {
        self.rooms.iter().map(|r| r.area()).sum()
    }
}
