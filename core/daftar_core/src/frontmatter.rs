/// Splits `---\n<yaml>\n---\n<body>` into (yaml, body). Accepts CRLF. Returns `None` for yaml when
/// the document has no frontmatter.
pub fn split(doc: &str) -> (Option<&str>, &str) {
    let rest = match doc
        .strip_prefix("---\n")
        .or_else(|| doc.strip_prefix("---\r\n"))
    {
        Some(r) => r,
        None => return (None, doc),
    };
    let mut offset = 0;
    for line in rest.split_inclusive('\n') {
        if line.trim_end_matches(['\r', '\n']) == "---" {
            let yaml = &rest[..offset];
            let body = &rest[offset + line.len()..];
            return (Some(yaml), body);
        }
        offset += line.len();
    }
    (None, doc)
}

/// Quotes a scalar for YAML output when needed (always double-quoted, JSON-style escapes are valid YAML).
pub fn quote(s: &str) -> String {
    serde_json::to_string(s).expect("string serialises")
}

/// Replaces the value of a top-level `key:` line inside the frontmatter, keeping everything else
/// byte-for-byte. Returns `None` if the document has no such key.
pub fn replace_scalar(doc: &str, key: &str, value: &str) -> Option<String> {
    let (yaml, _) = split(doc);
    let yaml = yaml?;
    let start = doc.find(yaml)?;
    let prefix = format!("{key}:");
    let mut out = String::with_capacity(doc.len());
    out.push_str(&doc[..start]);
    let mut found = false;
    for line in yaml.split_inclusive('\n') {
        if !found && line.starts_with(&prefix) {
            let nl = if line.ends_with("\r\n") {
                "\r\n"
            } else if line.ends_with('\n') {
                "\n"
            } else {
                ""
            };
            out.push_str(&format!("{prefix} {value}{nl}"));
            found = true;
        } else {
            out.push_str(line);
        }
    }
    out.push_str(&doc[start + yaml.len()..]);
    found.then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits() {
        assert_eq!(split("---\na: 1\n---\nbody\n"), (Some("a: 1\n"), "body\n"));
        assert_eq!(
            split("---\r\na: 1\r\n---\r\nbody"),
            (Some("a: 1\r\n"), "body")
        );
        assert_eq!(split("no frontmatter"), (None, "no frontmatter"));
        assert_eq!(split("---\nunterminated"), (None, "---\nunterminated"));
    }

    #[test]
    fn replaces_only_the_key() {
        let d = "---\nid: x\nstatus: pending\nnote: \"status: pending\"\n---\nstatus: pending\n";
        assert_eq!(
            replace_scalar(d, "status", "ingested").unwrap(),
            "---\nid: x\nstatus: ingested\nnote: \"status: pending\"\n---\nstatus: pending\n"
        );
        assert!(replace_scalar(d, "missing", "x").is_none());
    }
}
