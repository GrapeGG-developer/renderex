//! CLI: компилятор + интерпретатор языка renderex.

use renderex::diag::print_diag;
use renderex::engine::{headless, window};
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "\
renderex — декларативный язык рендеринга окон

ИСПОЛЬЗОВАНИЕ:
    renderex <файл.rx> [ПАРАМЕТРЫ]

ПАРАМЕТРЫ:
    -o, --output <путь>   Вместо окна отрисовать сцену в PNG
    -h, --help            Показать эту справку
    -V, --version         Показать версию

В окне: Esc или закрытие окна — выход.
";

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let mut input: Option<PathBuf> = None;
    let mut output: Option<PathBuf> = None;

    while let Some(a) = args.next() {
        match a.as_str() {
            "-h" | "--help" => {
                print!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            "-V" | "--version" => {
                println!("renderex {}", env!("CARGO_PKG_VERSION"));
                return ExitCode::SUCCESS;
            }
            "-o" | "--output" => {
                output = Some(PathBuf::from(args.next().unwrap_or_else(|| {
                    eprintln!("error: параметр '{a}' требует путь к файлу");
                    std::process::exit(2);
                })));
            }
            s if s.starts_with('-') => {
                eprintln!("error: неизвестный параметр '{s}'\n{USAGE}");
                return ExitCode::from(2);
            }
            _ => input = Some(PathBuf::from(a)),
        }
    }

    let Some(input) = input else {
        eprintln!("error: укажите файл сцены (.rx)\n{USAGE}");
        return ExitCode::from(2);
    };

    let source = match std::fs::read_to_string(&input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: не удалось прочитать '{}': {e}", input.display());
            return ExitCode::from(2);
        }
    };

    let scene = match renderex::compile(&source) {
        Ok(s) => s,
        Err(d) => {
            print_diag(&input.display().to_string(), &source, &d);
            return ExitCode::FAILURE;
        }
    };

    let base_dir = input
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    let result = match output {
        Some(out) => headless::render_to_png(&scene, &out, &base_dir),
        None => window::show_window(&scene, &base_dir).map_err(|e| renderex::diag::Diag::new(e, 1, 1, 1)),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(d) => {
            print_diag(&input.display().to_string(), &source, &d);
            ExitCode::FAILURE
        }
    }
}
