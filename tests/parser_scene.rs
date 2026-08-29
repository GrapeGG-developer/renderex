//! Интеграционные тесты парсера и AST.

use renderex::ast::{Object, Scene};
use renderex::color::Color;
use renderex::{compile, parse};

#[test]
fn minimal_scene() {
    let s = parse("window 400 300\nbackground white\ncircle 200 150 80 #ff8800").unwrap();
    assert_eq!(s.window.width, 400);
    assert_eq!(s.window.height, 300);
    assert_eq!(s.window.title, "Renderex"); // заголовок по умолчанию
    assert_eq!(s.background, Color::rgb(255, 255, 255));
    assert_eq!(
        s.objects,
        vec![Object::Circle {
            x: 200,
            y: 150,
            r: 80,
            color: Color::rgb(0xff, 0x88, 0x00)
        }]
    );
}

#[test]
fn all_commands_parse() {
    let src = r#"
window 800 600 "Моё окно"
background #112233
square 10 10 20 red
rect 0 0 50 25 #00ff00
circle 100 100 5 blue
ellipse 50 50 30 10 yellow
triangle 0 0 10 0 5 10 cyan
line 0 0 100 100 white 3
line 0 0 50 50 gray
image 20 20 320 180 "https://example.com/a.png"
"#;
    let s = compile(src).unwrap();
    assert_eq!(s.window.title, "Моё окно");
    assert_eq!(s.object_count(), 8);
}

#[test]
fn shortcuts_and_glued_dims() {
    let s = parse("win 640x480\nbg black\nsq 1 2 3 white\ncirc 4 5 6 red\ntri 0 0 1 1 2 0 blue\nimg 0 0 10 10 \"https://x.y/z.png\"").unwrap();
    assert_eq!(s.window.width, 640);
    assert_eq!(s.window.height, 480);
    assert_eq!(s.object_count(), 4);
}

#[test]
fn semicolon_separator() {
    let s = parse("window 100 100; background black; square 0 0 10 red; circle 50 50 5 blue").unwrap();
    assert_eq!(s.object_count(), 2);
}

#[test]
fn missing_window() {
    let err = compile("circle 10 10 5 red").unwrap_err();
    assert!(err.message.contains("не задано окно"), "got: {}", err.message);
}

#[test]
fn double_window() {
    let err = compile("window 100 100\nwindow 200 200").unwrap_err();
    assert!(err.message.contains("только одна"), "got: {}", err.message);
}

#[test]
fn unknown_command() {
    let err = compile("window 100 100\ncircl 10 10 5 red").unwrap_err();
    assert!(err.message.contains("неизвестная команда 'circl'"));
    assert_eq!(err.line, 2);
    assert_eq!(err.col, 1);
}

#[test]
fn unknown_color() {
    let err = compile("window 100 100\ncircle 10 10 5 #fffffff").unwrap_err();
    assert!(err.message.contains("HEX-цвет"), "got: {}", err.message);
}

#[test]
fn missing_arg() {
    let err = compile("window 100 100\nsquare 5 5").unwrap_err();
    assert!(err.message.contains("ожидалось число"), "got: {}", err.message);
}

#[test]
fn negative_size_rejected() {
    let err = compile("window 100 100\ncircle 10 10 -5 red").unwrap_err();
    assert!(err.message.contains("не может быть отрицательным"));
}

#[test]
fn trailing_junk() {
    let err = compile("window 100 100 extra").unwrap_err();
    assert!(err.message.contains("лишние аргументы"), "got: {}", err.message);
}

#[test]
fn huge_window_rejected() {
    let err = compile("window 999999 999999").unwrap_err();
    assert!(err.message.contains("слишком большое"));
}

#[test]
fn zero_window_rejected() {
    let err = compile("window 0 300").unwrap_err();
    assert!(err.message.contains("положительными"));
}

#[test]
fn empty_source() {
    let err = compile("").unwrap_err();
    assert!(err.message.contains("не задано окно"));
}

#[test]
fn rgba_hex_in_scene() {
    let s: Scene = parse("window 10 10\nrect 0 0 5 5 #ffffff44").unwrap();
    match &s.objects[0] {
        Object::Rect { color, .. } => assert_eq!(*color, Color::rgba(255, 255, 255, 0x44)),
        _ => panic!("ожидался rect"),
    }
}
