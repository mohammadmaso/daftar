//! `.daftar/config.json`: non-secret settings shared by all devices. Unknown fields are preserved
//! so older app versions never drop settings written by newer ones.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::{Error, REPO_SCHEMA_VERSION, Result};

/// The vault that holds the journal, weekly reviews and anything no other vault fits.
pub const JOURNAL_VAULT: &str = "life";
/// The vault that holds fiction, one folder per story, isolated from facts about the user.
pub const FICTION_VAULT: &str = "stories";

const BUILTIN_VAULT: &str = "Life and Stories can be renamed but not removed.";

/// Vaults the code gives a role to. They can be renamed and re-described but never archived or
/// removed; every other vault, the default ones included, is the user's to change.
pub fn is_builtin_vault(id: &str) -> bool {
    id == JOURNAL_VAULT || id == FICTION_VAULT
}

fn clean_title(t: Bilingual) -> Result<Bilingual> {
    let en = t.en.trim().to_owned();
    let fa = t.fa.trim().to_owned();
    if en.is_empty() && fa.is_empty() {
        return Err(Error::invalid("A vault needs a name."));
    }
    // One language is enough; the other falls back to it.
    Ok(Bilingual {
        en: if en.is_empty() {
            fa.clone()
        } else {
            en.clone()
        },
        fa: if fa.is_empty() { en } else { fa },
    })
}

