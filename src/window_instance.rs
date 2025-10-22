// Representing the instance of a Window.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WindowInstance {
    pub open: i64,
    pub close: i64,
    pub has_triggered_and_emitted: bool,
}

impl WindowInstance {
    pub fn new(open: i64, close: i64) -> Self {
        Self {
            open,
            close,
            has_triggered_and_emitted: false,
        }
    }

    pub fn set_triggered_and_emitted(&mut self, val: bool) {
        self.has_triggered_and_emitted = val;
    }

    pub fn is_same_window(&self, other: &WindowInstance) -> bool {
        self.open == other.open && self.close == other.close
    }
}
