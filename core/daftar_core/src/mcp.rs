//! MCP client (§10): servers are configured in the shared `config.json` without secrets; each device
//! supplies its own credentials (from platform secure storage) and authenticates once.
//!
//! Transports: Streamable HTTP (rmcp), legacy HTTP+SSE (below; rmcp dropped its client), and stdio
//! on desktop. Auth: none, bearer, API key in a header or query parameter, custom headers, OAuth 2.1
//! authorization code + PKCE with discovery and dynamic registration (rmcp's `auth`), and client
//! credentials. All HTTP goes through `tls::http_client` (one trust store, proxy rules; ADR-0015).

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rmcp::model::{ClientJsonRpcMessage, ServerJsonRpcMessage};
use rmcp::service::{RoleClient, RunningService, serve_client};
use rmcp::transport::Transport;
use rmcp::transport::auth::{
    AuthClient, AuthError, AuthorizationManager, AuthorizationRequest, AuthorizationSession,
    ClientCredentialsConfig, CredentialStore, OAuthHttpClient, OAuthHttpClientFuture,
    OAuthHttpRequest, StoredCredentials,
};
use rmcp::transport::streamable_http_client::{
    StreamableHttpClientTransport, StreamableHttpClientTransportConfig,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::providers::ToolSpec;

// ─────────────────────────── configuration (synced, no secrets) ───────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum McpTransport {
    StreamableHttp {
        url: String,
    },
    /// Legacy HTTP+SSE (2024-11-05 servers).
    Sse {
        url: String,
    },
    /// Desktop only. Environment variable *values* are per-device secrets.
    Stdio {
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        env_names: Vec<String>,
        #[serde(default)]
        cwd: Option<String>,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum McpAuth {
    #[default]
    None,
    /// `Authorization: Bearer <token>`.
    Bearer,
    ApiKeyHeader {
        header: String,
    },
    ApiKeyQuery {
        param: String,
    },
    /// Arbitrary headers; the values are secrets.
    Headers {
        names: Vec<String>,
    },
    /// Authorization code + PKCE (§10). Without `client_id`, dynamic client registration is used.
    #[serde(rename = "oauth")]
    OAuth {
        #[serde(default)]
        client_id: Option<String>,
        #[serde(default)]
        scopes: Vec<String>,
    },
    ClientCredentials {
        client_id: String,
        #[serde(default)]
        scopes: Vec<String>,
    },
}

/// When a tool may run without asking (§10).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolPolicy {
    AskEveryTime,
    /// Tools annotated read-only run; everything else asks.
    #[default]
    AutoReadOnly,
    AlwaysAllow,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub id: String,
    pub name: String,
    pub transport: McpTransport,
    #[serde(default)]
    pub auth: McpAuth,
    #[serde(default)]
    pub policy: ToolPolicy,
    #[serde(default = "enabled")]
    pub enabled: bool,
}

fn enabled() -> bool {
    true
}

/// Per-device credentials for one server, kept in platform secure storage by the app.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct McpSecrets {
    /// Bearer token or API key.
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub client_secret: Option<String>,
    /// OAuth credentials (tokens) as saved after authorization or refresh.
    #[serde(default)]
    pub oauth: Option<Value>,
}

pub fn stdio_supported() -> bool {
    cfg!(not(any(target_os = "android", target_os = "ios")))
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum McpError {
    /// The user has to connect this server on this device (§10 per-device auth card).
    #[error("Connect this server on this device first.")]
    NeedsAuth,
    #[error("This server runs a local program, which only works on desktop.")]
    DesktopOnly,
    #[error("{0}")]
    Failed(String),
}

fn failed(e: impl std::fmt::Display) -> McpError {
    McpError::Failed(e.to_string())
}

// ─────────────────────────── HTTP plumbing ───────────────────────────

/// rmcp's OAuth state machine over our HTTP client.
struct OurOAuthHttp {
    follow: reqwest::Client,
    stop: reqwest::Client,
}

impl OurOAuthHttp {
    fn new() -> Self {
        let follow = crate::tls::http_client(Duration::from_secs(30));
        let stop = reqwest::Client::builder()
            .no_proxy()
            .proxy(reqwest::Proxy::custom(|url| {
                crate::tls::env_proxy_for(url).and_then(|p| reqwest::Url::parse(&p).ok())
            }))
            .use_preconfigured_tls((*crate::tls::client_config()).clone())
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(30))
            .build()
            .expect("static client configuration is valid");
        Self { follow, stop }
    }
}

