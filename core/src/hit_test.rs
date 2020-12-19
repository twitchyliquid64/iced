use crate::Vector;

/// The result of hit testing on text.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HitTestResult {
    /// The point was within the bounds of the returned character index.
    CharOffset(usize),
    /// The provided point was not within the bounds of a glyph. The index
    /// of the character with the closest centeroid position is returned,
    /// as well as its delta.
    NearestCharOffset(usize, Vector),
}

impl HitTestResult {
    /// Computes the cursor position corresponding to this [`HitTestResult`] .
    pub fn cursor(&self) -> usize {
        match self {
            HitTestResult::CharOffset(i) => *i,
            HitTestResult::NearestCharOffset(i, delta) => {
                if delta.x > f32::EPSILON {
                    i + 1
                } else {
                    *i
                }
            }
        }
    }
}
