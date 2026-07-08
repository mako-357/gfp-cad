//! glyphon text layer. Loads bundled Gen Interface JP; falls back to system fonts.

use glyphon::{
    Attrs, Buffer as TextBuffer, Cache, Family, FontSystem, Metrics, Resolution, Shaping,
    SwashCache, TextArea, TextAtlas, TextRenderer, Viewport, Weight,
};

/// Font family bundled in `assets/fonts/`. Falls back to system fonts if absent.
pub const FAMILY: &str = "Gen Interface JP";

pub struct Text {
    pub font_system: FontSystem,
    swash_cache: SwashCache,
    atlas: TextAtlas,
    viewport: Viewport,
    renderer: TextRenderer,
}

impl Text {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let mut font_system = FontSystem::new();
        load_bundled_fonts(&mut font_system);

        let swash_cache = SwashCache::new();
        let cache = Cache::new(device);
        let viewport = Viewport::new(device, &cache);
        let mut atlas = TextAtlas::new(device, queue, &cache, format);
        let renderer =
            TextRenderer::new(&mut atlas, device, wgpu::MultisampleState::default(), None);

        Self {
            font_system,
            swash_cache,
            atlas,
            viewport,
            renderer,
        }
    }

    /// Build a shaped, unbounded (non-wrapping) text buffer for a label at a given weight.
    pub fn make_label(&mut self, text: &str, px: f32, weight: Weight) -> TextBuffer {
        let mut buffer = TextBuffer::new(&mut self.font_system, Metrics::new(px, px * 1.25));
        buffer.set_size(&mut self.font_system, None, None);
        buffer.set_text(
            &mut self.font_system,
            text,
            &Attrs::new().family(Family::Name(FAMILY)).weight(weight),
            Shaping::Advanced,
            None,
        );
        buffer.shape_until_scroll(&mut self.font_system, false);
        buffer
    }

    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        areas: Vec<TextArea>,
    ) {
        self.viewport.update(queue, Resolution { width, height });
        // A prepare failure (e.g. atlas exhaustion) must not crash the viewer;
        // skip this frame's text instead.
        if let Err(e) = self.renderer.prepare(
            device,
            queue,
            &mut self.font_system,
            &mut self.atlas,
            &self.viewport,
            areas,
            &mut self.swash_cache,
        ) {
            log::error!("glyphon prepare failed: {e}");
        }
    }

    pub fn render(&self, pass: &mut wgpu::RenderPass<'_>) {
        if let Err(e) = self.renderer.render(&self.atlas, &self.viewport, pass) {
            log::error!("glyphon render failed: {e}");
        }
    }
}

fn load_bundled_fonts(font_system: &mut FontSystem) {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/fonts");
    let Ok(entries) = std::fs::read_dir(dir) else {
        log::warn!("font dir not found: {dir}; using system fonts");
        return;
    };
    let db = font_system.db_mut();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "ttf" || e == "otf")
            && let Ok(bytes) = std::fs::read(&path)
        {
            db.load_font_data(bytes);
            log::debug!("loaded font {}", path.display());
        }
    }

    // Report the faces registered under our family so weight selection is verifiable.
    let mut faces: Vec<(String, u16)> = font_system
        .db()
        .faces()
        .filter(|f| f.families.iter().any(|(n, _)| n == FAMILY))
        .map(|f| (f.families[0].0.clone(), f.weight.0))
        .collect();
    faces.sort_by_key(|(_, w)| *w);
    log::info!("'{FAMILY}' faces: {faces:?}");
}
