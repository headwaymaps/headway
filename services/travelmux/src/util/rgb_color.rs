use serde::{Serialize, Serializer};
use std::fmt;

/// A color as GTFS writes it: "RRGGBB" hex, without a leading "#".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RgbColor {
    red: u8,
    green: u8,
    blue: u8,
}

impl RgbColor {
    /// `None` for anything but six hex digits.
    pub fn parse(hex: &str) -> Option<Self> {
        if hex.len() != 6 || !hex.is_ascii() {
            return None;
        }
        let channel = |range| u8::from_str_radix(&hex[range], 16).ok();
        Some(Self {
            red: channel(0..2)?,
            green: channel(2..4)?,
            blue: channel(4..6)?,
        })
    }
}

impl RgbColor {
    pub const BLACK: Self = Self::new(0x00, 0x00, 0x00);
    pub const WHITE: Self = Self::new(0xFF, 0xFF, 0xFF);

    pub const fn new(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }

    /// Black or white, whichever reads better as text on this color.
    pub fn readable_text_color(&self) -> Self {
        let channel = |value: u8| {
            let value = f64::from(value) / 255.0;
            if value <= 0.04045 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        };
        let luminance =
            0.2126 * channel(self.red) + 0.7152 * channel(self.green) + 0.0722 * channel(self.blue);
        // Above where white and black contrast equally (0.179), since white reads better than the
        // math says on a saturated color: LA Metro's B Line red, at 0.18, wants white. Route
        // labels are bold, which buys some slack on contrast.
        if luminance > 0.25 {
            Self::BLACK
        } else {
            Self::WHITE
        }
    }
}

impl fmt::Display for RgbColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02X}{:02X}{:02X}", self.red, self.green, self.blue)
    }
}

impl Serialize for RgbColor {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_writes_gtfs_hex() {
        let color = RgbColor::parse("eb131b").unwrap();
        assert_eq!(color.to_string(), "EB131B");
        assert_eq!(serde_json::to_string(&color).unwrap(), "\"EB131B\"");
    }

    #[test]
    fn readable_text_color_leans_white() {
        let hex = |s| RgbColor::parse(s).unwrap();
        // White on a saturated red, though black contrasts as well.
        assert_eq!(hex("EB131B").readable_text_color(), RgbColor::WHITE);
        assert_eq!(hex("28813F").readable_text_color(), RgbColor::WHITE);
        assert_eq!(hex("FDB71A").readable_text_color(), RgbColor::BLACK);
        assert_eq!(hex("1296FF").readable_text_color(), RgbColor::BLACK);
    }

    #[test]
    fn rejects_anything_but_six_hex_digits() {
        assert_eq!(RgbColor::parse("#EB131B"), None);
        assert_eq!(RgbColor::parse("FFF"), None);
        assert_eq!(RgbColor::parse("GG0000"), None);
        assert_eq!(RgbColor::parse("é00000"), None);
        assert_eq!(RgbColor::parse(""), None);
    }
}
