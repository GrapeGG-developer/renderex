//! Парсер: поток токенов → [Scene].
//!
//! Синтаксис команды: `имя арг1 арг2 ...`, команды разделяются переводом
//! строки или `;`. Команда `window` обязательна и может встречаться один
//! раз; `background` можно переопределять (последнее значение выигрывает).

use crate::ast::{Object, Scene, Window};
use crate::color::{named, parse_hex, Color};
use crate::diag::{Diag, Result};
use crate::lexer::{lex, Token, TokenKind};

/// Максимальный размер стороны окна (защита от случайных гигантских значений).
pub const MAX_WINDOW_SIDE: i64 = 16_384;

pub fn parse(src: &str) -> Result<Scene> {
    Parser::new(lex(src)?).parse_scene()
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn err_at(&self, t: &Token, message: impl Into<String>) -> Diag {
        Diag::new(message, t.line, t.col, t.len.max(1))
    }

    fn expect_number(&mut self, what: &str) -> Result<i64> {
        let t = self.peek().clone();
        match t.kind {
            TokenKind::Number(n) => {
                self.pos += 1;
                Ok(n)
            }
            _ => Err(self.err_at(&t, format!("ожидалось число ({what}), получено '{}'", t.describe()))),
        }
    }

    /// Число, а также склейка `800x600` (используется после первого числа).
    fn expect_dim(&mut self) -> Result<i64> {
        if let TokenKind::Ident(s) = self.peek().kind.clone() {
            if let Some(rest) = s.strip_prefix('x') {
                if !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()) {
                    self.pos += 1;
                    return Ok(rest.parse().unwrap_or(0));
                }
            }
        }
        self.expect_number("размер")
    }

    fn expect_string(&mut self) -> Result<String> {
        let t = self.peek().clone();
        match t.kind {
            TokenKind::Str(s) => {
                self.pos += 1;
                Ok(s)
            }
            _ => Err(self.err_at(&t, format!("ожидалась строка в кавычках, получено '{}'", t.describe()))),
        }
    }

    fn expect_color(&mut self) -> Result<Color> {
        let t = self.peek().clone();
        match &t.kind {
            TokenKind::Hex(h) => {
                self.pos += 1;
                parse_hex(h).ok_or_else(|| self.err_at(&t, format!("неверный HEX-цвет '{h}'")))
            }
            TokenKind::Ident(name) => match named(name) {
                Some(c) => {
                    self.pos += 1;
                    Ok(c)
                }
                None => Err(self.err_at(&t, format!("неизвестный цвет '{name}'"))),
            },
            _ => Err(self.err_at(&t, format!("ожидался цвет (например #ff0000 или red), получено '{}'", t.describe()))),
        }
    }

    /// Неотрицательное число для размеров фигур.
    fn expect_size(&mut self, what: &str, stmt: &Token) -> Result<u64> {
        let n = self.expect_number(what)?;
        if n < 0 {
            return Err(self.err_at(stmt, format!("{what} не может быть отрицательным ({n})")));
        }
        Ok(n as u64)
    }

    fn eat_separators(&mut self) {
        while matches!(self.peek().kind, TokenKind::Newline) {
            self.pos += 1;
        }
    }

    /// После команды допустимы только разделитель или конец файла.
    fn end_of_statement(&mut self, kw: &str) -> Result<()> {
        match self.peek().kind {
            TokenKind::Newline => {
                self.eat_separators();
                Ok(())
            }
            TokenKind::Eof => Ok(()),
            _ => {
                let t = self.peek().clone();
                Err(self.err_at(&t, format!("лишние аргументы после '{kw}': '{}'", t.describe())))
            }
        }
    }

    fn parse_scene(&mut self) -> Result<Scene> {
        let mut window: Option<(u32, u32, String)> = None;
        let mut window_token: Option<Token> = None;
        let mut background = Color::rgb(0, 0, 0);
        let mut objects: Vec<Object> = Vec::new();

        self.eat_separators();
        while !matches!(self.peek().kind, TokenKind::Eof) {
            let t = self.peek().clone();
            let kw = match &t.kind {
                TokenKind::Ident(s) => s.clone(),
                _ => {
                    return Err(self.err_at(&t, format!("ожидалась команда, получено '{}'", t.describe())));
                }
            };
            self.pos += 1;

            match kw.as_str() {
                "window" | "win" => {
                    if window.is_some() {
                        return Err(self.err_at(&t, "окно уже задано: команда window может быть только одна"));
                    }
                    let w = self.expect_number("ширина")?;
                    let h = self.expect_dim()?;
                    if w <= 0 || h <= 0 {
                        return Err(self.err_at(&t, format!("размеры окна должны быть положительными ({w}x{h})")));
                    }
                    if w > MAX_WINDOW_SIDE || h > MAX_WINDOW_SIDE {
                        return Err(self.err_at(&t, format!("окно слишком большое: максимум {MAX_WINDOW_SIDE}x{MAX_WINDOW_SIDE}")));
                    }
                    let title = if matches!(self.peek().kind, TokenKind::Str(_)) {
                        self.expect_string()?
                    } else {
                        "Renderex".to_string()
                    };
                    window = Some((w as u32, h as u32, title));
                    window_token = Some(t);
                }
                "background" | "bg" => {
                    background = self.expect_color()?;
                }
                "square" | "sq" => {
                    let x = self.expect_number("x")?;
                    let y = self.expect_number("y")?;
                    let size = self.expect_size("размер", &t)?;
                    let color = self.expect_color()?;
                    objects.push(Object::Square { x, y, size, color });
                }
                "rect" => {
                    let x = self.expect_number("x")?;
                    let y = self.expect_number("y")?;
                    let w = self.expect_size("ширина", &t)?;
                    let h = self.expect_dim_sized(&t)?;
                    let color = self.expect_color()?;
                    objects.push(Object::Rect { x, y, w, h, color });
                }
                "circle" | "circ" => {
                    let x = self.expect_number("x")?;
                    let y = self.expect_number("y")?;
                    let r = self.expect_size("радиус", &t)?;
                    let color = self.expect_color()?;
                    objects.push(Object::Circle { x, y, r, color });
                }
                "ellipse" => {
                    let x = self.expect_number("x")?;
                    let y = self.expect_number("y")?;
                    let rx = self.expect_size("радиус по x", &t)?;
                    let ry = self.expect_dim_sized(&t)?;
                    let color = self.expect_color()?;
                    objects.push(Object::Ellipse { x, y, rx, ry, color });
                }
                "triangle" | "tri" => {
                    let x1 = self.expect_number("x1")?;
                    let y1 = self.expect_number("y1")?;
                    let x2 = self.expect_number("x2")?;
                    let y2 = self.expect_number("y2")?;
                    let x3 = self.expect_number("x3")?;
                    let y3 = self.expect_number("y3")?;
                    let color = self.expect_color()?;
                    objects.push(Object::Triangle { x1, y1, x2, y2, x3, y3, color });
                }
                "line" => {
                    let x1 = self.expect_number("x1")?;
                    let y1 = self.expect_number("y1")?;
                    let x2 = self.expect_number("x2")?;
                    let y2 = self.expect_number("y2")?;
                    let color = self.expect_color()?;
                    let width = if matches!(self.peek().kind, TokenKind::Number(_)) {
                        self.expect_size("толщина", &t)?
                    } else {
                        1
                    };
                    objects.push(Object::Line { x1, y1, x2, y2, width, color });
                }
                "image" | "img" => {
                    let x = self.expect_number("x")?;
                    let y = self.expect_number("y")?;
                    let w = self.expect_size("ширина", &t)?;
                    let h = self.expect_dim_sized(&t)?;
                    let src = self.expect_string()?;
                    objects.push(Object::Image { x, y, w, h, src });
                }
                other => {
                    return Err(self.err_at(&t, format!("неизвестная команда '{other}'")));
                }
            }
            self.end_of_statement(&kw)?;
        }

        let (w, h, title) = window.ok_or_else(|| {
            let t = window_token
                .clone()
                .or_else(|| Some(self.peek().clone()))
                .unwrap();
            Diag::new(
                "не задано окно: добавьте команду 'window <ширина> <высота>'",
                t.line,
                t.col,
                t.len.max(1),
            )
        })?;

        Ok(Scene {
            window: Window { width: w, height: h, title },
            background,
            objects,
        })
    }
}

impl Parser {
    /// Второе измерение: число или склейка `x600`.
    fn expect_dim_sized(&mut self, stmt: &Token) -> Result<u64> {
        let n = self.expect_dim()?;
        if n < 0 {
            return Err(self.err_at(stmt, format!("размер не может быть отрицательным ({n})")));
        }
        Ok(n as u64)
    }
}
