use serde::{Deserialize, Serialize};

/// 通り芯システム
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GridSystem {
    /// X 方向の通り芯（南北方向、名前: A, B, C...）
    pub x_axes: Vec<GridAxis>,
    /// Y 方向の通り芯（東西方向、名前: 1, 2, 3... or D, E, F...）
    pub y_axes: Vec<GridAxis>,
}

/// 通り芯の1本
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridAxis {
    /// 名前（"A", "B", "1", "2" 等）
    pub name: String,
    /// 位置 (mm)
    pub position: f64,
}

impl GridAxis {
    pub fn new(name: impl Into<String>, position: f64) -> Self {
        Self {
            name: name.into(),
            position,
        }
    }
}

impl GridSystem {
    /// X 方向のスパン一覧 (mm)
    pub fn x_spans(&self) -> Vec<(String, f64)> {
        spans(&self.x_axes)
    }

    /// Y 方向のスパン一覧 (mm)
    pub fn y_spans(&self) -> Vec<(String, f64)> {
        spans(&self.y_axes)
    }
}

fn spans(axes: &[GridAxis]) -> Vec<(String, f64)> {
    if axes.len() < 2 {
        return Vec::new();
    }
    let mut sorted: Vec<_> = axes.iter().collect();
    sorted.sort_by(|a, b| a.position.partial_cmp(&b.position).unwrap());
    sorted
        .windows(2)
        .map(|w| {
            let name = format!("{}-{}", w[0].name, w[1].name);
            let span = (w[1].position - w[0].position).abs();
            (name, span)
        })
        .collect()
}
