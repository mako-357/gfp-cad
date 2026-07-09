//! Application state, winit event handling, and per-frame orchestration (M1).

use std::sync::Arc;

use glyphon::{Buffer as TextBuffer, Color as TextColor, TextArea, TextBounds, Weight};
use lyon::tessellation::VertexBuffers;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

use cad_core::{NodeId, Point2D};

use crate::camera::Camera;
use crate::document::{Document, Selection};
use crate::gpu::{Gpu, Renderer, Text};
use crate::scene::hit;
use crate::tools::{Preview, Tool, ToolState};
use crate::{config, io, scene};

/// Screen-space movement (px) above which a left-drag counts as pan, not a click.
const CLICK_SLOP: f64 = 3.0;

struct Label {
    buffer: TextBuffer,
    world: [f64; 2],
    color: TextColor,
}

struct State {
    window: Arc<Window>,
    gpu: Gpu,
    renderer: Renderer,
    text: Text,
    camera: Camera,
    document: Document,
    labels: Vec<Label>,
    inspector: Option<TextBuffer>,
    hud: Option<TextBuffer>,
    tool: Tool,
    tool_state: ToolState,
    /// Transient one-line hint shown in the HUD (e.g. a missed AddRoom click).
    hint: Option<String>,
    /// In-app onboarding guide overlay (shown on first run; toggle with H / ?).
    show_help: bool,
    help_title: Option<TextBuffer>,
    help_body: Option<TextBuffer>,
    help_hint: Option<TextBuffer>,
    /// DPI scale (physical px per logical px). Text sizes/positions are physical
    /// px, so font sizes and screen-space chrome must be multiplied by this.
    ui_scale: f32,
    last_revision: u64,
    cursor: [f64; 2],
    press_cursor: [f64; 2],
    dragging: bool,
    moved: bool,
    /// The node currently being dragged (Select mode grab), if any.
    drag_node: Option<NodeId>,
    /// Whether the current node drag actually moved the node (→ one undo step).
    drag_moved: bool,
    ctrl: bool,
    /// Set when something changed; drives on-demand redraw (idle = no repaint).
    needs_redraw: bool,
    /// The first `Resized` carries the true surface size; refit once when it arrives.
    awaiting_first_resize: bool,
}

impl State {
    fn new(window: Arc<Window>) -> Self {
        let gpu = Gpu::new(window.clone());
        let renderer = Renderer::new(&gpu.device, gpu.config.format);
        let text = Text::new(&gpu.device, &gpu.queue, gpu.config.format);
        let document = Document::new(io::sample::building());
        let ui_scale = window.scale_factor() as f32;

        let mut state = Self {
            window,
            gpu,
            renderer,
            text,
            camera: Camera::default(),
            document,
            labels: Vec::new(),
            inspector: None,
            hud: None,
            tool: Tool::Select,
            tool_state: ToolState::default(),
            hint: None,
            show_help: true, // 初回は手厚くガイドを出す
            help_title: None,
            help_body: None,
            help_hint: None,
            ui_scale,
            last_revision: 0,
            cursor: [0.0, 0.0],
            press_cursor: [0.0, 0.0],
            dragging: false,
            moved: false,
            drag_node: None,
            drag_moved: false,
            ctrl: false,
            needs_redraw: true,
            awaiting_first_resize: true,
        };
        state.rebuild_model();
        state.refresh_hud();
        state.build_help();
        state.fit();
        state
    }

