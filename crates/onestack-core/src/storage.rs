use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("object not found: {0}")]
    NotFound(String),
    #[error("storage I/O error: {0}")]
    Io(#[from] io::Error),
}

pub trait ObjectStorage {
    fn put(&self, key: &str, bytes: &[u8]) -> Result<(), StorageError>;
    fn get(&self, key: &str) -> Result<Vec<u8>, StorageError>;
    fn delete(&self, key: &str) -> Result<(), StorageError>;
    fn uri(&self, key: &str) -> String;
}

#[derive(Debug, Clone)]
pub struct LocalObjectStorage {
    root: PathBuf,
    uri_prefix: String,
}

impl LocalObjectStorage {
    pub fn new(root: impl Into<PathBuf>, uri_prefix: impl Into<String>) -> Self {
        Self {
            root: root.into(),
            uri_prefix: uri_prefix.into(),
        }
    }

    fn path_for(&self, key: &str) -> PathBuf {
        let mut path = self.root.clone();
        for segment in Path::new(key).components() {
            if let std::path::Component::Normal(part) = segment {
                path.push(part);
            }
        }
        path
    }
}

impl ObjectStorage for LocalObjectStorage {
    fn put(&self, key: &str, bytes: &[u8]) -> Result<(), StorageError> {
        let path = self.path_for(key);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, bytes)?;
        Ok(())
    }

    fn get(&self, key: &str) -> Result<Vec<u8>, StorageError> {
        let path = self.path_for(key);
        match fs::File::open(path) {
            Ok(mut file) => {
                let mut bytes = Vec::new();
                file.read_to_end(&mut bytes)?;
                Ok(bytes)
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                Err(StorageError::NotFound(key.into()))
            }
            Err(err) => Err(StorageError::Io(err)),
        }
    }

    fn delete(&self, key: &str) -> Result<(), StorageError> {
        let path = self.path_for(key);
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                Err(StorageError::NotFound(key.into()))
            }
            Err(err) => Err(StorageError::Io(err)),
        }
    }

    fn uri(&self, key: &str) -> String {
        format!("{}/{}", self.uri_prefix.trim_end_matches('/'), key.trim_start_matches('/'))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn local_storage_round_trip() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("onestack-storage-{unique}"));
        let store = LocalObjectStorage::new(&root, "local://");

        store.put("videos/clip.bin", b"hello").unwrap();
        assert_eq!(store.get("videos/clip.bin").unwrap(), b"hello");
        assert_eq!(store.uri("videos/clip.bin"), "local://videos/clip.bin");

        store.delete("videos/clip.bin").unwrap();
        assert!(matches!(
            store.get("videos/clip.bin"),
            Err(StorageError::NotFound(_))
        ));

        let _ = fs::remove_dir_all(root);
    }
}
