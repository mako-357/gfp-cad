// Camera: clip.xy = pos * transform.xy + transform.zw (world mm -> NDC).
struct Camera { transform: vec4<f32> };
@group(0) @binding(0) var<uniform> camera: Camera;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs(@location(0) pos: vec2<f32>, @location(1) color: u32) -> VsOut {
    var out: VsOut;
    let clip = pos * camera.transform.xy + camera.transform.zw;
    out.pos = vec4<f32>(clip, 0.0, 1.0);
    let r = f32((color >> 24u) & 255u) / 255.0;
    let g = f32((color >> 16u) & 255u) / 255.0;
    let b = f32((color >> 8u) & 255u) / 255.0;
    let a = f32(color & 255u) / 255.0;
    out.color = vec4<f32>(r, g, b, a);
    return out;
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    return in.color;
}
