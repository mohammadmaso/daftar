//! `.daftar/config.json`: non-secret settings shared by all devices. Unknown fields are preserved
//! so older app versions never drop settings written by newer ones.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::REPO_SCHEMA_VERSION;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bilingual {
    pub en: String,
    pub fa: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VaultConfig {
    pub id: String,
    pub title: Bilingual,
    pub purpose: String,
    #[serde(default)]
    pub archived: bool,
    /// Fiction vaults are isolated from claims about the user (§3.2).
    #[serde(default)]
    pub fiction: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub schema_version: u32,
    #[serde(default = "default_branch")]
    pub branch: String,
    pub vaults: Vec<VaultConfig>,
    #[serde(default)]
    pub ai: crate::providers::AiConfig,
    /// Routing confidence below which a "Was this right?" card is added (§4.2).
    #[serde(default = "default_threshold")]
    pub routing_threshold: f64,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

fn default_threshold() -> f64 {
    0.6
}

fn default_branch() -> String {
    "main".into()
}

fn vault(id: &str, en: &str, fa: &str, purpose: &str, fiction: bool) -> VaultConfig {
    VaultConfig {
        id: id.into(),
        title: Bilingual {
            en: en.into(),
            fa: fa.into(),
        },
        purpose: purpose.into(),
        archived: false,
        fiction,
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            schema_version: REPO_SCHEMA_VERSION,
            branch: default_branch(),
            vaults: vec![
                vault(
                    "life",
                    "Life",
                    "زندگی",
                    "Journal, people, places, goals, concerns, ideas.",
                    false,
                ),
                vault(
                    "health",
                    "Health",
                    "سلامت",
                    "Medical profile: conditions, medications, labs, visits, symptoms.",
                    false,
                ),
                vault(
                    "mind",
                    "Mind",
                    "ذهن",
                    "Psychological self-model: moods, patterns, values.",
                    false,
                ),
                vault(
                    "work",
                    "Work",
                    "کار",
                    "Projects, learning, professional notes.",
                    false,
                ),
                vault(
                    "stories",
                    "Stories",
                    "داستان‌ها",
                    "Fiction. Each story is an isolated workspace.",
                    true,
                ),
            ],
            ai: Default::default(),
            routing_threshold: default_threshold(),
            extra: Map::new(),
        }
    }
}

impl Config {
    pub fn active_vaults(&self) -> impl Iterator<Item = &VaultConfig> {
        self.vaults.iter().filter(|v| !v.archived)
    }

    pub fn vault(&self, id: &str) -> Option<&VaultConfig> {
        self.vaults.iter().find(|v| v.id == id)
    }

    pub fn to_pretty_json(&self) -> String {
        let mut s = serde_json::to_string_pretty(self).expect("config serialises");
        s.push('\n');
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_fields_survive_round_trip() {
        let mut v: Value = serde_json::to_value(Config::default()).unwrap();
        v["future_setting"] = Value::from(42);
        let c: Config = serde_json::from_value(v).unwrap();
        assert_eq!(c.extra["future_setting"], 42);
        assert!(c.to_pretty_json().contains("future_setting"));
    }
}
