//! MCP servers for the Flutter app (§10): configs sync in the repository without secrets; each
//! device keeps its own credentials (as a JSON string in secure storage) and connects once.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use daftar_core::mcp::{
    self, ApprovalRequest, Approver, McpAuth, McpError, McpSecrets, McpServerConfig, McpTransport,
    ToolPolicy,
};
use flutter_rust_bridge::frb;
use tokio::sync::oneshot;

use super::library::LibraryHandle;

pub enum McpTransportKind {
    StreamableHttp,
    Sse,
    Stdio,
}

pub enum McpAuthKind {
    None,
    Bearer,
    ApiKeyHeader,
    ApiKeyQuery,
    Headers,
    OAuth,
    ClientCredentials,
}

pub enum McpPolicy {
    AskEveryTime,
    AutoReadOnly,
    AlwaysAllow,
}

pub struct McpServer {
    /// Empty when adding.
    pub id: String,
    pub name: String,
    pub transport: McpTransportKind,
    /// URL for HTTP transports, the command for stdio.
    pub target: String,
    pub args: Vec<String>,
    pub env_names: Vec<String>,
    pub auth: McpAuthKind,
    /// Header or query parameter name (API key), header names (custom headers).
    pub auth_names: Vec<String>,
    /// OAuth / client credentials client id; empty = dynamic registration.
    pub client_id: String,
    pub scopes: Vec<String>,
    pub policy: McpPolicy,
    pub enabled: bool,
}

pub enum McpStatusKind {
    Connected,
    NeedsAuth,
    DesktopOnly,
    Error,
    Disabled,
}

pub struct McpToolInfo {
    pub name: String,
    pub description: String,
    pub read_only: bool,
}

pub struct McpStatus {
    pub kind: McpStatusKind,
    pub message: Option<String>,
    pub tools: Vec<McpToolInfo>,
    /// Refreshed credentials to write back to secure storage (JSON), if they changed.
    pub updated_secrets: Option<String>,
}

pub struct OAuthStart {
    pub flow_id: String,
    pub auth_url: String,
}

fn err(e: impl std::fmt::Display) -> anyhow::Error {
    anyhow::anyhow!(e.to_string())
}

fn to_config(s: McpServer) -> McpServerConfig {
    let first = |v: &[String]| v.first().cloned().unwrap_or_default();
    McpServerConfig {
        id: s.id,
        name: s.name.trim().to_owned(),
        transport: match s.transport {
            McpTransportKind::StreamableHttp => McpTransport::StreamableHttp {
                url: s.target.trim().into(),
            },
            McpTransportKind::Sse => McpTransport::Sse {
                url: s.target.trim().into(),
            },
            McpTransportKind::Stdio => McpTransport::Stdio {
                command: s.target.trim().into(),
                args: s.args,
                env_names: s.env_names,
                cwd: None,
            },
        },
        auth: match s.auth {
            McpAuthKind::None => McpAuth::None,
            McpAuthKind::Bearer => McpAuth::Bearer,
            McpAuthKind::ApiKeyHeader => McpAuth::ApiKeyHeader {
                header: first(&s.auth_names),
            },
            McpAuthKind::ApiKeyQuery => McpAuth::ApiKeyQuery {
                param: first(&s.auth_names),
            },
            McpAuthKind::Headers => McpAuth::Headers {
                names: s.auth_names,
            },
            McpAuthKind::OAuth => McpAuth::OAuth {
                client_id: Some(s.client_id).filter(|c| !c.trim().is_empty()),
                scopes: s.scopes,
            },
            McpAuthKind::ClientCredentials => McpAuth::ClientCredentials {
                client_id: s.client_id,
                scopes: s.scopes,
            },
        },
        policy: match s.policy {
            McpPolicy::AskEveryTime => ToolPolicy::AskEveryTime,
            McpPolicy::AutoReadOnly => ToolPolicy::AutoReadOnly,
            McpPolicy::AlwaysAllow => ToolPolicy::AlwaysAllow,
        },
        enabled: s.enabled,
    }
}

