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
    fn level_ups_carry_remainder() {
        assert_eq!(add_xp(1, 0, 99), (1, 99, 0));
        assert_eq!(add_xp(1, 99, 1), (2, 0, 1));
        assert_eq!(add_xp(1, 0, 100 + 283 + 5), (3, 5, 2));
    }
}
