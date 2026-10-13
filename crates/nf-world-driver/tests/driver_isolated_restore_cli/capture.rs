use super::{check, plain};
use sha2::{Digest, Sha256};
use std::{fs::{File, OpenOptions}, io::{Read, Write}, path::Path, time::Instant};

pub fn new_file(path: &Path) -> File {
    OpenOptions::new().write(true).create_new(true).open(path).unwrap()
}

pub fn receipt(path: &Path, text: &str) {
    assert!(text.len() <= 16_384);
    let mut file = new_file(path);
    writeln!(file, "{text}").unwrap();
    file.sync_all().unwrap();
}

pub fn retained_bytes(path: &Path, bytes: &[u8]) {
    assert!(bytes.len() <= 1_048_576);
    let mut file = new_file(path);
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
}

pub fn bounded(path: &Path, maximum: u64) -> Vec<u8> {
    let metadata = plain(path);
    assert!(metadata.is_file() && metadata.len() <= maximum);
    let mut bytes = Vec::new();
    File::open(path).unwrap().take(maximum + 1).read_to_end(&mut bytes).unwrap();
    assert!(bytes.len() as u64 <= maximum);
    bytes
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn same(before: &std::fs::Metadata, after: &std::fs::Metadata) {
    assert!(before.is_file() && after.is_file());
    assert_eq!(before.len(), after.len());
    assert_eq!(before.modified().unwrap(), after.modified().unwrap());
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        assert_eq!((before.dev(), before.ino(), before.ctime(), before.ctime_nsec()),
            (after.dev(), after.ino(), after.ctime(), after.ctime_nsec()));
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        assert_eq!((before.file_attributes(), before.creation_time(), before.last_write_time()),
            (after.file_attributes(), after.creation_time(), after.last_write_time()));
    }
}

pub fn binary_digest(path: &Path, deadline: Instant) -> String {
    check(deadline);
    let metadata = plain(path);
    assert!(metadata.is_file() && metadata.len() > 0 && metadata.len() <= 268_435_456);
    let mut file = File::open(path).unwrap();
    same(&metadata, &file.metadata().unwrap());
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 65_536];
    let mut remaining = metadata.len();
    while remaining > 0 {
        check(deadline);
        let count = remaining.min(buffer.len() as u64) as usize;
        let read = file.read(&mut buffer[..count]).unwrap();
        assert!(read > 0);
        digest.update(&buffer[..read]);
        remaining -= read as u64;
    }
    let mut extra = [0u8; 1];
    assert_eq!(file.read(&mut extra).unwrap(), 0);
    same(&metadata, &file.metadata().unwrap());
    same(&metadata, &plain(path));
    check(deadline);
    hex(&digest.finalize())
}
