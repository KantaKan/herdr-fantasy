//! Which stage screen is up and which hero is selected (SPEC §5.2 keys). Pure.

use crossterm::event::KeyCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Battle,
    Quests,
    Sheet,
    Town,
    Help,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct View {
    pub screen: Screen,
    /// Selected hero (footer `▶`, character sheet, town, Enter).
    pub selected: usize,
    /// Quest log scroll, in rows from the newest.
    pub scroll: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Key {
    Stay,
    /// Focus the selected hero's pane in herdr.
    Focus(usize),
}

impl Default for View {
    fn default() -> Self {
        View {
            screen: Screen::Battle,
            selected: 0,
            scroll: 0,
        }
    }
}

impl View {
    /// Handle one key press; `heroes` is the party size.
    pub fn key(&mut self, code: KeyCode, heroes: usize) -> Key {
        let toggle = |v: &mut View, s: Screen| {
            v.screen = if v.screen == s { Screen::Battle } else { s };
            v.scroll = 0;
        };
        match code {
            KeyCode::Char('t') => toggle(self, Screen::Town),
            KeyCode::Char('q') => toggle(self, Screen::Quests),
            KeyCode::Char('c') => toggle(self, Screen::Sheet),
            KeyCode::Char('?') => toggle(self, Screen::Help),
            KeyCode::Esc => self.screen = Screen::Battle,
            KeyCode::Left if heroes > 0 => self.selected = (self.selected + heroes - 1) % heroes,
            KeyCode::Right if heroes > 0 => self.selected = (self.selected + 1) % heroes,
            KeyCode::Down => self.scroll += 1,
            KeyCode::Up => self.scroll = self.scroll.saturating_sub(1),
            KeyCode::Enter if heroes > 0 => return Key::Focus(self.selected.min(heroes - 1)),
            _ => {}
        }
        Key::Stay
    }

    /// Keep the selection valid as heroes come and go.
    pub fn clamp(&mut self, heroes: usize) {
        self.selected = self.selected.min(heroes.saturating_sub(1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screens_toggle_and_esc_goes_back() {
        let mut v = View::default();
        v.key(KeyCode::Char('q'), 2);
        assert_eq!(v.screen, Screen::Quests);
        v.key(KeyCode::Char('q'), 2);
        assert_eq!(v.screen, Screen::Battle);
        v.key(KeyCode::Char('t'), 2);
        v.key(KeyCode::Char('c'), 2);
        assert_eq!(v.screen, Screen::Sheet);
        v.key(KeyCode::Esc, 2);
        assert_eq!(v.screen, Screen::Battle);
    }

    #[test]
    fn select_wraps_and_enter_focuses() {
        let mut v = View::default();
        v.key(KeyCode::Left, 3);
        assert_eq!(v.selected, 2);
        v.key(KeyCode::Right, 3);
        assert_eq!(v.selected, 0);
        assert_eq!(v.key(KeyCode::Enter, 3), Key::Focus(0));
        assert_eq!(v.key(KeyCode::Enter, 0), Key::Stay);
        v.selected = 5;
        v.clamp(2);
        assert_eq!(v.selected, 1);
    }
}
