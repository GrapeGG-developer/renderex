//! AST (абстрактное синтаксическое дерево) языка renderex.

use crate::color::Color;

/// Окно: разрешение и заголовок.
#[derive(Debug, Clone, PartialEq)]
pub struct Window {
    pub width: u32,
    pub height: u32,
    pub title: String,
}

/// Объект сцены. Координаты — в пикселях, начало координат — левый
/// верхний угол окна.
#[derive(Debug, Clone, PartialEq)]
pub enum Object {
    /// Квадрат: `square x y size color`
    Square { x: i64, y: i64, size: u64, color: Color },
    /// Прямоугольник: `rect x y w h color`
    Rect { x: i64, y: i64, w: u64, h: u64, color: Color },
    /// Круг: `circle x y radius color`
    Circle { x: i64, y: i64, r: u64, color: Color },
    /// Эллипс: `ellipse x y rx ry color`
    Ellipse { x: i64, y: i64, rx: u64, ry: u64, color: Color },
    /// Треугольник: `triangle x1 y1 x2 y2 x3 y3 color`
    Triangle {
        x1: i64, y1: i64,
        x2: i64, y2: i64,
        x3: i64, y3: i64,
        color: Color,
    },
    /// Отрезок: `line x1 y1 x2 y2 color [width]`
    Line {
        x1: i64, y1: i64,
        x2: i64, y2: i64,
        width: u64,
        color: Color,
    },
    /// Изображение из интернета или локального файла: `image x y w h "src"`
    Image { x: i64, y: i64, w: u64, h: u64, src: String },
}

/// Вся сцена: окно, фон и объекты (рисуются в порядке перечисления).
#[derive(Debug, Clone, PartialEq)]
pub struct Scene {
    pub window: Window,
    pub background: Color,
    pub objects: Vec<Object>,
}

impl Scene {
    pub fn object_count(&self) -> usize {
        self.objects.len()
    }
}
