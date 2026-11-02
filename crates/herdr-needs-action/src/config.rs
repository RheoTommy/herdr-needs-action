//! The user's settings, in `config.toml` under the plugin's config directory.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// The text a marked pane shows as `$needs_action`.
    pub label: String,
}

impl Default for Config {
    fn default() -> Self {
        // A glyph rather than words, which crowd an Agent sidebar row. Not
        // the flag emoji, which terminals draw two cells wide.
        Self {
            label: "⚑".to_owned(),
        }
    }
}

impl Config {
    pub fn load(config_dir: &Path) -> Result<Self> {
        let path = config_dir.join("config.toml");
        match fs::read_to_string(&path) {
            Ok(text) => Self::parse(&text).with_context(|| format!("in {}", path.display())),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e).with_context(|| format!("cannot read {}", path.display())),
        }
    }

    fn parse(text: &str) -> Result<Self> {
        Ok(toml::from_str(text)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_file_keeps_the_defaults() {
        assert_eq!(Config::parse("").unwrap(), Config::default());
    }

    #[test]
    fn reads_the_label() {
        assert_eq!(
            Config::parse(r#"label = "要対応""#).unwrap().label,
            "要対応"
        );
    }

    #[test]
    fn rejects_a_misspelled_key() {
        assert!(Config::parse(r#"lable = "x""#).is_err());
    }

    #[test]
    fn a_missing_file_keeps_the_defaults() {
        // create_dir fails on an existing directory, so this one is new and
        // empty, whatever an earlier run or anyone else left behind.
        let dir =
            std::env::temp_dir().join(format!("herdr-needs-action-test-{}", std::process::id()));
        fs::create_dir(&dir).unwrap();
        let config = Config::load(&dir);
        fs::remove_dir(&dir).unwrap();
        assert_eq!(config.unwrap(), Config::default());
    }
}
