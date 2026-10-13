use std::collections::{BTreeMap, BTreeSet};

pub const READY: &str = "RESTORE_CALL_READY\\n";
pub const ACK: &str = "RESTORE_QUALIFIED_ACK\\n";

#[derive(Clone, Debug)]
pub struct Event {
    pub tid: u32,
    pub body: String,
}

pub fn events(trace: &str) -> Vec<Event> {
    records(trace, true)
}

fn records(trace: &str, complete: bool) -> Vec<Event> {
    assert!(trace.len() <= 1_048_576);
    assert!(!complete || trace.ends_with('\n'), "complete trailing original record required");
    let mut pending = BTreeMap::new();
    let mut result = Vec::new();
    for line in trace.lines() {
        let Some((tid, body)) = original_line(line) else {
            continue;
        };
        if let Some(prefix) = body.strip_suffix("<unfinished ...>") {
            let name = prefix.split_once('(').unwrap().0;
            assert!(pending.insert((tid, name.to_owned()), prefix.to_owned()).is_none());
        } else if let Some(resumed) = body.strip_prefix("<... ") {
            let (name, tail) = resumed.split_once(" resumed>").unwrap();
            let prefix = pending.remove(&(tid, name.to_owned())).expect("matched syscall entry");
            result.push(Event { tid, body: prefix + tail });
        } else {
            result.push(Event { tid, body: body.to_owned() });
        }
    }
    assert!(!complete || pending.is_empty(), "complete original syscall records required");
    result
}

fn original_line(line: &str) -> Option<(u32, &str)> {
    let line = line.trim_start();
    if line.is_empty() {
        return None;
    }
    let (pid, body) = line.split_once(' ').expect("original numeric PID prefix required");
    let tid = pid.parse().expect("unambiguous original numeric PID");
    let body = body.trim_start();
    assert!(!body.is_empty(), "original event body required");
    Some((tid, body))
}

/// Live capture consumes only completed lines; final records require no tail.
pub fn tids(trace: &str) -> BTreeSet<u32> {
    let complete = trace.rsplit_once('\n').map_or("", |(complete, _)| complete);
    complete.lines().filter_map(original_line).map(|(tid, _)| tid).collect()
}

fn returned(body: &str) -> i32 {
    body.rsplit_once(" = ").unwrap().1.chars()
        .take_while(|character| character.is_ascii_digit() || *character == '-')
        .collect::<String>().parse().unwrap()
}

fn name(body: &str) -> &str {
    body.split_once('(').map_or("", |(name, _)| name)
}

fn open(body: &str) -> bool {
    matches!(name(body), "open" | "openat" | "openat2")
}

fn fd(body: &str) -> i32 {
    body.split_once('(').unwrap().1.split_once('<').unwrap().0.parse().unwrap()
}

