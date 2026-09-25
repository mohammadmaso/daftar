//! Adapters against a local HTTP server that replays provider-shaped responses.

use daftar_core::providers::{
    self, ChatRequest, LlmProvider, Message, ProviderConfig, ProviderErrorKind, ProviderKind,
    StopReason, ToolSpec,
};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// Serves one response and returns (base_url, request-body receiver).
async fn serve_once(
    status: u16,
    content_type: &str,
    body: String,
) -> (String, tokio::sync::oneshot::Receiver<(String, String)>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = tokio::sync::oneshot::channel();
    let ct = content_type.to_owned();
    tokio::spawn(async move {
        let (mut sock, _) = listener.accept().await.unwrap();
        let mut buf = Vec::new();
        let mut tmp = [0u8; 8192];
        let (head_end, content_len) = loop {
            let n = sock.read(&mut tmp).await.unwrap();
            buf.extend_from_slice(&tmp[..n]);
            if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                let head = String::from_utf8_lossy(&buf[..i]).to_lowercase();
                let len = head
                    .lines()
                    .find_map(|l| l.strip_prefix("content-length:"))
                    .map(|v| v.trim().parse::<usize>().unwrap())
                    .unwrap_or(0);
                break (i + 4, len);
            }
        };
        while buf.len() < head_end + content_len {
            let n = sock.read(&mut tmp).await.unwrap();
            buf.extend_from_slice(&tmp[..n]);
        }
        let head = String::from_utf8_lossy(&buf[..head_end]).into_owned();
        let req_body = String::from_utf8_lossy(&buf[head_end..]).into_owned();
        let reason = if status == 200 { "OK" } else { "ERR" };
        let resp = format!(
            "HTTP/1.1 {status} {reason}\r\ncontent-type: {ct}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        );
        sock.write_all(resp.as_bytes()).await.unwrap();
        let _ = tx.send((head, req_body));
    });
    (format!("http://{addr}"), rx)
}

fn cfg(kind: ProviderKind, base: &str) -> ProviderConfig {
    ProviderConfig {
        id: "p".into(),
        name: "Test".into(),
        kind,
        base_url: base.into(),
        extra_headers: [("X-Title".to_string(), "Daftar".to_string())].into(),
        timeout_s: 10,
    }
}

fn request() -> ChatRequest {
    ChatRequest {
        model: "m1".into(),
        system: "SYS".into(),
        messages: vec![Message::user("سلام")],
        tools: vec![ToolSpec {
            name: "search".into(),
            description: "find".into(),
            parameters: json!({"type": "object", "properties": {"q": {"type": "string"}}, "additionalProperties": false}),
        }],
        max_tokens: 256,
        temperature: Some(0.2),
        json: false,
        params: Default::default(),
    }
}

#[tokio::test]
async fn openai_streams_text_tools_and_usage() {
    let sse = [
        json!({"choices": [{"delta": {"content": "سلا"}}]}),
        json!({"choices": [{"delta": {"content": "م"}}]}),
        json!({"choices": [{"delta": {"tool_calls": [{"index": 0, "id": "c1", "function": {"name": "search", "arguments": "{\"q\":"}}]}}]}),
        json!({"choices": [{"delta": {"tool_calls": [{"index": 0, "function": {"arguments": "\"sara\"}"}}]}, "finish_reason": "tool_calls"}]}),
        json!({"choices": [], "usage": {"prompt_tokens": 12, "completion_tokens": 7, "prompt_tokens_details": {"cached_tokens": 4}}}),
    ]
    .iter()
    .map(|v| format!("data: {v}\n\n"))
    .collect::<String>()
        + "data: [DONE]\n\n";
    let (url, rx) = serve_once(200, "text/event-stream", sse).await;
    let p = providers::build(
        &cfg(ProviderKind::OpenaiCompatible, &url),
        Some("sk-test".into()),
    )
    .unwrap();
    let deltas = std::sync::Mutex::new(String::new());
    let cb = |d: &str| deltas.lock().unwrap().push_str(d);
    let r = p.chat(&request(), Some(&cb)).await.unwrap();
    assert_eq!(r.text, "سلام");
    assert_eq!(*deltas.lock().unwrap(), "سلام");
    assert_eq!(r.stop, StopReason::ToolUse);
    assert_eq!(r.tool_calls[0].name, "search");
    assert_eq!(r.tool_calls[0].arguments, json!({"q": "sara"}));
    assert_eq!(
        (
            r.usage.input_tokens,
            r.usage.output_tokens,
            r.usage.cached_input_tokens
        ),
        (12, 7, 4)
    );

    let (head, body) = rx.await.unwrap();
    assert!(head.starts_with("POST /chat/completions"));
    assert!(
        head.to_lowercase()
            .contains("authorization: bearer sk-test")
    );
    assert!(head.to_lowercase().contains("x-title: daftar"));
    let b: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(
        b["messages"][0],
        json!({"role": "system", "content": "SYS"})
    );
    assert_eq!(b["max_tokens"], 256);
    assert_eq!(b["tools"][0]["function"]["name"], "search");
    assert_eq!(b["stream"], true);
}

