use anyhow::Context;
use std::fs;
use std::path::Path;

pub fn write_text(path: &Path, content: impl AsRef<[u8]>) -> anyhow::Result<()> {
    fs::write(path, content).with_context(|| format!("cannot write {}", path.display()))
}

pub fn refresh_managed_file(
    path: &Path,
    block: &str,
    block_start: &str,
    block_end: &str,
) -> anyhow::Result<()> {
    let content = if path.exists() {
        let existing =
            fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))?;
        if existing.contains(block_start) {
            let before = existing
                .find(block_start)
                .map(|i| &existing[..i])
                .unwrap_or("");
            let after_end = existing
                .find(block_end)
                .map(|i| &existing[i + block_end.len()..])
                .unwrap_or("");
            format!("{}{}{}", before, block, after_end)
        } else {
            format!("{}\n\n{}\n", existing.trim_end(), block)
        }
    } else {
        block.to_string()
    };

    write_text(path, content)?;
    Ok(())
}
