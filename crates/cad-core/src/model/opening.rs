use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 開口部（ドア・窓）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Opening {
    pub id: Uuid,
    /// 所属する壁の ID
    pub wall_id: Uuid,
    /// 壁上の位置 — 壁の始点からの距離 (mm)
    pub position: f64,
    /// 開口幅 (mm)
    pub width: f64,
    /// 開口高 (mm)
    pub height: f64,
    /// 窓台高 / 床からの高さ (mm)
    pub sill_height: f64,
    /// 種別
    pub kind: OpeningKind,
    /// 型番・仕様
    pub spec: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OpeningKind {
    /// 片開きドア
    SingleDoor,
    /// 両開きドア
    DoubleDoor,
    /// 引き戸
    SlidingDoor,
    /// 引違い窓
    SlidingWindow,
    /// FIX 窓
    FixedWindow,
    /// 上げ下げ窓
    HungWindow,
    /// 開き窓
    CasementWindow,
    /// その他
    Other(String),
}

impl Opening {
    pub fn door(wall_id: Uuid, position: f64, width: f64, height: f64) -> Self {
        Self {
            id: Uuid::now_v7(),
            wall_id,
            position,
            width,
            height,
            sill_height: 0.0,
            kind: OpeningKind::SingleDoor,
            spec: None,
        }
    }

    pub fn window(wall_id: Uuid, position: f64, width: f64, height: f64, sill: f64) -> Self {
        Self {
            id: Uuid::now_v7(),
            wall_id,
            position,
            width,
            height,
            sill_height: sill,
            kind: OpeningKind::SlidingWindow,
            spec: None,
        }
    }
}
