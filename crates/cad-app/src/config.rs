//! Visual constants and defaults (colors are packed 0xRRGGBBAA).

pub const CLEAR: wgpu::Color = wgpu::Color {
    r: 0.09,
    g: 0.10,
    b: 0.12,
    a: 1.0,
};

/// Selection highlight (translucent amber).
pub const SEL: u32 = 0xFF_D2_4A_88;
/// Tool preview (in-progress wall/room), bright cyan.
pub const PREVIEW: u32 = 0x66_E0_FF_CC;
/// Node handle (grabbable graph vertex, Select mode), muted steel.
pub const NODE: u32 = 0x8A_93_A2_DD;
/// Selected/grabbed node handle, bright amber (opaque).
pub const NODE_SEL: u32 = 0xFF_C8_3A_FF;

/// Node handle half-size in screen px (converted to world mm via camera scale).
pub const NODE_HANDLE_PX: f64 = 4.0;

/// Grid-snap radius in screen px (converted to world mm via camera scale).
pub const SNAP_PX: f64 = 12.0;

pub const GRID: u32 = 0x3A_3F_4A_FF;
pub const WALL_INT: u32 = 0x9A_A0_AA_FF;
pub const WALL_EXT: u32 = 0xE2_E6_EC_FF;
pub const OPENING: u32 = 0x4F_A3_E3_FF;
pub const ROOM_FILL: u32 = 0x2E_6F_B0_40;

/// Label text colors (r, g, b).
pub const LABEL_ROOM: (u8, u8, u8) = (225, 228, 234);
pub const LABEL_GRID: (u8, u8, u8) = (140, 146, 158);

/// World-space half-widths (mm) for thin chrome. M1 keeps these world-fixed;
/// M2+ switches to screen-constant widths regenerated on zoom.
pub const GRID_HALF_W: f64 = 8.0;

/// Zoom limits (px/mm).
pub const MIN_SCALE: f64 = 0.002;
pub const MAX_SCALE: f64 = 2.0;