impl OAuthHttpClient for OurOAuthHttp {
    fn execute(&self, request: OAuthHttpRequest) -> OAuthHttpClientFuture<'_> {
        Box::pin(async move {
            let follow = matches!(
                request.redirect_policy,
                rmcp::transport::auth::OAuthHttpRedirectPolicy::Follow
            );
            let client = if follow { &self.follow } else { &self.stop };
            let req = reqwest::Request::try_from(request.request)?;
            let resp = client.execute(req).await?;
            let mut b = http::Response::builder()
                .status(resp.status())
                .version(resp.version());
            for (k, v) in resp.headers() {
                b = b.header(k, v);
            }
            let body = resp.bytes().await?;
            if body.len() > 1024 * 1024 {
                return Err("OAuth response too large".into());
            }
            Ok(b.body(body.to_vec())?)
        })
    }
}

/// Credentials shared between rmcp's manager and the app (which persists them after refresh).
#[derive(Clone, Default)]
pub struct SharedCredentials(Arc<Mutex<Option<StoredCredentials>>>);

impl SharedCredentials {
    pub fn from_json(v: Option<&Value>) -> Self {
        let c = v.and_then(|v| serde_json::from_value(v.clone()).ok());
        Self(Arc::new(Mutex::new(c)))
    }
    pub fn to_json(&self) -> Option<Value> {
        self.0
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .as_ref()
            .and_then(|c| serde_json::to_value(c).ok())
    }
    fn has_token(&self) -> bool {
        self.0
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .as_ref()
            .is_some_and(|c| c.token_response.is_some())
    }
}

#[async_trait::async_trait]
impl CredentialStore for SharedCredentials {
    async fn load(&self) -> Result<Option<StoredCredentials>, AuthError> {
        Ok(self.0.lock().unwrap_or_else(|p| p.into_inner()).clone())
    }
    async fn save(&self, c: StoredCredentials) -> Result<(), AuthError> {
        *self.0.lock().unwrap_or_else(|p| p.into_inner()) = Some(c);
        Ok(())
    }
    async fn clear(&self) -> Result<(), AuthError> {
        *self.0.lock().unwrap_or_else(|p| p.into_inner()) = None;
        Ok(())
    }
}

fn server_url(cfg: &McpServerConfig) -> Result<&str, McpError> {
    match &cfg.transport {
        McpTransport::StreamableHttp { url } | McpTransport::Sse { url } => {
            let u = url::Url::parse(url).map_err(|_| failed("The server address is not a URL."))?;
            let loopback = matches!(
                u.host_str(),
                Some("127.0.0.1" | "localhost" | "[::1]" | "::1")
            );
            if u.scheme() != "https" && !loopback {
                return Err(failed(
                    "MCP servers must use https (http only on this computer).",
                ));
            }
            Ok(url)
        }
        McpTransport::Stdio { .. } => Err(failed("A local program has no web address.")),
    }
}

async fn auth_manager(
    url: &str,
    creds: SharedCredentials,
) -> Result<AuthorizationManager, McpError> {
    let mut m =
        AuthorizationManager::new_with_oauth_http_client(url, Arc::new(OurOAuthHttp::new()))
            .await
            .map_err(failed)?;
    m.set_credential_store(creds);
    let meta = m.resolve_metadata().await.map_err(failed)?;
    m.set_metadata(meta.metadata);
    Ok(m)
}

