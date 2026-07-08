use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::Point2D;

/// 壁グラフのノード ID。壁は端点をこの ID で参照する。
pub type NodeId = Uuid;

/// 壁グラフの頂点（接合点）。座標が存在する唯一の場所。
///
/// 壁は端点を `NodeId` で参照する。2 枚の壁が同じ `Node` を共有する = 接合していて
/// **構造的に離れない**（接合点の座標がモデル上に1つしか無いため、ノードを動かせば
/// 両壁の端が必ず追従する）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: Uuid,
    /// 位置 (mm)
    pub point: Point2D,
}

impl Node {
    pub fn new(point: Point2D) -> Self {
        Self {
            id: Uuid::now_v7(),
            point,
        }
    }
}