    /// Build the in-app guide text buffers (rebuilt on DPI change so font px match).
    fn build_help(&mut self) {
        const BODY: &str = "\
【ツール】数字キーで切替
  0 選択    クリックで選択 / ノードをドラッグで移動
  1 壁      2点クリックで壁（連続で描ける）
  2 部屋    囲まれた領域を1クリック（面積を自動計算）
  3 開口    壁の近くをクリックでドア

【マウス】
  ドラッグ … 画面移動（パン）
  ホイール … ズーム（カーソル位置を固定）
  ノードのハンドルをドラッグ … 壁・部屋ごと移動

【編集・ファイル】
  Ctrl+Z / Ctrl+Y … 元に戻す / やり直し
  Ctrl+S 保存   O 開く   L サンプル   F 全体表示
  Enter 確定   Esc 取消・選択解除

【ポイント】
  壁は共有ノードで接合し、離れません。部屋は壁の
  閉領域から導出されるので、壁を動かすと追従します。";
        self.help_title = Some(self.label("gfp-cad — 使い方ガイド", 21.0, 700));
        self.help_body = Some(self.label(BODY, 15.0, 400));
        self.help_hint = Some(self.label("H でこのガイドを開閉  ・  Esc で閉じる", 13.0, 500));
    }

    fn viewport(&self) -> (f64, f64) {
        (self.gpu.config.width as f64, self.gpu.config.height as f64)
    }

    /// Build a text buffer at a DPI-corrected physical font size.
    fn label(&mut self, text: &str, px: f32, weight: u16) -> TextBuffer {
        let s = self.ui_scale;
        self.text.make_label(text, px * s, Weight(weight))
    }

    /// Rebuild model geometry + labels from the current building. Does NOT touch
    /// selection/overlay — callers decide (fresh load clears it; edits keep it).
    fn rebuild_model_keep_selection(&mut self) {
        let geo = scene::geometry::build(&self.document.building, self.document.active_floor);
        self.renderer
            .upload_model(&self.gpu.device, &self.gpu.queue, &geo);

        self.labels.clear();
        let specs = scene::labels::build(&self.document.building, self.document.active_floor);
        for spec in specs {
            let buffer = self.label(&spec.text, spec.size_px, spec.weight);
            self.labels.push(Label {
                buffer,
                world: spec.world,
                color: TextColor::rgb(spec.color.0, spec.color.1, spec.color.2),
            });
        }
        self.last_revision = self.document.revision;
    }

    /// Full reset for a fresh document (load/sample): rebuild + drop selection.
    fn rebuild_model(&mut self) {
        self.rebuild_model_keep_selection();
        self.document.selection = Selection::None;
        self.inspector = None;
        self.tool_state.cancel();
        self.renderer
            .upload_overlay(&self.gpu.device, &self.gpu.queue, &VertexBuffers::new());
    }

    /// Top-of-screen status line: active tool, dirty flag, undo/redo availability.
    fn refresh_hud(&mut self) {
        let dirty = if self.document.dirty { " *" } else { "" };
        let undo = if self.document.can_undo() {
            "元"
        } else {
            "・"
        };
        let redo = if self.document.can_redo() {
            "戻"
        } else {
            "・"
        };
        let hint = match &self.hint {
            Some(h) => format!("   ⚠ {h}"),
            None => String::new(),
        };
        let text = format!(
            "[{}]{}  {undo}{redo}   1:壁 2:部屋 3:開口 0:選択  ↵確定 Esc取消 Ctrl+Z/Y Ctrl+S保存  H:ヘルプ{hint}",
            self.tool.label(),
            dirty,
        );
        self.hud = Some(self.label(&text, 14.0, 500));
    }

    /// Set the selection and rebuild its highlight overlay + inspector text.
    fn select(&mut self, sel: Selection) {
        self.document.selection = sel;
        self.rebuild_overlay();
        self.refresh_inspector();
    }

    /// Rebuild the inspector text buffer from the current selection (also picks up
    /// a new DPI scale, since `label()` bakes `ui_scale` into the font size).
    fn refresh_inspector(&mut self) {
        let sel = self.document.selection;
        let text = self
            .document
            .active_floor()
            .and_then(|floor| inspector_text(floor, sel));
        self.inspector = text.map(|t| self.label(&t, 15.0, 400));
    }