fn exact_fd(body: &str, destination: &str) -> bool {
    body.contains(&format!("<{destination}>"))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cut {
    pub tid: u32,
    pub fd: i32,
    pub syscall: &'static str,
    pub ordinal: u32,
}

pub fn final_sync(
    trace: &str,
    destination: &str,
    leaf: &str,
    writer: u32,
    fault: bool,
) -> Cut {
    assert!(!destination.chars().any(|character| matches!(character, '"' | '<' | '>' | '\\' | '\n')));
    let events = events(trace);
    let markers: Vec<_> = events
        .iter()
        .enumerate()
        .filter(|(_, event)| event.body.starts_with("write(1<") && event.body.contains(READY))
        .collect();
    assert_eq!(markers.len(), 1, "one original call marker");
    let ready = markers[0].0;
    assert_eq!(markers[0].1.tid, writer, "actual writing TID, not main PID");
    assert_eq!(returned(&markers[0].1.body), (READY.len() - 1) as i32);
    let reservations: Vec<_> = events
        .iter()
        .enumerate()
        .filter(|(index, event)| {
            *index > ready && event.tid == writer && open(&event.body)
                && (event.body.contains(&format!("\"{leaf}\""))
                    || event.body.contains(&format!("\"{destination}\"")))
                && event.body.contains("O_CREAT") && event.body.contains("O_EXCL")
                && returned(&event.body) >= 0 && exact_fd(&event.body, destination)
        })
        .collect();
    assert_eq!(reservations.len(), 1, "one successful exact CreateNew reservation");
    let reservation = reservations[0].0;
    let writable: Vec<_> = events
        .iter()
        .enumerate()
        .filter(|(index, event)| {
            *index > reservation && event.tid == writer && open(&event.body)
                && (event.body.contains(&format!("\"{leaf}\""))
                    || event.body.contains(&format!("\"{destination}\"")))
                && event.body.contains("O_RDWR") && !event.body.contains("O_CREAT")
                && returned(&event.body) >= 0 && exact_fd(&event.body, destination)
        })
        .collect();
    assert_eq!(writable.len(), 2, "SQLite connection then standalone fsync file only");
    let prior = writable[0].0;
    let reserved_fd = returned(&reservations[0].1.body);
    assert_eq!(events[reservation + 1..prior].iter().filter(|event| {
        event.tid == writer && name(&event.body) == "close"
            && fd(&event.body) == reserved_fd && exact_fd(&event.body, destination)
            && returned(&event.body) == 0
    }).count(), 1, "successful reservation descriptor close precedes SQLite open");
    let standalone = writable[1].0;
    let descriptor = returned(&writable[1].1.body);
    let prior_fd = returned(&writable[0].1.body);
    assert!(events[prior + 1..standalone].iter().any(|event| {
        event.tid == writer && name(&event.body) == "close"
            && fd(&event.body) == prior_fd && exact_fd(&event.body, destination)
            && returned(&event.body) == 0
    }), "SQLite destination descriptor closes before standalone open");
    let close = (standalone + 1..events.len())
        .find(|index| {
            let event = &events[*index];
            event.tid == writer && name(&event.body) == "close"
                && fd(&event.body) == descriptor && exact_fd(&event.body, destination)
                && returned(&event.body) == 0
        })
        .expect("standalone file closed");
    let syncs: Vec<_> = (standalone + 1..close)
        .filter(|index| {
            let event = &events[*index];
            event.tid == writer && matches!(name(&event.body), "fsync" | "fdatasync")
                && fd(&event.body) == descriptor && exact_fd(&event.body, destination)
        })
        .collect();
    assert_eq!(syncs.len(), 1, "one unambiguous standalone target sync");
    let index = syncs[0];
    let syscall = match name(&events[index].body) {
        "fsync" => "fsync",
        "fdatasync" => "fdatasync",
        _ => unreachable!(),
    };
    let ordinal = events[..=index].iter()
        .filter(|event| event.tid == writer && name(&event.body) == syscall)
        .count() as u32;
    assert!((1..=65535).contains(&ordinal));
    assert!(!events.iter().any(|event| {
        event.tid != writer && name(&event.body) == syscall
    }), "no competing harness sync injection sequence");
    if fault {
        assert!(events[index].body.contains(" = -1 EIO") && events[index].body.contains("INJECTED"));
        assert_eq!(events.iter().filter(|event| event.body.contains("INJECTED")).count(), 1);
        assert!(!events.iter().any(|event| event.body.contains(ACK)), "no qualified ACK");
    } else {
        assert_eq!(returned(&events[index].body), 0);
        assert!(events[close + 1..].iter().any(|event| {
            event.tid == writer && event.body.starts_with("write(1<")
                && event.body.contains(ACK) && returned(&event.body) == (ACK.len() - 1) as i32
        }), "sync precedes actual read-only qualified ACK");
    }
    Cut { tid: writer, fd: descriptor, syscall, ordinal }
}

pub fn births(trace: &str, owned: &BTreeSet<u32>, main: u32) -> bool {
    let trace = trace.rsplit_once('\n').map_or("", |(complete, _)| complete);
    let events = records(trace, false);
    let execs: Vec<_> = events.iter().filter(|event| {
        name(&event.body) == "execve" && returned(&event.body) == 0
    }).collect();
    if execs.is_empty() {
        return false;
    }
    assert_eq!(execs.len(), 1);
    assert_eq!(execs[0].tid, main);
    assert!(!events.iter().any(|event| {
        matches!(name(&event.body), "setsid" | "setpgid" | "ptrace")
    }), "no detachment or group/session escape calls");
    let mut births = BTreeSet::from([main]);
    for event in &events {
        if matches!(name(&event.body), "clone" | "clone3" | "fork" | "vfork")
            && returned(&event.body) > 0
        {
            assert!(event.body.contains("CLONE_THREAD"), "unexpected process descendant");
            assert!(!event.body.contains("CLONE_UNTRACED"), "no untraced descendant");
            assert!(births.insert(returned(&event.body) as u32), "unique thread birth");
        }
    }
    &births == owned
}

pub fn exited(trace: &str, owned: &BTreeSet<u32>, main: u32) {
    let events = events(trace);
    assert_eq!(&tids(trace), owned, "no unaccounted process/thread birth after release");
    assert!(births(trace, owned, main), "every tracee has original exec/clone birth");
    for tid in owned {
        assert!(events.iter().any(|event| {
            event.tid == *tid && event.body == "+++ exited with 0 +++"
        }), "every actual tracee exit retained");
    }
}
