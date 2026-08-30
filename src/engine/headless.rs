//! Отрисовка без окна: рендер сцены в PNG-файл (нужно для CI и для
//! встраивания renderex в другие языки).

use crate::diag::{Diag, Result};
use crate::engine::{load_images, render_scene};
use crate::Scene;
use std::path::Path;

/// Отрисовать сцену и сохранить результат в PNG.
pub fn render_to_png(scene: &Scene, out_path: &Path, base_dir: &Path) -> Result<()> {
    let images = load_images(scene, base_dir)
        .map_err(|(line, msg)| Diag::new(msg, line.max(1), 1, 1))?;
    let fb = render_scene(scene, &images);

    let mut bytes = Vec::with_capacity(fb.pixels.len() * 4);
    for p in &fb.pixels {
        bytes.extend_from_slice(&p.to_be_bytes());
    }
    image::RgbaImage::from_raw(fb.width, fb.height, bytes)
        .ok_or_else(|| Diag::new("не удалось собрать изображение", 1, 1, 1))?
        .save_with_format(out_path, image::ImageFormat::Png)
        .map_err(|e| Diag::new(format!("не удалось сохранить PNG '{}': {e}", out_path.display()), 1, 1, 1))
}
