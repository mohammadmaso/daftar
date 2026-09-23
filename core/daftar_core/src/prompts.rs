//! System prompts shipped with the app (§15), versioned in `core/prompts/` and compiled in.

pub const ROUTER: &str = include_str!("../../prompts/router.md");
pub const INGEST: &str = include_str!("../../prompts/ingest.md");
pub const VISION_DESCRIBE: &str = include_str!("../../prompts/vision_describe.md");

/// Replaces `{{name}}` placeholders. Unknown placeholders are left visible so tests catch them.
pub fn render(template: &str, vars: &[(&str, &str)]) -> String {
    let mut out = template.to_owned();
    for (k, v) in vars {
        out = out.replace(&format!("{{{{{k}}}}}"), v);
    }
    out
}

/// `<!-- prompt: name vN -->` header, recorded in the ledger for reproducibility.
pub fn version(template: &str) -> &str {
    template.lines().next().and_then(|l| l.strip_prefix("<!-- prompt: ")).and_then(|l| l.strip_suffix(" -->")).unwrap_or("unknown")
}

#[cfg(test)]
mod tests {
    #[test]
    fn versions_and_render() {
        assert_eq!(super::version(super::INGEST), "ingest v1");
        assert_eq!(super::render("a {{x}} {{y}}", &[("x", "1")]), "a 1 {{y}}");
    }
}
