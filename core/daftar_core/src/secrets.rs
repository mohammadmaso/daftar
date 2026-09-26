//! Pre-commit secret guard (§12): nothing that looks like an API key, token or private key is
//! committed. Captures and edits containing one are held back and the user is offered a redaction;
//! AI changes containing one are rejected by the validator.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretHit {
    /// What it looks like: "OpenAI key", "GitHub token", "private key", …
    pub kind: String,
    /// Byte range in the text.
    pub start: usize,
    pub end: usize,
}

const TOKEN_CHARS: fn(char) -> bool = |c| c.is_ascii_alphanumeric() || c == '_' || c == '-';

/// (prefix, kind, minimum token length after the prefix)
const PREFIXES: &[(&str, &str, usize)] = &[
    ("sk-ant-", "Anthropic key", 30),
    ("sk-proj-", "OpenAI key", 30),
    ("sk-or-v1-", "OpenRouter key", 30),
    ("sk-", "API key", 32),
    ("ghp_", "GitHub token", 30),
    ("gho_", "GitHub token", 30),
    ("ghu_", "GitHub token", 30),
    ("ghs_", "GitHub token", 30),
    ("ghr_", "GitHub token", 30),
    ("github_pat_", "GitHub token", 40),
    ("glpat-", "GitLab token", 20),
    ("AKIA", "AWS key", 16),
    ("AIza", "Google API key", 35),
    ("xoxb-", "Slack token", 20),
    ("xoxp-", "Slack token", 20),
    ("gsk_", "Groq key", 40),
];

fn token_end(text: &str, from: usize) -> usize {
    text[from..]
        .char_indices()
        .find(|(_, c)| !TOKEN_CHARS(*c))
        .map_or(text.len(), |(i, _)| from + i)
}

/// Finds secrets in `text`, earliest first, non-overlapping.
pub fn scan(text: &str) -> Vec<SecretHit> {
    let mut hits: Vec<SecretHit> = Vec::new();
    // Private key blocks.
    let mut at = 0;
    while let Some(i) = text[at..].find("-----BEGIN ") {
        let start = at + i;
        let header_end = text[start..].find('\n').map_or(text.len(), |n| start + n);
        if text[start..header_end].contains("PRIVATE KEY") {
            let end = text[start..]
                .find("-----END ")
                .map(|e| start + e + 9)
                .and_then(|after| text[after..].find("-----").map(|x| after + x + 5))
                .unwrap_or(text.len());
            hits.push(SecretHit {
                kind: "private key".into(),
                start,
                end,
            });
            at = end;
        } else {
            at = header_end;
        }
    }
    // Prefixed tokens (a token must start at a word boundary).
    for (prefix, kind, min) in PREFIXES {
        let mut at = 0;
        while let Some(i) = text[at..].find(prefix) {
            let start = at + i;
            at = start + prefix.len();
            let boundary = text[..start]
                .chars()
                .next_back()
                .is_none_or(|c| !TOKEN_CHARS(c));
            if !boundary || hits.iter().any(|h| h.start <= start && start < h.end) {
                continue;
            }
            let end = token_end(text, start + prefix.len());
            let body = &text[start + prefix.len()..end];
            let mixed = body.chars().any(|c| c.is_ascii_digit())
                && body.chars().any(|c| c.is_ascii_alphabetic());
            if body.len() >= *min && mixed {
                hits.push(SecretHit {
                    kind: (*kind).into(),
                    start,
                    end,
                });
                at = end;
            }
        }
    }
    // JSON Web Tokens and bearer headers.
    let mut at = 0;
    while let Some(i) = text[at..].find("eyJ") {
        let start = at + i;
        at = start + 3;
        let end = text[start..]
            .char_indices()
            .find(|(_, c)| !(TOKEN_CHARS(*c) || *c == '.'))
            .map_or(text.len(), |(j, _)| start + j);
        let t = &text[start..end];
        if t.matches('.').count() == 2
            && t.len() >= 60
            && !hits.iter().any(|h| h.start <= start && start < h.end)
        {
            hits.push(SecretHit {
                kind: "access token".into(),
                start,
                end,
            });
            at = end;
        }
    }
    hits.sort_by_key(|h| h.start);
    hits
}

/// The text with every secret replaced by `[redacted <kind>]`.
pub fn redact(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for h in scan(text) {
        out.push_str(&text[last..h.start]);
        out.push_str(&format!("[redacted {}]", h.kind));
        last = h.end;
    }
    out.push_str(&text[last..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_common_secrets_and_leaves_prose_alone() {
        let key = "sk-proj-abcDEF1234567890abcDEF1234567890xyz";
        let gh = "ghp_0123456789abcdefghijABCDEFGHIJ012345";
        let text = format!(
            "My key is {key} and token {gh}.\n-----BEGIN OPENSSH PRIVATE KEY-----\nb3Blbn\n-----END OPENSSH PRIVATE KEY-----\nfine"
        );
        let hits = scan(&text);
        assert_eq!(
            hits.iter().map(|h| h.kind.as_str()).collect::<Vec<_>>(),
            vec!["OpenAI key", "GitHub token", "private key"]
        );
        let r = redact(&text);
        assert!(
            !r.contains(key) && !r.contains(gh) && !r.contains("b3Blbn"),
            "{r}"
        );
        assert!(r.ends_with("fine"));
        for prose in [
            "I asked about task-management and sk-style skiing.",
            "The disk-space ran out",
            "AKIA is a name",
            "سارا گفت کلید را گم کرده",
        ] {
            assert!(scan(prose).is_empty(), "{prose}");
        }
    }
}
