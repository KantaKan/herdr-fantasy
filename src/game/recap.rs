//! Weekly recap (TASKS M15): the last 7 days of the quest log, summed. Pure.

use std::collections::BTreeMap;

use super::world::LogEntry;

#[derive(Debug, Default, PartialEq)]
pub struct Week {
    pub quests: u32,
    pub xp: u64,
    pub won: u32,
    pub lost: u32,
    /// Longest run of boss wins in a row this week.
    pub best_run: u32,
    /// Quests per day, oldest first; the last one is today.
    pub per_day: [u32; 7],
    /// (hero, XP) with the most XP.
    pub top_hero: Option<(String, u64)>,
    /// (repo key, quests) with the most quests.
    pub top_repo: Option<(String, u32)>,
}

/// Monday = 0. 1970-01-01 was a Thursday.
pub fn weekday(day: u32) -> usize {
    ((day + 3) % 7) as usize
}

pub const WEEKDAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

/// Sum the log entries from the 7 days ending `today` (days since 1970, UTC).
pub fn week(log: &[LogEntry], today: u32) -> Week {
    let first = today.saturating_sub(6);
    let mut w = Week::default();
    let mut heroes: BTreeMap<&str, u64> = BTreeMap::new();
    let mut repos: BTreeMap<String, u32> = BTreeMap::new();
    let mut run = 0;
    for e in log.iter().filter(|e| e.day >= first && e.day <= today) {
        w.quests += 1;
        w.xp += e.xp;
        w.per_day[(e.day - first) as usize] += 1;
        *heroes.entry(&e.hero).or_default() += e.xp;
        *repos.entry(e.repo.clone().unwrap_or_default()).or_default() += 1;
        for c in e.bosses.chars() {
            if c == '✓' {
                w.won += 1;
                run += 1;
                w.best_run = w.best_run.max(run);
            } else if c == '✗' {
                w.lost += 1;
                run = 0;
            }
        }
    }
    w.top_hero = heroes
        .into_iter()
        .max_by_key(|(_, xp)| *xp)
        .map(|(h, xp)| (h.to_string(), xp));
    w.top_repo = repos.into_iter().max_by_key(|(_, n)| *n);
    w
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(hero: &str, day: u32, xp: u64, bosses: &str) -> LogEntry {
        LogEntry {
            repo: Some("r1".into()),
            hero: hero.into(),
            title: String::new(),
            duration_ms: 0,
            xp,
            result: "complete".into(),
            bosses: bosses.into(),
            chapter: 1,
            day,
        }
    }

    #[test]
    fn sums_the_last_seven_days() {
        let log = [
            entry("A", 92, 99, "✓"),
            entry("A", 93, 10, "✓✓"),
            entry("B", 99, 30, "✗✓"),
            entry("B", 99, 20, "✓"),
        ];
        let w = week(&log, 99);
        assert_eq!((w.quests, w.xp, w.won, w.lost), (3, 60, 4, 1));
        assert_eq!(w.best_run, 2);
        assert_eq!(w.per_day, [1, 0, 0, 0, 0, 0, 2]);
        assert_eq!(w.top_hero, Some(("B".into(), 50)));
        assert_eq!(w.top_repo, Some(("r1".into(), 3)));
    }

    #[test]
    fn weekdays() {
        assert_eq!(WEEKDAYS[weekday(0)], "Thu");
        assert_eq!(WEEKDAYS[weekday(20_378)], "Fri"); // 2025-10-17
    }
}
