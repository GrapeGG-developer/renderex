//! Диагностика ошибок с позицией (строка:колонка) и красивым выводом.

use std::fmt;

/// Ошибка компиляции/интерпретации с привязкой к месту в исходнике.
#[derive(Debug, Clone)]
pub struct Diag {
    pub message: String,
    /// Строка, с 1.
    pub line: usize,
    /// Колонка, с 1.
    pub col: usize,
    /// Длина проблемного фрагмента в символах (для подчёркивания).
    pub len: usize,
}

pub type Result<T> = std::result::Result<T, Diag>;

impl Diag {
    pub fn new(message: impl Into<String>, line: usize, col: usize, len: usize) -> Self {
        Self {
            message: message.into(),
            line,
            col,
            len,
        }
    }
}

impl fmt::Display for Diag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({}:{})", self.message, self.line, self.col)
    }
}

impl std::error::Error for Diag {}

/// Печатает ошибку в стиле rustc:
///
/// ```text
/// error: неизвестная команда 'circl'
///   --> demo.rx:2:1
///     |
///   2 | circl 10 10 5 red
///     | ^^^^^
/// ```
pub fn print_diag(path: &str, source: &str, diag: &Diag) {
    let line_text = source
        .lines()
        .nth(diag.line.saturating_sub(1))
        .unwrap_or("")
        .replace('\t', " ");
    eprintln!("error: {}", diag.message);
    eprintln!("  --> {}:{}:{}", path, diag.line, diag.col);
    eprintln!("    |");
    eprintln!(" {:>3} | {}", diag.line, line_text);
    let pad = " ".repeat(diag.col.saturating_sub(1).min(200));
    let tildes = "~".repeat(diag.len.max(1).min(200));
    eprintln!("    | {pad}{tildes}");
}