/// (URL with any API-key parameter, bearer token, extra headers).
type StaticAuth = (String, Option<String>, BTreeMap<String, String>);

/// Static headers for the non-OAuth schemes, and the URL with an API key parameter if needed.
fn static_auth(
    cfg: &McpServerConfig,
    secrets: &McpSecrets,
    url: &str,
) -> Result<StaticAuth, McpError> {
    let token = || {
        secrets
            .token
            .clone()
            .filter(|t| !t.is_empty())
            .ok_or(McpError::NeedsAuth)
    };
    let mut headers = BTreeMap::new();
    let mut bearer = None;
    let mut url = url.to_owned();
    match &cfg.auth {
        McpAuth::None | McpAuth::OAuth { .. } | McpAuth::ClientCredentials { .. } => {}
        McpAuth::Bearer => bearer = Some(token()?),
        McpAuth::ApiKeyHeader { header } => {
            headers.insert(header.clone(), token()?);
        }
        McpAuth::ApiKeyQuery { param } => {
            let mut u = url::Url::parse(&url).map_err(failed)?;
            u.query_pairs_mut().append_pair(param, &token()?);
            url = u.to_string();
        }
        McpAuth::Headers { names } => {
            for n in names {
                let v = secrets.headers.get(n).ok_or(McpError::NeedsAuth)?;
                headers.insert(n.clone(), v.clone());
            }
        }
    }
    Ok((url, bearer, headers))
}

// ─────────────────────────── OAuth authorization (§10) ───────────────────────────

/// An authorization in progress: the app opens `auth_url` in the system browser and hands the
/// redirect back to `complete`.
pub struct OAuthFlow {
    session: AuthorizationSession,
    creds: SharedCredentials,
    pub auth_url: String,
}

pub async fn oauth_begin(
    cfg: &McpServerConfig,
    secrets: &McpSecrets,
    redirect_uri: &str,
) -> Result<OAuthFlow, McpError> {
    let McpAuth::OAuth { client_id, scopes } = &cfg.auth else {
        return Err(failed("This server does not use OAuth."));
    };
    let url = server_url(cfg)?;
    let creds = SharedCredentials::default();
    let mgr = auth_manager(url, creds.clone()).await?;
    let mut req = AuthorizationRequest::new(redirect_uri).with_client_name(crate::APP_NAME);
    if !scopes.is_empty() {
        req = req.with_scopes(scopes.iter().cloned());
    }
    if let Some(id) = client_id.as_deref().filter(|s| !s.is_empty()) {
        req = req.with_preregistered_client(id);
        if let Some(s) = secrets.client_secret.as_deref().filter(|s| !s.is_empty()) {
            req = req.with_client_secret(s);
        }
    }
    let session = AuthorizationSession::new(mgr, req)
        .await
        .map_err(|(_, e)| failed(e))?;
    let auth_url = session.get_authorization_url().to_owned();
    Ok(OAuthFlow {
        session,
        creds,
        auth_url,
    })
}

impl OAuthFlow {
    /// Exchanges the code from the redirect URL; returns the credentials to store on this device.
    pub async fn complete(self, callback_url: &str) -> Result<Value, McpError> {
        self.session
            .handle_callback_url(callback_url)
            .await
            .map_err(failed)?;
        self.creds
            .to_json()
            .ok_or_else(|| failed("The server did not return a token."))
    }
}