fn from_config(c: &McpServerConfig) -> McpServer {
    let (transport, target, args, env_names) = match &c.transport {
        McpTransport::StreamableHttp { url } => (
            McpTransportKind::StreamableHttp,
            url.clone(),
            vec![],
            vec![],
        ),
        McpTransport::Sse { url } => (McpTransportKind::Sse, url.clone(), vec![], vec![]),
        McpTransport::Stdio {
            command,
            args,
            env_names,
            ..
        } => (
            McpTransportKind::Stdio,
            command.clone(),
            args.clone(),
            env_names.clone(),
        ),
    };
    let (auth, auth_names, client_id, scopes) = match &c.auth {
        McpAuth::None => (McpAuthKind::None, vec![], String::new(), vec![]),
        McpAuth::Bearer => (McpAuthKind::Bearer, vec![], String::new(), vec![]),
        McpAuth::ApiKeyHeader { header } => (
            McpAuthKind::ApiKeyHeader,
            vec![header.clone()],
            String::new(),
            vec![],
        ),
        McpAuth::ApiKeyQuery { param } => (
            McpAuthKind::ApiKeyQuery,
            vec![param.clone()],
            String::new(),
            vec![],
        ),
        McpAuth::Headers { names } => (McpAuthKind::Headers, names.clone(), String::new(), vec![]),
        McpAuth::OAuth { client_id, scopes } => (
            McpAuthKind::OAuth,
            vec![],
            client_id.clone().unwrap_or_default(),
            scopes.clone(),
        ),
        McpAuth::ClientCredentials { client_id, scopes } => (
            McpAuthKind::ClientCredentials,
            vec![],
            client_id.clone(),
            scopes.clone(),
        ),
    };
    McpServer {
        id: c.id.clone(),
        name: c.name.clone(),
        transport,
        target,
        args,
        env_names,
        auth,
        auth_names,
        client_id,
        scopes,
        policy: match c.policy {
            ToolPolicy::AskEveryTime => McpPolicy::AskEveryTime,
            ToolPolicy::AutoReadOnly => McpPolicy::AutoReadOnly,
            ToolPolicy::AlwaysAllow => McpPolicy::AlwaysAllow,
        },
        enabled: c.enabled,
    }
}

pub(crate) fn secrets_of(json: &str) -> McpSecrets {
    serde_json::from_str(json).unwrap_or_default()
}

/// Whether local (stdio) servers can run on this device.
#[frb(sync)]
pub fn mcp_stdio_supported() -> bool {
    mcp::stdio_supported()
}

// ─────────────── OAuth flows in progress ───────────────

struct Flow {
    flow: Option<mcp::OAuthFlow>,
    loopback: Option<oneshot::Receiver<String>>,
    secrets: McpSecrets,
}

fn flows() -> &'static Mutex<HashMap<String, Flow>> {
    static F: OnceLock<Mutex<HashMap<String, Flow>>> = OnceLock::new();
    F.get_or_init(Default::default)
}

// ─────────────── tool approvals (Ask, voice) ───────────────

pub(crate) struct AppApprover {
    pub(crate) on_request: Box<dyn Fn(String, ApprovalRequest) + Send + Sync>,
}

fn pending() -> &'static Mutex<HashMap<String, oneshot::Sender<bool>>> {
    static P: OnceLock<Mutex<HashMap<String, oneshot::Sender<bool>>>> = OnceLock::new();
    P.get_or_init(Default::default)
}

#[async_trait::async_trait]
impl Approver for AppApprover {
    async fn approve(&self, req: ApprovalRequest) -> bool {
        let id = daftar_core::ids::new_id().to_string();
        let (tx, rx) = oneshot::channel();
        pending()
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .insert(id.clone(), tx);
        (self.on_request)(id, req);
        // No answer within ten minutes counts as "no".
        tokio::time::timeout(std::time::Duration::from_secs(600), rx)
            .await
            .ok()
            .and_then(Result::ok)
            .unwrap_or(false)
    }
}

