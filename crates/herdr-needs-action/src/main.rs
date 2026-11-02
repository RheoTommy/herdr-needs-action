//! The command that herdr runs for the plugin's event hook and actions.

mod config;
mod herdr;
mod interpret;

use std::fs::File;
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use needs_action::Pane;

use crate::config::Config;
use crate::herdr::Plugin;

const USAGE: &str = "usage: herdr-needs-action observe|mark|clear|toggle";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("herdr-needs-action: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let (Some(command), None) = (args.next(), args.next()) else {
        bail!(USAGE);
    };
    let plugin = Plugin::from_env()?;
    let change: fn(Pane) -> Pane = match command.as_str() {
        "observe" => return observe(&plugin),
        "mark" => Pane::mark,
        "clear" => Pane::clear,
        "toggle" => Pane::toggle,
        _ => bail!(USAGE),
    };
    update(&plugin, &herdr::required("HERDR_PANE_ID")?, change)
}

fn observe(plugin: &Plugin) -> Result<()> {
    let event = interpret::status_change(&herdr::required("HERDR_PLUGIN_EVENT_JSON")?)?;
    let Some(activity) = interpret::activity(&event.agent_status) else {
        return Ok(());
    };
    update(plugin, &event.pane_id, |pane| pane.observe(activity))
}

fn update(plugin: &Plugin, pane_id: &str, change: impl FnOnce(Pane) -> Pane) -> Result<()> {
    let config = Config::load(&plugin.config_dir)?;
    // herdr starts a hook for each event without waiting for the previous
    // one, and each run reads the pane's tokens and writes them back.
    let lock_path = plugin.state_dir.join("lock");
    let lock =
        File::create(&lock_path).with_context(|| format!("cannot open {}", lock_path.display()))?;
    lock.lock()
        .with_context(|| format!("cannot lock {}", lock_path.display()))?;

    let before = interpret::pane(&plugin.pane_tokens(pane_id)?);
    let patch = interpret::patch(before, change(before), &config.label);
    if !patch.is_empty() {
        plugin.patch_pane_tokens(pane_id, &patch)?;
    }
    Ok(())
}
