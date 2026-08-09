//! Terminal UI for searching a workspace with ripgrep.

mod app;
mod config;
mod search;
mod terminal;
mod ui;

use std::{env, path::PathBuf};

use anyhow::{Context, Result, bail};

use app::App;

/// Resolves the optional workspace argument or the current directory.
///
/// # Errors
///
/// Returns an error when the workspace does not exist, is not a directory, or
/// more than one workspace argument is supplied.
pub fn workspace_root() -> Result<PathBuf> {
    let mut arguments = env::args_os().skip(1);
    let root = match (arguments.next(), arguments.next()) {
        (None, None) => env::current_dir().context("could not get the current directory")?,
        (Some(path), None) => PathBuf::from(path),
        (_, Some(_)) => bail!("usage: riff [workspace]"),
    };
    let root = root
        .canonicalize()
        .with_context(|| format!("workspace does not exist: {}", root.display()))?;
    if !root.is_dir() {
        bail!("workspace is not a directory: {}", root.display());
    }
    Ok(root)
}

/// Runs the interactive find-in-files terminal UI for `root`.
///
/// # Errors
///
/// Returns an error when the workspace configuration, terminal raw mode, the
/// alternate screen, or terminal drawing cannot be initialized or restored.
pub fn run(root: PathBuf) -> Result<()> {
    let mut app = App::new(root)?;
    terminal::run(&mut app)
}
