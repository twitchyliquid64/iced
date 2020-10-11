//! Track the cursor of a text input.
use crate::{AnimationState, widget::text_input::Value};

use std::time::{Duration, Instant};
const BLINK_MAX: Duration = Duration::from_secs(10);

/// The cursor of a text input.
#[derive(Debug, Copy, Clone)]
pub struct Cursor {
    state: State,
    last_ui_update: Instant,
}

/// The state of a [`Cursor`].
///
/// [`Cursor`]: struct.Cursor.html
#[derive(Debug, Copy, Clone)]
pub enum State {
    /// Cursor without a selection
    Index(usize),

    /// Cursor selecting a range of text
    Selection {
        /// The start of the selection
        start: usize,
        /// The end of the selection
        end: usize,
    },
}

impl Default for Cursor {
    fn default() -> Self {
        Cursor {
            state: State::Index(0),
            last_ui_update: Instant::now(),
        }
    }
}

impl Cursor {
    /// Returns the [`State`] of the [`Cursor`].
    ///
    /// [`State`]: struct.State.html
    /// [`Cursor`]: struct.Cursor.html
    pub fn state(&self, value: &Value) -> State {
        match self.state {
            State::Index(index) => State::Index(index.min(value.len())),
            State::Selection { start, end } => {
                let start = start.min(value.len());
                let end = end.min(value.len());

                if start == end {
                    State::Index(start)
                } else {
                    State::Selection { start, end }
                }
            }
        }
    }

    /// Returns true if the cursor should be visible. Conversely, a false
    /// value indicates that the cursor should be hidden, as it is at the
    /// invisible phase of blinking.
    pub fn blink_visible(&self) -> bool {
        let since = self.last_ui_update.elapsed();

        if since > BLINK_MAX {
            true
        } else {
            (since.subsec_millis() / 500) == 0
        }
    }

    /// Returns the animation requirements of the cursor.
    pub(crate) fn next_animation(&self) -> AnimationState {
        match self.state {
            State::Index(_) => {
                if self.last_ui_update.elapsed() > BLINK_MAX {
                    AnimationState::NotAnimating
                } else {
                    AnimationState::AnimateIn(Instant::now() + Duration::from_millis(500))
                }
            },
            _ => AnimationState::NotAnimating,
        }
    }

    /// Updates the last_ui_update state when the input is clicked.
    pub(crate) fn on_click(&mut self) {
        self.last_ui_update = Instant::now();
    }

    pub(crate) fn move_to(&mut self, position: usize) {
        self.last_ui_update = Instant::now();
        self.state = State::Index(position);
    }

    pub(crate) fn move_right(&mut self, value: &Value) {
        self.move_right_by_amount(value, 1)
    }

    pub(crate) fn move_right_by_words(&mut self, value: &Value) {
        self.move_to(value.next_end_of_word(self.right(value)))
    }

    pub(crate) fn move_right_by_amount(
        &mut self,
        value: &Value,
        amount: usize,
    ) {
        match self.state(value) {
            State::Index(index) => {
                self.move_to(index.saturating_add(amount).min(value.len()))
            }
            State::Selection { start, end } => self.move_to(end.max(start)),
        }
    }

    pub(crate) fn move_left(&mut self, value: &Value) {
        match self.state(value) {
            State::Index(index) if index > 0 => self.move_to(index - 1),
            State::Selection { start, end } => self.move_to(start.min(end)),
            _ => self.move_to(0),
        }
    }

    pub(crate) fn move_left_by_words(&mut self, value: &Value) {
        self.move_to(value.previous_start_of_word(self.left(value)));
    }

    pub(crate) fn select_range(&mut self, start: usize, end: usize) {
        if start == end {
            self.state = State::Index(start);
        } else {
            self.state = State::Selection { start, end };
        }
        self.last_ui_update = Instant::now();
    }

    pub(crate) fn select_left(&mut self, value: &Value) {
        match self.state(value) {
            State::Index(index) if index > 0 => {
                self.select_range(index, index - 1)
            }
            State::Selection { start, end } if end > 0 => {
                self.select_range(start, end - 1)
            }
            _ => (),
        }
    }

    pub(crate) fn select_right(&mut self, value: &Value) {
        match self.state(value) {
            State::Index(index) if index < value.len() => {
                self.select_range(index, index + 1)
            }
            State::Selection { start, end } if end < value.len() => {
                self.select_range(start, end + 1)
            }
            _ => (),
        }
    }

    pub(crate) fn select_left_by_words(&mut self, value: &Value) {
        match self.state(value) {
            State::Index(index) => {
                self.select_range(index, value.previous_start_of_word(index))
            }
            State::Selection { start, end } => {
                self.select_range(start, value.previous_start_of_word(end))
            }
        }
    }

    pub(crate) fn select_right_by_words(&mut self, value: &Value) {
        match self.state(value) {
            State::Index(index) => {
                self.select_range(index, value.next_end_of_word(index))
            }
            State::Selection { start, end } => {
                self.select_range(start, value.next_end_of_word(end))
            }
        }
    }

    pub(crate) fn select_all(&mut self, value: &Value) {
        self.select_range(0, value.len());
    }

    pub(crate) fn start(&self, value: &Value) -> usize {
        let start = match self.state {
            State::Index(index) => index,
            State::Selection { start, .. } => start,
        };

        start.min(value.len())
    }

    pub(crate) fn end(&self, value: &Value) -> usize {
        let end = match self.state {
            State::Index(index) => index,
            State::Selection { end, .. } => end,
        };

        end.min(value.len())
    }

    pub(crate) fn selection(&self, value: &Value) -> Option<(usize, usize)> {
        match self.state(value) {
            State::Selection { start, end } => {
                Some((start.min(end), start.max(end)))
            }
            _ => None,
        }
    }

    fn left(&self, value: &Value) -> usize {
        match self.state(value) {
            State::Index(index) => index,
            State::Selection { start, end } => start.min(end),
        }
    }

    fn right(&self, value: &Value) -> usize {
        match self.state(value) {
            State::Index(index) => index,
            State::Selection { start, end } => start.max(end),
        }
    }
}
