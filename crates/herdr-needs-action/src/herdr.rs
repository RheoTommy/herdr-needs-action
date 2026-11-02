//! The plugin's runtime environment and the herdr CLI calls it makes.

use std::env;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::interpret::{Patch, Tokens};

/// What herdr passes to every plugin command.
pub struct Plugin {
    bin: PathBuf,
    source: String,
    pub state_dir: PathBuf,
    pub config_dir: PathBuf,
}

impl Plugin {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            bin: required("HERDR_BIN_PATH")?.into(),
            source: format!("plugin:{}", required("HERDR_PLUGIN_ID")?),
            state_dir: required("HERDR_PLUGIN_STATE_DIR")?.into(),
            config_dir: required("HERDR_PLUGIN_CONFIG_DIR")?.into(),
        })
    }

    pub fn pane_tokens(&self, pane_id: &str) -> Result<Tokens> {
        #[derive(Deserialize)]
        struct Response {
            result: Body,
        }
        #[derive(Deserialize)]
        struct Body {
            pane: Pane,
        }
        #[derive(Deserialize)]
        struct Pane {
            #[serde(default)]
            tokens: Tokens,
        }
        let stdout = self.run(["pane", "get", pane_id])?;
        let response: Response = serde_json::from_slice(&stdout)
            .with_context(|| format!("cannot parse `herdr pane get {pane_id}`"))?;
        Ok(response.result.pane.tokens)
    }

    pub fn patch_pane_tokens(&self, pane_id: &str, patch: &Patch) -> Result<()> {
        let mut args = vec![
            "pane".to_owned(),
            "report-metadata".to_owned(),
            pane_id.to_owned(),
            "--source".to_owned(),
            self.source.clone(),
        ];
        for (name, value) in patch {
            match value {
                Some(value) => args.extend(["--token".to_owned(), format!("{name}={value}")]),
                None => args.extend(["--clear-token".to_owned(), (*name).to_owned()]),
            }
        }
        self.run(&args)?;
        Ok(())
    }

    fn run<I, S>(&self, args: I) -> Result<Vec<u8>>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut command = Command::new(&self.bin);
        command.args(args);
        let output = command
            .output()
            .with_context(|| format!("cannot run {}", self.bin.display()))?;
        if !output.status.success() {
            bail!(
                "{:?} failed with {}: {}",
                command,
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        Ok(output.stdout)
    }
}

pub fn required(name: &str) -> Result<String> {
    env::var(name).with_context(|| format!("{name} is not set; is herdr running this command?"))
}
