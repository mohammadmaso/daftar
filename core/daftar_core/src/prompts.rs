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

/// The repo's SCHEMA.md for a prompt, followed by the vaults as the user has them now. SCHEMA.md
/// is written once at init and its vault table goes stale when the user adds, renames or archives
/// a vault, so the appended list is marked as the one to follow.
pub fn schema(lib: &crate::library::Library) -> String {
    let mut out = std::fs::read_to_string(lib.path(crate::layout::SCHEMA_FILE)).unwrap_or_default();
    if let Ok(config) = lib.config() {
        out.push_str(&format!(
            "\n\n## Current vaults (authoritative)\nThe user manages their vaults in the app. This \
             list replaces the vault table above: write only into these vaults (a vault not listed \
             is archived or gone), and for a vault the table does not describe, file by its \
             purpose and choose clear folder names inside it.\n{}\n",
            config.vaults_for_prompt()
        ));
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
                super::version(p)
                    .rsplit_once(" v")
                    .is_some_and(|(_, n)| n.parse::<u32>().is_ok()),
                "prompt without a version header: {}",
                super::version(p)
            );
        }
    }

    #[test]
    fn schema_carries_the_current_vaults() {
        let (_tmp, lib) = crate::testutil::lib_in();
        crate::vaults::add(
            &lib,
            crate::config::Bilingual {
                en: "Travel".into(),
                fa: "سفر".into(),
            },
            "Trips and visas.",
        )
        .unwrap();
        crate::vaults::set_archived(&lib, "work", true).unwrap();
        let s = super::schema(&lib);
        let current = s.split("## Current vaults").nth(1).expect("vault section");
        assert!(current.contains("- `travel` (Travel · سفر): Trips and visas."));
        assert!(current.contains("`stories`") && current.contains("[fiction"));
        assert!(!current.contains("`work`"));
    }
}