    /// Rebuild the overlay buffer: current selection highlight + live tool preview.
    fn rebuild_overlay(&mut self) {
        let mut geo = match self.document.active_floor() {
            Some(floor) => scene::geometry::build_overlay(floor, self.document.selection),
            None => VertexBuffers::new(),
        };
        // Node handles (grabbable graph vertices) in Select mode, screen-constant size.
        if self.tool == Tool::Select
            && let Some(floor) = self.document.active_floor()
        {
            let hw = config::NODE_HANDLE_PX / self.camera.scale;
            scene::geometry::push_node_handles(&mut geo, floor, hw, self.document.selection);
        }
        // Tool preview follows the (snapped) cursor.
        let hw = config::GRID_HALF_W.max(2.0 / self.camera.scale);
        match self.tool_state.preview(self.tool, self.snapped_cursor()) {
            Preview::None => {}
            Preview::Segment(a, b) => {
                scene::geometry::push_polyline(&mut geo, &[a, b], hw, config::PREVIEW);
            }
        }
        self.renderer
            .upload_overlay(&self.gpu.device, &self.gpu.queue, &geo);
    }

    /// World-mm cursor, grid-snapped (used by tools and previews).
    fn snapped_cursor(&self) -> [f64; 2] {
        let (w, h) = self.viewport();
        let world = self
            .camera
            .screen_to_world(self.cursor[0], self.cursor[1], w, h);
        let tol = config::SNAP_PX / self.camera.scale;
        hit::snap(
            &self.document.building,
            self.document.active_floor(),
            world,
            tol,
        )
    }

    fn set_tool(&mut self, tool: Tool) {
        self.tool_state.cancel();
        self.finish_node_drag(); // finalize any in-progress drag as its own undo step
        self.tool = tool;
        self.refresh_hud();
        self.rebuild_overlay();
    }

    /// Finalize an in-progress node drag as one undo step (commit if it moved).
    /// Safe to call when no drag is active. Used on release / tool-switch /
    /// focus-loss / before history ops so the state machine never leaves a
    /// half-applied drag with no undo entry.
    fn finish_node_drag(&mut self) {
        if self.drag_node.take().is_some() {
            let moved = self.drag_moved;
            self.document.commit_transaction(moved);
            self.drag_moved = false;
            if moved {
                self.after_edit();
            }
        }
    }

    /// Abort an in-progress node drag, rolling the model back to the pre-drag state
    /// (used by Escape). No undo entry — the drag is undone in place.
    fn cancel_node_drag(&mut self) {
        if self.drag_node.take().is_some() {
            self.document.rollback_transaction();
            self.drag_moved = false;
            self.after_edit();
        }
    }

    /// World-mm cursor without snapping (used for selection hit-testing).
    fn raw_cursor(&self) -> [f64; 2] {
        let (w, h) = self.viewport();
        self.camera
            .screen_to_world(self.cursor[0], self.cursor[1], w, h)
    }

    /// Left-click at the current cursor: either edit (tool active) or select.
    fn on_click(&mut self) {
        if self.tool == Tool::Select {
            // Selection uses the raw cursor — snapping is for drawing only, and
            // would otherwise pull picks onto grid lines / wall endpoints.
            let world = self.raw_cursor();
            let tol = 5.0 / self.camera.scale;
            let sel = self
                .document
                .active_floor()
                .map(|floor| hit::pick(floor, world, tol))
                .unwrap_or(Selection::None);
            self.select(sel);
        } else {
            let p = self.snapped_cursor();
            match self.tool_state.on_click(self.tool, &mut self.document, p) {
                Some(sel) => {
                    self.document.selection = sel;
                    self.hint = None;
                }
                None if self.tool == Tool::AddRoom => {
                    self.hint = Some("囲まれた領域内をクリックしてください".into());
                }
                None => {}
            }
            self.after_edit();
        }
    }

    fn commit_tool(&mut self) {
        if let Some(sel) = self.tool_state.on_commit(self.tool, &mut self.document) {
            self.document.selection = sel;
        }
        self.after_edit();
    }

