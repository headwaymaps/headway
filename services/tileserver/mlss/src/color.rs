//! Colors that can be manipulated arithmetically and rendered back to CSS.

use serde_json::Value as Json;
use std::fmt;

/// How a color renders back to CSS.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorFormat {
    Hex,
    Rgb,
    Hsl,
}

/// Constructs an HSL color from CSS-style components.
#[macro_export]
macro_rules! hsl {
    ($hue:literal, $saturation:literal %, $lightness:literal %) => {
        $crate::Color::parse(concat!(
            "hsl(",
            stringify!($hue),
            ",",
            stringify!($saturation),
            "%,",
            stringify!($lightness),
            "%)"
        ))
    };
}

/// Constructs an HSL color with an alpha channel.
#[macro_export]
macro_rules! hsla {
    ($hue:literal, $saturation:literal %, $lightness:literal %, $alpha:literal) => {
        $crate::Color::parse(concat!(
            "hsla(",
            stringify!($hue),
            ",",
            stringify!($saturation),
            "%,",
            stringify!($lightness),
            "%,",
            stringify!($alpha),
            ")"
        ))
    };
}

/// Constructs an RGB color from byte components.
#[macro_export]
macro_rules! rgb {
    ($red:literal, $green:literal, $blue:literal) => {
        $crate::Color::parse(concat!(
            "rgb(",
            stringify!($red),
            ",",
            stringify!($green),
            ",",
            stringify!($blue),
            ")"
        ))
    };
}

/// Constructs an RGB color with an alpha channel.
#[macro_export]
macro_rules! rgba {
    ($red:literal, $green:literal, $blue:literal, $alpha:literal) => {
        $crate::Color::parse(concat!(
            "rgba(",
            stringify!($red),
            ",",
            stringify!($green),
            ",",
            stringify!($blue),
            ",",
            stringify!($alpha),
            ")"
        ))
    };
}

/// An HSLA color, remembering the notation it was written in.
#[derive(Clone, Debug)]
pub struct Color {
    hue: f64,
    saturation: f64,
    lightness: f64,
    alpha: f64,
    notation: ColorFormat,
    /// The source text, preserved verbatim until the color is modified.
    literal: Option<String>,
}

impl PartialEq for Color {
    fn eq(&self, other: &Self) -> bool {
        self.to_css() == other.to_css()
    }
}

impl Color {
    pub fn hsla(hue: f64, saturation: f64, lightness: f64, alpha: f64) -> Self {
        Self {
            hue: hue.rem_euclid(360.0),
            saturation: saturation.clamp(0.0, 1.0),
            lightness: lightness.clamp(0.0, 1.0),
            alpha: alpha.clamp(0.0, 1.0),
            notation: ColorFormat::Hsl,
            literal: None,
        }
    }

    /// Saturation and lightness are percentages, as in CSS.
    pub fn hsl(hue: f64, saturation: f64, lightness: f64) -> Self {
        Self::hsla(hue, saturation / 100.0, lightness / 100.0, 1.0)
    }

    pub fn rgba(red: u8, green: u8, blue: u8, alpha: f64) -> Self {
        let mut color = Self::from_rgb_f64(
            red as f64 / 255.0,
            green as f64 / 255.0,
            blue as f64 / 255.0,
            alpha,
        );
        color.notation = ColorFormat::Rgb;
        color
    }

    pub fn rgb(red: u8, green: u8, blue: u8) -> Self {
        Self::rgba(red, green, blue, 1.0)
    }

    /// Parses any of the CSS notations maplibre accepts, panicking on anything else.
    pub fn parse(css: &str) -> Self {
        Self::try_parse(css).unwrap_or_else(|| panic!("unrecognized color: {css}"))
    }

    pub fn try_parse(css: &str) -> Option<Self> {
        let trimmed = css.trim();
        let mut color = if let Some(hex) = trimmed.strip_prefix('#') {
            Self::parse_hex(hex)?
        } else {
            Self::parse_functional(trimmed)?
        };
        color.literal = Some(trimmed.to_owned());
        Some(color)
    }

