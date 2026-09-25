//! Scenario 10 (§13): a full OAuth 2.1 PKCE + dynamic client registration flow against a local
//! authorization server protecting a Streamable HTTP MCP server; token refresh; tool approval
//! policies. Also static-token auth and the legacy HTTP+SSE transport.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use base64::Engine;
use daftar_core::agent::ExternalTools;
use daftar_core::mcp::{
    self, ApprovalRequest, Approver, McpAuth, McpError, McpSecrets, McpServerConfig, McpToolset,
    McpTransport, ToolPolicy,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

#[derive(Default)]
struct AuthState {
    clients: HashMap<String, Vec<String>>,
    /// code → (challenge, client_id, resource)
    codes: HashMap<String, (String, String, Option<String>)>,
    /// access token → expiry
    tokens: HashMap<String, Instant>,
    refresh: HashMap<String, String>,
    issued: usize,
    refreshed: usize,
    resource_seen: Vec<String>,
    tool_calls: Vec<String>,
    sse_tx: Option<tokio::sync::mpsc::UnboundedSender<String>>,
}

struct Req {
    method: String,
    path: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

fn query(path: &str) -> HashMap<String, String> {
    url::Url::parse(&format!("http://x{path}"))
        .unwrap()
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect()
}

fn form(body: &[u8]) -> HashMap<String, String> {
    url::form_urlencoded::parse(body)
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect()
}

fn resp(status: &str, headers: &[(&str, String)], body: &str) -> String {
    let mut s = format!("HTTP/1.1 {status}\r\ncontent-length: {}\r\n", body.len());
    for (k, v) in headers {
        s.push_str(&format!("{k}: {v}\r\n"));
    }
    s.push_str("\r\n");
    s.push_str(body);
    s
}

fn json_resp(v: Value) -> String {
    resp(
        "200 OK",
        &[("content-type", "application/json".into())],
        &v.to_string(),
    )
}

fn rpc(req: &Value, state: &Mutex<AuthState>) -> Option<Value> {
    let id = req.get("id")?.clone();
    let method = req["method"].as_str().unwrap_or_default();
    let result = match method {
        "initialize" => json!({
            "protocolVersion": req["params"]["protocolVersion"],
            "capabilities": {"tools": {}, "resources": {}, "prompts": {}},
            "serverInfo": {"name": "notes", "version": "1.0.0"}
        }),
        "tools/list" => json!({"tools": [
            {"name": "search", "description": "Search notes", "inputSchema": {"type": "object", "properties": {"q": {"type": "string"}}}, "annotations": {"readOnlyHint": true}},
            {"name": "delete_note", "description": "Delete a note", "inputSchema": {"type": "object", "properties": {"id": {"type": "string"}}}}
        ]}),
        "tools/call" => {
            let name = req["params"]["name"]
                .as_str()
                .unwrap_or_default()
                .to_owned();
            state.lock().unwrap().tool_calls.push(name.clone());
            json!({"content": [{"type": "text", "text": format!("{name} ran with {}", req["params"]["arguments"])}], "isError": false})
        }
        "resources/list" => json!({"resources": [{"uri": "notes://today", "name": "Today"}]}),
        "resources/read" => json!({"contents": [{"uri": "notes://today", "text": "Buy bread."}]}),
        "prompts/list" => {
            json!({"prompts": [{"name": "summarize", "description": "Summarize", "arguments": [{"name": "topic"}]}]})
        }
        "prompts/get" => {
            json!({"messages": [{"role": "user", "content": {"type": "text", "text": format!("Summarize {}", req["params"]["arguments"]["topic"])}}]})
        }
        _ => {
            return Some(
                json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32601, "message": "no"}}),
            );
        }
    };
    Some(json!({"jsonrpc": "2.0", "id": id, "result": result}))
}

