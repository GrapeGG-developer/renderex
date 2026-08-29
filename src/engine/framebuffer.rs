//! Фреймбуфер: массив пикселей `0xRRGGBBAA` и программная отрисовка
//! примитивов с альфа-смешиванием. Отрисовка идёт «поверх» — объекты,
//! перечисленные позже, рисуются поверх более ранних.

use crate::color::Color;
use image::RgbaImage;

/// За пределами фреймбуфера пиксели отбрасываются, объект просто
/// обрезается краем окна.
pub struct Framebuffer {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u32>,
}

impl Framebuffer {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; width as usize * height as usize],
        }
    }

    pub fn clear(&mut self, c: Color) {
        self.pixels.fill(c.pack());
    }

    #[inline]
    pub fn in_bounds(&self, x: i64, y: i64) -> bool {
        x >= 0 && y >= 0 && (x as u32) < self.width && (y as u32) < self.height
    }

    /// Рисует пиксель с альфа-смешиванием поверх уже нарисованного.
    #[inline]
    pub fn blend(&mut self, x: i64, y: i64, c: Color) {
        if !self.in_bounds(x, y) {
            return;
        }
        let i = y as usize * self.width as usize + x as usize;
        let a = c.a as u32;
        if a == 0 {
            return;
        }
        let dst = Color::unpack(self.pixels[i]);
        if a == 255 || dst.a == 0 {
            self.pixels[i] = c.pack();
            return;
        }
        let sa = a;
        let da = dst.a as u32;
        let out_a = sa + da * (255 - sa) / 255;
        if out_a == 0 {
            self.pixels[i] = 0;
            return;
        }
        let mix = |s: u8, d: u8| -> u8 {
            ((s as u32 * sa + d as u32 * da * (255 - sa) / 255) / out_a) as u8
        };
        self.pixels[i] = Color::rgba(
            mix(c.r, dst.r),
            mix(c.g, dst.g),
            mix(c.b, dst.b),
            out_a as u8,
        )
        .pack();
    }

    /// Залитый прямоугольник.
    pub fn fill_rect(&mut self, x: i64, y: i64, w: i64, h: i64, c: Color) {
        for py in y..y + h {
            for px in x..x + w {
                self.blend(px, py, c);
            }
        }
    }

    /// Залитый круг (центр + радиус).
    pub fn fill_circle(&mut self, cx: i64, cy: i64, r: i64, c: Color) {
        let r2 = r * r;
        for py in cy - r..=cy + r {
            for px in cx - r..=cx + r {
                let dx = px - cx;
                let dy = py - cy;
                if dx * dx + dy * dy <= r2 {
                    self.blend(px, py, c);
                }
            }
        }
    }

    /// Залитый эллипс.
    pub fn fill_ellipse(&mut self, cx: i64, cy: i64, rx: i64, ry: i64, c: Color) {
        if rx <= 0 || ry <= 0 {
            return;
        }
        let (rx2, ry2) = (rx * rx, ry * ry);
        for py in cy - ry..=cy + ry {
            for px in cx - rx..=cx + rx {
                let dx = px - cx;
                let dy = py - cy;
                if dx * dx * ry2 + dy * dy * rx2 <= rx2 * ry2 {
                    self.blend(px, py, c);
                }
            }
        }
    }

    /// Залитый треугольник по трём вершинам.
    pub fn fill_triangle(&mut self, a: (i64, i64), b: (i64, i64), c: (i64, i64), color: Color) {
        let min_x = a.0.min(b.0).min(c.0);
        let max_x = a.0.max(b.0).max(c.0);
        let min_y = a.1.min(b.1).min(c.1);
        let max_y = a.1.max(b.1).max(c.1);
        let edge = |p: (i64, i64), q: (i64, i64), r: (i64, i64)| -> i64 {
            (q.0 - p.0) * (r.1 - p.1) - (q.1 - p.1) * (r.0 - p.0)
        };
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let p = (x, y);
                let e1 = edge(a, b, p);
                let e2 = edge(b, c, p);
                let e3 = edge(c, a, p);
                let all_pos = e1 >= 0 && e2 >= 0 && e3 >= 0;
                let all_neg = e1 <= 0 && e2 <= 0 && e3 <= 0;
                if all_pos || all_neg {
                    self.blend(x, y, color);
                }
            }
        }
    }

    /// Толстый отрезок: рисуется как «капсула» из кругов вдоль линии
    /// (для толщины 1 — классический алгоритм Брезенхэма).
    pub fn draw_line(&mut self, x1: i64, y1: i64, x2: i64, y2: i64, c: Color, width: i64) {
        if width <= 1 {
            let (mut x, mut y) = (x1, y1);
            let dx = (x2 - x1).abs();
            let dy = -(y2 - y1).abs();
            let sx = if x1 < x2 { 1 } else { -1 };
            let sy = if y1 < y2 { 1 } else { -1 };
            let mut err = dx + dy;
            loop {
                self.blend(x, y, c);
                if x == x2 && y == y2 {
                    break;
                }
                let e2 = 2 * err;
                if e2 >= dy {
                    err += dy;
                    x += sx;
                }
                if e2 <= dx {
                    err += dx;
                    y += sy;
                }
            }
            return;
        }

        // Параметризация с шагом ~0.5px, диски заданного радиуса.
        let steps = ((x2 - x1).abs().max((y2 - y1).abs()) * 2).max(1) as usize;
        for s in 0..=steps {
            let t = s as f64 / steps as f64;
            let px = x1 as f64 + (x2 - x1) as f64 * t;
            let py = y1 as f64 + (y2 - y1) as f64 * t;
            self.fill_circle(px.round() as i64, py.round() as i64, width / 2, c);
        }
    }

    /// Копирует изображение с масштабированием (nearest-neighbor)
    /// и альфа-смешиванием.
    pub fn blit_scaled(&mut self, x: i64, y: i64, w: u64, h: u64, img: &RgbaImage) {
        if w == 0 || h == 0 {
            return;
        }
        let iw = img.width() as f64;
        let ih = img.height() as f64;
        for py in 0..h as i64 {
            for px in 0..w as i64 {
                let sx = ((px as f64 / w as f64) * iw) as u32;
                let sy = ((py as f64 / h as f64) * ih) as u32;
                let sx = sx.min(img.width() - 1);
                let sy = sy.min(img.height() - 1);
                let p = img.get_pixel(sx, sy);
                self.blend(x + px, y + py, Color::rgba(p[0], p[1], p[2], p[3]));
            }
        }
    }
}
