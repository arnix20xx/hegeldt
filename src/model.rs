use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AmpModel {
    Rost,
    H95,
    H120,
    H190,
    H390,
    H590,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputSource {
    pub id: u8,
    pub name: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

impl InputSource {
    pub fn new(id: u8, name: &str) -> Self {
        InputSource {
            id,
            name: name.to_string(),
            label: None,
            enabled: true,
        }
    }

    /// The user label if non-empty, else the factory name.
    pub fn display_name(&self) -> &str {
        match self.label.as_deref() {
            Some(l) if !l.is_empty() => l,
            _ => &self.name,
        }
    }
}

fn table(names: &[&str], first_id: u8) -> Vec<InputSource> {
    names
        .iter()
        .enumerate()
        .map(|(i, n)| InputSource::new(first_id + i as u8, n))
        .collect()
}

impl AmpModel {
    pub const ALL: [AmpModel; 6] = [
        AmpModel::Rost,
        AmpModel::H95,
        AmpModel::H120,
        AmpModel::H190,
        AmpModel::H390,
        AmpModel::H590,
    ];

    pub fn label(self) -> &'static str {
        match self {
            AmpModel::Rost => "Röst",
            AmpModel::H95 => "H95",
            AmpModel::H120 => "H120",
            AmpModel::H190 => "H190",
            AmpModel::H390 => "H390",
            AmpModel::H590 => "H590",
        }
    }

    pub fn default_inputs(self) -> Vec<InputSource> {
        match self {
            AmpModel::H95 => table(
                &[
                    "Analog 1",
                    "Analog 2",
                    "Coaxial",
                    "Optical 1",
                    "Optical 2",
                    "Optical 3",
                    "USB",
                    "Network",
                ],
                1,
            ),
            AmpModel::Rost | AmpModel::H120 | AmpModel::H190 => table(
                &[
                    "Balanced",
                    "Analog 1",
                    "Analog 2",
                    "Coaxial",
                    "Optical 1",
                    "Optical 2",
                    "Optical 3",
                    "USB",
                    "Network",
                ],
                1,
            ),
            AmpModel::H390 => table(
                &[
                    "XLR",
                    "Analog 1",
                    "Analog 2",
                    "BNC",
                    "Coaxial",
                    "Optical 1",
                    "Optical 2",
                    "Optical 3",
                    "USB",
                    "Network",
                ],
                1,
            ),
            AmpModel::H590 => table(
                &[
                    "XLR1",
                    "XLR2",
                    "Analog 1",
                    "Analog 2",
                    "Analog 3",
                    "BNC",
                    "Coaxial",
                    "Optical 1",
                    "Optical 2",
                    "Optical 3",
                    "USB",
                    "Network",
                ],
                1,
            ),
        }
    }

    /// Guesses the model from an SSDP `modelName` string such as "Hegel H390".
    pub fn guess_from_model_name(name: &str) -> Option<AmpModel> {
        let n = name.to_lowercase();
        if n.contains("h590") {
            Some(AmpModel::H590)
        } else if n.contains("h390") {
            Some(AmpModel::H390)
        } else if n.contains("h190") {
            Some(AmpModel::H190)
        } else if n.contains("h120") {
            Some(AmpModel::H120)
        } else if n.contains("h95") {
            Some(AmpModel::H95)
        } else if n.contains("röst") || n.contains("rost") {
            Some(AmpModel::Rost)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts() {
        assert_eq!(AmpModel::H95.default_inputs().len(), 8);
        assert_eq!(AmpModel::Rost.default_inputs().len(), 9);
        assert_eq!(AmpModel::H120.default_inputs().len(), 9);
        assert_eq!(AmpModel::H190.default_inputs().len(), 9);
        assert_eq!(AmpModel::H390.default_inputs().len(), 10);
        assert_eq!(AmpModel::H590.default_inputs().len(), 12);
    }

    #[test]
    fn ids_sequential_from_one() {
        for m in AmpModel::ALL {
            for (i, s) in m.default_inputs().iter().enumerate() {
                assert_eq!(s.id as usize, i + 1);
            }
        }
    }

    #[test]
    fn h390_table() {
        let t = AmpModel::H390.default_inputs();
        let names: Vec<_> = t.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "XLR",
                "Analog 1",
                "Analog 2",
                "BNC",
                "Coaxial",
                "Optical 1",
                "Optical 2",
                "Optical 3",
                "USB",
                "Network"
            ]
        );
    }

    #[test]
    fn other_tables() {
        let h95 = AmpModel::H95.default_inputs();
        assert_eq!(h95[2].name, "Coaxial");
        assert_eq!(h95[7].name, "Network");
        let h190 = AmpModel::H190.default_inputs();
        assert_eq!(h190[0].name, "Balanced");
        assert_eq!(h190[3].name, "Coaxial");
        let h590 = AmpModel::H590.default_inputs();
        assert_eq!(h590[5].name, "BNC");
        assert_eq!(h590[6].name, "Coaxial");
        assert_eq!(h590[11].name, "Network");
    }

    #[test]
    fn labels_and_display() {
        assert_eq!(AmpModel::Rost.label(), "Röst");
        let mut s = InputSource::new(1, "XLR");
        assert_eq!(s.display_name(), "XLR");
        s.label = Some(String::new());
        assert_eq!(s.display_name(), "XLR");
        s.label = Some("Turntable".into());
        assert_eq!(s.display_name(), "Turntable");
    }

    #[test]
    fn guess() {
        assert_eq!(
            AmpModel::guess_from_model_name("Hegel H390"),
            Some(AmpModel::H390)
        );
        assert_eq!(AmpModel::guess_from_model_name("h95"), Some(AmpModel::H95));
        assert_eq!(
            AmpModel::guess_from_model_name("Röst"),
            Some(AmpModel::Rost)
        );
        assert_eq!(AmpModel::guess_from_model_name("Sonos"), None);
    }

    #[test]
    fn serde_roundtrip() {
        let j = serde_json::to_string(&AmpModel::H390).unwrap();
        assert_eq!(
            serde_json::from_str::<AmpModel>(&j).unwrap(),
            AmpModel::H390
        );
    }
}
