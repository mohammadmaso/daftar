//! System prompts shipped with the app (§15), versioned in `core/prompts/` and compiled in.

pub const ROUTER: &str = include_str!("../../prompts/router.md");
pub const INGEST: &str = include_str!("../../prompts/ingest.md");
pub const VISION_DESCRIBE: &str = include_str!("../../prompts/vision_describe.md");
pub const QUERY: &str = include_str!("../../prompts/query.md");
pub const VOICE: &str = include_str!("../../prompts/voice.md");
pub const STORY_COWRITER: &str = include_str!("../../prompts/story_cowriter.md");
pub const COMPENSATE: &str = include_str!("../../prompts/compensate.md");
pub const LINT: &str = include_str!("../../prompts/lint.md");
pub const REFLECT_DAILY: &str = include_str!("../../prompts/reflect_daily.md");
pub const REFLECT_WEEKLY: &str = include_str!("../../prompts/reflect_weekly.md");

/// Every shipped prompt, for version checks and the eval harness.
pub const ALL: &[&str] = &[
    ROUTER,
    INGEST,
    VISION_DESCRIBE,
    QUERY,
    VOICE,
    STORY_COWRITER,
    COMPENSATE,
    LINT,
    REFLECT_DAILY,
    REFLECT_WEEKLY,
];

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
    template
        .lines()
        .next()
        .and_then(|l| l.strip_prefix("<!-- prompt: "))
        .and_then(|l| l.strip_suffix(" -->"))
        .unwrap_or("unknown")
}

#[cfg(test)]
mod tests {
    #[test]
    fn versions_and_render() {
        assert_eq!(super::version(super::INGEST), "ingest v1");
        assert_eq!(super::render("a {{x}} {{y}}", &[("x", "1")]), "a 1 {{y}}");
        for p in super::ALL {
            assert!(
                super::version(p).ends_with(" v1"),
                "prompt without a version header: {}",
                super::version(p)
            );
        }
    }
}
