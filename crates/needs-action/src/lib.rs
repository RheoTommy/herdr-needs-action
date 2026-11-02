//! Tracks which agent panes need the user's action.
//!
//! A pane gets the mark when its agent finishes a turn, and keeps it until
//! the agent starts another turn or the user clears it. Looking at the pane
//! does not clear it, so a turn that was read but not yet handled stays
//! visible. This crate knows nothing about how the activity is observed or
//! where the mark is shown.

/// What an agent is doing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Activity {
    /// The agent is running a turn.
    Working,
    /// The agent stopped mid-turn to ask the user something, such as for a
    /// permission.
    Waiting,
    /// The agent has nothing to do until it gets a new prompt.
    Resting,
}

/// The mark of one pane, and the activity it was last seen in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Pane {
    pub marked: bool,
    /// `None` until the first activity is observed.
    pub last: Option<Activity>,
}

impl Pane {
    /// Records that the agent in this pane is now in `activity`.
    #[must_use]
    pub fn observe(self, activity: Activity) -> Self {
        let marked = match (self.last, activity) {
            (_, Activity::Working) => false,
            // Not from Waiting: an agent goes from Waiting to Resting when
            // the user turns its question down, and the user is already
            // handling the pane then.
            (Some(Activity::Working), Activity::Resting) => true,
            _ => self.marked,
        };
        Self {
            marked,
            last: Some(activity),
        }
    }

    #[must_use]
    pub fn mark(self) -> Self {
        Self {
            marked: true,
            ..self
        }
    }

    #[must_use]
    pub fn clear(self) -> Self {
        Self {
            marked: false,
            ..self
        }
    }

    #[must_use]
    pub fn toggle(self) -> Self {
        Self {
            marked: !self.marked,
            ..self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Activity::{Resting, Waiting, Working};
    use super::*;

    fn observe_all(pane: Pane, activities: &[Activity]) -> Pane {
        activities.iter().fold(pane, |pane, &a| pane.observe(a))
    }

    #[test]
    fn finishing_a_turn_marks_the_pane() {
        assert!(observe_all(Pane::default(), &[Working, Resting]).marked);
    }

    #[test]
    fn finishing_a_turn_after_answering_a_question_marks_the_pane() {
        assert!(observe_all(Pane::default(), &[Working, Waiting, Working, Resting]).marked);
    }

    #[test]
    fn starting_a_turn_clears_the_mark() {
        let marked = Pane::default().mark();
        assert!(!marked.observe(Working).marked);
    }

    #[test]
    fn resting_again_keeps_the_mark() {
        let finished = observe_all(Pane::default(), &[Working, Resting]);
        assert!(finished.observe(Resting).marked);
    }

    #[test]
    fn waiting_keeps_the_mark() {
        let marked = Pane::default().mark();
        assert!(marked.observe(Waiting).marked);
    }

    #[test]
    fn turning_a_question_down_does_not_mark_the_pane() {
        assert!(!observe_all(Pane::default(), &[Working, Waiting, Resting]).marked);
    }

    #[test]
    fn a_pane_first_seen_resting_is_not_marked() {
        assert!(!Pane::default().observe(Resting).marked);
    }

    #[test]
    fn observing_remembers_the_activity() {
        assert_eq!(Pane::default().observe(Waiting).last, Some(Waiting));
    }

    #[test]
    fn manual_changes_keep_the_last_activity() {
        let pane = Pane::default().observe(Working);
        assert_eq!(pane.mark().last, Some(Working));
        assert_eq!(pane.mark().clear().last, Some(Working));
        assert_eq!(pane.toggle().last, Some(Working));
    }

    #[test]
    fn toggle_flips_the_mark() {
        assert!(Pane::default().toggle().marked);
        assert!(!Pane::default().mark().toggle().marked);
    }

    #[test]
    fn a_manual_mark_survives_until_the_next_turn() {
        let pane = Pane::default().observe(Resting).mark();
        assert!(pane.observe(Resting).marked);
        assert!(!pane.observe(Working).marked);
    }
}
