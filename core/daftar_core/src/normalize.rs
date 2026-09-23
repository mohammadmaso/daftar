//! Search normalisation (§4.5), applied identically at index and query time.
//!
//! * Arabic ي/ى → Persian ی, ك → ک; ة/ۀ → ه; أ/إ/آ/ٱ → ا; ؤ → و; ئ → ی
//! * tatweel and harakat removed
//! * Persian/Arabic-Indic digits → ASCII
//! * Latin lower-cased and accent-folded
//! * ZWNJ: `normalize` splits at ZWNJ (word boundary); `joined_variants` adds the joined spelling so
//!   both "می‌خواهم" and "میخواهم" find the same page.

use unicode_normalization::UnicodeNormalization;

const ZWNJ: char = '\u{200C}';

fn map_char(c: char) -> Option<char> {
    Some(match c {
        '\u{064A}' | '\u{0649}' | '\u{0626}' => 'ی',
        '\u{0643}' => 'ک',
        '\u{0629}' | '\u{06C0}' => 'ه',
        '\u{0623}' | '\u{0625}' | '\u{0622}' | '\u{0671}' => 'ا',
        '\u{0624}' => 'و',
        '\u{0640}' => return None,                        // tatweel
        '\u{064B}'..='\u{065F}' | '\u{0670}' => return None, // harakat, superscript alef
        '\u{06F0}'..='\u{06F9}' => char::from(b'0' + (c as u32 - 0x06F0) as u8),
        '\u{0660}'..='\u{0669}' => char::from(b'0' + (c as u32 - 0x0660) as u8),
        '\u{200D}' | '\u{200E}' | '\u{200F}' | '\u{FEFF}' => return None, // ZWJ, bidi marks
        _ => c,
    })
}

fn is_latin_combining(c: char) -> bool {
    ('\u{0300}'..='\u{036F}').contains(&c)
}

fn fold(text: &str, zwnj: char) -> String {
    let mut out = String::with_capacity(text.len());
    // Decompose only Latin letters with accents; Arabic-script letters are mapped explicitly so
    // NFD does not split letters like آ into base + madda unexpectedly.
    for c in text.chars() {
        if c == ZWNJ {
            if zwnj != '\0' {
                out.push(zwnj);
            }
            continue;
        }
        if c.is_ascii() {
            out.push(c.to_ascii_lowercase());
        } else if ('\u{00C0}'..='\u{024F}').contains(&c) || ('\u{1E00}'..='\u{1EFF}').contains(&c) {
            for d in c.to_string().nfd() {
                if !is_latin_combining(d) {
                    out.extend(d.to_lowercase());
                }
            }
        } else if let Some(m) = map_char(c) {
            out.extend(m.to_lowercase());
        }
    }
    out
}

/// Canonical searchable form (ZWNJ treated as a word break).
pub fn normalize(text: &str) -> String {
    fold(text, ' ')
}

/// Joined spellings of every ZWNJ-compound word, to be indexed alongside `normalize(text)`.
pub fn joined_variants(text: &str) -> String {
    text.split_whitespace()
        .filter(|w| w.contains(ZWNJ))
        .map(|w| fold(w, '\0'))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Index form: normalised text plus joined variants.
pub fn index_form(text: &str) -> String {
    let j = joined_variants(text);
    if j.is_empty() { normalize(text) } else { format!("{} {j}", normalize(text)) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arabic_and_persian_letters_unify() {
        assert_eq!(normalize("كتاب علي"), normalize("کتاب علی"));
        assert_eq!(normalize("مدرسة"), normalize("مدرسه"));
        assert_eq!(normalize("أحمد إيمان آب"), "احمد ایمان اب");
    }

    #[test]
    fn diacritics_tatweel_digits() {
        assert_eq!(normalize("مُحَمَّد"), "محمد");
        assert_eq!(normalize("کـــتاب"), "کتاب");
        assert_eq!(normalize("۱۴۰۵ و ٢٠٢٦"), "1405 و 2026");
    }

    #[test]
    fn latin_folding() {
        assert_eq!(normalize("Café CRÈME Ünïcode"), "cafe creme unicode");
    }

    #[test]
    fn zwnj_both_ways() {
        let idx = index_form("من می‌خواهم بروم");
        assert!(idx.contains("می خواهم"));
        assert!(idx.contains("میخواهم"));
        assert_eq!(normalize("می‌خواهم"), "می خواهم");
    }
}
