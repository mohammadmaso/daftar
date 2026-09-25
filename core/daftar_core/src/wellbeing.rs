//! Wellbeing guardrail (§4.7): recognise signs of crisis in the user's own words so Ask and Reflect
//! can surface the "Talk to someone" card. Deliberately a small, conservative phrase list in both
//! languages (after search normalisation); the model's own `[[talk-to-someone]]` marker is the
//! second signal. False positives only show a gentle card; nothing is ever sent anywhere.

use crate::normalize::normalize;

const PHRASES: &[&str] = &[
    // English
    "kill myself",
    "killing myself",
    "end my life",
    "ending my life",
    "take my own life",
    "suicide",
    "suicidal",
    "want to die",
    "wanna die",
    "better off dead",
    "self harm",
    "self-harm",
    "hurt myself",
    "cut myself",
    "no reason to live",
    "can't go on",
    "cannot go on",
    // Persian (normalised forms: ی/ک, no ZWNJ)
    "خودکشی",
    "خودم را بکشم",
    "خودمو بکشم",
    "خودم رو بکشم",
    "به زندگیم پایان",
    "به زندگی ام پایان",
    "میخواهم بمیرم",
    "میخوام بمیرم",
    "دلم میخواد بمیرم",
    "آرزوی مرگ",
    "به خودم آسیب",
    "دیگه نمیکشم",
    "دیگر نمیکشم",
    "دلیلی برای زنده ماندن",
];

/// Whether `text` contains a phrase that suggests the writer may be at risk.
pub fn signals_crisis(text: &str) -> bool {
    let n = normalize(text);
    let joined = n.replace(' ', "");
    PHRASES.iter().any(|p| {
        let p = normalize(p);
        n.contains(&p) || joined.contains(&p.replace(' ', ""))
    })
}

/// One way to reach help, shown on the "Talk to someone" card.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Helpline {
    pub name_en: String,
    pub name_fa: String,
    /// A number to call, or empty when `url` is the way in.
    pub phone: String,
    pub url: String,
}

fn line(name_en: &str, name_fa: &str, phone: &str, url: &str) -> Helpline {
    Helpline {
        name_en: name_en.into(),
        name_fa: name_fa.into(),
        phone: phone.into(),
        url: url.into(),
    }
}

/// Curated defaults (§4.7), by ISO country code; the user's own choice is shown first by the app.
/// The list is short on purpose and should be reviewed by the owner before release (PLAN §4).
pub fn helplines(country: &str) -> Vec<Helpline> {
    let mut out = match country.to_ascii_uppercase().as_str() {
        "IR" => vec![
            line("Social emergency (Behzisti)", "اورژانس اجتماعی", "123", ""),
            line(
                "Voice of the counsellor (Behzisti)",
                "صدای مشاور بهزیستی",
                "1480",
                "",
            ),
            line("Emergency medical services", "اورژانس", "115", ""),
        ],
        "US" => vec![line(
            "988 Suicide & Crisis Lifeline",
            "خط بحران ۹۸۸",
            "988",
            "https://988lifeline.org",
        )],
        "GB" | "UK" => vec![line(
            "Samaritans",
            "Samaritans",
            "116 123",
            "https://www.samaritans.org",
        )],
        "DE" => vec![line(
            "TelefonSeelsorge",
            "TelefonSeelsorge",
            "0800 111 0 111",
            "https://www.telefonseelsorge.de",
        )],
        "CA" => vec![line(
            "9-8-8 Suicide Crisis Helpline",
            "خط بحران ۹۸۸",
            "988",
            "https://988.ca",
        )],
        _ => vec![],
    };
    out.push(line(
        "Find a helpline in your country",
        "یافتن خط کمک در کشور شما",
        "",
        "https://findahelpline.com",
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::signals_crisis;

    #[test]
    fn helplines_have_an_international_fallback() {
        assert_eq!(super::helplines("ir")[0].phone, "123");
        assert_eq!(super::helplines("").len(), 1);
        assert!(super::helplines("XX")[0].url.contains("findahelpline"));
    }

    #[test]
    fn recognises_both_languages_and_spellings() {
        assert!(signals_crisis("Some days I just want to die."));
        assert!(signals_crisis("به خودکشی فکر می‌کنم"));
        assert!(signals_crisis("دیگه نمی‌کشم"));
        assert!(signals_crisis("مي‌خواهم بميرم")); // Arabic yeh
        assert!(!signals_crisis("I killed it at the presentation today"));
        assert!(!signals_crisis("سردرد داشتم و خوابم نبرد"));
    }
}
