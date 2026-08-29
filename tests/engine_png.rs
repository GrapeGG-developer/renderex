//! Тесты движка: рендер в PNG (без окна).

use renderex::engine::headless::render_to_png;
use renderex::color::Color;
use std::path::Path;

const TMP: &str = "/tmp/renderex_tests";

fn render(src: &str, name: &str) -> (u32, u32, Vec<u8>) {
    let scene = renderex::compile(src).expect("сцена должна компилироваться");
    std::fs::create_dir_all(TMP).unwrap();
    let out = Path::new(TMP).join(name);
    render_to_png(&scene, &out, Path::new("/tmp/renderex_tests")).expect("PNG должен сохраниться");
    let img = image::open(&out).unwrap().to_rgba8();
    (img.width(), img.height(), img.into_raw())
}

fn px(raw: &[u8], w: u32, x: u32, y: u32) -> Color {
    let i = ((y * w + x) * 4) as usize;
    Color::rgba(raw[i], raw[i + 1], raw[i + 2], raw[i + 3])
}

#[test]
fn renders_circle_over_background() {
    let (w, h, raw) = render(
        "window 100 100\nbackground #000000\ncircle 50 50 10 #ff0000",
        "circle.png",
    );
    assert_eq!((w, h), (100, 100));
    assert_eq!(px(&raw, w, 0, 0), Color::rgb(0, 0, 0));
    assert_eq!(px(&raw, w, 50, 50), Color::rgb(255, 0, 0));
    assert_eq!(px(&raw, w, 50, 40), Color::rgb(255, 0, 0)); // внутри круга
    assert_eq!(px(&raw, w, 50, 39), Color::rgb(0, 0, 0)); // вне круга
}

#[test]
fn later_objects_draw_on_top() {
    let (w, h, raw) = render(
        "window 100 100\nbackground black\nsquare 0 0 100 red\ncircle 50 50 20 blue",
        "layers.png",
    );
    assert_eq!(px(&raw, w, 50, 50), Color::rgb(0, 0, 255));
    assert_eq!(px(&raw, w, 5, 5), Color::rgb(255, 0, 0));
}

#[test]
fn alpha_rect_blends() {
    // Полупрозрачный белый поверх чёрного → серый.
    let (w, h, raw) = render(
        "window 10 10\nbackground black\nrect 0 0 10 10 #ffffff44",
        "alpha.png",
    );
    let c = px(&raw, w, 5, 5);
    // 0x44 = 68/255 ≈ 0.267; поверх непрозрачного фона результат непрозрачен.
    assert_eq!(c.a, 255);
    assert!(c.r > 50 && c.r < 90, "ожидался серый ~68, получено {}", c.r);
}

#[test]
fn image_from_internet_renders() {
    let (w, h, raw) = render(
        "window 40 40\nbackground white\nimage 0 0 40 40 \"https://picsum.photos/seed/renderex_test/40/40\"",
        "net.png",
    );
    assert_eq!((w, h), (40, 40));
    // Скачанное фото точно не останется белым фоном.
    let mut non_white = 0;
    for y in 0..h {
        for x in 0..w {
            let c = px(&raw, w, x, y);
            if c.r != 255 || c.g != 255 || c.b != 255 {
                non_white += 1;
            }
        }
    }
    assert!(non_white > 100, "изображение не отрисовалось ({} пикселей)", non_white);
}
