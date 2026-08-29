//! Лексер: превращает исходник renderex в поток токенов.
//!
//! - команды разделяются переводом строки или `;`
//! - комментарии — `//` до конца строки
//! - числа — целые, могут быть отрицательными
//! - строки — в двойных кавычках, экранирование `\" \\ \n \t`
//! - HEX-цвета — `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`

use crate::diag::{Diag, Result};

#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
    /// Слово: имя команды или именованный цвет.
    Ident(String),
    /// Целое число (может быть отрицательным).
    Number(i64),
    /// Строковый литерал (значение без кавычек).
    Str(String),
    /// HEX-цвет, включая `#` и проверенные цифры.
    Hex(String),
    /// Разделитель команд (перевод строки или `;`).
    Newline,
    Eof,
}

#[derive(Clone, Debug)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub col: usize,
    pub len: usize,
}

impl Token {
    /// Человекочитаемое описание токена для сообщений об ошибках.
    pub fn describe(&self) -> String {
        match &self.kind {
            TokenKind::Ident(s) | TokenKind::Hex(s) => s.clone(),
            TokenKind::Number(n) => n.to_string(),
            TokenKind::Str(s) => format!("\"{s}\""),
            TokenKind::Newline => "перевод строки".to_string(),
            TokenKind::Eof => "конец файла".to_string(),
        }
    }
}

pub fn lex(src: &str) -> Result<Vec<Token>> {
    let chars: Vec<char> = src.chars().collect();
    let len = chars.len();
    let mut out: Vec<Token> = Vec::new();
    let mut i = 0usize;
    let mut line = 1usize;
    let mut col = 1usize;
    // Не плодим подряд идущие разделители.
    let mut last_was_sep = true;

    while i < len {
        let ch = chars[i];

        // Пробелы.
        if ch == ' ' || ch == '\t' || ch == '\r' {
            i += 1;
            col += 1;
            continue;
        }

        // Комментарий до конца строки.
        if ch == '/' && i + 1 < len && chars[i + 1] == '/' {
            while i < len && chars[i] != '\n' {
                i += 1;
                col += 1;
            }
            continue;
        }

        // Разделители команд.
        if ch == '\n' || ch == ';' {
            if !last_was_sep {
                out.push(Token { kind: TokenKind::Newline, line, col, len: 1 });
            }
            last_was_sep = true;
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
            i += 1;
            continue;
        }

        // Числа (в т.ч. отрицательные). После числа допускается склейка
        // вида `800x600` — лексер пропускает `x<цифры>` без ошибки,
        // парсер разберёт её как пару «ширина x высота».
        if ch.is_ascii_digit() || (ch == '-' && i + 1 < len && chars[i + 1].is_ascii_digit()) {
            let start_col = col;
            let mut num: i64 = 0;
            let mut neg = false;
            if ch == '-' {
                neg = true;
                i += 1;
                col += 1;
            }
            while i < len && chars[i].is_ascii_digit() {
                num = num
                    .saturating_mul(10)
                    .saturating_add((chars[i] as i64) - ('0' as i64));
                i += 1;
                col += 1;
            }
            if neg {
                num = -num;
            }
            let ok_x = chars.get(i) == Some(&'x')
                && chars.get(i + 1).is_some_and(|c| c.is_ascii_digit());
            if i < len && chars[i].is_ascii_alphanumeric() && !ok_x {
                return Err(Diag::new(
                    format!("неожиданный символ '{}' после числа", chars[i]),
                    line,
                    col,
                    1,
                ));
            }
            out.push(Token { kind: TokenKind::Number(num), line, col: start_col, len: col - start_col });
            last_was_sep = false;
            continue;
        }

        // Строки.
        if ch == '"' {
            let start_col = col;
            i += 1;
            col += 1;
            let mut s = String::new();
            loop {
                if i >= len {
                    return Err(Diag::new(
                        "незакрытая строка: ожидался символ '\"'",
                        line,
                        start_col,
                        1,
                    ));
                }
                let c = chars[i];
                match c {
                    '"' => {
                        i += 1;
                        col += 1;
                        break;
                    }
                    '\n' => {
                        return Err(Diag::new(
                            "незакрытая строка: перенос строки внутри строки",
                            line,
                            start_col,
                            1,
                        ));
                    }
                    '\\' => {
                        i += 1;
                        col += 1;
                        if i >= len {
                            return Err(Diag::new(
                                "незакрытая строка: ожидался символ '\"'",
                                line,
                                start_col,
                                1,
                            ));
                        }
                        s.push(match chars[i] {
                            'n' => '\n',
                            't' => '\t',
                            '"' => '"',
                            '\\' => '\\',
                            other => other,
                        });
                        i += 1;
                        col += 1;
                    }
                    other => {
                        s.push(other);
                        i += 1;
                        col += 1;
                    }
                }
            }
            out.push(Token { kind: TokenKind::Str(s), line, col: start_col, len: col - start_col });
            last_was_sep = false;
            continue;
        }

        // HEX-цвета.
        if ch == '#' {
            let start_col = col;
            i += 1;
            col += 1;
            let mut hex = String::new();
            while i < len && chars[i].is_ascii_hexdigit() {
                hex.push(chars[i]);
                i += 1;
                col += 1;
            }
            if !matches!(hex.len(), 3 | 4 | 6 | 8) {
                return Err(Diag::new(
                    format!("HEX-цвет должен содержать 3, 4, 6 или 8 цифр, найдено {}", hex.len()),
                    line,
                    start_col,
                    col - start_col,
                ));
            }
            if i < len && chars[i].is_ascii_alphanumeric() {
                return Err(Diag::new(
                    format!("неверный HEX-цвет: неожиданный символ '{}'", chars[i]),
                    line,
                    col,
                    1,
                ));
            }
            out.push(Token { kind: TokenKind::Hex(format!("#{hex}")), line, col: start_col, len: col - start_col });
            last_was_sep = false;
            continue;
        }

        // Идентификаторы.
        if ch.is_ascii_alphabetic() || ch == '_' {
            let start_col = col;
            let mut s = String::new();
            while i < len && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                s.push(chars[i]);
                i += 1;
                col += 1;
            }
            out.push(Token { kind: TokenKind::Ident(s), line, col: start_col, len: col - start_col });
            last_was_sep = false;
            continue;
        }

        return Err(Diag::new(
            format!("неожиданный символ '{ch}'"),
            line,
            col,
            1,
        ));
    }

    out.push(Token { kind: TokenKind::Eof, line, col, len: 1 });
    Ok(out)
}
