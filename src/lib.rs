//! # Renderex
//!
//! Renderex — маленький декларативный язык для рендеринга окон.
//! Сцена описывается простыми командами (окно, фон, фигуры, изображения),
//! а движок отрисовывает её в нативном окне или в PNG-файл.
//!
//! ```text
//! // demo.rx
//! window 800 600 "Привет, renderex!"
//! background #101020
//! circle 400 300 120 #3498db
//! image  100 100 320 180 "https://example.com/pic.png"
//! ```
//!
//! Конвейер: исходник → [lexer] → токены → [parser] → [ast::Scene] →
//! [engine] → пиксели → окно (winit) или PNG.
//!
//! Ядро языка отделено от рендера, поэтому renderex легко встроить
//! в другой язык программирования как модуль: достаточно вызвать
//! [`compile`] и передать полученную сцену в [`engine`].

pub mod ast;
pub mod color;
pub mod diag;
pub mod engine;
pub mod fetch;
pub mod lexer;
pub mod parser;

pub use ast::Scene;
pub use parser::parse;

/// Скомпилировать исходный код renderex в сцену (лексер + парсер).
/// Возвращает подробную диагностику с позицией ошибки.
pub fn compile(source: &str) -> diag::Result<Scene> {
    parse(source)
}
