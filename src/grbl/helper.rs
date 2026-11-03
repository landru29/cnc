use std::collections::HashMap;
use serde_yaml;
use anyhow::Result;

const DE_YAML: &str = include_str!("de.yaml");
const EN_YAML: &str = include_str!("en.yaml");
const ES_YAML: &str = include_str!("es.yaml");
const FR_YAML: &str = include_str!("fr.yaml");
const PT_YAML: &str = include_str!("pt.yaml");

pub enum Lang {
    De,
    En,
    Es,
    Fr,
    Pt,
}

impl From<String> for Lang {
    fn from(value: String) -> Self {
        if value.to_ascii_lowercase() == "fr" {
            Self::Fr
        } else if value.to_ascii_lowercase() == "es" {
            Self::Es
        } else if value.to_ascii_lowercase() == "de" {
            Self::De
        } else if value.to_ascii_lowercase() == "pt" {
            Self::Pt
        } else {
            Self::En
        }
    }
}

pub struct Helper {
    data: HashMap<String, String>
}

impl Helper {
    pub fn new(lang: Lang) -> Result<Self> {
        let data = match lang {
            Lang::De => serde_yaml::from_str(DE_YAML)?,
            Lang::En => serde_yaml::from_str(EN_YAML)?,
            Lang::Es => serde_yaml::from_str(ES_YAML)?,
            Lang::Fr => serde_yaml::from_str(FR_YAML)?,
            Lang::Pt => serde_yaml::from_str(PT_YAML)?,
        };

        Ok(Self{ data })
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        let trimmed_key = match key.split_once(' ') {
            Some((before_space, _)) => before_space,
            None => key,
        };

        self.data.get(trimmed_key).map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn en_yaml_is_embedded_and_parsed() {
        let helper = Helper::new(Lang::En).unwrap();
        let g0 = helper.get("G0").expect("G0 should be present");
        assert!(g0.contains("Rapid Positioning"));
        assert!(helper.get("G38.2").is_some());
        assert!(helper.get("G0 Y0 X0").is_some());
    }

    #[test]
    fn fr_yaml_is_embedded_and_parsed() {
        let helper = Helper::new(Lang::Fr).unwrap();
        let g0 = helper.get("G0").expect("G0 should be present");
        assert!(g0.contains("Positionnement rapide"));
        assert!(helper.get("G38.2").is_some());
    }
}