    fn parse_hex(hex: &str) -> Option<Self> {
        let nibble = |c: u8| (c as char).to_digit(16).map(|d| d as f64 / 15.0);
        let byte = |pair: &[u8]| {
            let hi = (pair[0] as char).to_digit(16)?;
            let lo = (pair[1] as char).to_digit(16)?;
            Some((hi * 16 + lo) as f64 / 255.0)
        };
        let bytes = hex.as_bytes();
        let (red, green, blue, alpha) = match bytes.len() {
            3 => (nibble(bytes[0])?, nibble(bytes[1])?, nibble(bytes[2])?, 1.0),
            6 => (
                byte(&bytes[0..2])?,
                byte(&bytes[2..4])?,
                byte(&bytes[4..6])?,
                1.0,
            ),
            8 => (
                byte(&bytes[0..2])?,
                byte(&bytes[2..4])?,
                byte(&bytes[4..6])?,
                byte(&bytes[6..8])?,
            ),
            _ => return None,
        };
        let mut color = Self::from_rgb_f64(red, green, blue, alpha);
        color.notation = ColorFormat::Hex;
        Some(color)
    }

    fn parse_functional(css: &str) -> Option<Self> {
        let open = css.find('(')?;
        let name = css[..open].trim().to_ascii_lowercase();
        let inner = css[open + 1..].strip_suffix(')')?;
        let args: Vec<&str> = inner.split(',').map(str::trim).collect();
        let number = |arg: &str| arg.trim_end_matches('%').trim().parse::<f64>().ok();
        let alpha = match args.len() {
            3 => 1.0,
            4 => number(args[3])?,
            _ => return None,
        };
        match name.as_str() {
            "rgb" | "rgba" => {
                let mut color = Self::from_rgb_f64(
                    number(args[0])? / 255.0,
                    number(args[1])? / 255.0,
                    number(args[2])? / 255.0,
                    alpha,
                );
                color.notation = ColorFormat::Rgb;
                Some(color)
            }
            "hsl" | "hsla" => Some(Self::hsla(
                number(args[0])?,
                number(args[1])? / 100.0,
                number(args[2])? / 100.0,
                alpha,
            )),
            _ => None,
        }
    }

    fn from_rgb_f64(red: f64, green: f64, blue: f64, alpha: f64) -> Self {
        let max = red.max(green).max(blue);
        let min = red.min(green).min(blue);
        let lightness = (max + min) / 2.0;
        let delta = max - min;
        let (hue, saturation) = if delta == 0.0 {
            (0.0, 0.0)
        } else {
            let saturation = delta / (1.0 - (2.0 * lightness - 1.0).abs());
            let hue = if max == red {
                60.0 * (((green - blue) / delta) % 6.0)
            } else if max == green {
                60.0 * ((blue - red) / delta + 2.0)
            } else {
                60.0 * ((red - green) / delta + 4.0)
            };
            (hue, saturation)
        };
        let mut color = Self::hsla(hue, saturation, lightness, alpha);
        color.notation = ColorFormat::Hex;
        color
    }

    fn to_rgb_f64(&self) -> (f64, f64, f64) {
        let chroma = (1.0 - (2.0 * self.lightness - 1.0).abs()) * self.saturation;
        let sector = self.hue / 60.0;
        let second = chroma * (1.0 - (sector % 2.0 - 1.0).abs());
        let (red, green, blue) = match sector as u8 {
            0 => (chroma, second, 0.0),
            1 => (second, chroma, 0.0),
            2 => (0.0, chroma, second),
            3 => (0.0, second, chroma),
            4 => (second, 0.0, chroma),
            _ => (chroma, 0.0, second),
        };
        let lift = self.lightness - chroma / 2.0;
        (red + lift, green + lift, blue + lift)
    }

    fn derived(&self, mutate: impl FnOnce(&mut Self)) -> Self {
        let mut color = self.clone();
        color.literal = None;
        mutate(&mut color);
        color.saturation = color.saturation.clamp(0.0, 1.0);
        color.lightness = color.lightness.clamp(0.0, 1.0);
        color.alpha = color.alpha.clamp(0.0, 1.0);
        color
    }

    pub fn darken(&self, amount: f64) -> Self {
        self.derived(|color| color.lightness -= amount)
    }

    pub fn lighten(&self, amount: f64) -> Self {
        self.derived(|color| color.lightness += amount)
    }