/// Desktop redirect target (§10): `http://127.0.0.1:<random>/callback`. Resolves with the full
/// callback URL once the browser comes back.
pub async fn loopback_redirect() -> std::io::Result<(String, tokio::sync::oneshot::Receiver<String>)>
{
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    let redirect = format!("http://127.0.0.1:{port}/callback");
    let (tx, rx) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        let Ok(Ok((mut sock, _))) =
            tokio::time::timeout(Duration::from_secs(600), listener.accept()).await
        else {
            return;
        };
        let mut buf = vec![0u8; 8192];
        let n = sock.read(&mut buf).await.unwrap_or(0);
        let req = String::from_utf8_lossy(&buf[..n]);
        let path = req
            .lines()
            .next()
            .and_then(|l| l.split_whitespace().nth(1))
            .unwrap_or("/")
            .to_owned();
        let body = format!(
            "<!doctype html><meta charset=utf-8><title>{app}</title><body style=\"font-family:sans-serif;margin:3em\">You can close this window and return to {app}.</body>",
            app = crate::APP_NAME
        );
        let _ = sock
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: text/html; charset=utf-8\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                )
                .as_bytes(),
            )
            .await;
        let _ = tx.send(format!("http://127.0.0.1:{port}{path}"));
    });
    Ok((redirect, rx))
}

// ─────────────────────────── legacy HTTP+SSE transport ───────────────────────────

/// Client side of the 2024-11-05 HTTP+SSE transport: a long-lived `GET` event stream whose first
/// `endpoint` event names the URL to `POST` messages to; replies arrive as `message` events.
pub struct LegacySse {
    http: reqwest::Client,
    endpoint: url::Url,
    headers: Vec<(String, String)>,
    rx: tokio::sync::mpsc::UnboundedReceiver<ServerJsonRpcMessage>,
    reader: tokio::task::JoinHandle<()>,
}

impl LegacySse {
    pub async fn connect(url: &str, headers: Vec<(String, String)>) -> Result<Self, McpError> {
        use futures::StreamExt;
        let http = crate::tls::http_client(Duration::from_secs(3600));
        let mut req = http.get(url).header("accept", "text/event-stream");
        for (k, v) in &headers {
            req = req.header(k, v);
        }
        let resp = req.send().await.map_err(failed)?;
        if resp.status() == 401 || resp.status() == 403 {
            return Err(McpError::NeedsAuth);
        }
        if !resp.status().is_success() {
            return Err(failed(format!("The server answered {}.", resp.status())));
        }
        let base = url::Url::parse(url).map_err(failed)?;
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let (etx, erx) = tokio::sync::oneshot::channel::<String>();
        let reader = tokio::spawn(async move {
            let mut parser = crate::providers::sse::Parser::default();
            let mut etx = Some(etx);
            let mut body = resp.bytes_stream();
            while let Some(Ok(chunk)) = body.next().await {
                for ev in parser.push(&String::from_utf8_lossy(&chunk)) {
                    match ev.event.as_deref() {
                        Some("endpoint") => {
                            if let Some(t) = etx.take() {
                                let _ = t.send(ev.data.trim().to_owned());
                            }
                        }
                        _ => {
                            if let Ok(m) = serde_json::from_str::<ServerJsonRpcMessage>(&ev.data)
                                && tx.send(m).is_err()
                            {
                                return;
                            }
                        }
                    }
                }
            }
        });
        let endpoint = tokio::time::timeout(Duration::from_secs(15), erx)
            .await
            .map_err(|_| failed("The server did not announce its message endpoint."))?
            .map_err(|_| failed("The server closed the event stream."))?;
        let endpoint = base.join(&endpoint).map_err(failed)?;
        if endpoint.origin() != base.origin() {
            return Err(failed("The server's message endpoint is on another host."));
        }
        Ok(Self {
            http,
            endpoint,
            headers,
            rx,
            reader,
        })
    }
}

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct SseError(String);

impl Transport<RoleClient> for LegacySse {
    type Error = SseError;

    fn send(
        &mut self,
        item: ClientJsonRpcMessage,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send + 'static {
        let mut req = self.http.post(self.endpoint.clone()).json(&item);
        for (k, v) in &self.headers {
            req = req.header(k, v);
        }
        async move {
            let r = req.send().await.map_err(|e| SseError(e.to_string()))?;
            if r.status().is_success() {
                Ok(())
            } else {
                Err(SseError(format!("The server answered {}.", r.status())))
            }
        }
    }

