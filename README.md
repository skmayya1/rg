# riff

An interactive terminal UI for searching a workspace with `ripgrep`.

## Install

Install ripgrep, clone the repository, then install from the cloned folder:

```sh
brew install ripgrep
git clone https://github.com/skmayya1/rg.git riff
cd riff
cargo install --path .
```

To update an existing installation after pulling changes, run
`cargo install --path . --force` from the cloned repository.

Run it from any workspace:

```sh
riff
riff /path/to/workspace
```

## Controls

- Type to search
- `↑`/`↓`, `Ctrl-P`/`Ctrl-N`: select a result
- `Enter`: copy the selected file's absolute path
- `y`: copy `path:line:column`
- `Esc`/`q`: quit

## Ignores

Default rules: [`config/default.toml`](config/default.toml).

Workspace override: `.riff.toml`

```toml
[ignore]
extensions = ["csv"]
directories = ["docs"]
files = ["*.min.js"]
```
