//! 2D pan/zoom camera. World = mm, y-up. Screen = physical px, y-down, origin top-left.

use crate::config;

#[derive(Clone, Copy)]
pub struct Camera {
    /// World point (mm) at the center of the viewport.
    pub center: [f64; 2],
    /// Zoom in px/mm.
    pub scale: f64,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            center: [0.0, 0.0],
            scale: 0.05,
        }
    }
}

impl Camera {
    /// `clip = pos * xy + zw` — the uniform the vertex shader applies. World mm → NDC.
    pub fn transform(&self, w: f32, h: f32) -> [f32; 4] {
        let s = self.scale as f32;
        let sx = 2.0 * s / w;
        let sy = 2.0 * s / h;
        [
            sx,
            sy,
            -sx * self.center[0] as f32,
            -sy * self.center[1] as f32,
        ]
    }

    /// Screen px (y-down) → world mm (y-up).
    pub fn screen_to_world(&self, sx: f64, sy: f64, w: f64, h: f64) -> [f64; 2] {
        [
            self.center[0] + (sx - w / 2.0) / self.scale,
            self.center[1] - (sy - h / 2.0) / self.scale,
        ]
    }

    /// World mm (y-up) → screen px (y-down).
    pub fn world_to_screen(&self, wx: f64, wy: f64, w: f64, h: f64) -> [f32; 2] {
        [
            ((wx - self.center[0]) * self.scale + w / 2.0) as f32,
            (h / 2.0 - (wy - self.center[1]) * self.scale) as f32,
        ]
    }

    /// Pan by a screen-space delta (px).
    pub fn pan(&mut self, dsx: f64, dsy: f64) {
        self.center[0] -= dsx / self.scale;
        self.center[1] += dsy / self.scale;
    }

    /// Zoom by `factor` while keeping the world point under the cursor fixed.
    pub fn zoom_at(&mut self, factor: f64, cursor: [f64; 2], w: f64, h: f64) {
        let world = self.screen_to_world(cursor[0], cursor[1], w, h);
        self.scale = (self.scale * factor).clamp(config::MIN_SCALE, config::MAX_SCALE);
        // Re-anchor: cursor must still map to `world` after the scale change.
        self.center[0] = world[0] - (cursor[0] - w / 2.0) / self.scale;
        self.center[1] = world[1] + (cursor[1] - h / 2.0) / self.scale;
    }

    /// Fit an axis-aligned world bbox `[min_x, min_y, max_x, max_y]` into the viewport.
    pub fn fit(&mut self, bbox: [f64; 4], w: f64, h: f64) {
        let (bw, bh) = (bbox[2] - bbox[0], bbox[3] - bbox[1]);
        self.center = [(bbox[0] + bbox[2]) / 2.0, (bbox[1] + bbox[3]) / 2.0];
        if bw <= 0.0 || bh <= 0.0 {
            return;
        }
        let margin = 0.9;
        let s = (w / bw).min(h / bh) * margin;
        self.scale = s.clamp(config::MIN_SCALE, config::MAX_SCALE);
    }
}
