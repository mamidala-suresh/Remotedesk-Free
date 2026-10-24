use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};

const CHUNK_SIZE: usize = 64 * 1024; // 64 KB

fn compute_hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

#[test]
fn test_file_chunking_and_cryptographic_integrity() {
    let temp_dir = tempfile::tempdir().unwrap();
    let src_file = temp_dir.path().join("source_payload.bin");
    let dst_file = temp_dir.path().join("transferred_payload.bin");

    // Generate 200 KB test payload (spans 4 chunks)
    let mut original_data = vec![0u8; 200 * 1024];
    for (i, b) in original_data.iter_mut().enumerate() {
        *b = (i % 251) as u8;
    }
    fs::write(&src_file, &original_data).unwrap();
    let original_sha256 = compute_hash(&original_data);

    // Simulate chunked file transfer over network
    let file_size = original_data.len() as u64;
    let total_chunks = (file_size + CHUNK_SIZE as u64 - 1) / CHUNK_SIZE as u64;
    assert_eq!(total_chunks, 4);

    let mut dest = File::create(&dst_file).unwrap();

    for chunk_idx in 0..total_chunks {
        let mut src = File::open(&src_file).unwrap();
        src.seek(SeekFrom::Start(chunk_idx * CHUNK_SIZE as u64)).unwrap();

        let mut buf = vec![0u8; CHUNK_SIZE];
        let bytes_read = src.read(&mut buf).unwrap();
        buf.truncate(bytes_read);

        // Write to destination at expected offset
        dest.seek(SeekFrom::Start(chunk_idx * CHUNK_SIZE as u64)).unwrap();
        dest.write_all(&buf).unwrap();
    }
    dest.flush().unwrap();

    // Verify transferred file integrity
    let transferred_data = fs::read(&dst_file).unwrap();
    assert_eq!(transferred_data.len(), original_data.len());

    let transferred_sha256 = compute_hash(&transferred_data);
    assert_eq!(
        original_sha256, transferred_sha256,
        "Transferred file SHA-256 hash must match original file exactly"
    );
}

#[test]
fn test_directory_enumeration() {
    let temp_dir = tempfile::tempdir().unwrap();
    let sub_dir = temp_dir.path().join("SubFolder");
    let file1 = temp_dir.path().join("alpha.txt");
    let file2 = temp_dir.path().join("beta.log");

    fs::create_dir(&sub_dir).unwrap();
    fs::write(&file1, b"Alpha data").unwrap();
    fs::write(&file2, b"Beta data log").unwrap();

    let mut entries = Vec::new();
    for entry in fs::read_dir(temp_dir.path()).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = entry.metadata().unwrap().is_dir();
        entries.push((name, is_dir));
    }

    assert_eq!(entries.len(), 3);
    assert!(entries.iter().any(|(n, is_dir)| n == "SubFolder" && *is_dir));
    assert!(entries.iter().any(|(n, is_dir)| n == "alpha.txt" && !*is_dir));
    assert!(entries.iter().any(|(n, is_dir)| n == "beta.log" && !*is_dir));
}