#[tokio::test]
async fn anthropic_streams_and_caches_system() {
    let events = [
        (
            "message_start",
            json!({"type": "message_start", "message": {"usage": {"input_tokens": 5, "cache_read_input_tokens": 100}}}),
        ),
        (
            "content_block_start",
            json!({"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}}),
        ),
        (
            "content_block_delta",
            json!({"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": "Hi"}}),
        ),
        (
            "content_block_start",
            json!({"type": "content_block_start", "index": 1, "content_block": {"type": "tool_use", "id": "tu1", "name": "search"}}),
        ),
        (
            "content_block_delta",
            json!({"type": "content_block_delta", "index": 1, "delta": {"type": "input_json_delta", "partial_json": "{\"q\": \"x\"}"}}),
        ),
        (
            "message_delta",
            json!({"type": "message_delta", "delta": {"stop_reason": "tool_use"}, "usage": {"output_tokens": 9}}),
        ),
    ];
    let sse: String = events
        .iter()
        .map(|(e, d)| format!("event: {e}\ndata: {d}\n\n"))
        .collect();
    let (url, rx) = serve_once(200, "text/event-stream", sse).await;
    let p = providers::build(&cfg(ProviderKind::Anthropic, &url), Some("ak".into())).unwrap();
    let r = p.chat(&request(), None).await.unwrap();
    assert_eq!(r.text, "Hi");
    assert_eq!(r.tool_calls[0].arguments, json!({"q": "x"}));
    assert_eq!(r.stop, StopReason::ToolUse);
    assert_eq!(
        (
            r.usage.input_tokens,
            r.usage.cached_input_tokens,
            r.usage.output_tokens
        ),
        (105, 100, 9)
    );
    let (head, body) = rx.await.unwrap();
    assert!(head.starts_with("POST /messages"));
    assert!(head.to_lowercase().contains("x-api-key: ak"));
    let b: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(b["system"][0]["cache_control"]["type"], "ephemeral");
    assert_eq!(b["tools"][0]["input_schema"]["type"], "object");
}

#[tokio::test]
async fn gemini_streams_function_calls() {
    let chunks = [
        json!({"candidates": [{"content": {"parts": [{"text": "OK "}]}}]}),
        json!({"candidates": [{"content": {"parts": [{"functionCall": {"name": "search", "args": {"q": "y"}}}]}, "finishReason": "STOP"}], "usageMetadata": {"promptTokenCount": 3, "candidatesTokenCount": 2}}),
    ];
    let sse: String = chunks
        .iter()
        .map(|c| format!("data: {c}\r\n\r\n"))
        .collect();
    let (url, rx) = serve_once(200, "text/event-stream", sse).await;
    let p = providers::build(&cfg(ProviderKind::Gemini, &url), Some("gk".into())).unwrap();
    let r = p.chat(&request(), None).await.unwrap();
    assert_eq!(r.text, "OK ");
    assert_eq!(r.tool_calls[0].arguments, json!({"q": "y"}));
    assert_eq!(r.stop, StopReason::ToolUse);
    let (head, body) = rx.await.unwrap();
    assert!(head.starts_with("POST /models/m1:streamGenerateContent?alt=sse"));
    let b: Value = serde_json::from_str(&body).unwrap();
    assert!(
        b["tools"][0]["functionDeclarations"][0]["parameters"]
            .get("additionalProperties")
            .is_none()
    );
}

#[tokio::test]
async fn http_errors_become_human_messages() {
    let (url, _rx) = serve_once(
        401,
        "application/json",
        r#"{"error":{"message":"invalid x-api-key"}}"#.into(),
    )
    .await;
    let p = providers::build(&cfg(ProviderKind::Anthropic, &url), Some("bad".into())).unwrap();
    let e = p.chat(&request(), None).await.unwrap_err();
    assert_eq!(e.kind, ProviderErrorKind::Auth);
    assert_eq!(e.message, "Test rejected the API key. (invalid x-api-key)");
}

#[tokio::test]
async fn openai_transcription_and_models() {
    let (url, rx) = serve_once(
        200,
        "application/json",
        r#"{"text": " امروز خسته بودم "}"#.into(),
    )
    .await;
    let p = providers::build(&cfg(ProviderKind::OpenaiCompatible, &url), Some("k".into())).unwrap();
    let t = p
        .transcribe("whisper-large-v3", b"abc".to_vec(), "a.m4a", Some("fa"))
        .await
        .unwrap();
    assert_eq!(t, "امروز خسته بودم");
    let (head, body) = rx.await.unwrap();
    assert!(head.starts_with("POST /audio/transcriptions"));
    assert!(
        body.contains("name=\"model\"")
            && body.contains("whisper-large-v3")
            && body.contains("name=\"language\"")
    );

    let (url, _rx) = serve_once(
        200,
        "application/json",
        r#"{"data":[{"id":"b"},{"id":"a"}]}"#.into(),
    )
    .await;
    let p = providers::build(&cfg(ProviderKind::OpenaiCompatible, &url), None).unwrap();
    assert_eq!(p.list_models().await.unwrap(), vec!["a", "b"]);
}
