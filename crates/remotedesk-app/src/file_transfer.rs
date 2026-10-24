use remotedesk_core::protocol::DirEntryInfo;
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

#[allow(dead_code)]
pub const CHUNK_SIZE: usize = 64 * 1024; // 64 KB per chunk

/// Information about an active file transfer.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ActiveTransfer {
    pub transfer_id: String,
    pub filename: String,
    pub total_size: u64,
    pub is_download: bool,
    pub target_path: PathBuf,
    pub bytes_transferred: u64,
    pub expected_sha256: String,
    pub current_chunk: u64,
    pub total_chunks: u64,
    pub is_complete: bool,
    pub error: Option<String>,
}

pub struct FileManager;

impl FileManager {
    /// Lists entries in a local folder.
    pub fn list_directory(dir_path: impl AsRef<Path>) -> Result<Vec<DirEntryInfo>, std::io::Error> {
        let mut entries = Vec::new();
        let path = dir_path.as_ref();

        if !path.exists() {
            return Ok(entries);
        }

        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            let is_dir = metadata.is_dir();
            let size_bytes = if is_dir { 0 } else { metadata.len() };
            let modified_timestamp = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);

            entries.push(DirEntryInfo {
                name: entry.file_name().to_string_lossy().to_string(),
                is_dir,
                size_bytes,
                modified_timestamp,
            });
        }

        // Sort directories first, then alphabetical
        entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        });

        Ok(entries)
    }

    /// Computes SHA-256 cryptographic hash of a file.
    #[allow(dead_code)]
    pub fn compute_file_hash(path: impl AsRef<Path>) -> Result<String, std::io::Error> {
        let mut file = File::open(path)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 8192];

        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        Ok(hex::encode(hasher.finalize()))
    }

    /// Reads a 64KB chunk from a file at a specific chunk index.
    #[allow(dead_code)]
    pub fn read_chunk(path: impl AsRef<Path>, chunk_idx: u64) -> Result<(Vec<u8>, bool), std::io::Error> {
        let mut file = File::open(path)?;
        file.seek(SeekFrom::Start(chunk_idx * CHUNK_SIZE as u64))?;
        let mut buf = vec![0u8; CHUNK_SIZE];
        let bytes_read = file.read(&mut buf)?;
        buf.truncate(bytes_read);
        let is_last = bytes_read < CHUNK_SIZE;
        Ok((buf, is_last))
    }

    /// Appends or writes a chunk into the destination file.
    #[allow(dead_code)]
    pub fn write_chunk(
        destination_path: impl AsRef<Path>,
        chunk_idx: u64,
        data: &[u8],
    ) -> Result<(), std::io::Error> {
        let path = destination_path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(path)?;

        file.seek(SeekFrom::Start(chunk_idx * CHUNK_SIZE as u64))?;
        file.write_all(data)?;
        file.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_transfer_chunking_and_hashing() {
        let temp_dir = tempfile::tempdir().unwrap();
        let src_path = temp_dir.path().join("source.txt");
        let dst_path = temp_dir.path().join("dest.txt");

        let test_data = b"Hello RemoteDesk Free File Transfer Test! 1234567890";
        fs::write(&src_path, test_data).unwrap();

        let hash_src = FileManager::compute_file_hash(&src_path).unwrap();

        let (chunk, is_last) = FileManager::read_chunk(&src_path, 0).unwrap();
        assert!(is_last);
        assert_eq!(&chunk, test_data);

        FileManager::write_chunk(&dst_path, 0, &chunk).unwrap();
        let hash_dst = FileManager::compute_file_hash(&dst_path).unwrap();

        assert_eq!(hash_src, hash_dst);
    }
}
