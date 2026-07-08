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

    /// 通り芯の外接矩形 `[min_x, min_y, max_x, max_y]` (mm)。
    ///
    /// x_axes は垂直線(x=position)、y_axes は水平線(y=position)なので、
    /// 矩形は各軸の実 min/max。両方向に軸が無ければ `None`。
    /// (原点(0,0)を無条件に含めてしまう誤りを避けるための正しい定義)
    pub fn bounds(&self) -> Option<[f64; 4]> {
        let xs = minmax(self.x_axes.iter().map(|a| a.position))?;
        let ys = minmax(self.y_axes.iter().map(|a| a.position))?;
        Some([xs.0, ys.0, xs.1, ys.1])
    }
}

fn minmax(iter: impl Iterator<Item = f64>) -> Option<(f64, f64)> {
    iter.fold(None, |acc, v| match acc {
        None => Some((v, v)),
        Some((lo, hi)) => Some((lo.min(v), hi.max(v))),
    })
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