fn clean_purpose(p: &str) -> Result<String> {
    let p = p.split_whitespace().collect::<Vec<_>>().join(" ");
    if p.is_empty() {
        return Err(Error::invalid(
            "Say what belongs in the vault; the assistant files by it.",
        ));
    }
    Ok(p)
}

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
    /// MCP servers (§10); credentials are per device and never stored here.
    #[serde(default)]
    pub mcp: Vec<crate::mcp::McpServerConfig>,
    #[serde(default)]
    pub reflect: crate::reflect::ReflectSettings,
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
                    "Medical profile: conditions, medications, labs, visits, symptoms, sleep, doctors, test results.",
                    false,
                ),
                vault(
                    "mind",
                    "Mind",
                    "ذهن",
                    "Psychological self-model: moods, emotional states, recurring thoughts, patterns, values.",
                    false,
                ),
                vault(
                    "work",
                    "Work",
                    "کار",
                    "Projects, learning, professional matters.",
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
            mcp: vec![],
            reflect: Default::default(),
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

    /// A vault that exists and is not archived: the only kind the assistant may write into.
    pub fn active_vault(&self, id: &str) -> Option<&VaultConfig> {
        self.vault(id).filter(|v| !v.archived)
    }

    /// The current vaults as the assistant sees them in every prompt. This list, not the vault
    /// table in SCHEMA.md, is authoritative: the user adds, renames and archives vaults.
    pub fn vaults_for_prompt(&self) -> String {
        self.active_vaults()
            .map(|v| {
                let mut line = format!(
                    "- `{}` ({} · {}): {}",
                    v.id, v.title.en, v.title.fa, v.purpose
                );
                if v.fiction {
                    line.push_str(" [fiction: never about the user]");
                }
                line
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Adds a vault made by the user and returns its id: a slug of the English title, unique
    /// among all vaults (archived ones included, so an archived vault's folder is never reused).
    pub fn add_vault(&mut self, title: Bilingual, purpose: &str) -> Result<String> {
        let title = clean_title(title)?;
        let purpose = clean_purpose(purpose)?;
        let mut base: String = crate::ids::kebab_slug(&title.en).chars().take(32).collect();
        while base.ends_with('-') {
            base.pop();
        }
        if base.is_empty() {
            base = "vault".into();
        }
        let mut id = base.clone();
        let mut n = 2;
        while self.vault(&id).is_some() {
            id = format!("{base}-{n}");
            n += 1;
        }
        self.vaults.push(VaultConfig {
            id: id.clone(),
            title,
            purpose,
            archived: false,
            fiction: false,
        });
        Ok(id)
    }

    /// Renames a vault or changes what belongs in it. The id (its folder) never changes, so
    /// links into the vault stay valid.
    pub fn edit_vault(&mut self, id: &str, title: Bilingual, purpose: &str) -> Result<()> {
        let title = clean_title(title)?;
        let purpose = clean_purpose(purpose)?;
        let v = self.vault_mut(id)?;
        v.title = title;
        v.purpose = purpose;
        Ok(())
    }

    /// Archiving hides a vault from the app and the assistant; its pages stay in the repo.
    pub fn set_vault_archived(&mut self, id: &str, archived: bool) -> Result<()> {
        if archived && is_builtin_vault(id) {
            return Err(Error::invalid(BUILTIN_VAULT));
        }
        self.vault_mut(id)?.archived = archived;
        Ok(())
    }

    /// Drops a vault from the list. The caller has checked that it holds no pages.
    pub fn remove_vault(&mut self, id: &str) -> Result<()> {
        if is_builtin_vault(id) {
            return Err(Error::invalid(BUILTIN_VAULT));
        }
        self.vault_mut(id)?;
        self.vaults.retain(|v| v.id != id);
        Ok(())
    }

    fn vault_mut(&mut self, id: &str) -> Result<&mut VaultConfig> {
        self.vaults
            .iter_mut()
            .find(|v| v.id == id)
            .ok_or_else(|| Error::invalid(format!("There is no vault called '{id}'.")))
    }

    pub fn to_pretty_json(&self) -> String {
        let mut s = serde_json::to_string_pretty(self).expect("config serialises");
        s.push('\n');
        s
    }
}

/// Three-way merge of two edits of `config.json` (two devices changed settings while apart).
/// Objects merge key by key; lists of objects with an `id` or `role` merge element by element;
/// when both sides changed the same value, `local` wins. Never produces conflict markers.
pub fn merge_json(base: Option<&Value>, remote: &Value, local: &Value) -> Value {
    if remote == local {
        return remote.clone();
    }
    match base {
        Some(b) if b == remote => return local.clone(),
        Some(b) if b == local => return remote.clone(),
        _ => {}
    }
    match (remote, local) {
        (Value::Object(r), Value::Object(l)) => {
            let b = base.and_then(Value::as_object);
            let mut out = Map::new();
            for (k, rv) in r {
                let bv = b.and_then(|b| b.get(k));
                match l.get(k) {
                    Some(lv) => {
                        out.insert(k.clone(), merge_json(bv, rv, lv));
                    }
                    // Deleted locally; keep it only if the remote changed it meanwhile.
                    None if bv.is_some_and(|bv| bv != rv) => {
                        out.insert(k.clone(), rv.clone());
                    }
                    None if bv.is_none() => {
                        out.insert(k.clone(), rv.clone());
                    }
                    None => {}
                }
            }
            for (k, lv) in l {
                if r.contains_key(k) {
                    continue;
                }
                let bv = b.and_then(|b| b.get(k));
                if bv.is_none_or(|bv| bv != lv) {
                    out.insert(k.clone(), lv.clone());
                }
            }
            Value::Object(out)
        }
        (Value::Array(r), Value::Array(l)) if keyed(r).is_some() && keyed(r) == keyed(l) => {
            let key = keyed(r).expect("checked");
            let id = |v: &Value| v.get(key).cloned();
            let b: Vec<Value> = base.and_then(Value::as_array).cloned().unwrap_or_default();
            let find = |list: &[Value], k: &Value| {
                list.iter().find(|x| id(x).as_ref() == Some(k)).cloned()
            };
            let mut out = Vec::new();
            for rv in r {
                let k = id(rv).expect("keyed");
                let bv = find(&b, &k);
                match find(l, &k) {
                    Some(lv) => out.push(merge_json(bv.as_ref(), rv, &lv)),
                    None if bv.as_ref().is_some_and(|bv| bv != rv) || bv.is_none() => {
                        out.push(rv.clone())
                    }
                    None => {}
                }
            }
            for lv in l {
                let k = id(lv).expect("keyed");
                if find(r, &k).is_some() {
                    continue;
                }
                let bv = find(&b, &k);
                if bv.as_ref().is_none_or(|bv| bv != lv) {
                    out.push(lv.clone());
                }
            }
            Value::Array(out)
        }
        _ => local.clone(),
    }
}

/// The identifying field shared by every element of a list of objects, if any.
fn keyed(list: &[Value]) -> Option<&'static str> {
    ["id", "role"]
        .into_iter()
        .find(|k| !list.is_empty() && list.iter().all(|v| v.get(*k).is_some_and(Value::is_string)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concurrent_settings_edits_merge() {
        let base = serde_json::json!({
            "schema_version": 1,
            "ai": {
                "providers": [{"id": "p1", "name": "OpenRouter"}],
                "roles": [{"role": "chat", "provider": "p1", "model": "a"}]
            },
            "routing_threshold": 0.6
        });
        // Phone renames the provider and adds an STT role; laptop adds a provider and changes
        // the chat model and the threshold.
        let remote = serde_json::json!({
            "schema_version": 1,
            "ai": {
                "providers": [{"id": "p1", "name": "My OpenRouter"}],
                "roles": [
                    {"role": "chat", "provider": "p1", "model": "a"},
                    {"role": "stt", "provider": "p1", "model": "whisper"}
                ]
            },
            "routing_threshold": 0.6
        });
        let local = serde_json::json!({
            "schema_version": 1,
            "ai": {
                "providers": [{"id": "p1", "name": "OpenRouter"}, {"id": "p2", "name": "Anthropic"}],
                "roles": [{"role": "chat", "provider": "p1", "model": "b"}]
            },
            "routing_threshold": 0.7
        });
        let m = merge_json(Some(&base), &remote, &local);
        assert_eq!(m["ai"]["providers"][0]["name"], "My OpenRouter");
        assert_eq!(m["ai"]["providers"][1]["id"], "p2");
        assert_eq!(m["ai"]["roles"][0]["model"], "b");
        assert_eq!(m["ai"]["roles"][1]["role"], "stt");
        assert_eq!(m["routing_threshold"], 0.7);
        let _: Config = serde_json::from_value(serde_json::json!({
            "schema_version": 1, "vaults": []
        }))
        .unwrap();
    }

    #[test]
    fn deletions_survive_unless_changed_on_the_other_side() {
        let base =
            serde_json::json!({"roles": [{"role": "chat", "m": 1}, {"role": "stt", "m": 1}]});
        let remote = serde_json::json!({"roles": [{"role": "chat", "m": 1}]});
        let local =
            serde_json::json!({"roles": [{"role": "chat", "m": 1}, {"role": "stt", "m": 2}]});
        let m = merge_json(Some(&base), &remote, &local);
        assert_eq!(
            m["roles"].as_array().unwrap().len(),
            2,
            "local edit wins over remote delete"
        );
        let local2 =
            serde_json::json!({"roles": [{"role": "chat", "m": 1}, {"role": "stt", "m": 1}]});
        let m = merge_json(Some(&base), &remote, &local2);
        assert_eq!(m["roles"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn unknown_fields_survive_round_trip() {
        let mut v: Value = serde_json::to_value(Config::default()).unwrap();
        v["future_setting"] = Value::from(42);
        let c: Config = serde_json::from_value(v).unwrap();
        assert_eq!(c.extra["future_setting"], 42);
        assert!(c.to_pretty_json().contains("future_setting"));
    }
}