    /// On press in Select mode: if a node is under the cursor, grab it for dragging
    /// (open a coalesced transaction so the whole drag is one undo step).
    fn try_grab_node(&mut self) {
        if self.tool != Tool::Select || self.show_help {
            return;
        }
        // Safety: never coalesce a new grab into a stale transaction.
        self.finish_node_drag();
        let world = self.raw_cursor();
        // Grab radius tracks the visible handle size (+ a small margin) so a press
        // outside the handle falls through to normal wall/room selection.
        let tol = config::NODE_HANDLE_PX * 2.5 / self.camera.scale;
        if let Some(nid) = self
            .document
            .active_floor()
            .and_then(|f| hit::pick_node(f, world, tol))
        {
            self.drag_node = Some(nid);
            self.drag_moved = false;
            self.document.begin_transaction();
            self.select(Selection::Node(nid));
            self.needs_redraw = true;
        }
    }

    /// While dragging: move the grabbed node to the grid-snapped cursor; the walls
    /// referencing it and the derived rooms follow automatically.
    fn drag_selected_node(&mut self) {
        let Some(nid) = self.drag_node else { return };
        let (w, h) = self.viewport();
        let world = self
            .camera
            .screen_to_world(self.cursor[0], self.cursor[1], w, h);
        let tol = config::SNAP_PX / self.camera.scale;
        let p = hit::snap_to_grid(&self.document.building, world, tol);
        let floor_idx = self.document.active_floor;
        self.document.mutate(|b| {
            if let Some(f) = b.floors.get_mut(floor_idx) {
                f.move_node(nid, Point2D::new(p[0], p[1]));
            }
        });
        self.drag_moved = true;
        self.rebuild_model_keep_selection();
        self.rebuild_overlay(); // node handles/highlight must follow the drag (M2)
        self.refresh_inspector();
        self.needs_redraw = true;
    }

    /// After any model mutation: refresh labels/model if revision changed, then overlay + hud.
    fn after_edit(&mut self) {
        if self.document.revision != self.last_revision {
            self.rebuild_model_keep_selection();
        }
        self.refresh_inspector();
        self.rebuild_overlay();
        self.refresh_hud();
    }

    fn fit(&mut self) {
        let (w, h) = self.viewport();
        self.camera.fit(self.document.bounds(), w, h);
    }

    fn load_sample(&mut self) {
        self.document = Document::new(io::sample::building());
        self.rebuild_model();
        self.fit();
    }

    fn open_dialog(&mut self) {
        let picked = rfd::FileDialog::new()
            .add_filter("Building JSON", &["json"])
            .pick_file();
        if let Some(path) = picked {
            match io::json::load(&path) {
                Ok(building) => {
                    let mut doc = Document::new(building);
                    doc.path = Some(path);
                    self.document = doc;
                    self.rebuild_model();
                    self.refresh_hud();
                    self.fit();
                }
                Err(e) => log::error!("failed to load: {e}"),
            }
        }
    }

    fn save(&mut self) {
        let path = self.document.path.clone().or_else(|| {
            rfd::FileDialog::new()
                .add_filter("Building JSON", &["json"])
                .set_file_name("building.json")
                .save_file()
        });
        let Some(path) = path else { return };
        match io::json::save(&path, &self.document.building) {
            Ok(()) => {
                self.document.path = Some(path);
                self.document.dirty = false;
                self.refresh_hud();
                log::info!("saved");
            }
            Err(e) => log::error!("save failed: {e}"),
        }
    }

    fn undo(&mut self) {
        self.finish_node_drag(); // finalize any in-progress drag before touching history
        if self.document.undo() {
            self.reconcile_after_history();
        }
    }

    fn redo(&mut self) {
        self.finish_node_drag();
        if self.document.redo() {
            self.reconcile_after_history();
        }
    }

