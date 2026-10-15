//! XP and level formulas (SPEC §7). Pure functions.

/// XP needed to go from `level` to `level + 1`: `round(100 * level^1.5)`.
pub fn xp_to_next(level: u32) -> u64 {
    (100.0 * (level.max(1) as f64).powf(1.5)).round() as u64
}

/// Add `gained` XP; returns the new `(level, xp_into_level, levels_gained)`.
pub fn add_xp(level: u32, xp: u64, gained: u64) -> (u32, u64, u32) {
    let (mut level, mut xp) = (level.max(1), xp + gained);
    let start = level;
    while xp >= xp_to_next(level) {
        xp -= xp_to_next(level);
        level += 1;
    }
    (level, xp, level - start)
}

/// Time XP for agents without richer signals: 1 XP per 10 s working, capped at 60 per
/// quest (SPEC §7.1). The cap lands at 10 min, before the 20 min "halve the rate" rule
/// could ever apply, so that rule is moot.
pub fn time_xp(working_ms: u64) -> u64 {
    (working_ms / 10_000).min(60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curve() {
        assert_eq!(xp_to_next(1), 100);
        assert_eq!(xp_to_next(2), 283);
        assert_eq!(xp_to_next(4), 800);
        assert_eq!(xp_to_next(0), 100);
    }

    #[test]
    fn time_xp_caps_at_60() {
        assert_eq!(time_xp(9_999), 0);
        assert_eq!(time_xp(10_000), 1);
        assert_eq!(time_xp(3_600_000), 60);
    }

    #[test]
    fn level_ups_carry_remainder() {
        assert_eq!(add_xp(1, 0, 99), (1, 99, 0));
        assert_eq!(add_xp(1, 99, 1), (2, 0, 1));
        assert_eq!(add_xp(1, 0, 100 + 283 + 5), (3, 5, 2));
    }
}
