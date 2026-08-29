//! Движок рендеринга: загрузка изображений, программная отрисовка сцены
//! во фреймбуфер, вывод в PNG и в нативное окно (winit).

pub mod framebuffer;
pub mod headless;
pub mod window;

use crate::ast::{Object, Scene};
use crate::color::Color;
use framebuffer::Framebuffer;
use image::RgbaImage;
use std::collections::HashMap;
use std::path::Path;

/// Загруженные изображения по индексу объекта в сцене.
pub type ImageMap = HashMap<usize, RgbaImage>;

/// Скачивает/читает все изображения сцены.
/// Локальные пути (без `://`) считаются относительными к `base_dir`.
pub fn load_images(scene: &Scene, base_dir: &Path) -> Result<ImageMap, String> {
    let mut map = HashMap::new();
    for (i, obj) in scene.objects.iter().enumerate() {
        if let Object::Image { src, .. } = obj {
            map.insert(i, load_image(src, base_dir)?);
        }
    }
    Ok(map)
}

/// Загружает одно изображение: по http/https ссылке или из локального файла.
pub fn load_image(src: &str, base_dir: &Path) -> Result<RgbaImage, String> {
    let bytes: Vec<u8> = if src.contains("://") {
        if !(src.starts_with("http://") || src.starts_with("https://")) {
            return Err(format!("поддерживаются только ссылки http/https, получено: {src}"));
        }
        use std::io::Read;
        let agent = ureq::AgentBuilder::new()
            .timeout(std::time::Duration::from_secs(30))
            .build();
        let resp = agent
            .get(src)
            .call()
            .map_err(|e| format!("не удалось скачать изображение '{src}': {e}"))?;
        let mut buf = Vec::new();
        resp.into_reader()
            .read_to_end(&mut buf)
            .map_err(|e| format!("ошибка чтения '{src}': {e}"))?;
        buf
    } else {
        let path = if Path::new(src).is_absolute() {
            Path::new(src).to_path_buf()
        } else {
            base_dir.join(src)
        };
        std::fs::read(&path).map_err(|e| format!("не удалось прочитать файл '{}': {e}", path.display()))?
    };
    image::load_from_memory(&bytes)
        .map_err(|e| format!("не удалось декодировать изображение '{src}': {e}"))
        .map(|img| img.to_rgba8())
}

/// Отрисовывает сцену во фреймбуфер (пиксели `0xRRGGBBAA`).
pub fn render_scene(scene: &Scene, images: &ImageMap) -> Framebuffer {
    let mut fb = Framebuffer::new(scene.window.width, scene.window.height);
    fb.clear(scene.background);
    for (i, obj) in scene.objects.iter().enumerate() {
        match obj {
            Object::Square { x, y, size, color } => {
                fb.fill_rect(*x, *y, *size as i64, *size as i64, *color)
            }
            Object::Rect { x, y, w, h, color } => {
                fb.fill_rect(*x, *y, *w as i64, *h as i64, *color)
            }
            Object::Circle { x, y, r, color } => fb.fill_circle(*x, *y, *r as i64, *color),
            Object::Ellipse { x, y, rx, ry, color } => {
                fb.fill_ellipse(*x, *y, *rx as i64, *ry as i64, *color)
            }
            Object::Triangle { x1, y1, x2, y2, x3, y3, color } => {
                fb.fill_triangle((*x1, *y1), (*x2, *y2), (*x3, *y3), *color)
            }
            Object::Line { x1, y1, x2, y2, width, color } => {
                fb.draw_line(*x1, *y1, *x2, *y2, *color, *width as i64)
            }
            Object::Image { x, y, w, h, .. } => {
                if let Some(img) = images.get(&i) {
                    fb.blit_scaled(*x, *y, *w, *h, img);
                }
            }
        }
    }
    fb
}

/// Подсветка синтаксиса для будущего `--dump` (заглушка, API стабилен).
pub fn _scene_stats(scene: &Scene) -> (u32, u32, usize, Color) {
    (
        scene.window.width,
        scene.window.height,
        scene.objects.len(),
        scene.background,
    )
}
