//! Body motion that makes heroes feel alive (issue #9): dashes, hops, sway, dust. Pure
//! offsets from time; they only decorate real work and are off under `reduced_motion`.

use crate::game::hero::Action;

/// Columns a hero moves toward the enemy while `action` plays (`t` = 0.0–1.0).
/// Attacks dash in fast and drift back; held props take a small step; spells recoil.
pub fn lunge(action: Action, t: f32) -> i32 {
    let t = t.clamp(0.0, 1.0);
    // Fast out (first 30%), slow return.
    let arc = |reach: f32| {
        let k = if t < 0.3 { t / 0.3 } else { (1.0 - t) / 0.7 };
        (reach * k).round() as i32
    };
    match action {
        Action::Attack | Action::Craft => arc(5.0),
        Action::Open | Action::Study | Action::Scroll | Action::Scout | Action::Seal => arc(1.0),
        Action::Fireball | Action::Bird | Action::PigeonOut => -arc(1.0),
        _ => 0,
    }
}

/// Pixels up for an occasional hop while running: every 6–9 s per hero, a 400 ms arc
/// up to 3 px.
pub fn hop(clock_ms: u64, seed: u64) -> i32 {
    let period = 6_000 + (seed * 1_337) % 3_000;
    let phase = (clock_ms + seed * 2_111) % period;
    if phase >= 400 {
        return 0;
    }
    let x = phase as f32 / 400.0;
    (12.0 * x * (1.0 - x)).round() as i32 // peaks at 3 px
}

/// A gentle −1…+1 column sway so heroes don't stand on a rigid grid.
pub fn sway(clock_ms: u64, seed: u64) -> i32 {
    match ((clock_ms + seed * 977) / 900) % 4 {
        1 => 1,
        3 => -1,
        _ => 0,
    }
}

/// Dust puff pixels behind the feet for this walk-cycle time: `(dx, dy)` relative to the
/// sprite's top-left, newest first. Empty between strides.
pub fn dust(anim_ms: u64) -> &'static [(i32, i32)] {
    // The run cycle is 4 frames × 100 ms; strides land on frames 0 and 2.
    match anim_ms % 200 {
        0..=59 => &[(2, 15)],
        60..=139 => &[(1, 14), (2, 15)],
        _ => &[(0, 13)],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attack_dashes_in_and_comes_back() {
        assert_eq!(lunge(Action::Attack, 0.0), 0);
        assert_eq!(lunge(Action::Attack, 0.3), 5, "peak");
        assert!(lunge(Action::Attack, 0.6) < 5);
        assert_eq!(lunge(Action::Attack, 1.0), 0);
        assert_eq!(lunge(Action::Fireball, 0.3), -1, "recoil");
        assert_eq!(lunge(Action::Doze, 0.3), 0);
    }

    #[test]
    fn hops_are_rare_short_and_low() {
        let ups: Vec<i32> = (0..20_000).step_by(10).map(|t| hop(t, 1)).collect();
        assert!(ups.iter().all(|&h| (0..=3).contains(&h)));
        let airborne = ups.iter().filter(|&&h| h > 0).count();
        assert!(airborne > 0 && airborne < ups.len() / 10, "{airborne}");
        let other: Vec<i32> = (0..20_000).step_by(10).map(|t| hop(t, 2)).collect();
        assert_ne!(ups, other, "heroes don't hop in lockstep");
    }

    #[test]
    fn sway_stays_within_one_column() {
        assert!((0..10_000).all(|t| sway(t, 3).abs() <= 1));
    }

    #[test]
    fn dust_follows_the_stride() {
        assert_eq!(dust(0), &[(2, 15)]);
        assert_eq!(dust(250).len(), 1);
    }
}
