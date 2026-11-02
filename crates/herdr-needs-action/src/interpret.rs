//! Reads herdr's state as the domain's, and writes the domain's state back as
//! pane tokens.
//!
//! The mark and the last activity live in pane tokens rather than in the
//! plugin's state directory: herdr drops tokens when the pane closes and does
//! not restore them after a server restart, which is the lifetime the mark
//! should have.

use std::collections::BTreeMap;

use anyhow::{Context, Result};
use needs_action::{Activity, Pane};
use serde::Deserialize;

/// Shown in the Agent sidebar as `$needs_action`.
pub const MARK_TOKEN: &str = "needs_action";
pub const LAST_TOKEN: &str = "needs_action_last";

pub type Tokens = BTreeMap<String, String>;

/// A token change: `Some` sets the value, `None` clears the token.
pub type Patch = Vec<(&'static str, Option<String>)>;

#[derive(Debug, PartialEq, Eq)]
pub struct StatusChange {
    pub pane_id: String,
    pub agent_status: String,
}

/// Parses `HERDR_PLUGIN_EVENT_JSON` of a `pane.agent_status_changed` hook.
pub fn status_change(event_json: &str) -> Result<StatusChange> {
    #[derive(Deserialize)]
    struct Envelope {
        data: Data,
    }
    #[derive(Deserialize)]
    struct Data {
        pane_id: String,
        agent_status: String,
    }
    let envelope: Envelope =
        serde_json::from_str(event_json).context("cannot parse the agent status event")?;
    Ok(StatusChange {
        pane_id: envelope.data.pane_id,
        agent_status: envelope.data.agent_status,
    })
}

/// `None` for `unknown`, which means herdr lost track of the agent for a
/// moment, not that the agent did anything.
pub fn activity(agent_status: &str) -> Option<Activity> {
    match agent_status {
        "working" => Some(Activity::Working),
        "blocked" => Some(Activity::Waiting),
        // done is idle that nobody has looked at yet. Whether someone looked
        // is exactly what the mark ignores.
        "idle" | "done" => Some(Activity::Resting),
        _ => None,
    }
}

pub fn pane(tokens: &Tokens) -> Pane {
    Pane {
        marked: tokens.contains_key(MARK_TOKEN),
        last: tokens.get(LAST_TOKEN).and_then(|v| parse_activity(v)),
    }
}

/// The token changes that turn `before` into `after`.
pub fn patch(before: Pane, after: Pane, label: &str) -> Patch {
    let mut patch = Patch::new();
    if before.marked != after.marked {
        patch.push((MARK_TOKEN, after.marked.then(|| label.to_owned())));
    }
    if before.last != after.last {
        patch.push((LAST_TOKEN, after.last.map(|a| activity_name(a).to_owned())));
    }
    patch
}

fn activity_name(activity: Activity) -> &'static str {
    match activity {
        Activity::Working => "working",
        Activity::Waiting => "waiting",
        Activity::Resting => "resting",
    }
}

fn parse_activity(name: &str) -> Option<Activity> {
    [Activity::Working, Activity::Waiting, Activity::Resting]
        .into_iter()
        .find(|&a| activity_name(a) == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(pairs: &[(&str, &str)]) -> Tokens {
        pairs
            .iter()
            .map(|&(k, v)| (k.to_owned(), v.to_owned()))
            .collect()
    }

    #[test]
    fn parses_the_event_that_herdr_0_9_5_sends() {
        let json = r#"{"event":"pane_agent_status_changed","data":{"type":"pane_agent_status_changed","pane_id":"w1:p1","workspace_id":"w1","agent_status":"idle","agent":"claude"}}"#;
        assert_eq!(
            status_change(json).unwrap(),
            StatusChange {
                pane_id: "w1:p1".to_owned(),
                agent_status: "idle".to_owned(),
            }
        );
    }

    #[test]
    fn rejects_an_event_without_a_status() {
        assert!(status_change(r#"{"event":"pane_focused","data":{"pane_id":"w1:p1"}}"#).is_err());
    }

    #[test]
    fn reads_idle_and_done_alike() {
        assert_eq!(activity("idle"), Some(Activity::Resting));
        assert_eq!(activity("done"), Some(Activity::Resting));
    }

    #[test]
    fn reads_blocked_as_waiting() {
        assert_eq!(activity("blocked"), Some(Activity::Waiting));
    }

    #[test]
    fn ignores_unknown_and_unrecognized_statuses() {
        assert_eq!(activity("unknown"), None);
        assert_eq!(activity("sleeping"), None);
    }

    #[test]
    fn a_pane_without_tokens_is_unmarked_and_unobserved() {
        assert_eq!(pane(&Tokens::new()), Pane::default());
    }

    #[test]
    fn reads_the_mark_whatever_its_label() {
        assert!(pane(&tokens(&[(MARK_TOKEN, "要対応")])).marked);
    }

    #[test]
    fn ignores_a_last_activity_it_does_not_know() {
        assert_eq!(pane(&tokens(&[(LAST_TOKEN, "flying")])).last, None);
    }

    #[test]
    fn writes_what_it_reads_back() {
        let written = Pane {
            marked: true,
            last: Some(Activity::Waiting),
        };
        let applied: Tokens = patch(Pane::default(), written, "needs action")
            .into_iter()
            .filter_map(|(k, v)| Some((k.to_owned(), v?)))
            .collect();
        assert_eq!(pane(&applied), written);
    }

    #[test]
    fn sets_the_mark_with_the_label() {
        let p = patch(Pane::default(), Pane::default().mark(), "要対応");
        assert_eq!(p, vec![(MARK_TOKEN, Some("要対応".to_owned()))]);
    }

    #[test]
    fn clears_the_mark() {
        let marked = Pane::default().mark();
        assert_eq!(patch(marked, marked.clear(), "x"), vec![(MARK_TOKEN, None)]);
    }

    #[test]
    fn writes_nothing_when_nothing_changed() {
        let pane = Pane::default().mark().observe(Activity::Resting);
        assert!(patch(pane, pane, "x").is_empty());
    }
}
