pub mod process;
pub mod trace;
pub mod worker;

use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::Path,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

pub const TRACE_LIMIT: usize = 1_048_576;
pub const OUT_LIMIT: usize = 4096;
pub const ERR_LIMIT: usize = 1024;

pub fn clock(deadline: Instant) {
    assert!(Instant::now() < deadline, "one absolute20sec case deadline");
}

pub fn epoch_millis() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()
}

pub fn bounded(path: &Path, maximum: usize) -> Vec<u8> {
    let file = File::open(path).unwrap();
    assert!(file.metadata().unwrap().len() <= maximum as u64);
    let mut bytes = Vec::new();
    file.take(maximum as u64 + 1).read_to_end(&mut bytes).unwrap();
    assert!(bytes.len() <= maximum);
    bytes
}

pub fn create(path: &Path, bytes: &[u8]) {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path).unwrap();
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
}

pub fn small(path: &Path, maximum: u64) -> bool {
    fs::metadata(path).is_ok_and(|metadata| metadata.len() <= maximum)
}