async fn read_req(r: &mut BufReader<tokio::net::tcp::OwnedReadHalf>) -> Option<Req> {
    let mut line = String::new();
    if r.read_line(&mut line).await.ok()? == 0 {
        return None;
    }
    let mut parts = line.split_whitespace();
    let method = parts.next()?.to_owned();
    let path = parts.next()?.to_owned();
    let mut headers = HashMap::new();
    loop {
        let mut h = String::new();
        r.read_line(&mut h).await.ok()?;
        let h = h.trim_end();
        if h.is_empty() {
            break;
        }
        if let Some((k, v)) = h.split_once(':') {
            headers.insert(k.trim().to_lowercase(), v.trim().to_owned());
        }
    }
    let n: usize = headers
        .get("content-length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let mut body = vec![0; n];
    r.read_exact(&mut body).await.ok()?;
    Some(Req {
        method,
        path,
        headers,
        body,
    })
}

/// Authorization server + protected MCP server (Streamable HTTP at /mcp, legacy SSE at /sse).
async fn serve(
    state: Arc<Mutex<AuthState>>,
    token_ttl: Duration,
    static_token: Option<&'static str>,
) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port());
    let b = base.clone();
    tokio::spawn(async move {
        loop {
            let Ok((sock, _)) = listener.accept().await else {
                return;
            };
            let (state, base) = (state.clone(), b.clone());
            tokio::spawn(async move {
                let (rd, mut wr) = sock.into_split();
                let mut rd = BufReader::new(rd);
                while let Some(req) = read_req(&mut rd).await {
                    let path_only = req.path.split('?').next().unwrap_or("").to_owned();
                    let authorized = || {
                        let auth = req
                            .headers
                            .get("authorization")
                            .cloned()
                            .unwrap_or_default();
                        let tok = auth.trim_start_matches("Bearer ").to_owned();
                        if let Some(st) = static_token {
                            return tok == st;
                        }
                        state
                            .lock()
                            .unwrap()
                            .tokens
                            .get(&tok)
                            .is_some_and(|exp| Instant::now() < *exp)
                    };
                    let challenge = format!(
                        "Bearer resource_metadata=\"{base}/.well-known/oauth-protected-resource/mcp\""
                    );
                    let out = match (req.method.as_str(), path_only.as_str()) {
                        ("GET", p) if p.starts_with("/.well-known/oauth-protected-resource") => {
                            json_resp(json!({
                                "resource": format!("{base}/mcp"),
                                "authorization_servers": [base],
                                "scopes_supported": ["notes"]
                            }))
                        }
                        ("GET", "/.well-known/oauth-authorization-server") => json_resp(json!({
                            "issuer": base,
                            "authorization_endpoint": format!("{base}/authorize"),
                            "token_endpoint": format!("{base}/token"),
                            "registration_endpoint": format!("{base}/register"),
                            "response_types_supported": ["code"],
                            "grant_types_supported": ["authorization_code", "refresh_token"],
                            "code_challenge_methods_supported": ["S256"],
                            "token_endpoint_auth_methods_supported": ["none"],
                            "scopes_supported": ["notes"]
                        })),
                        ("GET", p) if p.starts_with("/.well-known/") => {
                            resp("404 Not Found", &[], "")
                        }
                        ("POST", "/register") => {
                            let v: Value = serde_json::from_slice(&req.body).unwrap();
                            let id = format!("client-{}", state.lock().unwrap().clients.len() + 1);
                            let uris: Vec<String> =
                                serde_json::from_value(v["redirect_uris"].clone()).unwrap();
                            state
                                .lock()
                                .unwrap()
                                .clients
                                .insert(id.clone(), uris.clone());
                            resp("201 Created", &[("content-type", "application/json".into())], &json!({
                                "client_id": id, "redirect_uris": uris, "token_endpoint_auth_method": "none",
                                "grant_types": ["authorization_code", "refresh_token"], "response_types": ["code"]
                            }).to_string())
                        }
                        ("GET", "/authorize") => {
                            let q = query(&req.path);
                            let mut st = state.lock().unwrap();
                            let client = q["client_id"].clone();
                            assert!(
                                st.clients[&client].contains(&q["redirect_uri"]),
                                "registered redirect"
                            );
                            assert_eq!(q["code_challenge_method"], "S256");
                            let code = format!("code-{}", st.codes.len() + 1);
                            st.codes.insert(
                                code.clone(),
                                (
                                    q["code_challenge"].clone(),
                                    client,
                                    q.get("resource").cloned(),
                                ),
                            );
                            if let Some(r) = q.get("resource") {
                                st.resource_seen.push(r.clone());
                            }
                            let loc = format!(
                                "{}?code={code}&state={}&iss={}",
                                q["redirect_uri"],
                                q["state"],
                                urlencode(&base)
                            );
                            resp("302 Found", &[("location", loc)], "")
                        }
                        ("POST", "/token") => {
                            let f = form(&req.body);
                            let mut st = state.lock().unwrap();
                            let ok = match f["grant_type"].as_str() {
                                "authorization_code" => {
                                    let (challenge, _, _) =
                                        st.codes.remove(&f["code"]).expect("known code");
                                    let digest = Sha256::digest(f["code_verifier"].as_bytes());
                                    let calc = base64::engine::general_purpose::URL_SAFE_NO_PAD
                                        .encode(digest);
                                    assert_eq!(
                                        calc, challenge,
                                        "PKCE verifier matches the challenge"
                                    );
                                    if let Some(r) = f.get("resource") {
                                        st.resource_seen.push(r.clone());
                                    }
                                    true
                                }
                                "refresh_token" => {
                                    st.refreshed += 1;
                                    st.refresh.contains_key(&f["refresh_token"])
                                }
                                _ => false,
                            };
                            if !ok {
                                resp(
                                    "400 Bad Request",
                                    &[("content-type", "application/json".into())],
                                    r#"{"error":"invalid_grant"}"#,
                                )
                            } else {
                                st.issued += 1;
                                let at = format!("at-{}", st.issued);
                                let rt = format!("rt-{}", st.issued);
                                st.tokens.insert(at.clone(), Instant::now() + token_ttl);
                                st.refresh.insert(rt.clone(), at.clone());
                                json_resp(
                                    json!({"access_token": at, "token_type": "Bearer", "expires_in": token_ttl.as_secs().max(1), "refresh_token": rt, "scope": "notes"}),
                                )
                            }
                        }
                        ("POST", "/mcp") if !authorized() => {
                            resp("401 Unauthorized", &[("www-authenticate", challenge)], "")
                        }
                        ("POST", "/mcp") => {
                            let v: Value = serde_json::from_slice(&req.body).unwrap();
                            match rpc(&v, &state) {
                                Some(r) => resp(
                                    "200 OK",
                                    &[
                                        ("content-type", "application/json".into()),
                                        ("mcp-session-id", "s-1".into()),
                                    ],
                                    &r.to_string(),
                                ),
                                None => resp("202 Accepted", &[], ""),
                            }
                        }
                        ("GET", "/mcp") => resp("405 Method Not Allowed", &[], ""),
                        ("DELETE", "/mcp") => resp("200 OK", &[], ""),
                        // Legacy HTTP+SSE: the stream stays open on this connection.
                        ("GET", "/sse") => {
                            if !authorized() {
                                let _ = wr
                                    .write_all(resp("401 Unauthorized", &[], "").as_bytes())
                                    .await;
                                continue;
                            }
                            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
                            state.lock().unwrap().sse_tx = Some(tx);
                            let _ = wr
                                .write_all(b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncache-control: no-cache\r\n\r\nevent: endpoint\ndata: /messages?session=1\n\n")
                                .await;
                            while let Some(m) = rx.recv().await {
                                if wr
                                    .write_all(format!("event: message\ndata: {m}\n\n").as_bytes())
                                    .await
                                    .is_err()
                                {
                                    break;
                                }
                            }
                            return;
                        }
                        ("POST", "/messages") => {
                            let v: Value = serde_json::from_slice(&req.body).unwrap();
                            if let Some(r) = rpc(&v, &state) {
                                let tx = state.lock().unwrap().sse_tx.clone();
                                if let Some(tx) = tx {
                                    let _ = tx.send(r.to_string());
                                }
                            }
                            resp("202 Accepted", &[], "")
                        }
                        _ => resp("404 Not Found", &[], ""),
                    };
                    if wr.write_all(out.as_bytes()).await.is_err() {
                        return;
                    }
                }
            });
        }
    });
    base
}

