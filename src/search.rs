//! Ripgrep invocation and JSON result parsing.

use std::{path::Path, process::Command};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::config::SearchOptions;

pub(crate) const MAX_RESULTS: usize = 500;

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum RipgrepEvent {
    Match {
        data: RipgrepMatch,
    },
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
struct RipgrepMatch {
    path: RipgrepText,
    line_number: u64,
    lines: RipgrepText,
    #[serde(default)]
    submatches: Vec<RipgrepSubmatch>,
}

#[derive(Debug, Deserialize)]
struct RipgrepText {
    text: String,
}

#[derive(Debug, Deserialize)]
struct RipgrepSubmatch {
    start: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct SearchMatch {
    pub(crate) path: String,
    pub(crate) line: u64,
    pub(crate) column: u64,
    pub(crate) text: String,
}

impl From<RipgrepMatch> for SearchMatch {
    fn from(result: RipgrepMatch) -> Self {
        Self {
            path: result.path.text.trim_start_matches("./").to_owned(),
            line: result.line_number,
            column: result
                .submatches
                .first()
                .map_or(1, |submatch| submatch.start.saturating_add(1)),
            text: result.lines.text.trim_end().to_owned(),
        }
    }
}

impl SearchMatch {
    pub(crate) fn location(&self) -> String {
        format!("{}:{}:{}", self.path, self.line, self.column)
    }
}

pub(crate) fn search(
    root: &Path,
    options: &SearchOptions,
    query: &str,
) -> Result<Vec<SearchMatch>> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }

    let max_results = MAX_RESULTS.to_string();
    let output = Command::new("rg")
        .args([
            "--json",
            "--line-number",
            "--column",
            "--smart-case",
            "--hidden",
            "--max-count",
            &max_results,
        ])
        .args(options.ignored_globs().flat_map(|glob| ["--glob", glob]))
        .args(["--", query, "."])
        .current_dir(root)
        .output()
        .context("could not start ripgrep; install rg and add it to PATH")?;

    if !matches!(output.status.code(), Some(0 | 1)) {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("ripgrep failed: {}", stderr.trim());
    }

    parse_ripgrep_output(&output.stdout)
}

fn parse_ripgrep_output(output: &[u8]) -> Result<Vec<SearchMatch>> {
    let mut matches = Vec::with_capacity(MAX_RESULTS);
    for line in output
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
    {
        let event: RipgrepEvent =
            serde_json::from_slice(line).context("could not parse ripgrep JSON output")?;
        if let RipgrepEvent::Match { data } = event {
            matches.push(data.into());
            if matches.len() == MAX_RESULTS {
                break;
            }
        }
    }
    Ok(matches)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_match_events_into_search_matches() {
        let output = br#"{"type":"begin","data":{}}
{"type":"match","data":{"path":{"text":"./src/main.rs"},"lines":{"text":"fn main() {}\n"},"line_number":7,"submatches":[{"start":3}]}}
{"type":"summary","data":{}}
"#;

        let matches = parse_ripgrep_output(output).expect("fixture is valid ripgrep JSON");

        assert_eq!(
            matches,
            vec![SearchMatch {
                path: "src/main.rs".to_owned(),
                line: 7,
                column: 4,
                text: "fn main() {}".to_owned(),
            }]
        );
    }
}
