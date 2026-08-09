//! Workspace configuration and ripgrep ignore rules.

use std::{fs, path::Path};

use anyhow::{Context, Result};
use serde::Deserialize;

const CONFIG_FILE_NAME: &str = ".riff.toml";
const DEFAULT_CONFIG: &str = include_str!("../config/default.toml");

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkspaceConfig {
    #[serde(default)]
    ignore: IgnoreConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct IgnoreConfig {
    /// File extensions to exclude. Both `md` and `.md` are accepted.
    #[serde(default)]
    extensions: Vec<String>,
    /// Directory paths, relative to the workspace, to exclude recursively.
    #[serde(default)]
    directories: Vec<String>,
    /// File names or ripgrep glob patterns to exclude.
    #[serde(default)]
    files: Vec<String>,
}

impl IgnoreConfig {
    fn extend(&mut self, additional: Self) {
        self.extensions.extend(additional.extensions);
        self.directories.extend(additional.directories);
        self.files.extend(additional.files);
    }
}

#[derive(Default)]
pub(crate) struct SearchOptions {
    ignored_globs: Vec<String>,
}

impl SearchOptions {
    pub(crate) fn load(root: &Path) -> Result<Self> {
        let config_path = root.join(CONFIG_FILE_NAME);
        let mut config: WorkspaceConfig = toml::from_str(DEFAULT_CONFIG)
            .context("could not parse bundled default configuration")?;
        if config_path
            .try_exists()
            .with_context(|| format!("could not inspect {}", config_path.display()))?
        {
            let contents = fs::read_to_string(&config_path)
                .with_context(|| format!("could not read {}", config_path.display()))?;
            let workspace_config: WorkspaceConfig = toml::from_str(&contents)
                .with_context(|| format!("could not parse {}", config_path.display()))?;
            config.ignore.extend(workspace_config.ignore);
        }

        Ok(Self {
            ignored_globs: ignore_globs(&config.ignore),
        })
    }

    pub(crate) fn ignored_globs(&self) -> impl Iterator<Item = &str> {
        self.ignored_globs.iter().map(String::as_str)
    }
}

fn ignore_globs(config: &IgnoreConfig) -> Vec<String> {
    let mut globs =
        Vec::with_capacity(config.directories.len() + config.extensions.len() + config.files.len());
    globs.extend(
        config
            .directories
            .iter()
            .filter_map(|directory| directory_glob(directory)),
    );
    globs.extend(
        config
            .extensions
            .iter()
            .filter_map(|extension| extension_glob(extension)),
    );
    globs.extend(config.files.iter().filter_map(|file| file_glob(file)));
    globs.sort_unstable();
    globs.dedup();
    globs
}

fn directory_glob(directory: &str) -> Option<String> {
    let directory = directory.trim().trim_matches('/');
    (!directory.is_empty()).then(|| format!("!{directory}/**"))
}

fn extension_glob(extension: &str) -> Option<String> {
    let extension = extension.trim().trim_start_matches('.');
    (!extension.is_empty()).then(|| format!("!*.{extension}"))
}

fn file_glob(file: &str) -> Option<String> {
    let file = file.trim();
    (!file.is_empty()).then(|| format!("!{file}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_globs_for_configured_extensions_directories_and_files() {
        let config = IgnoreConfig {
            extensions: vec!["log".to_owned(), ".md".to_owned()],
            directories: vec!["docs/".to_owned()],
            files: vec!["package-lock.json".to_owned()],
        };

        let globs = ignore_globs(&config);

        assert!(globs.contains(&"!*.log".to_owned()));
        assert!(globs.contains(&"!*.md".to_owned()));
        assert!(globs.contains(&"!docs/**".to_owned()));
        assert!(globs.contains(&"!package-lock.json".to_owned()));
    }

    #[test]
    fn bundled_toml_contains_the_default_ignore_rules() {
        let config: WorkspaceConfig =
            toml::from_str(DEFAULT_CONFIG).expect("bundled default configuration is valid");
        let globs = ignore_globs(&config.ignore);

        assert!(globs.contains(&"!*.log".to_owned()));
        assert!(globs.contains(&"!*.md".to_owned()));
        assert!(globs.contains(&"!*.lock".to_owned()));
        assert!(globs.contains(&"!pnpm-lock.yaml".to_owned()));
        assert!(globs.contains(&"!bun.lockb".to_owned()));
        assert!(globs.contains(&"!node_modules/**".to_owned()));
        assert!(globs.contains(&"!log/**".to_owned()));
    }

    #[test]
    fn rejects_unknown_config_keys() {
        let config = "[ignore]\nextensions = [\"md\"]\nunknown = []\n";

        assert!(toml::from_str::<WorkspaceConfig>(config).is_err());
    }
}
