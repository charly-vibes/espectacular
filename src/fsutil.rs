use anyhow::Context;
use std::fs;
use std::path::Path;

pub fn write_text(path: &Path, content: impl AsRef<[u8]>) -> anyhow::Result<()> {
    fs::write(path, content).with_context(|| format!("cannot write {}", path.display()))
}
