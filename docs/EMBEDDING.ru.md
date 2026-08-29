# Встраивание Renderex в другой язык

Renderex спроектирован как **модуль**: ядро языка (лексер + парсер + AST)
полностью отделено от рендера. Ниже — схема встраивания в ваш будущий язык
программирования.

## Уровень 1: как внешний процесс

Самый простой вариант — вызывать готовый бинарник:

```
renderex scene.rx --output result.png
```

Ваш язык генерирует `.rx`-файл, вызывает `renderex`, читает PNG/получает
код возврата. Ошибки renderex печатает в stderr с позицией в исходнике —
их можно показывать пользователю как есть.

## Уровень 2: как нативная библиотека (FFI)

Соберите renderex как библиотеку (cdylib или staticlib):

```toml
[lib]
crate-type = ["rlib", "cdylib"]
```

Публичное API — функции на C-ABI поверх ядра:

```rust
// render.h (сгенерированный интерфейс)
typedef struct { int32_t width, height; } rx_window;
typedef struct { uint32_t r, g, b, a; } rx_color;

// компиляция: исходник → ошибка или 0
typedef struct { const char *message; uint32_t line, col, len; } rx_error;
int rx_compile(const char *source, rx_error *err);

// рендер в буфер пикселей (0xRRGGBBAA)
int rx_render_to_buffer(const char *source, uint32_t *out_pixels,
                        uint32_t *out_width, uint32_t *out_height,
                        rx_error *err);

// показать окно
int rx_show_window(const char *source, rx_error *err);
```

Любой язык с FFI (C, C++, Zig, Rust, Go, Python через ctypes/cffi, .NET
через P/Invoke) сможет подключить её как обычный модуль.

## Уровень 3: как библиотека на том же языке

Если ваш будущий язык реализуется на Rust — просто добавляете зависимость:

```toml
[dependencies]
renderex = { path = "..." }
```

и пользуетесь высокоуровневым API:

```rust
use renderex::{compile, engine};

// 1. Скомпилировать исходник в AST
let scene = compile("window 800 600\ncircle 400 300 100 red")?;

// 2. Отрисовать без окна
let fb = engine::render_scene(&scene, &Default::default());

// 3. Или показать нативное окно
engine::window::show_window(&scene, &std::path::Path::new("."))?;
```

## Почему ядро отделено

- `lexer` / `parser` / `ast` — чистые функции: `&str → Result<Scene>`,
  без ввода-вывода и без зависимости от графики. Их можно вызвать из
  любого места программы.
- `engine` — `Scene → пиксели`: детерминированно, работает даже без
  дисплея (headless), что удобно для тестов и серверов.
- `engine::window` — тонкий слой над winit: единственная часть, которой
  нужен графический дисплей.

Благодаря этому ваш будущий язык может использовать renderex на любом
уровне — от генерации `.rx`-файлов до вызова функций напрямую.
