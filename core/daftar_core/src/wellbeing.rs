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

#[cfg(test)]
mod tests {
    use super::signals_crisis;

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
