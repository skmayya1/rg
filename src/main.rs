use anyhow::Result;

fn main() -> Result<()> {
    riff::run(riff::workspace_root()?)
}