    pub fn saturate(&self, amount: f64) -> Self {
        self.derived(|color| color.saturation += amount)
    }

    pub fn desaturate(&self, amount: f64) -> Self {
        self.derived(|color| color.saturation -= amount)
    }

    pub fn rotate_hue(&self, degrees: f64) -> Self {
        self.derived(|color| color.hue = (color.hue + degrees).rem_euclid(360.0))
    }

    pub fn alpha(&self, alpha: f64) -> Self {
        self.derived(|color| color.alpha = alpha)
    }

    /// Blends towards `other`, where 0 is entirely self and 1 entirely `other`.
    pub fn mix(&self, other: &Color, amount: f64) -> Self {
        let (self_red, self_green, self_blue) = self.to_rgb_f64();
        let (other_red, other_green, other_blue) = other.to_rgb_f64();
        let lerp = |a: f64, b: f64| a + (b - a) * amount;
        let mut color = Self::from_rgb_f64(
            lerp(self_red, other_red),
            lerp(self_green, other_green),
            lerp(self_blue, other_blue),
            lerp(self.alpha, other.alpha),
        );
        color.notation = self.notation;
        color
    }

    pub fn to_css(&self) -> String {
        if let Some(literal) = &self.literal {
            return literal.clone();
        }
        match self.notation {
            ColorFormat::Hex if self.alpha == 1.0 => {
                let (red, green, blue) = self.to_rgb_f64();
                let channel = |value: f64| (value * 255.0).round() as u8;
                format!(
                    "#{:02x}{:02x}{:02x}",
                    channel(red),
                    channel(green),
                    channel(blue)
                )
            }
            ColorFormat::Hex | ColorFormat::Rgb => {
                let (red, green, blue) = self.to_rgb_f64();
                let channel = |value: f64| (value * 255.0).round() as u8;
                let (red, green, blue) = (channel(red), channel(green), channel(blue));
                if self.alpha == 1.0 {
                    format!("rgb({red}, {green}, {blue})")
                } else {
                    format!("rgba({red}, {green}, {blue}, {})", trim(self.alpha))
                }
            }
            ColorFormat::Hsl => {
                let hue = trim(self.hue);
                let saturation = trim(self.saturation * 100.0);
                let lightness = trim(self.lightness * 100.0);
                if self.alpha == 1.0 {
                    format!("hsl({hue}, {saturation}%, {lightness}%)")
                } else {
                    format!(
                        "hsla({hue}, {saturation}%, {lightness}%, {})",
                        trim(self.alpha)
                    )
                }
            }
        }
    }
}

fn trim(value: f64) -> String {
    let rounded = (value * 1000.0).round() / 1000.0;
    let mut text = format!("{rounded}");
    if text.ends_with(".0") {
        text.truncate(text.len() - 2);
    }
    text
}

impl fmt::Display for Color {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_css())
    }
}

impl From<Color> for Json {
    fn from(color: Color) -> Json {
        Json::String(color.to_css())
    }
}

impl From<&str> for Color {
    fn from(css: &str) -> Color {
        Color::parse(css)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_the_written_notation() {
        for css in ["#fc8", "rgb(141,220,248)", "hsl(101,62.6%,63.3%)"] {
            assert_eq!(Color::parse(css).to_css(), css);
        }
    }

    #[test]
    fn macros_keep_their_css_notation() {
        assert_eq!(crate::hsl!(0, 0%, 86.6667%).to_css(), "hsl(0,0%,86.6667%)");
        assert_eq!(
            crate::rgba!(255, 255, 255, 0.8).to_css(),
            "rgba(255,255,255,0.8)"
        );
    }

    #[test]
    fn darkening_drops_the_literal() {
        assert_eq!(
            Color::parse("hsl(0, 0%, 50%)").darken(0.1).to_css(),
            "hsl(0, 0%, 40%)"
        );
        assert_eq!(Color::parse("#ffffff").darken(0.5).to_css(), "#808080");
    }

    #[test]
    fn mixes_towards_the_other_color() {
        let black = Color::parse("#000000");
        assert_eq!(black.mix(&Color::parse("#ffffff"), 0.5).to_css(), "#808080");
    }
}