/// The user's answer to a tool approval card.
#[frb(sync)]
pub fn answer_tool_approval(request_id: String, allowed: bool) {
    if let Some(tx) = pending()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .remove(&request_id)
    {
        let _ = tx.send(allowed);
    }
}

/// Connects every enabled server that can run here, for one Ask or voice session.
pub(crate) async fn toolset(
    lib: &daftar_core::library::Library,
    secrets: &HashMap<String, String>,
    approver: Arc<dyn Approver>,
) -> Option<Arc<dyn daftar_core::agent::ExternalTools>> {
    let servers = lib.config().ok()?.mcp;
    let mut conns = Vec::new();
    for s in servers.iter().filter(|s| s.enabled) {
        let sec = secrets
            .get(&s.id)
            .map(|j| secrets_of(j))
            .unwrap_or_default();
        match tokio::time::timeout(std::time::Duration::from_secs(10), mcp::connect(s, &sec)).await
        {
            Ok(Ok(c)) => conns.push(Arc::new(c)),
            Ok(Err(e)) => tracing::info!("MCP {} not available: {e}", s.name),
            Err(_) => tracing::info!("MCP {} timed out", s.name),
        }
    }
    if conns.is_empty() {
        return None;
    }
    Some(Arc::new(mcp::McpToolset {
        connections: conns,
        approver,
    }))
}

impl LibraryHandle {
    pub fn mcp_servers(&self) -> anyhow::Result<Vec<McpServer>> {
        Ok(self
            .session()
            .library()
            .config()
            .map_err(err)?
            .mcp
            .iter()
            .map(from_config)
            .collect())
    }

    /// Adds or updates a server; returns its id.
    pub fn save_mcp_server(&self, server: McpServer) -> anyhow::Result<String> {
        let mut cfg = to_config(server);
        if cfg.name.is_empty() {
            return Err(anyhow::anyhow!("Give the server a name."));
        }
        if cfg.id.is_empty() {
            cfg.id = daftar_core::wiki::sanitize_slug(&cfg.name);
        }
        let id = cfg.id.clone();
        self.session()
            .library()
            .update_config(|c| match c.mcp.iter_mut().find(|m| m.id == cfg.id) {
                Some(m) => *m = cfg,
                None => c.mcp.push(cfg),
            })
            .map_err(err)?;
        Ok(id)
    }

    pub fn remove_mcp_server(&self, id: String) -> anyhow::Result<()> {
        self.session()
            .library()
            .update_config(|c| c.mcp.retain(|m| m.id != id))
            .map_err(err)
    }

    /// Connects once to report status and tools (§10 server list).
    pub async fn mcp_check(&self, id: String, secrets_json: String) -> anyhow::Result<McpStatus> {
        let cfg = self
            .session()
            .library()
            .config()
            .map_err(err)?
            .mcp
            .into_iter()
            .find(|m| m.id == id)
            .ok_or_else(|| anyhow::anyhow!("That server was removed."))?;
        if !cfg.enabled {
            return Ok(McpStatus {
                kind: McpStatusKind::Disabled,
                message: None,
                tools: vec![],
                updated_secrets: None,
            });
        }
        let mut secrets = secrets_of(&secrets_json);
        let res = tokio::time::timeout(
            std::time::Duration::from_secs(20),
            mcp::connect(&cfg, &secrets),
        )
        .await;
        Ok(match res {
            Err(_) => McpStatus {
                kind: McpStatusKind::Error,
                message: Some("The server did not answer in time.".into()),
                tools: vec![],
                updated_secrets: None,
            },
            Ok(Err(McpError::NeedsAuth)) => McpStatus {
                kind: McpStatusKind::NeedsAuth,
                message: None,
                tools: vec![],
                updated_secrets: None,
            },
            Ok(Err(McpError::DesktopOnly)) => McpStatus {
                kind: McpStatusKind::DesktopOnly,
                message: None,
                tools: vec![],
                updated_secrets: None,
            },
            Ok(Err(McpError::Failed(m))) => McpStatus {
                kind: McpStatusKind::Error,
                message: Some(m),
                tools: vec![],
                updated_secrets: None,
            },
            Ok(Ok(conn)) => {
                let tools = conn
                    .tools
                    .iter()
                    .map(|t| McpToolInfo {
                        name: t.name.clone(),
                        description: t.description.clone(),
                        read_only: t.read_only,
                    })
                    .collect();
                let updated = conn
                    .credentials
                    .as_ref()
                    .and_then(|c| c.to_json())
                    .filter(|c| Some(c) != secrets.oauth.as_ref());
                let updated_secrets = updated.map(|c| {
                    secrets.oauth = Some(c);
                    serde_json::to_string(&secrets).unwrap_or_default()
                });
                conn.close().await;
                McpStatus {
                    kind: McpStatusKind::Connected,
                    message: None,
                    tools,
                    updated_secrets,
                }
            }
        })
    }