    /// After undo/redo the building changed wholesale: rebuild and drop any now-stale selection.
    /// (Any in-progress drag was already finalized by `undo`/`redo` before this runs.)
    fn reconcile_after_history(&mut self) {
        self.rebuild_model_keep_selection();
        self.document.selection = Selection::None;
        self.inspector = None;
        self.tool_state.cancel();
        self.rebuild_overlay();
        self.refresh_hud();
    }

    fn render(&mut self) {
        // Safety fallback; normal mutation paths already keep last_revision in sync.
        if self.document.revision != self.last_revision {
            self.rebuild_model_keep_selection();
        }

        let (w, h) = self.viewport();
        let (pw, ph) = (self.gpu.config.width, self.gpu.config.height);
        self.renderer
            .set_camera(&self.gpu.queue, self.camera.transform(w as f32, h as f32));

        let camera = self.camera;
        let sc = self.ui_scale;

        // Screen-space UI layer (in-app guide). The panel rect is reused for its text.
        let (pwf, phf) = (pw as f32, ph as f32);
        let pad = 40.0 * sc;
        let panel_w = (640.0 * sc).min(pwf - 2.0 * pad);
        let panel_h = (470.0 * sc).min(phf - 2.0 * pad);
        let (px0, py0) = ((pwf - panel_w) / 2.0, (phf - panel_h) / 2.0);
        let mut ui: VertexBuffers<crate::gpu::Vertex, u32> = VertexBuffers::new();
        if self.show_help {
            scene::geometry::push_rect(&mut ui, [0.0, 0.0, pwf, phf], 0x0D_0F_14_99); // dim
            let panel = [px0, py0, px0 + panel_w, py0 + panel_h];
            scene::geometry::push_rect(&mut ui, panel, 0x20_25_31_FA); // panel body
            scene::geometry::push_rect(
                &mut ui,
                [px0, py0, px0 + panel_w, py0 + 6.0 * sc],
                0x66_E0_FF_FF,
            ); // accent
        }
        self.renderer
            .set_screen(&self.gpu.queue, [2.0 / pwf, -2.0 / phf, -1.0, 1.0]);
        self.renderer
            .upload_ui(&self.gpu.device, &self.gpu.queue, &ui);

        // Project label world positions to screen and build text areas.
        // Screen-space chrome offsets are physical px, so scale by DPI.
        let bounds = TextBounds {
            left: 0,
            top: 0,
            right: pw as i32,
            bottom: ph as i32,
        };
        let mut areas: Vec<TextArea> = self
            .labels
            .iter()
            .map(|l| {
                let s = camera.world_to_screen(l.world[0], l.world[1], w, h);
                TextArea {
                    buffer: &l.buffer,
                    left: s[0] - 8.0 * sc,
                    top: s[1] - 10.0 * sc,
                    scale: 1.0,
                    bounds,
                    default_color: l.color,
                    custom_glyphs: &[],
                }
            })
            .collect();
        // HUD status line (top-left).
        if let Some(buffer) = self.hud.as_ref() {
            areas.push(TextArea {
                buffer,
                left: 16.0 * sc,
                top: 12.0 * sc,
                scale: 1.0,
                bounds,
                default_color: TextColor::rgb(150, 200, 235),
                custom_glyphs: &[],
            });
        }
        // Inspector readout (below the HUD).
        if let Some(buffer) = self.inspector.as_ref() {
            areas.push(TextArea {
                buffer,
                left: 16.0 * sc,
                top: 40.0 * sc,
                scale: 1.0,
                bounds,
                default_color: TextColor::rgb(238, 240, 245),
                custom_glyphs: &[],
            });
        }
        // In-app guide text, positioned inside the panel drawn above.
        if self.show_help {
            let tx = px0 + 28.0 * sc;
            for (buf, top, color) in [
                (
                    &self.help_title,
                    py0 + 22.0 * sc,
                    TextColor::rgb(150, 224, 255),
                ),
                (
                    &self.help_body,
                    py0 + 66.0 * sc,
                    TextColor::rgb(223, 227, 235),
                ),
                (
                    &self.help_hint,
                    py0 + panel_h - 30.0 * sc,
                    TextColor::rgb(140, 146, 158),
                ),
            ] {
                if let Some(buffer) = buf.as_ref() {
                    areas.push(TextArea {
                        buffer,
                        left: tx,
                        top,
                        scale: 1.0,
                        bounds,
                        default_color: color,
                        custom_glyphs: &[],
                    });
                }
            }
        }
        self.text
            .prepare(&self.gpu.device, &self.gpu.queue, pw, ph, areas);

        let frame = match self.gpu.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t)
            | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            _ => {
                self.gpu
                    .surface
                    .configure(&self.gpu.device, &self.gpu.config);
                return;
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(config::CLEAR),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            self.renderer.draw(&mut pass);
            self.text.render(&mut pass);
        }
        self.gpu.queue.submit(Some(encoder.finish()));
        frame.present();
        self.needs_redraw = false;
    }
}

