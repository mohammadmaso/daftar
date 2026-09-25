//! Reflect and Lint for the Flutter app (§4.6, §4.7, §8.6 Settings › Reflect).

use daftar_core::reflect::ReflectSettings;

use super::library::LibraryHandle;

pub struct ReflectPrefs {
    pub daily: bool,
    /// "HH:MM", local time.
    pub daily_time: String,
    pub weekly: bool,
    /// 1 = Monday … 7 = Sunday.
    pub weekly_day: u8,
    pub notifications: bool,
    /// ISO country for the helpline card ("" = international).
    pub helpline_country: String,
}

pub struct ReflectSignals {
    /// Local notifications to show (at most three lines each, never sensitive).
    pub notifications: Vec<String>,
    /// A reflection saw signs of crisis: show the Talk to someone card.
    pub needs_help: bool,
}

pub struct LintSummary {
    pub findings: u32,
    pub new_cards: u32,
}

fn err(e: impl std::fmt::Display) -> anyhow::Error {
    anyhow::anyhow!(e.to_string())
}

impl LibraryHandle {
    pub fn reflect_prefs(&self) -> anyhow::Result<ReflectPrefs> {
        let r = self.session().library().config().map_err(err)?.reflect;
        Ok(ReflectPrefs {
            daily: r.daily,
            daily_time: r.daily_time,
            weekly: r.weekly,
            weekly_day: r.weekly_day,
            notifications: r.notifications,
            helpline_country: r.helpline_country,
        })
    }

    pub fn set_reflect_prefs(&self, prefs: ReflectPrefs) -> anyhow::Result<()> {
        self.session()
            .library()
            .update_config(|c| {
                let keep: ReflectSettings = c.reflect.clone();
                c.reflect = ReflectSettings {
                    daily: prefs.daily,
                    daily_time: prefs.daily_time,
                    weekly: prefs.weekly,
                    weekly_day: prefs.weekly_day.clamp(1, 7),
                    notifications: prefs.notifications,
                    helpline_country: prefs.helpline_country.to_uppercase(),
                    ..keep
                };
            })
            .map_err(err)
    }

    /// Queues what is due now (reflections, and lint when enough was filed). Returns how many jobs.
    pub fn schedule_due(&self) -> anyhow::Result<u32> {
        let s = self.session();
        let n = s.schedule_reflections(&jiff::Zoned::now()).map_err(err)?;
        let lint = s.schedule_lint_if_due(&jiff::Zoned::now()).map_err(err)?;
        Ok(n as u32 + u32::from(lint))
    }

    pub fn take_reflect_signals(&self) -> ReflectSignals {
        let (notifications, needs_help) = self.session().take_reflect_signals();
        ReflectSignals {
            notifications,
            needs_help,
        }
    }

    /// Settings › Check the wiki now (code checks only; model checks run with the jobs).
    pub fn lint_now(&self) -> anyhow::Result<LintSummary> {
        let r = self.session().lint_now().map_err(err)?;
        Ok(LintSummary {
            findings: r.findings.len() as u32,
            new_cards: r.new_cards as u32,
        })
    }
}