    fn receive(&mut self) -> impl Future<Output = Option<ServerJsonRpcMessage>> + Send {
        self.rx.recv()
    }

    fn close(&mut self) -> impl Future<Output = Result<(), Self::Error>> + Send {
        self.reader.abort();
        async { Ok(()) }
    }
}

// ─────────────────────────── connections ───────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    pub read_only: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpResource {
    pub uri: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpPrompt {
    pub name: String,
    pub description: String,
    pub arguments: Vec<String>,
}

pub struct McpConnection {
    pub config: McpServerConfig,
    service: RunningService<RoleClient, ()>,
    pub tools: Vec<McpTool>,
    /// OAuth credentials as they stand after any refresh; the app writes them back to storage.
    pub credentials: Option<SharedCredentials>,
}

fn tool_of(v: Value) -> McpTool {
    McpTool {
        name: v["name"].as_str().unwrap_or_default().to_owned(),
        description: v["description"].as_str().unwrap_or_default().to_owned(),
        input_schema: v
            .get("inputSchema")
            .cloned()
            .unwrap_or(json!({"type": "object"})),
        read_only: v
            .pointer("/annotations/readOnlyHint")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    }
}

fn params<T: serde::de::DeserializeOwned>(v: Value) -> Result<T, McpError> {
    serde_json::from_value(v).map_err(failed)
}

pub async fn connect(
    cfg: &McpServerConfig,
    secrets: &McpSecrets,
) -> Result<McpConnection, McpError> {
    let mut credentials = None;
    let service = match &cfg.transport {
        McpTransport::Stdio {
            command,
            args,
            env_names,
            cwd,
        } => {
            if !stdio_supported() {
                return Err(McpError::DesktopOnly);
            }
            spawn_stdio(command, args, env_names, cwd.as_deref(), secrets).await?
        }
        McpTransport::Sse { .. } => {
            let url = server_url(cfg)?;
            let (url, bearer, headers) = static_auth(cfg, secrets, url)?;
            let mut hs: Vec<(String, String)> = headers.into_iter().collect();
            if let Some(b) = bearer {
                hs.push(("authorization".into(), format!("Bearer {b}")));
            }
            let t = LegacySse::connect(&url, hs).await?;
            serve_client((), t).await.map_err(failed)?
        }
        McpTransport::StreamableHttp { .. } => {
            let url = server_url(cfg)?;
            let (url, bearer, headers) = static_auth(cfg, secrets, url)?;
            let mut custom = std::collections::HashMap::new();
            for (k, v) in headers {
                custom.insert(
                    http::HeaderName::try_from(k.as_str()).map_err(failed)?,
                    http::HeaderValue::try_from(v.as_str()).map_err(failed)?,
                );
            }
            let mut tc =
                StreamableHttpClientTransportConfig::with_uri(url.clone()).custom_headers(custom);
            if let Some(b) = bearer {
                tc = tc.auth_header(b);
            }
            let http = crate::tls::http_client(Duration::from_secs(120));
            match &cfg.auth {
                McpAuth::OAuth { .. } | McpAuth::ClientCredentials { .. } => {
                    let creds = SharedCredentials::from_json(secrets.oauth.as_ref());
                    let mut mgr = auth_manager(&url, creds.clone()).await?;
                    if let McpAuth::ClientCredentials { client_id, scopes } = &cfg.auth {
                        let cc = ClientCredentialsConfig::ClientSecret {
                            client_id: client_id.clone(),
                            client_secret: secrets
                                .client_secret
                                .clone()
                                .ok_or(McpError::NeedsAuth)?,
                            scopes: scopes.clone(),
                            resource: Some(url.clone()),
                        };
                        mgr.configure_client_credentials(&cc).map_err(failed)?;
                        if !creds.has_token() {
                            mgr.exchange_client_credentials(&cc).await.map_err(failed)?;
                        }
                    } else if !creds.has_token()
                        || !mgr.initialize_from_store().await.map_err(failed)?
                    {
                        return Err(McpError::NeedsAuth);
                    }
                    credentials = Some(creds);
                    let client = AuthClient::new(http, mgr);
                    let t = StreamableHttpClientTransport::with_client(client, tc);
                    serve_client((), t).await.map_err(map_init)?
                }
                _ => {
                    let t = StreamableHttpClientTransport::with_client(http, tc);
                    serve_client((), t).await.map_err(map_init)?
                }
            }
        }
    };
    let tools = service
        .list_all_tools()
        .await
        .map_err(failed)?
        .into_iter()
        .map(|t| tool_of(serde_json::to_value(t).unwrap_or_default()))
        .collect();
    Ok(McpConnection {
        config: cfg.clone(),
        service,
        tools,
        credentials,
    })
}