/// Read-only property readout for the current selection.
fn inspector_text(floor: &cad_core::Floor, sel: Selection) -> Option<String> {
    match sel {
        Selection::None => None,
        Selection::Wall(id) => {
            let w = floor.walls.iter().find(|w| w.id == id)?;
            Some(format!(
                "壁\n長さ: {:.0} mm\n厚み: {:.0} mm\n材料: {:?}\n{}",
                floor.wall_length(w).unwrap_or(0.0),
                w.thickness,
                w.material,
                if w.is_exterior { "外壁" } else { "内壁" },
            ))
        }
        Selection::Opening(id) => {
            let o = floor.openings.iter().find(|o| o.id == id)?;
            Some(format!(
                "開口\n種類: {:?}\n幅: {:.0} mm\n高さ: {:.0} mm\n窓台: {:.0} mm",
                o.kind, o.width, o.height, o.sill_height,
            ))
        }
        Selection::Room(id) => {
            let r = floor.rooms.iter().find(|r| r.id == id)?;
            // 面を1回だけ解決して面積・周長を両方求める（faces() の重複計算を避ける）。
            let (area, perim) = match floor.face_at(r.seed) {
                Some(f) => {
                    let n = f.polygon.len();
                    let perim: f64 = (0..n)
                        .map(|i| f.polygon[i].distance_to(&f.polygon[(i + 1) % n]))
                        .sum();
                    (f.area / 1_000_000.0, perim)
                }
                None => (0.0, 0.0),
            };
            Some(format!(
                "部屋\n名前: {}\n面積: {area:.2} ㎡\n周長: {perim:.0} mm",
                r.name,
            ))
        }
        Selection::Node(id) => {
            let n = floor.node(id)?;
            Some(format!(
                "ノード\n座標: ({:.0}, {:.0}) mm\n接続壁: {} 本\nドラッグで移動",
                n.point.x,
                n.point.y,
                floor.walls_at_node(id),
            ))
        }
    }
}

