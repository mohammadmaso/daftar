use jiff::Zoned;

use crate::layout::Date;

pub fn date_of(t: &Zoned) -> Date {
    Date {
        year: t.year() as i32,
        month: t.month() as u8,
        day: t.day() as u8,
    }
}

/// `20260923T141502` — sortable, filename-safe local timestamp.
pub fn compact(t: &Zoned) -> String {
    t.strftime("%Y%m%dT%H%M%S").to_string()
}

/// RFC 3339 with numeric offset, e.g. `2026-09-23T14:15:02+02:00`.
pub fn rfc3339(t: &Zoned) -> String {
    t.strftime("%Y-%m-%dT%H:%M:%S%:z").to_string()
}

pub fn parse_rfc3339(s: &str) -> crate::Result<jiff::Timestamp> {
    Ok(s.parse::<jiff::Timestamp>()?)
}

pub fn now_ms() -> i64 {
    jiff::Timestamp::now().as_millisecond()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats() {
        let t: Zoned = "2026-09-23T14:15:02+02:00[Europe/Berlin]".parse().unwrap();
        assert_eq!(compact(&t), "20260923T141502");
        assert_eq!(rfc3339(&t), "2026-09-23T14:15:02+02:00");
        assert_eq!(
            date_of(&t),
            Date {
                year: 2026,
                month: 9,
                day: 23
            }
        );
        assert_eq!(
            parse_rfc3339("2026-09-23T14:15:02+02:00")
                .unwrap()
                .as_second(),
            t.timestamp().as_second()
        );
    }
}
