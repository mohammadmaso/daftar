//! Minimal Server-Sent Events reader over a byte stream.

use futures::StreamExt;

use super::{ProviderError, ProviderResult};

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Event {
    pub event: Option<String>,
    pub data: String,
}

/// Incrementally splits SSE text into events (handles `\n` and `\r\n`, multi-line `data:`).
#[derive(Default)]
pub struct Parser {
    buf: String,
    cur: Event,
    has_data: bool,
}

impl Parser {
    pub fn push(&mut self, chunk: &str) -> Vec<Event> {
        self.buf.push_str(chunk);
        let mut out = Vec::new();
        while let Some(nl) = self.buf.find('\n') {
            let line = self.buf[..nl].trim_end_matches('\r').to_owned();
            self.buf.drain(..=nl);
            if line.is_empty() {
                if self.has_data {
                    out.push(std::mem::take(&mut self.cur));
                    self.has_data = false;
                }
                continue;
            }
            if line.starts_with(':') {
                continue;
            }
            let (field, value) = match line.split_once(':') {
                Some((f, v)) => (f, v.strip_prefix(' ').unwrap_or(v)),
                None => (line.as_str(), ""),
            };
            match field {
                "event" => self.cur.event = Some(value.to_owned()),
                "data" => {
                    if self.has_data {
                        self.cur.data.push('\n');
                    }
                    self.cur.data.push_str(value);
                    self.has_data = true;
                }
                _ => {}
            }
        }
        out
    }
}

/// Feeds a response body through the parser, calling `on_event` for each event.
pub async fn read(
    provider: &str,
    resp: reqwest::Response,
    mut on_event: impl FnMut(Event) -> ProviderResult<()>,
) -> ProviderResult<()> {
    let mut parser = Parser::default();
    let mut stream = resp.bytes_stream();
    let mut pending: Vec<u8> = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| ProviderError::from_reqwest(provider, e))?;
        pending.extend_from_slice(&chunk);
        // Only decode complete UTF-8 prefixes; multi-byte characters can straddle chunks.
        let valid = match std::str::from_utf8(&pending) {
            Ok(_) => pending.len(),
            Err(e) => e.valid_up_to(),
        };
        let text = String::from_utf8(pending.drain(..valid).collect()).expect("validated");
        for ev in parser.push(&text) {
            on_event(ev)?;
        }
    }
    for ev in parser.push("\n\n") {
        on_event(ev)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_split_events() {
        let mut p = Parser::default();
        assert!(p.push("event: message_start\r\ndata: {\"a\"").is_empty());
        let evs = p.push(":1}\r\n\r\ndata: x\ndata: y\n\n: comment\n\n");
        assert_eq!(evs.len(), 2);
        assert_eq!(evs[0].event.as_deref(), Some("message_start"));
        assert_eq!(evs[0].data, "{\"a\":1}");
        assert_eq!(evs[1].data, "x\ny");
    }
}
