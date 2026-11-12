//! Colours taken from the Android app (Color.kt) plus a few dark-theme extras.

use gpui::{Rgba, rgb};

pub fn background() -> Rgba {
    rgb(0x222222)
}
pub fn surface() -> Rgba {
    rgb(0x333333)
}
pub fn surface_hover() -> Rgba {
    rgb(0x444444)
}
pub fn surface_active() -> Rgba {
    rgb(0x555555)
}
pub fn muted() -> Rgba {
    rgb(0x888888)
}
pub fn text() -> Rgba {
    rgb(0xe6e6e6)
}
pub fn display_bg() -> Rgba {
    rgb(0x000000)
}
pub fn display_text() -> Rgba {
    rgb(0xf1f1f2)
}
pub fn ok() -> Rgba {
    rgb(0x03dac5)
}
pub fn error() -> Rgba {
    rgb(0xcf6679)
}
pub fn accent() -> Rgba {
    rgb(0xbb86fc)
}
