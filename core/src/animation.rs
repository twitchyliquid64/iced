use std::cmp::Ordering;
use std::time::Instant;

/// Animation requirements of a widget.
///
/// NotAnimating is greater than any value of AnimateIn. This allows the use of min() to reduce
/// a set of [`AnimationState`] values into a value representing the soonest needed animation.
///
/// [`AnimationState`]: struct.AnimationState.html
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationState {
    /// The widget is not animating. It will only change in response to events or user interaction.
    NotAnimating,

    /// The widget needs to animate itself at the provided moment.
    AnimateIn(Instant),
}

impl Ord for AnimationState {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (AnimationState::NotAnimating, AnimationState::NotAnimating) => {
                Ordering::Equal
            }
            (_, AnimationState::NotAnimating) => Ordering::Less,
            (AnimationState::NotAnimating, _) => Ordering::Greater,
            (AnimationState::AnimateIn(a), AnimationState::AnimateIn(b)) => {
                a.cmp(b)
            }
        }
    }
}

impl PartialOrd for AnimationState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn ordering() {
        let now = Instant::now();
        let (less, more) = (
            now.checked_add(Duration::from_millis(1)).unwrap(),
            now.checked_add(Duration::from_millis(10)).unwrap(),
        );

        // Eq
        assert_eq!(AnimationState::NotAnimating, AnimationState::NotAnimating);
        assert_eq!(
            AnimationState::AnimateIn(now),
            AnimationState::AnimateIn(now)
        );

        // PartialOrd
        assert!(AnimationState::AnimateIn(now) < AnimationState::NotAnimating);
        assert!(AnimationState::NotAnimating > AnimationState::AnimateIn(now));
        assert!(
            AnimationState::AnimateIn(less) < AnimationState::AnimateIn(more)
        );
        assert!(
            AnimationState::AnimateIn(more) > AnimationState::AnimateIn(less)
        );

        // Ord
        assert!(AnimationState::AnimateIn(now) <= AnimationState::NotAnimating);
        assert!(AnimationState::NotAnimating >= AnimationState::AnimateIn(now));
        assert!(
            AnimationState::AnimateIn(less) <= AnimationState::AnimateIn(more)
        );
        assert!(
            AnimationState::AnimateIn(more) >= AnimationState::AnimateIn(more)
        );
        assert!(
            AnimationState::AnimateIn(now) <= AnimationState::AnimateIn(now)
        );
        assert!(
            AnimationState::AnimateIn(now) >= AnimationState::AnimateIn(now)
        );
    }
}
