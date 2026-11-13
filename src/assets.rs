//! Icons embedded in the binary and served to GPUI's `svg()` element.

use std::borrow::Cow;

use anyhow::Result;
use gpui::{AssetSource, SharedString};

const ICONS: &[(&str, &[u8])] = &[
    (
        "icons/power.svg",
        include_bytes!("../assets/icons/power.svg"),
    ),
    (
        "icons/volume-on.svg",
        include_bytes!("../assets/icons/volume-on.svg"),
    ),
    (
        "icons/volume-off.svg",
        include_bytes!("../assets/icons/volume-off.svg"),
    ),
    (
        "icons/volume-up.svg",
        include_bytes!("../assets/icons/volume-up.svg"),
    ),
    (
        "icons/volume-down.svg",
        include_bytes!("../assets/icons/volume-down.svg"),
    ),
    (
        "icons/settings.svg",
        include_bytes!("../assets/icons/settings.svg"),
    ),
    (
        "icons/chevron-down.svg",
        include_bytes!("../assets/icons/chevron-down.svg"),
    ),
];

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Ok(ICONS
            .iter()
            .find(|(p, _)| *p == path)
            .map(|(_, data)| Cow::Borrowed(*data)))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(ICONS
            .iter()
            .filter(|(p, _)| p.starts_with(path))
            .map(|(p, _)| SharedString::from(*p))
            .collect())
    }
}
