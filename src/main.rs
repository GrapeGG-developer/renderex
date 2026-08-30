//! CLI: компилятор + интерпретатор языка renderex.

// В release-сборке на Windows консоль не создаётся (windows_subsystem =
// "windows"): при двойном клике по .rx-файлу не будет чёрного окна,
// а ошибки и справка показываются в диалоговых окнах (MessageBox).
// В debug-сборке консоль есть — так удобнее разрабатывать.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::process::ExitCode;

use renderex::diag::Diag;
use renderex::engine::{headless, window};
#[cfg(not(all(windows, not(debug_assertions))))]
use renderex::diag::print_diag;

#[cfg(windows)]
mod winmsg {
    // В debug-сборке на Windows консоль есть, диалоги не нужны.
    #![cfg_attr(debug_assertions, allow(dead_code))]

    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        MessageBoxW, MB_ICONERROR, MB_ICONINFORMATION, MB_OK,
    };

    fn wide(s: &str) -> Vec<u16> {
        OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
    }

    /// Диалог с ошибкой.
    pub fn error(msg: &str) {
        let m = wide(msg);
        let t = wide("Renderex — ошибка");
        unsafe {
            MessageBoxW(std::ptr::null_mut(), m.as_ptr(), t.as_ptr(), MB_OK | MB_ICONERROR);
        }
    }

    /// Информационный диалог (справка, версия).
    pub fn info(title: &str, msg: &str) {
        let m = wide(msg);
        let t = wide(title);
        unsafe {
            MessageBoxW(std::ptr::null_mut(), m.as_ptr(), t.as_ptr(), MB_OK | MB_ICONINFORMATION);
        }
    }
}

const USAGE: &str = "\
renderex — декларативный язык рендеринга окон

ИСПОЛЬЗОВАНИЕ:
    renderex <файл.rx> [ПАРАМЕТРЫ]

ПАРАМЕТРЫ:
    -o, --output <путь>   Вместо окна отрисовать сцену в PNG
    -h, --help            Показать эту справку
    -V, --version         Показать версию

В окне: Esc или закрытие окна — выход.

УСТАНОВКА НА WINDOWS (чтобы .rx открывался двойным кликом):
    powershell -ExecutionPolicy Bypass -File install\\install.ps1 -Rebuild
";

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let mut input: Option<PathBuf> = None;
    let mut output: Option<PathBuf> = None;

    while let Some(a) = args.next() {
        match a.as_str() {
            "-h" | "--help" => return show_text("Renderex — справка", USAGE),
            "-V" | "--version" => {
                return show_text("Renderex", &format!("renderex {}", env!("CARGO_PKG_VERSION")));
            }
            "-o" | "--output" => {
                let Some(path) = args.next() else {
                    return fail(&format!("параметр '{a}' требует путь к файлу"));
                };
                output = Some(PathBuf::from(path));
            }
            s if s.starts_with('-') => {
                return fail(&format!("неизвестный параметр '{s}'"));
            }
            _ => input = Some(PathBuf::from(a)),
        }
    }

    let Some(input) = input else {
        return show_text("Renderex", USAGE);
    };

    let source = match std::fs::read_to_string(&input) {
        Ok(s) => s,
        Err(e) => {
            return fail(&format!("не удалось прочитать '{}': {e}", input.display()));
        }
    };

    let scene = match renderex::compile(&source) {
        Ok(s) => s,
        Err(d) => return emit_diag(&input.display().to_string(), &source, &d),
    };

    let base_dir = input
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    let result = match output {
        Some(out) => headless::render_to_png(&scene, &out, &base_dir),
        None => window::show_window(&scene, &base_dir).map_err(|e| Diag::new(e, 1, 1, 1)),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(d) => emit_diag(&input.display().to_string(), &source, &d),
    }
}

/// Выводит текст пользователю: в консоль или (release на Windows без
/// консоли) в информационный диалог.
fn show_text(title: &str, text: &str) -> ExitCode {
    #[cfg(all(windows, not(debug_assertions)))]
    {
        winmsg::info(title, text);
        ExitCode::SUCCESS
    }
    #[cfg(not(all(windows, not(debug_assertions))))]
    {
        let _ = title;
        print!("{text}");
        ExitCode::SUCCESS
    }
}

/// Показывает диагностику компилятора с учётом платформы.
fn emit_diag(path: &str, source: &str, d: &Diag) -> ExitCode {
    #[cfg(all(windows, not(debug_assertions)))]
    {
        let line_text = source
            .lines()
            .nth(d.line.saturating_sub(1))
            .unwrap_or("")
            .replace('\t', " ");
        let msg = format!(
            "{}\n\n{}:{}:{}\n\n{}",
            d.message, path, d.line, d.col, line_text
        );
        winmsg::error(&msg);
        ExitCode::FAILURE
    }
    #[cfg(not(all(windows, not(debug_assertions))))]
    {
        print_diag(path, source, d);
        ExitCode::FAILURE
    }
}

/// Простая ошибка без позиции (файл не найден, неверный параметр и т.п.).
fn fail(msg: &str) -> ExitCode {
    #[cfg(all(windows, not(debug_assertions)))]
    {
        winmsg::error(msg);
        ExitCode::FAILURE
    }
    #[cfg(not(all(windows, not(debug_assertions))))]
    {
        eprintln!("error: {msg}");
        ExitCode::FAILURE
    }
}
