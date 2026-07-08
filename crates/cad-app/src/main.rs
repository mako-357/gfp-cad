//! cad-app — gfp-cad interactive CAD editor.
//!
//! M1: render-only viewer. Loads a Building (built-in sample or JSON), draws the
//! plan (grid / walls / openings / rooms + labels) on a pan/zoom canvas.
//!
//! Controls: drag = pan, wheel = zoom, F = fit, L = load sample, O = open JSON.

mod app;
mod camera;
mod config;
mod document;
mod gpu;
mod io;
mod scene;
mod tools;

use winit::event_loop::EventLoop;

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new().expect("event loop");
    let mut app = app::App::default();
    event_loop.run_app(&mut app).expect("run app");
}
