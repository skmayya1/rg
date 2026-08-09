//! Application state, selection, previews, and clipboard actions.

use std::{
    fs::File,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::{Context, Result, bail};

use crate::{
    config::SearchOptions,
    search::{MAX_RESULTS, SearchMatch, search},
};

const PREVIEW_CONTEXT: u64 = 12;
const PREVIEW_CAPACITY: usize = 25;

pub(crate) struct Preview {
    pub(crate) path: String,
    pub(crate) first_line: u64,
    pub(crate) focused_line: u64,
    pub(crate) lines: Vec<String>,
    pub(crate) message: Option<String>,
}

impl Default for Preview {
    fn default() -> Self {
        Self {
            path: String::new(),
            first_line: 1,
            focused_line: 0,
            lines: Vec::new(),
            message: Some("Select a match to preview its file".to_owned()),
        }
    }
}

pub(crate) struct App {
    root: PathBuf,
    search_options: SearchOptions,
    pub(crate) query: String,
    pub(crate) matches: Vec<SearchMatch>,
    pub(crate) selected: Option<usize>,
    pub(crate) preview: Preview,
    pub(crate) status: String,
}

impl App {
    pub(crate) fn new(root: PathBuf) -> Result<Self> {
        let search_options = SearchOptions::load(&root)?;
        Ok(Self {
            root,
            search_options,
            query: String::new(),
            matches: Vec::new(),
            selected: None,
            preview: Preview::default(),
            status: "Type to search".to_owned(),
        })
    }

    pub(crate) fn append_query(&mut self, character: char) {
        self.query.push(character);
        self.refresh();
    }

    pub(crate) fn delete_last_character(&mut self) {
        self.query.pop();
        self.refresh();
    }

    pub(crate) fn delete_previous_word(&mut self) {
        remove_previous_word(&mut self.query);
        self.refresh();
    }

    pub(crate) fn refresh(&mut self) {
        match search(&self.root, &self.search_options, &self.query) {
            Ok(matches) => self.set_matches(matches),
            Err(error) => {
                self.matches.clear();
                self.selected = None;
                self.update_preview();
                self.status = format!("Search failed: {error:#}");
            }
        }
    }

    pub(crate) fn select_previous(&mut self) {
        if let Some(index) = self.selected {
            self.selected = Some(index.saturating_sub(1));
            self.update_preview();
        }
    }

    pub(crate) fn select_next(&mut self) {
        if let (Some(index), Some(last_index)) = (self.selected, self.matches.len().checked_sub(1))
        {
            self.selected = Some(index.saturating_add(1).min(last_index));
            self.update_preview();
        }
    }

    pub(crate) fn copy_selected_file(&mut self) {
        let Some(result) = self.selected_match() else {
            return;
        };
        let path = self.root.join(&result.path).display().to_string();
        let status = match copy_to_clipboard(&path) {
            Ok(()) => format!("Copied file: {}", result.path),
            Err(error) => format!("Clipboard unavailable: {error:#}"),
        };
        self.status = status;
    }

    pub(crate) fn copy_selected_location(&mut self) {
        let Some(result) = self.selected_match() else {
            return;
        };
        let location = result.location();
        let status = match copy_to_clipboard(&location) {
            Ok(()) => format!("Copied {location}"),
            Err(error) => format!("Clipboard unavailable: {error:#}"),
        };
        self.status = status;
    }

    fn set_matches(&mut self, matches: Vec<SearchMatch>) {
        self.selected = self
            .selected
            .filter(|&index| index < matches.len())
            .or_else(|| (!matches.is_empty()).then_some(0));
        self.matches = matches;
        self.update_preview();
        self.status = if self.query.trim().is_empty() {
            "Type to search".to_owned()
        } else if self.matches.len() == MAX_RESULTS {
            format!("First {MAX_RESULTS} results shown")
        } else {
            format!("{} result(s)", self.matches.len())
        };
    }

    fn selected_match(&self) -> Option<&SearchMatch> {
        self.selected.and_then(|index| self.matches.get(index))
    }

    fn update_preview(&mut self) {
        self.preview = self
            .selected_match()
            .map_or_else(Preview::default, |result| load_preview(&self.root, result));
    }
}

fn load_preview(root: &Path, result: &SearchMatch) -> Preview {
    let first_line = result.line.saturating_sub(PREVIEW_CONTEXT).max(1);
    let last_line = result.line.saturating_add(PREVIEW_CONTEXT);
    let path = root.join(&result.path);
    let file = match File::open(&path) {
        Ok(file) => file,
        Err(error) => {
            return Preview {
                path: result.path.clone(),
                first_line,
                focused_line: result.line,
                lines: Vec::new(),
                message: Some(format!("Could not read file: {error}")),
            };
        }
    };

    let mut lines = Vec::with_capacity(PREVIEW_CAPACITY);
    for (line_number, line) in (1_u64..).zip(BufReader::new(file).lines()) {
        if line_number < first_line {
            continue;
        }
        if line_number > last_line {
            break;
        }
        lines.push(line.unwrap_or_else(|_| "<unreadable line>".to_owned()));
    }

    Preview {
        path: result.path.clone(),
        first_line,
        focused_line: result.line,
        lines,
        message: None,
    }
}

fn copy_to_clipboard(value: &str) -> Result<()> {
    let mut process = Command::new("pbcopy")
        .stdin(Stdio::piped())
        .spawn()
        .context("could not start pbcopy")?;
    let mut stdin = process
        .stdin
        .take()
        .context("pbcopy did not expose a standard input")?;
    stdin
        .write_all(value.as_bytes())
        .context("could not write to pbcopy")?;
    drop(stdin);

    let status = process.wait().context("could not wait for pbcopy")?;
    if status.success() {
        Ok(())
    } else {
        bail!("pbcopy exited with {status}")
    }
}

fn remove_previous_word(query: &mut String) {
    let trimmed = query.trim_end();
    let new_length = trimmed
        .rsplit_once(char::is_whitespace)
        .map_or(0, |(before, _)| before.len());
    query.truncate(new_length);
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn removes_previous_word_and_trailing_whitespace() {
        let mut query = "find  these words  ".to_owned();

        remove_previous_word(&mut query);

        assert_eq!(query, "find  these");
    }

    #[test]
    fn selection_is_cleared_when_results_are_empty() {
        let mut app = App {
            root: PathBuf::from("."),
            search_options: SearchOptions::default(),
            query: String::new(),
            matches: Vec::new(),
            selected: Some(3),
            preview: Preview::default(),
            status: String::new(),
        };

        app.set_matches(Vec::new());

        assert_eq!(app.selected, None);
        assert!(app.preview.message.is_some());
    }
}
