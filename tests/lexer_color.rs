//! Тесты цвета и лексера.

use renderex::color::{named, parse_hex, Color};
use renderex::diag::{Diag, Result};
use renderex::lexer::{lex, TokenKind};

fn err_of(r: Result<Vec<renderex::lexer::Token>>) -> Option<Diag> {
    r.err()
}

#[test]
fn hex_forms() {
    assert_eq!(parse_hex("#f00"), Some(Color::rgb(255, 0, 0)));
    assert_eq!(parse_hex("#ff0000"), Some(Color::rgb(255, 0, 0)));
    assert_eq!(parse_hex("#0f08"), Some(Color::rgba(0, 255, 0, 136)));
    assert_eq!(parse_hex("#10203040"), Some(Color::rgba(0x10, 0x20, 0x30, 0x40)));
    assert_eq!(parse_hex("#12345"), None);
    assert_eq!(parse_hex("ff0000"), None);
}

#[test]
fn named_colors_case_insensitive() {
    assert_eq!(named("Red"), Some(Color::rgb(255, 0, 0)));
    assert_eq!(named("TEAL"), Some(Color::rgb(0, 128, 128)));
    assert_eq!(named("notacolor"), None);
}

#[test]
fn lex_basic_statement() {
    let toks = lex("circle 10 20 5 #ff0000\n").unwrap();
    let kinds: Vec<TokenKind> = toks.into_iter().map(|t| t.kind).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::Ident("circle".into()),
            TokenKind::Number(10),
            TokenKind::Number(20),
            TokenKind::Number(5),
            TokenKind::Hex("#ff0000".into()),
            TokenKind::Newline,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lex_string_escape_and_comment() {
    let toks = lex(r#"window 800 600 "a\"b\\c" // comment
"#)
    .unwrap();
    let strs: Vec<String> = toks
        .iter()
        .filter_map(|t| match &t.kind {
            TokenKind::Str(s) => Some(s.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(strs, vec!["a\"b\\c"]);
}

#[test]
fn lex_negative_number() {
    let toks = lex("line 0 0 -5 -5 red").unwrap();
    assert!(toks.iter().any(|t| t.kind == TokenKind::Number(-5)));
}

#[test]
fn lex_glued_dims() {
    let toks = lex("window 800x600").unwrap();
    let kinds: Vec<TokenKind> = toks.into_iter().map(|t| t.kind).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::Ident("window".into()),
            TokenKind::Number(800),
            TokenKind::Ident("x600".into()),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lex_bad_hex_len() {
    let err = err_of(lex("#12345")).unwrap();
    assert!(err.message.contains("HEX-цвет"), "got: {}", err.message);
    assert_eq!(err.col, 1);
}

#[test]
fn lex_unterminated_string() {
    let err = err_of(lex("image 0 0 10 10 \"abc")).unwrap();
    assert!(err.message.contains("незакрытая строка"));
}

#[test]
fn lex_unexpected_char() {
    let err = err_of(lex("circle @ 1 2 red")).unwrap();
    assert!(err.message.contains("неожиданный символ '@'"));
    assert_eq!(err.col, 8);
}
