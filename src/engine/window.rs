//! Нативное окно (winit) + программный фреймбуфер (softbuffer).

use crate::engine::{load_images, render_scene};
use crate::Scene;
use softbuffer::{Context, Surface};
use std::num::NonZeroU32;
use std::path::Path;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

type WindowRef = Arc<Window>;

/// Показать сцену в нативном окне. Работает и на Windows, и на macOS/Linux.
pub fn show_window(scene: &Scene, base_dir: &Path) -> Result<(), String> {
    let images = load_images(scene, base_dir)?;
    let framebuffer = render_scene(scene, &images);

    let event_loop = EventLoop::new().map_err(|e| format!("не удалось создать event loop: {e}"))?;
    let mut app = App {
        scene_pixels: framebuffer.pixels,
        width: scene.window.width,
        height: scene.window.height,
        title: scene.window.title.clone(),
        surface: None,
        window: None,
    };

    event_loop
        .run_app(&mut app)
        .map_err(|e| format!("ошибка цикла окна: {e}"))
}

struct App {
    scene_pixels: Vec<u32>,
    width: u32,
    height: u32,
    title: String,
    surface: Option<Surface<WindowRef, WindowRef>>,
    window: Option<WindowRef>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let attrs = Window::default_attributes()
                .with_title(self.title.clone())
                .with_inner_size(LogicalSize::new(self.width as f64, self.height as f64))
                .with_resizable(false);
            let window = Arc::new(
                event_loop
                    .create_window(attrs)
                    .expect("не удалось создать окно"),
            );
            let ctx =
                Context::new(window.clone()).expect("не удалось создать графический контекст");
            let mut surface = Surface::new(&ctx, window.clone())
                .expect("не удалось создать поверхность окна");
            surface
                .resize(
                    NonZeroU32::new(self.width).unwrap(),
                    NonZeroU32::new(self.height).unwrap(),
                )
                .expect("не удалось задать размер поверхности");
            self.surface = Some(surface);
            self.window = Some(window);
        }
        self.redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => self.redraw(),
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        logical_key: Key::Named(NamedKey::Escape),
                        ..
                    },
                ..
            } => event_loop.exit(),
            _ => {}
        }
    }
}

impl App {
    fn redraw(&mut self) {
        if let (Some(surface), Some(window)) = (&mut self.surface, &self.window) {
            if let Ok(mut buf) = surface.buffer_mut() {
                buf.copy_from_slice(&self.scene_pixels);
                if buf.present().is_ok() {
                    window.request_redraw();
                }
            }
        }
    }
}
