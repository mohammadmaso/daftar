//! Resolves model roles to live providers (§6.3, §9). Secrets are passed in by the app from
//! platform secure storage for each run and never stored by the core.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::providers::{self, AiConfig, DynProvider, ProviderError, ProviderErrorKind, Role, RoleConfig};

pub struct AiRuntime {
    pub config: AiConfig,
    secrets: HashMap<String, String>,
    cache: Mutex<HashMap<String, DynProvider>>,
}

impl AiRuntime {
    pub fn new(config: AiConfig, secrets: HashMap<String, String>) -> Self {
        Self { config, secrets, cache: Mutex::new(HashMap::new()) }
    }

    /// Test/eval hook: use `provider` for provider id `id` regardless of its kind.
    pub fn with_provider(self, id: &str, provider: DynProvider) -> Self {
        self.cache.lock().expect("lock").insert(id.to_owned(), provider);
        self
    }

    pub fn has(&self, role: Role) -> bool {
        self.config.role(role).is_some_and(|r| self.config.provider(&r.provider).is_some())
    }

    pub fn for_role(&self, role: Role) -> Result<(DynProvider, RoleConfig), ProviderError> {
        let rc = self.config.role(role).cloned().ok_or_else(|| {
            ProviderError::new(ProviderErrorKind::NotConfigured, format!("No model is set up for {}. Choose one in Settings › Models.", role.as_str()))
        })?;
        let mut cache = self.cache.lock().expect("lock");
        if let Some(p) = cache.get(&rc.provider) {
            return Ok((p.clone(), rc));
        }
        let pc = self.config.provider(&rc.provider).ok_or_else(|| ProviderError::new(ProviderErrorKind::NotConfigured, format!("The provider for {} was removed. Choose another in Settings › Models.", role.as_str())))?;
        let p = providers::build(pc, self.secrets.get(&pc.id).cloned())?;
        cache.insert(pc.id.clone(), p.clone());
        Ok((p, rc))
    }
}
