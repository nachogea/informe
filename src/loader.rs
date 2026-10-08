use crate::artifact::Artifact;
use std::{
    fs,
    path::Path,
    time::{Duration, Instant},
};

pub const POLL_INTERVAL: Duration = Duration::from_millis(250);

#[derive(Default)]
pub struct FileLoader {
    previous: Option<Result<Vec<u8>, String>>,
}

pub struct LoadResult {
    pub artifact: Result<Artifact, String>,
    pub elapsed: Duration,
}

impl FileLoader {
    /// Compare contents, not mtimes: handles rename-over saves and coarse timestamp resolution.
    /// An invalid snapshot is remembered too, so it is not reparsed every tick.
    pub fn poll(&mut self, path: &Path, force: bool) -> Option<LoadResult> {
        let start = Instant::now();
        let snapshot = fs::read(path).map_err(|e| format!("Cannot read {}: {e}", path.display()));
        if !force && self.previous.as_ref() == Some(&snapshot) {
            return None;
        }
        let artifact = match &snapshot {
            Ok(bytes) => Artifact::parse_document(bytes, path),
            Err(error) => Err(error.clone()),
        };
        self.previous = Some(snapshot);
        Some(LoadResult {
            artifact,
            elapsed: start.elapsed(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detects_changes_invalid_edits_atomic_saves_and_recreation() {
        let dir = std::env::temp_dir().join(format!(
            "informe-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&dir).unwrap();
        let path = dir.join("artifact.json");
        let valid = br#"{"version":1,"root":{"type":"text","id":"x","text":"a"}}"#;
        let mut loader = FileLoader::default();
        assert!(loader.poll(&path, false).unwrap().artifact.is_err());
        assert!(loader.poll(&path, false).is_none());
        fs::write(&path, valid).unwrap();
        assert!(loader.poll(&path, false).unwrap().artifact.is_ok());
        assert!(loader.poll(&path, false).is_none());
        assert!(loader.poll(&path, true).unwrap().artifact.is_ok());
        fs::write(&path, "{").unwrap();
        assert!(loader.poll(&path, false).unwrap().artifact.is_err());
        assert!(loader.poll(&path, false).is_none());
        let replacement = dir.join("replacement.json");
        fs::write(&replacement, valid).unwrap();
        fs::rename(&replacement, &path).unwrap();
        assert!(loader.poll(&path, false).unwrap().artifact.is_ok());
        fs::remove_file(&path).unwrap();
        assert!(loader.poll(&path, false).unwrap().artifact.is_err());
        fs::write(&path, valid).unwrap();
        assert!(loader.poll(&path, false).unwrap().artifact.is_ok());
        fs::remove_dir_all(dir).unwrap();
    }
}
