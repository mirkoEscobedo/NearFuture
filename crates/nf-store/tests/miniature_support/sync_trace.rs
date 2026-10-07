#![allow(dead_code)]
use std::collections::BTreeMap;
#[derive(Debug, Eq, PartialEq)]
pub struct SyncCut {
    pub tid: u32,
    pub syscall: &'static str,
    pub occurrence: u32,
    pub journal: bool,
}
pub fn target(trace: &str, filename: &str) -> SyncCut {
    assert!(trace.len() <= 1_048_576);
    let mut counts = BTreeMap::new();
    let mut marked = None;
    for line in trace.lines() {
        let Some((tid, body)) = split(line) else {
            continue;
        };
        if body.contains("MINIATURE_ADVANCE_COMMIT_READY") {
            assert!(
                marked.replace(tid).is_none(),
                "one actual advancing setup marker"
            );
        }
        if let Some(syscall) = syscall(body) {
            let count = counts.entry((tid, syscall)).or_insert(0u32);
            *count += 1;
            if marked == Some(tid) && body.contains(filename) {
                assert!((1..=65535).contains(count));
                assert!(!body.contains("unfinished") && !body.contains("resumed"));
                return SyncCut {
                    tid,
                    syscall,
                    occurrence: *count,
                    journal: body.contains(&format!("{filename}-journal")),
                };
            }
        }
    }
    panic!("no actual SQLite advancing sync after successful claim/resume/activity")
}
pub fn assert_injected(trace: &str, filename: &str, expected: &SyncCut) {
    let actual = target(trace, filename);
    assert_eq!(actual.syscall, expected.syscall);
    assert_eq!(actual.occurrence, expected.occurrence);
    assert_eq!(actual.journal, expected.journal);
    let mut count = 0;
    for line in trace.lines() {
        let Some((tid, body)) = split(line) else {
            continue;
        };
        if tid == actual.tid && syscall(body) == Some(actual.syscall) {
            count += 1;
            if count == actual.occurrence {
                assert!(
                    body.contains(filename) && body.contains("EIO") && body.contains("INJECTED"),
                    "calibrated actual SQLite sync must be injected"
                );
                return;
            }
        }
    }
    panic!("missing calibrated injection")
}
fn split(line: &str) -> Option<(u32, &str)> {
    let line = line.trim_start();
    let at = line.find(char::is_whitespace)?;
    Some((line[..at].parse().ok()?, line[at..].trim_start()))
}
fn syscall(line: &str) -> Option<&'static str> {
    if line.starts_with("fsync(") {
        Some("fsync")
    } else if line.starts_with("fdatasync(") {
        Some("fdatasync")
    } else {
        None
    }
}
