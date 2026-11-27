use anyhow::Context;
use std::path::Path;

pub fn write_text(path: &Path, content: impl AsRef<[u8]>) -> anyhow::Result<()> {
    genesis::fs::atomic_write(path, content.as_ref())
        .with_context(|| format!("cannot write {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn write_text_writes_content_atomically() -> anyhow::Result<()> {
        let dir = TempDir::new()?;
        let target = dir.path().join("out.txt");
        write_text(&target, "hello")?;
        let content = std::fs::read_to_string(&target)?;
        assert_eq!(content, "hello");
        // No staging debris left behind in the target's directory.
        let leftovers: Vec<_> = std::fs::read_dir(dir.path())?
            .filter_map(std::result::Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(leftovers, vec!["out.txt".to_string()]);
        Ok(())
    }
}
