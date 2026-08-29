//! Цвета: HEX (`#ff8800`, `#f80`, `#ff8800cc`) и именованные (`red`, `teal`).

/// Цвет RGBA, каждый канал 0..255.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Упаковка в u32: `0xRRGGBBAA` (порядок байт как в памяти RGBA-пикселя).
    pub const fn pack(self) -> u32 {
        ((self.r as u32) << 24) | ((self.g as u32) << 16) | ((self.b as u32) << 8) | self.a as u32
    }

    pub const fn unpack(p: u32) -> Self {
        Self {
            r: (p >> 24) as u8,
            g: (p >> 16) as u8,
            b: (p >> 8) as u8,
            a: p as u8,
        }
    }
}

/// Именованный цвет (регистр не важен).
pub fn named(name: &str) -> Option<Color> {
    let c = match name.to_ascii_lowercase().as_str() {
        "black" => Color::rgb(0, 0, 0),
        "white" => Color::rgb(255, 255, 255),
        "red" => Color::rgb(255, 0, 0),
        "green" => Color::rgb(0, 128, 0),
        "lime" => Color::rgb(0, 255, 0),
        "blue" => Color::rgb(0, 0, 255),
        "yellow" => Color::rgb(255, 255, 0),
        "cyan" | "aqua" => Color::rgb(0, 255, 255),
        "magenta" | "fuchsia" => Color::rgb(255, 0, 255),
        "gray" | "grey" => Color::rgb(128, 128, 128),
        "silver" => Color::rgb(192, 192, 192),
        "maroon" => Color::rgb(128, 0, 0),
        "purple" => Color::rgb(128, 0, 128),
        "olive" => Color::rgb(128, 128, 0),
        "navy" => Color::rgb(0, 0, 128),
        "teal" => Color::rgb(0, 128, 128),
        "orange" => Color::rgb(255, 165, 0),
        "pink" => Color::rgb(255, 192, 203),
        "brown" => Color::rgb(165, 42, 42),
        "gold" => Color::rgb(255, 215, 0),
        "violet" => Color::rgb(238, 130, 238),
        "indigo" => Color::rgb(75, 0, 130),
        "coral" => Color::rgb(255, 127, 80),
        "tomato" => Color::rgb(255, 99, 71),
        "salmon" => Color::rgb(250, 128, 114),
        "khaki" => Color::rgb(240, 230, 140),
        "turquoise" => Color::rgb(64, 224, 208),
        "crimson" => Color::rgb(220, 20, 60),
        "beige" => Color::rgb(245, 245, 220),
        _ => return None,
    };
    Some(c)
}

/// Разбор HEX-цвета: `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`.
pub fn parse_hex(s: &str) -> Option<Color> {
    let digits = s.strip_prefix('#')?;
    let byte = |i: usize| u8::from_str_radix(&digits[i..i + 2], 16).ok();
    let expand = |c: char| {
        let v = c.to_digit(16)? as u8;
        Some((v << 4) | v)
    };
    match digits.len() {
        3 => {
            let mut it = digits.chars();
            Some(Color::rgb(
                expand(it.next()?)?,
                expand(it.next()?)?,
                expand(it.next()?)?,
            ))
        }
        4 => {
            let mut it = digits.chars();
            Some(Color::rgba(
                expand(it.next()?)?,
                expand(it.next()?)?,
                expand(it.next()?)?,
                expand(it.next()?)?,
            ))
        }
        6 => Some(Color::rgb(byte(0)?, byte(2)?, byte(4)?)),
        8 => Some(Color::rgba(byte(0)?, byte(2)?, byte(4)?, byte(6)?)),
        _ => None,
    }
}