fn map_init(e: rmcp::service::ClientInitializeError) -> McpError {
    let s = e.to_string();
    if s.contains("401") || s.to_lowercase().contains("unauthorized") || s.contains("Auth") {
        McpError::NeedsAuth
    } else {
        failed(s)
    }
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
async fn spawn_stdio(
    command: &str,
    args: &[String],
    env_names: &[String],
    cwd: Option<&str>,
    secrets: &McpSecrets,
) -> Result<RunningService<RoleClient, ()>, McpError> {
    let mut cmd = tokio::process::Command::new(command);
    cmd.args(args);
    for n in env_names {
        if let Some(v) = secrets.env.get(n) {
            cmd.env(n, v);
        }
    }
    if let Some(d) = cwd {
        cmd.current_dir(d);
    }
    let t = rmcp::transport::TokioChildProcess::new(cmd)
        .map_err(|e| failed(format!("Couldn't start {command}: {e}")))?;
    serve_client((), t).await.map_err(failed)
}

#[cfg(any(target_os = "android", target_os = "ios"))]
async fn spawn_stdio(
    _: &str,
    _: &[String],
    _: &[String],
    _: Option<&str>,
    _: &McpSecrets,
) -> Result<RunningService<RoleClient, ()>, McpError> {
    Err(McpError::DesktopOnly)
}

impl McpConnection {
    pub async fn call_tool(
        &self,
        name: &str,
        arguments: Value,
    ) -> Result<(String, bool), McpError> {
        let args = match arguments {
            Value::Object(m) => m,
            Value::Null => Default::default(),
            other => {
                return Err(failed(format!(
                    "Tool arguments must be an object, got {other}"
                )));
            }
        };
        let r = self
            .service
            .call_tool(params(json!({"name": name, "arguments": args}))?)
            .await
            .map_err(failed)?;
        let v = serde_json::to_value(r).map_err(failed)?;
        let is_error = v["isError"].as_bool().unwrap_or(false);
        let mut text = content_text(&v["content"]);
        if text.is_empty()
            && let Some(s) = v.get("structuredContent")
        {
            text = s.to_string();
        }
        Ok((text, is_error))
    }

    pub async fn resources(&self) -> Result<Vec<McpResource>, McpError> {
        let rs = self.service.list_all_resources().await.map_err(failed)?;
        Ok(rs
            .into_iter()
            .map(|r| {
                let v = serde_json::to_value(r).unwrap_or_default();
                McpResource {
                    uri: v["uri"].as_str().unwrap_or_default().to_owned(),
                    name: v["name"].as_str().unwrap_or_default().to_owned(),
                    description: v["description"].as_str().unwrap_or_default().to_owned(),
                }
            })
            .collect())
    }

    /// Text of a resource, for adding it to an Ask conversation.
    pub async fn read_resource(&self, uri: &str) -> Result<String, McpError> {
        let r = self
            .service
            .read_resource(params(json!({"uri": uri}))?)
            .await
            .map_err(failed)?;
        let v = serde_json::to_value(r).map_err(failed)?;
        Ok(v["contents"]
            .as_array()
            .map(|cs| {
                cs.iter()
                    .filter_map(|c| c["text"].as_str())
                    .collect::<Vec<_>>()
                    .join("\n\n")
            })
            .unwrap_or_default())
    }

    pub async fn prompts(&self) -> Result<Vec<McpPrompt>, McpError> {
        let ps = self.service.list_all_prompts().await.map_err(failed)?;
        Ok(ps
            .into_iter()
            .map(|p| {
                let v = serde_json::to_value(p).unwrap_or_default();
                McpPrompt {
                    name: v["name"].as_str().unwrap_or_default().to_owned(),
                    description: v["description"].as_str().unwrap_or_default().to_owned(),
                    arguments: v["arguments"]
                        .as_array()
                        .map(|a| {
                            a.iter()
                                .filter_map(|x| x["name"].as_str().map(str::to_owned))
                                .collect()
                        })
                        .unwrap_or_default(),
                }
            })
            .collect())
    }

    /// A prompt's messages as plain text, used as a slash command in Ask (§10).
    pub async fn get_prompt(
        &self,
        name: &str,
        args: BTreeMap<String, String>,
    ) -> Result<String, McpError> {
        let r = self
            .service
            .get_prompt(params(json!({"name": name, "arguments": args}))?)
            .await
            .map_err(failed)?;
        let v = serde_json::to_value(r).map_err(failed)?;
        Ok(v["messages"]
            .as_array()
            .map(|ms| {
                ms.iter()
                    .map(|m| content_text(&json!([m["content"].clone()])))
                    .collect::<Vec<_>>()
                    .join("\n\n")
            })
            .unwrap_or_default())
    }

    pub async fn close(self) {
        let _ = self.service.cancel().await;
    }
}

fn content_text(content: &Value) -> String {
    content
        .as_array()
        .map(|cs| {
            cs.iter()
                .map(|c| match c["type"].as_str() {
                    Some("text") => c["text"].as_str().unwrap_or_default().to_owned(),
                    Some("resource") => c
                        .pointer("/resource/text")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_owned(),
                    Some("image") => "[image]".to_owned(),
                    Some(other) => format!("[{other}]"),
                    None => String::new(),
                })
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default()
}

// ─────────────────────────── tools for the agent ───────────────────────────

/// The wire name of an MCP tool. Model APIs only allow `[a-zA-Z0-9_-]`, so `mcp.<server>.<tool>`
/// (as shown to the user) is sent as `mcp__<server>__<tool>`.
pub fn wire_name(server_id: &str, tool: &str) -> String {
    let clean = |s: &str| {
        s.chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect::<String>()
    };
    let mut n = format!("mcp__{}__{}", clean(server_id), clean(tool));
    n.truncate(64);
    n
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub server_id: String,
    pub server_name: String,
    pub tool: String,
    pub arguments: Value,
    pub read_only: bool,
}

/// Asks the user whether a tool may run (inline card in Ask, spoken confirmation in voice).
#[async_trait::async_trait]
pub trait Approver: Send + Sync {
    async fn approve(&self, req: ApprovalRequest) -> bool;
}

pub fn needs_approval(policy: ToolPolicy, read_only: bool) -> bool {
    match policy {
        ToolPolicy::AlwaysAllow => false,
        ToolPolicy::AutoReadOnly => !read_only,
        ToolPolicy::AskEveryTime => true,
    }
}

/// Connected servers exposed to the agent loop as extra tools.
pub struct McpToolset {
    pub connections: Vec<Arc<McpConnection>>,
    pub approver: Arc<dyn Approver>,
}

impl McpToolset {
    fn find(&self, wire: &str) -> Option<(&Arc<McpConnection>, &McpTool)> {
        self.connections.iter().find_map(|c| {
            c.tools
                .iter()
                .find(|t| wire_name(&c.config.id, &t.name) == wire)
                .map(|t| (c, t))
        })
    }
}

#[async_trait::async_trait]
impl crate::agent::ExternalTools for McpToolset {
    fn specs(&self) -> Vec<ToolSpec> {
        self.connections
            .iter()
            .flat_map(|c| {
                c.tools.iter().map(|t| ToolSpec {
                    name: wire_name(&c.config.id, &t.name),
                    description: format!(
                        "[{} · outside service] {}",
                        c.config.name,
                        t.description.chars().take(600).collect::<String>()
                    ),
                    parameters: t.input_schema.clone(),
                })
            })
            .collect()
    }

    fn handles(&self, name: &str) -> bool {
        name.starts_with("mcp__") && self.find(name).is_some()
    }

    async fn call(&self, name: &str, arguments: &Value) -> String {
        let Some((conn, tool)) = self.find(name) else {
            return format!("ERROR: Unknown tool '{name}'.");
        };
        if needs_approval(conn.config.policy, tool.read_only) {
            let ok = self
                .approver
                .approve(ApprovalRequest {
                    server_id: conn.config.id.clone(),
                    server_name: conn.config.name.clone(),
                    tool: tool.name.clone(),
                    arguments: arguments.clone(),
                    read_only: tool.read_only,
                })
                .await;
            if !ok {
                return "ERROR: The user did not allow this tool call. Do not retry it; answer without it.".into();
            }
        }
        match conn.call_tool(&tool.name, arguments.clone()).await {
            Ok((text, false)) => text.chars().take(40_000).collect(),
            Ok((text, true)) => format!("ERROR: {}", text.chars().take(4_000).collect::<String>()),
            Err(e) => format!("ERROR: {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_round_trip_and_policies() {
        let s: McpServerConfig = serde_json::from_value(json!({
            "id": "gh", "name": "GitHub",
            "transport": {"kind": "streamable_http", "url": "https://api.example.com/mcp"},
            "auth": {"kind": "oauth"}
        }))
        .unwrap();
        assert!(matches!(s.auth, McpAuth::OAuth { .. }));
        assert!(s.enabled);
        assert_eq!(s.policy, ToolPolicy::AutoReadOnly);
        assert!(!needs_approval(ToolPolicy::AutoReadOnly, true));
        assert!(needs_approval(ToolPolicy::AutoReadOnly, false));
        assert!(needs_approval(ToolPolicy::AskEveryTime, true));
        assert!(!needs_approval(ToolPolicy::AlwaysAllow, false));
        assert_eq!(
            wire_name("git hub", "search.issues"),
            "mcp__git_hub__search_issues"
        );
    }

    #[test]
    fn plain_http_is_refused_except_loopback() {
        let cfg = |url: &str| McpServerConfig {
            id: "x".into(),
            name: "X".into(),
            transport: McpTransport::StreamableHttp { url: url.into() },
            auth: McpAuth::None,
            policy: ToolPolicy::default(),
            enabled: true,
        };
        assert!(server_url(&cfg("http://example.com/mcp")).is_err());
        assert!(server_url(&cfg("http://127.0.0.1:9000/mcp")).is_ok());
        assert!(server_url(&cfg("https://example.com/mcp")).is_ok());
    }

    #[test]
    fn api_key_in_query_and_missing_secrets() {
        let cfg = McpServerConfig {
            id: "x".into(),
            name: "X".into(),
            transport: McpTransport::StreamableHttp {
                url: "https://e.com/mcp".into(),
            },
            auth: McpAuth::ApiKeyQuery {
                param: "key".into(),
            },
            policy: ToolPolicy::default(),
            enabled: true,
        };
        assert_eq!(
            static_auth(&cfg, &McpSecrets::default(), "https://e.com/mcp").unwrap_err(),
            McpError::NeedsAuth
        );
        let s = McpSecrets {
            token: Some("k 1".into()),
            ..Default::default()
        };
        assert_eq!(
            static_auth(&cfg, &s, "https://e.com/mcp").unwrap().0,
            "https://e.com/mcp?key=k+1"
        );
    }
}