    /// Starts OAuth (§10). With no `redirect_uri` (desktop) a loopback listener on 127.0.0.1 is
    /// used; call `mcp_oauth_wait`. On mobile pass `daftar://oauth/callback` and hand the redirect
    /// to `mcp_oauth_complete`. The app opens `auth_url` in the system browser.
    pub async fn mcp_oauth_begin(
        &self,
        id: String,
        secrets_json: String,
        redirect_uri: Option<String>,
    ) -> anyhow::Result<OAuthStart> {
        let cfg = self
            .session()
            .library()
            .config()
            .map_err(err)?
            .mcp
            .into_iter()
            .find(|m| m.id == id)
            .ok_or_else(|| anyhow::anyhow!("That server was removed."))?;
        let secrets = secrets_of(&secrets_json);
        let (redirect, loopback) = match redirect_uri {
            Some(r) => (r, None),
            None => {
                let (r, rx) = mcp::loopback_redirect().await.map_err(err)?;
                (r, Some(rx))
            }
        };
        let flow = mcp::oauth_begin(&cfg, &secrets, &redirect)
            .await
            .map_err(err)?;
        let flow_id = daftar_core::ids::new_id().to_string();
        let auth_url = flow.auth_url.clone();
        flows().lock().unwrap_or_else(|p| p.into_inner()).insert(
            flow_id.clone(),
            Flow {
                flow: Some(flow),
                loopback,
                secrets,
            },
        );
        Ok(OAuthStart { flow_id, auth_url })
    }

    /// Desktop: waits for the browser to come back to the loopback listener. Returns the new
    /// secrets JSON for this server.
    pub async fn mcp_oauth_wait(&self, flow_id: String) -> anyhow::Result<String> {
        let rx = flows()
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get_mut(&flow_id)
            .and_then(|f| f.loopback.take())
            .ok_or_else(|| anyhow::anyhow!("This sign-in has expired; start again."))?;
        let url = rx
            .await
            .map_err(|_| anyhow::anyhow!("The sign-in was not completed."))?;
        self.mcp_oauth_complete(flow_id, url).await
    }

    /// Finishes OAuth with the redirect URL; returns the new secrets JSON for this server.
    pub async fn mcp_oauth_complete(
        &self,
        flow_id: String,
        callback_url: String,
    ) -> anyhow::Result<String> {
        let Some(mut f) = flows()
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .remove(&flow_id)
        else {
            return Err(anyhow::anyhow!("This sign-in has expired; start again."));
        };
        let flow = f
            .flow
            .take()
            .ok_or_else(|| anyhow::anyhow!("This sign-in has expired; start again."))?;
        let creds = flow.complete(&callback_url).await.map_err(err)?;
        f.secrets.oauth = Some(creds);
        Ok(serde_json::to_string(&f.secrets)?)
    }
}
