use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::discovery::DiscoveredDevice;
use crate::model::{AmpModel, InputSource};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub host: Option<String>,
    pub model: AmpModel,
    pub max_volume: u8,
    /// Keep the app in the macOS menu bar (status item); closing the window then hides it.
    pub menu_bar: bool,
    /// Saved per-model input lists (labels / enabled flags).
    pub inputs: HashMap<AmpModel, Vec<InputSource>>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            host: None,
            model: AmpModel::H390,
            max_volume: 60,
            menu_bar: false,
            inputs: HashMap::new(),
        }
    }
}

impl Config {
    /// Platform config dir: `~/Library/Application Support/hegeldt/config.json` on macOS, `~/.config/hegeldt/` on Linux, `%APPDATA%\hegeldt\` on Windows.
    pub fn default_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("hegeldt").join("config.json"))
    }

    /// Loads the config, returning defaults if missing or invalid.
    pub fn load() -> Config {
        Self::default_path()
            .map(|p| Self::load_from(&p))
            .unwrap_or_default()
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path =
            Self::default_path().ok_or_else(|| anyhow::anyhow!("no config directory available"))?;
        self.save_to(&path)
    }

    pub fn load_from(path: &Path) -> Config {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save_to(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    /// Uses a discovered amplifier: its IP becomes the host, and the model is set if recognised.
    pub fn apply_device(&mut self, dev: &DiscoveredDevice) {
        self.host = Some(dev.host.clone());
        if let Some(m) = AmpModel::guess_from_model_name(&dev.model) {
            self.model = m;
        }
    }

    /// Default inputs for the model with saved labels / enabled flags merged over them.
    pub fn inputs_for(&self, model: AmpModel) -> Vec<InputSource> {
        let saved = self.inputs.get(&model);
        model
            .default_inputs()
            .into_iter()
            .map(|mut d| {
                if let Some(s) = saved.and_then(|v| v.iter().find(|s| s.id == d.id)) {
                    d.label = s.label.clone();
                    d.enabled = s.enabled;
                }
                d
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("hegeldt-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d.join("sub").join("config.json")
    }

    #[test]
    fn defaults() {
        let c = Config::default();
        assert_eq!(c.host, None);
        assert_eq!(c.model, AmpModel::H390);
        assert_eq!(c.max_volume, 60);
        assert!(!c.menu_bar);
    }

    #[test]
    fn missing_and_invalid_give_defaults() {
        let p = tmp("missing");
        assert_eq!(Config::load_from(&p), Config::default());
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, "not json{").unwrap();
        assert_eq!(Config::load_from(&p), Config::default());
        std::fs::remove_dir_all(p.parent().unwrap().parent().unwrap()).unwrap();
    }

    #[test]
    fn roundtrip_and_merge() {
        let p = tmp("roundtrip");
        let mut c = Config {
            host: Some("127.0.0.1".into()),
            max_volume: 40,
            ..Default::default()
        };
        let mut i = InputSource::new(2, "Analog 1");
        i.label = Some("Turntable".into());
        i.enabled = false;
        c.inputs.insert(AmpModel::H390, vec![i]);
        c.save_to(&p).unwrap();
        let l = Config::load_from(&p);
        assert_eq!(l, c);
        let inputs = l.inputs_for(AmpModel::H390);
        assert_eq!(inputs.len(), 10);
        assert_eq!(inputs[1].display_name(), "Turntable");
        assert!(!inputs[1].enabled);
        assert!(inputs[0].enabled);
        assert_eq!(l.inputs_for(AmpModel::H95), AmpModel::H95.default_inputs());
        std::fs::remove_dir_all(p.parent().unwrap().parent().unwrap()).unwrap();
    }

    #[test]
    fn partial_json_uses_defaults() {
        let p = tmp("partial");
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, r#"{"host":"127.0.0.1"}"#).unwrap();
        let c = Config::load_from(&p);
        assert_eq!(c.host.as_deref(), Some("127.0.0.1"));
        assert_eq!(c.max_volume, 60);
        std::fs::remove_dir_all(p.parent().unwrap().parent().unwrap()).unwrap();
    }

    #[test]
    fn menu_bar_roundtrip_and_old_files_load() {
        let p = tmp("menubar");
        let c = Config {
            menu_bar: true,
            ..Default::default()
        };
        c.save_to(&p).unwrap();
        assert!(Config::load_from(&p).menu_bar);
        // A config written before the setting existed still loads, with the default.
        std::fs::write(&p, r#"{"host":"192.168.1.50","model":"H390","max_volume":50}"#).unwrap();
        let old = Config::load_from(&p);
        assert_eq!(old.host.as_deref(), Some("192.168.1.50"));
        assert_eq!(old.max_volume, 50);
        assert!(!old.menu_bar);
        std::fs::remove_dir_all(p.parent().unwrap().parent().unwrap()).unwrap();
    }

    #[test]
    fn apply_device_sets_host_and_known_model() {
        let mut c = Config::default();
        let mut d = DiscoveredDevice {
            friendly_name: "H390_ABC123".into(),
            host: "192.168.1.50".into(),
            model: "H390".into(),
            manufacturer: "Hegel".into(),
        };
        c.model = AmpModel::H95;
        c.apply_device(&d);
        assert_eq!(c.host.as_deref(), Some("192.168.1.50"));
        assert_eq!(c.model, AmpModel::H390);
        d.model = "Mystery".into();
        d.host = "10.0.0.5".into();
        c.apply_device(&d);
        assert_eq!(c.host.as_deref(), Some("10.0.0.5"));
        assert_eq!(
            c.model,
            AmpModel::H390,
            "unknown model leaves the setting alone"
        );
    }
}
