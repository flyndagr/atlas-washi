//! Independent panels, with Focus restoring the previous layout.
#[derive(Clone, Debug)]
pub struct Panels {
    pub sidebar: bool,
    pub inspector: bool,
    restore: (bool, bool),
}
impl Default for Panels {
    fn default() -> Self {
        Self {
            sidebar: true,
            inspector: true,
            restore: (true, true),
        }
    }
}
impl Panels {
    pub fn focused(&self) -> bool {
        !self.sidebar && !self.inspector
    }
    pub fn toggle_sidebar(&mut self) {
        self.sidebar = !self.sidebar;
    }
    pub fn toggle_focus(&mut self) {
        if self.focused() {
            (self.sidebar, self.inspector) = self.restore;
        } else {
            self.restore = (self.sidebar, self.inspector);
            self.sidebar = false;
            self.inspector = false;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sidebar_leaves_inspector_and_focus_restores_layout() {
        let mut p = Panels::default();
        p.toggle_sidebar();
        assert!(!p.sidebar && p.inspector && !p.focused());
        p.toggle_focus();
        assert!(p.focused());
        p.toggle_focus();
        assert!(!p.sidebar && p.inspector);
        p.toggle_sidebar();
        assert!(p.sidebar && p.inspector);
    }
    #[test]
    fn sidebar_from_focus_only_reveals_sidebar() {
        let mut p = Panels::default();
        p.toggle_focus();
        p.toggle_sidebar();
        assert!(p.sidebar && !p.inspector);
    }
}
