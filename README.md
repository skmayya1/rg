# rgfind

A Neovim-inspired terminal picker for searching a workspace with `ripgrep`.

## Stack

- Rust 2024
- [`ratatui`](https://ratatui.rs/) for layout and widgets
- [`crossterm`](https://github.com/crossterm-rs/crossterm) for terminal input
- `ripgrep` (`rg`) for content search

## Development

```sh
rgfind /path/to/workspace
```

Before changing the TUI, run the same checks enforced by the crate's Rust
quality settings:

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

## Controls

- Type to search (`rg --smart-case`)
- `↑`/`↓`, `Ctrl-P`/`Ctrl-N`: select a result
- `Enter`: copy the selected file's absolute path
- `y`: copy `path:line:column`
- `Esc`/`q`: quit

The UI follows Telescope's picker style: a compact prompt, a 40% file-match
list on the left, and a 60% source preview on the right. The selected result's
line is kept in the middle of the preview and highlighted in gray.

The ChatGPT/Codex desktop app does not expose a public command to open a file
in an existing Codex chat's tab. `codex-find` therefore never sends a file to
macOS `open`, which would attach it to a new chat. Instead, use the built-in
preview and paste the copied `path:line:column` into the Codex chat you already
have open when you need to reference that exact location.

# rg
