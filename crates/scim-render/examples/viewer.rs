//! `cargo run -p scim-render --example viewer -- [path]`
//!
//! Open a window and render every actor in the given save as a 100×100 unit
//! gray quad at its world-space (x, y) translation. Default save:
//! `crates/scim-savefile/tests/corpus/CREATIVE TEST.sav`.
//!
//! Controls:
//!   - Right-mouse drag: pan
//!   - Mouse wheel: zoom (anchored at cursor)
//!   - Resize: handled automatically
//!   - Escape or close button: quit

use std::path::PathBuf;
use std::sync::Arc;

use scim_render::{Camera2d, Renderer};
use scim_store::{import::import_save, Db};
use scim_world::WorldIndex;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

fn default_save_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("scim-savefile")
        .join("tests")
        .join("corpus")
        .join("CREATIVE TEST.sav")
}

struct App {
    save_path: PathBuf,
    state: Option<AppState>,
}

struct AppState {
    _db_dir: tempfile::TempDir,
    _db: Db,
    world: WorldIndex,
    window: Arc<Window>,
    renderer: Renderer,
    camera: Camera2d,
    cursor: [f32; 2],
    right_dragging: bool,
    drag_anchor_screen: [f32; 2],
    drag_anchor_center: [f32; 2],
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }

        let window = event_loop
            .create_window(
                Window::default_attributes()
                    .with_title("TerraFICS viewer (P1.5-b)")
                    .with_inner_size(LogicalSize::new(1280.0, 800.0)),
            )
            .expect("create_window");
        let window = Arc::new(window);

        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("viewer.scimdb");
        let mut db = Db::open(&db_path).expect("open db");
        let summary = import_save(&mut db, &self.save_path, "viewer").expect("import_save");
        let world =
            WorldIndex::from_snapshot(db.conn(), summary.snapshot_id).expect("from_snapshot");
        eprintln!(
            "loaded {} ({} actors, {} indexed placements)",
            self.save_path.display(),
            summary.total_actors,
            world.len()
        );

        let renderer = pollster::block_on(Renderer::new(window.clone())).expect("Renderer::new");

        let size = window.inner_size();
        let camera =
            Camera2d::with_params([0.0, 0.0], 200.0, [size.width.max(1), size.height.max(1)]);

        eprintln!("uploading {} placements via WorldIndex", world.len());

        let mut state = AppState {
            _db_dir: dir,
            _db: db,
            world,
            window,
            renderer,
            camera,
            cursor: [0.0, 0.0],
            right_dragging: false,
            drag_anchor_screen: [0.0, 0.0],
            drag_anchor_center: [0.0, 0.0],
        };
        state.renderer.upload_world(&state.world);
        state.renderer.set_camera(&state.camera);
        state.window.request_redraw();
        self.state = Some(state);
    }

    #[allow(clippy::too_many_lines)] // straightforward event-loop match; refactor when handlers grow.
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(state) = self.state.as_mut() else {
            return;
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput { event: ke, .. } => {
                if ke.state == ElementState::Pressed
                    && matches!(ke.physical_key, PhysicalKey::Code(KeyCode::Escape))
                {
                    event_loop.exit();
                }
            }
            WindowEvent::Resized(size) => {
                state.renderer.resize(size.width, size.height);
                state.camera.resize([size.width.max(1), size.height.max(1)]);
                state.renderer.set_camera(&state.camera);
                state.window.request_redraw();
            }
            WindowEvent::CursorMoved { position, .. } => {
                // Cast screen coords through f64-as-f32 — pixel coords never exceed 16M.
                #[allow(clippy::cast_possible_truncation)]
                let x = position.x as f32;
                #[allow(clippy::cast_possible_truncation)]
                let y = position.y as f32;
                state.cursor = [x, y];
                if state.right_dragging {
                    let dx = state.cursor[0] - state.drag_anchor_screen[0];
                    let dy = state.cursor[1] - state.drag_anchor_screen[1];
                    let upp = state.camera.units_per_pixel();
                    state.camera = Camera2d::with_params(
                        [
                            dx.mul_add(-upp, state.drag_anchor_center[0]),
                            dy.mul_add(upp, state.drag_anchor_center[1]),
                        ],
                        upp,
                        state.camera.viewport(),
                    );
                    state.renderer.set_camera(&state.camera);
                    state.window.request_redraw();
                }
            }
            WindowEvent::MouseInput {
                state: bstate,
                button,
                ..
            } => {
                if button == MouseButton::Right {
                    match bstate {
                        ElementState::Pressed => {
                            state.right_dragging = true;
                            state.drag_anchor_screen = state.cursor;
                            state.drag_anchor_center = state.camera.center();
                        }
                        ElementState::Released => {
                            state.right_dragging = false;
                        }
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let steps = match delta {
                    MouseScrollDelta::LineDelta(_x, y) => y,
                    #[allow(clippy::cast_possible_truncation)]
                    // Pixel-delta y is bounded by display height — no risk of meaningful precision loss.
                    MouseScrollDelta::PixelDelta(pos) => (pos.y as f32) / 100.0,
                };
                let factor = (1.15_f32).powf(steps);
                state.camera.zoom_at(factor, state.cursor);
                state.renderer.set_camera(&state.camera);
                state.window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                if let Err(e) = state.renderer.render() {
                    eprintln!("render error: {e}");
                }
            }
            _ => {}
        }
    }
}

fn main() {
    let save_path = std::env::args()
        .nth(1)
        .map_or_else(default_save_path, PathBuf::from);
    let event_loop = EventLoop::new().expect("EventLoop::new");
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
    let mut app = App {
        save_path,
        state: None,
    };
    event_loop.run_app(&mut app).expect("run_app");
}
