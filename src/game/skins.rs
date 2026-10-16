//! Hero skins (TASKS M14): costume recolors unlocked by level. Pure.

pub struct Skin {
    pub name: &'static str,
    pub level: u32,
    /// Palette key swaps, applied all at once.
    pub swaps: &'static [(char, char)],
}

pub const SKINS: &[Skin] = &[
    Skin {
        name: "Verdant",
        level: 5,
        swaps: &[('b', 'g'), ('n', 't'), ('c', 'G'), ('C', 'G')],
    },
    Skin {
        name: "Crimson",
        level: 10,
        swaps: &[
            ('b', 'r'),
            ('n', 'p'),
            ('c', 'o'),
            ('C', 'y'),
            ('g', 'r'),
            ('t', 'p'),
            ('G', 'o'),
        ],
    },
    Skin {
        name: "Royal Gold",
        level: 20,
        swaps: &[
            ('s', 'y'),
            ('S', 'o'),
            ('b', 'p'),
            ('n', 'd'),
            ('c', 'r'),
            ('g', 'p'),
            ('t', 'd'),
        ],
    },
];

/// The best skin unlocked at `level`, if any.
pub fn for_level(level: u32) -> Option<&'static Skin> {
    SKINS.iter().rev().find(|s| level >= s.level)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unlocks_by_level() {
        assert!(for_level(4).is_none());
        assert_eq!(for_level(5).unwrap().name, "Verdant");
        assert_eq!(for_level(19).unwrap().name, "Crimson");
        assert_eq!(for_level(40).unwrap().name, "Royal Gold");
    }
}