fn urlencode(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

struct Recorder {
    allow: bool,
    asked: Mutex<Vec<String>>,
}

#[async_trait::async_trait]
impl Approver for Recorder {
    async fn approve(&self, req: ApprovalRequest) -> bool {
        self.asked.lock().unwrap().push(req.tool);
        self.allow
    }
}

fn server(base: &str, auth: McpAuth, policy: ToolPolicy) -> McpServerConfig {
    McpServerConfig {
        id: "notes".into(),
        name: "Notes".into(),
        transport: McpTransport::StreamableHttp {
            url: format!("{base}/mcp"),
        },
        auth,
        policy,
        enabled: true,
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn oauth_pkce_dcr_refresh_and_policies() {
    let state = Arc::new(Mutex::new(AuthState::default()));
    let base = serve(state.clone(), Duration::from_secs(1), None).await;
    let cfg = server(
        &base,
        McpAuth::OAuth {
            client_id: None,
            scopes: vec![],
        },
        ToolPolicy::AutoReadOnly,
    );

    // Without credentials this device must authenticate first (the per-device card).
    assert_eq!(
        mcp::connect(&cfg, &McpSecrets::default()).await.err(),
        Some(McpError::NeedsAuth)
    );

    // Authorization code + PKCE with dynamic registration, redirect to the loopback listener.
    let (redirect, callback) = mcp::loopback_redirect().await.unwrap();
    let flow = mcp::oauth_begin(&cfg, &McpSecrets::default(), &redirect)
        .await
        .unwrap();
    assert!(flow.auth_url.contains("code_challenge="));
    // The "browser": follow the authorization server's redirect to the loopback listener.
    let http = reqwest::Client::builder()
        .use_preconfigured_tls((*daftar_core::tls::client_config()).clone())
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .build()
        .unwrap();
    let loc = http.get(&flow.auth_url).send().await.unwrap().headers()["location"]
        .to_str()
        .unwrap()
        .to_owned();
    assert!(loc.starts_with(&redirect));
    http.get(&loc).send().await.unwrap();
    let callback_url = callback.await.unwrap();
    let creds = flow.complete(&callback_url).await.unwrap();
    assert_eq!(
        state.lock().unwrap().clients.len(),
        1,
        "registered dynamically"
    );
    assert!(
        state
            .lock()
            .unwrap()
            .resource_seen
            .iter()
            .any(|r| r.ends_with("/mcp")),
        "RFC 8707 resource parameter sent"
    );

    let secrets = McpSecrets {
        oauth: Some(creds),
        ..Default::default()
    };
    let conn = Arc::new(mcp::connect(&cfg, &secrets).await.unwrap());
    assert_eq!(conn.tools.len(), 2);
    assert!(conn.tools.iter().any(|t| t.name == "search" && t.read_only));
    assert_eq!(conn.resources().await.unwrap()[0].uri, "notes://today");
    assert_eq!(
        conn.read_resource("notes://today").await.unwrap(),
        "Buy bread."
    );
    assert_eq!(
        conn.prompts().await.unwrap()[0].arguments,
        vec!["topic".to_string()]
    );
    let p = conn
        .get_prompt(
            "summarize",
            [("topic".to_string(), "sleep".to_string())].into(),
        )
        .await
        .unwrap();
    assert!(p.contains("sleep"));

    // Policies: read-only runs without asking; a mutating tool asks and a "no" stops it.
    let approver = Arc::new(Recorder {
        allow: false,
        asked: Mutex::default(),
    });
    let tools = McpToolset {
        connections: vec![conn.clone()],
        approver: approver.clone(),
    };
    let names: Vec<String> = tools.specs().into_iter().map(|s| s.name).collect();
    assert_eq!(names, vec!["mcp__notes__search", "mcp__notes__delete_note"]);
    let out = tools
        .call("mcp__notes__search", &json!({"q": "bread"}))
        .await;
    assert!(out.starts_with("search ran"), "{out}");
    let out = tools
        .call("mcp__notes__delete_note", &json!({"id": "1"}))
        .await;
    assert!(out.starts_with("ERROR: The user did not allow"), "{out}");
    assert_eq!(*approver.asked.lock().unwrap(), vec!["delete_note"]);
    assert_eq!(
        state.lock().unwrap().tool_calls,
        vec!["search"],
        "denied call never reached the server"
    );

    // Token refresh: the 1-second access token expires; the next call refreshes transparently.
    tokio::time::sleep(Duration::from_millis(1500)).await;
    let out = tools
        .call("mcp__notes__search", &json!({"q": "milk"}))
        .await;
    assert!(out.starts_with("search ran"), "{out}");
    assert!(state.lock().unwrap().refreshed >= 1, "refresh grant used");
    let saved = conn
        .credentials
        .as_ref()
        .unwrap()
        .to_json()
        .unwrap()
        .to_string();
    assert!(
        !saved.contains("\"at-1\""),
        "refreshed token is handed back for secure storage"
    );

    // Ask every time: even read-only tools ask.
    let strict = server(
        &base,
        McpAuth::OAuth {
            client_id: None,
            scopes: vec![],
        },
        ToolPolicy::AskEveryTime,
    );
    let secrets = McpSecrets {
        oauth: conn.credentials.as_ref().unwrap().to_json(),
        ..Default::default()
    };
    let conn2 = Arc::new(mcp::connect(&strict, &secrets).await.unwrap());
    let yes = Arc::new(Recorder {
        allow: true,
        asked: Mutex::default(),
    });
    let tools = McpToolset {
        connections: vec![conn2],
        approver: yes.clone(),
    };
    assert!(
        tools
            .call("mcp__notes__search", &json!({}))
            .await
            .starts_with("search ran")
    );
    assert_eq!(*yes.asked.lock().unwrap(), vec!["search"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn static_token_and_legacy_sse() {
    let state = Arc::new(Mutex::new(AuthState::default()));
    let base = serve(state.clone(), Duration::from_secs(60), Some("secret-1")).await;
    let cfg = server(&base, McpAuth::Bearer, ToolPolicy::AlwaysAllow);
    assert_eq!(
        mcp::connect(&cfg, &McpSecrets::default()).await.err(),
        Some(McpError::NeedsAuth)
    );
    let secrets = McpSecrets {
        token: Some("secret-1".into()),
        ..Default::default()
    };
    let conn = mcp::connect(&cfg, &secrets).await.unwrap();
    assert_eq!(conn.tools.len(), 2);
    let (text, err) = conn
        .call_tool("delete_note", json!({"id": "7"}))
        .await
        .unwrap();
    assert!(!err && text.contains("\"7\""), "{text}");

    let sse = McpServerConfig {
        transport: McpTransport::Sse {
            url: format!("{base}/sse"),
        },
        ..cfg.clone()
    };
    let conn = mcp::connect(&sse, &secrets).await.unwrap();
    assert_eq!(conn.tools.len(), 2);
    let (text, _) = conn.call_tool("search", json!({"q": "x"})).await.unwrap();
    assert!(text.starts_with("search ran"));
    let calls = AtomicUsize::new(state.lock().unwrap().tool_calls.len());
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}
