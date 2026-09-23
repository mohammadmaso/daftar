/// Script-based language detection for fa/en. Deliberately simple: the router model refines it.
/// Returns languages ordered by prevalence; empty if the text has no letters.
pub fn detect(text: &str) -> Vec<String> {
    let (mut arabic, mut latin) = (0usize, 0usize);
    for c in text.chars() {
        if is_arabic_script(c) {
            arabic += 1;
        } else if c.is_ascii_alphabetic() || ('\u{00C0}'..='\u{024F}').contains(&c) {
            latin += 1;
        }
    }
    let total = arabic + latin;
    if total == 0 {
        return vec![];
    }
    // A language counts when it makes up at least 10 % of letters (catches "meeting با Sara").
    let mut out: Vec<(usize, &str)> = [(arabic, "fa"), (latin, "en")]
        .into_iter()
        .filter(|(n, _)| *n * 10 >= total && *n > 0)
        .collect();
    out.sort_by_key(|x| std::cmp::Reverse(x.0));
    out.into_iter().map(|(_, l)| l.to_owned()).collect()
}

pub fn is_arabic_script(c: char) -> bool {
    matches!(c, '\u{0600}'..='\u{06FF}' | '\u{0750}'..='\u{077F}' | '\u{08A0}'..='\u{08FF}' | '\u{FB50}'..='\u{FDFF}' | '\u{FE70}'..='\u{FEFF}')
}

#[cfg(test)]
mod tests {
    use super::detect;

    #[test]
    fn detects() {
        assert_eq!(detect("امروز خیلی خسته‌ام"), vec!["fa"]);
        assert_eq!(detect("Slept badly today"), vec!["en"]);
        assert_eq!(
            detect("جلسه با Sara درباره‌ی project جدید"),
            vec!["fa", "en"]
        );
        assert!(detect("12:30 !!").is_empty());
    }
}