#[derive(Default)]
pub struct App {
    state: Option<State>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("cad-app — gfp-cad")
            .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 800.0));
        let window = Arc::new(event_loop.create_window(attrs).expect("create window"));
        self.state = Some(State::new(window));
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(state) = self.state.as_mut() else {
            return;
        };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                state.gpu.resize(size.width, size.height);
                // The first real surface size may differ from the requested one;
                // refit once so the plan is framed correctly on startup.
                if state.awaiting_first_resize {
                    state.awaiting_first_resize = false;
                    state.fit();
                }
                state.needs_redraw = true;
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                // Re-shape every text buffer at the new DPI: model labels, HUD,
                // the inspector readout (the N1 regression), and the guide.
                state.ui_scale = scale_factor as f32;
                state.rebuild_model_keep_selection();
                state.refresh_inspector();
                state.refresh_hud();
                state.build_help();
                state.needs_redraw = true;
            }
            WindowEvent::Focused(false) => {
                // Losing focus can swallow the mouse-up; finalize any drag so the
                // node can't keep following the cursor after the button is released.
                state.dragging = false;
                state.finish_node_drag();
                state.needs_redraw = true;
            }
            WindowEvent::RedrawRequested => state.render(),
            WindowEvent::MouseInput {
                state: btn_state,
                button: MouseButton::Left,
                ..
            } => match btn_state {
                ElementState::Pressed => {
                    state.dragging = true;
                    state.moved = false;
                    state.press_cursor = state.cursor;
                    state.try_grab_node();
                }
                ElementState::Released => {
                    state.dragging = false;
                    if state.drag_node.is_some() {
                        state.finish_node_drag();
                    } else if !state.moved {
                        state.on_click();
                    }
                    state.needs_redraw = true;
                }
            },
            WindowEvent::CursorMoved { position, .. } => {
                let new = [position.x, position.y];
                if state.drag_node.is_some() {
                    // Dragging a node: move it (grid-snapped); walls + rooms follow.
                    state.cursor = new;
                    state.drag_selected_node();
                } else if state.dragging {
                    state
                        .camera
                        .pan(new[0] - state.cursor[0], new[1] - state.cursor[1]);
                    let d = (new[0] - state.press_cursor[0]).hypot(new[1] - state.press_cursor[1]);
                    if d > CLICK_SLOP {
                        state.moved = true;
                    }
                    state.cursor = new;
                    state.needs_redraw = true;
                } else {
                    state.cursor = new;
                }
                // Live preview follows the cursor while a tool is mid-action.
                if state.tool != Tool::Select {
                    state.rebuild_overlay();
                    state.needs_redraw = true;
                }
            }
            WindowEvent::ModifiersChanged(mods) => {
                state.ctrl = mods.state().control_key();
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let dy = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y as f64,
                    MouseScrollDelta::PixelDelta(p) => p.y / 120.0,
                };
                let (w, h) = state.viewport();
                let factor = 1.1_f64.powf(dy);
                state.camera.zoom_at(factor, state.cursor, w, h);
                state.rebuild_overlay(); // node handles are screen-constant
                state.needs_redraw = true;
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                match event.logical_key.as_ref() {
                    // Ctrl combos first.
                    Key::Character("z" | "Z") if state.ctrl => state.undo(),
                    Key::Character("y" | "Y") if state.ctrl => state.redo(),
                    Key::Character("s" | "S") if state.ctrl => state.save(),
                    // Tool switches.
                    Key::Character("1") => state.set_tool(Tool::DrawWall),
                    Key::Character("2") => state.set_tool(Tool::AddRoom),
                    Key::Character("3") => state.set_tool(Tool::AddOpening),
                    Key::Character("0") => state.set_tool(Tool::Select),
                    // View / IO.
                    Key::Character("f" | "F") => state.fit(),
                    Key::Character("l" | "L") => state.load_sample(),
                    Key::Character("o" | "O") => state.open_dialog(),
                    Key::Character("h" | "H" | "?") => {
                        state.show_help = !state.show_help;
                    }
                    Key::Named(NamedKey::Enter) => state.commit_tool(),
                    Key::Named(NamedKey::Escape) => {
                        if state.show_help {
                            state.show_help = false;
                        } else if state.drag_node.is_some() {
                            state.cancel_node_drag(); // roll the drag back
                        } else if state.tool_state.is_active() {
                            state.tool_state.cancel();
                            state.rebuild_overlay();
                        } else {
                            state.select(Selection::None);
                        }
                    }
                    _ => {}
                }
                state.needs_redraw = true;
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // On-demand rendering: only repaint when state actually changed.
        if let Some(state) = self.state.as_ref()
            && state.needs_redraw
        {
            state.window.request_redraw();
        }
    }
}
