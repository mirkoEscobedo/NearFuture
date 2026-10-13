use super::{ERR_LIMIT, OUT_LIMIT, TRACE_LIMIT, bounded, clock, create, small, trace};
use std::{
    collections::BTreeSet,
    fs,
    io,
    path::PathBuf,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, Debug)]
struct Identity {
    tid: u32,
    ppid: u32,
    pgid: u32,
    session: u32,
    tgid: u32,
    tracer: u32,
    start: u64,
    raw: String,
}

fn generation(tid: u32) -> io::Result<(u64, String)> {
    let stat = fs::read_to_string(format!("/proc/{tid}/stat"))?;
    assert!(stat.len() <= 8192);
    let (prefix, tail) = stat.rsplit_once(") ").expect("unambiguous kernel stat");
    assert_eq!(prefix.split_once(' ').unwrap().0.parse::<u32>().unwrap(), tid);
    let fields: Vec<_> = tail.split_whitespace().collect();
    assert!(fields.len() >= 20);
    Ok((fields[19].parse().unwrap(), stat))
}

fn identity(tid: u32) -> io::Result<Identity> {
    let root = PathBuf::from(format!("/proc/{tid}"));
    let stat = fs::read_to_string(root.join("stat"))?;
    let status = fs::read_to_string(root.join("status"))?;
    assert!(stat.len() <= 8192 && status.len() <= 8192);
    let (prefix, tail) = stat.rsplit_once(") ").expect("unambiguous kernel stat");
    assert_eq!(prefix.split_once(' ').unwrap().0.parse::<u32>().unwrap(), tid);
    let fields: Vec<_> = tail.split_whitespace().collect();
    assert!(fields.len() >= 20);
    let field = |key: &str| -> u32 {
        status.lines().find_map(|line| line.strip_prefix(key)).unwrap().trim().parse().unwrap()
    };
    Ok(Identity {
        tid,
        ppid: fields[1].parse().unwrap(),
        pgid: fields[2].parse().unwrap(),
        session: fields[3].parse().unwrap(),
        tgid: field("Tgid:"),
        tracer: field("TracerPid:"),
        start: fields[19].parse().unwrap(),
        raw: format!("TID {tid}\n{stat}\n{status}\nEXE {:?}\n", fs::read_link(root.join("exe"))?),
    })
}

fn tasks(pid: u32) -> BTreeSet<u32> {
    let entries: BTreeSet<_> = fs::read_dir(format!("/proc/{pid}/task")).unwrap()
        .map(|entry| entry.unwrap().file_name().to_str().unwrap().parse().unwrap())
        .collect();
    assert!(!entries.is_empty() && entries.len() <= 16);
    entries
}

pub struct Run {
    pub control: PathBuf,
    pub main: u32,
    pub writer: u32,
    pub trace: String,
    pub stdout: Vec<u8>,
}

pub struct OwnedTrace {
    child: Child,
    control: PathBuf,
    deadline: Instant,
    direct: Option<Identity>,
    owned: Vec<Identity>,
    reaped: bool,
    qualified: bool,
}

impl OwnedTrace {
    pub fn spawn(mut command: Command, control: PathBuf, deadline: Instant) -> Self {
        clock(deadline - Duration::from_secs(2));
        let stdout = fs::OpenOptions::new().write(true).create_new(true)
            .open(control.join("stdout.raw")).unwrap();
        let stderr = fs::OpenOptions::new().write(true).create_new(true)
            .open(control.join("stderr.raw")).unwrap();
        command.stdin(Stdio::null()).stdout(Stdio::from(stdout)).stderr(Stdio::from(stderr));
        create(&control.join("command.txt"), format!("requested process_group(0); observed PGID must equal returned tracer PID\n{command:?}\n").as_bytes());
        let child = command.spawn().expect("maintained strace must be present in hosted qualifier");
        // Own the returned handle immediately, before any fallible generation capture.
        let mut owned = Self {
            child, control, deadline, direct: None, owned: Vec::new(),
            reaped: false, qualified: false,
        };
        let direct = identity(owned.child.id()).expect("live returned tracer identity");
        assert_eq!(direct.ppid, std::process::id());
        assert_eq!(direct.tgid, direct.tid);
        assert_eq!(direct.pgid, direct.tid, "actual contained process group");
        assert_eq!(tasks(direct.tid), BTreeSet::from([direct.tid]));
        owned.direct = Some(direct);
        owned
    }

    fn limits(&mut self) {
        clock(self.deadline - Duration::from_secs(2));
        for (name, maximum) in [
            ("trace.raw", TRACE_LIMIT), ("stdout.raw", OUT_LIMIT), ("stderr.raw", ERR_LIMIT),
        ] {
            let path = self.control.join(name);
            if path.exists() {
                assert!(small(&path, maximum as u64), "retained original exceeds closed limit");
            }
        }
        if let Some(status) = self.child.try_wait().unwrap() {
            self.reaped = true;
            panic!("trace ended before bounded qualification: {status}");
        }
    }

    pub fn release(&mut self) -> (u32, u32) {
        let ready = self.control.join("ready.txt");
        loop {
            self.limits();
            if ready.exists() {
                let bytes = bounded(&ready, 64);
                if bytes.ends_with(b"\n") {
                    let text = std::str::from_utf8(&bytes).unwrap();
                    let ids: Vec<u32> = text.split_whitespace().map(|id| id.parse().unwrap()).collect();
                    assert_eq!(ids.len(), 2);
                    let (main, writer) = (ids[0], ids[1]);
                    let actual = tasks(main);
                    assert!(actual.contains(&main) && actual.contains(&writer));
                    let raw = String::from_utf8(bounded(&self.control.join("trace.raw"), TRACE_LIMIT)).unwrap();
                    if trace::tids(&raw) != actual || !trace::births(&raw, &actual, main) {
                        thread::sleep(Duration::from_millis(10));
                        continue;
                    }
                    let tracer = self.direct.as_ref().unwrap().tid;
                    let mut originals = self.direct.as_ref().unwrap().raw.clone();
                    for tid in &actual {
                        let member = identity(*tid).expect("every live tracee generation before release");
                        assert_eq!(member.tgid, main);
                        assert_eq!(member.ppid, tracer);
                        assert_eq!(member.pgid, tracer);
                        assert_eq!(member.session, self.direct.as_ref().unwrap().session);
                        assert_eq!(member.tracer, tracer, "actual kernel tracing ownership");
                        assert_eq!(fs::read_link(format!("/proc/{tid}/exe")).unwrap(), std::env::current_exe().unwrap());
                        originals.push_str(&member.raw);
                        self.owned.push(member);
                    }
                    assert_eq!(tasks(main), actual, "stable held harness task set");
                    assert!(originals.len() <= 65536);
                    create(&self.control.join("before-proc.raw"), originals.as_bytes());
                    create(&self.control.join("release.txt"), b"one release after actual generations\n");
                    return (main, writer);
                }
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    pub fn finish(mut self, main: u32, writer: u32) -> Run {
        let status = loop {
            clock(self.deadline - Duration::from_secs(2));
            for (name, maximum) in [
                ("trace.raw", TRACE_LIMIT), ("stdout.raw", OUT_LIMIT), ("stderr.raw", ERR_LIMIT),
            ] {
                assert!(small(&self.control.join(name), maximum as u64));
            }
            if let Some(status) = self.child.try_wait().unwrap() {
                self.reaped = true;
                break status;
            }
            thread::sleep(Duration::from_millis(10));
        };
        create(&self.control.join("direct-reap.txt"), format!("returned Child {} status {status}\n", self.child.id()).as_bytes());
        let raw = String::from_utf8(bounded(&self.control.join("trace.raw"), TRACE_LIMIT)).unwrap();
        let tids = self.owned.iter().map(|member| member.tid).collect();
        trace::exited(&raw, &tids, main);
        assert!(status.success(), "actual returned tracer reaped successfully");
        let mut zero = String::new();
        for member in self.owned.iter().chain(self.direct.iter()) {
            match generation(member.tid) {
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    zero.push_str(&format!("TID {} START {} ABSENT\n", member.tid, member.start));
                }
                Ok((start, stat)) if start != member.start => {
                    zero.push_str(&format!("TID {} OLD_START {} REUSED_START {start}\n", member.tid, member.start));
                    zero.push_str(&stat);
                }
                result => panic!("zero owned generation not proved: {result:?}"),
            }
        }
        create(&self.control.join("zero-owned-generations.txt"), zero.as_bytes());
        let stdout = bounded(&self.control.join("stdout.raw"), OUT_LIMIT);
        let _ = bounded(&self.control.join("stderr.raw"), ERR_LIMIT);
        clock(self.deadline);
        self.qualified = true;
        Run { control: self.control.clone(), main, writer, trace: raw, stdout }
    }
}

impl Drop for OwnedTrace {
    fn drop(&mut self) {
        if self.qualified {
            return;
        }
        let cleanup_deadline = (Instant::now() + Duration::from_secs(2)).min(self.deadline);
        let mut receipt = format!("INSUFFICIENT_EVIDENCE tracer {} reaped {}\n", self.child.id(), self.reaped);
        if !self.reaped {
            // Only the returned, unreaped Child handle is signalled. EXITKILL aids
            // containment; it never substitutes for trace and generation-zero proof.
            receipt.push_str(&format!("returned-handle kill {:?}\n", self.child.kill()));
            while Instant::now() < cleanup_deadline {
                match self.child.try_wait() {
                    Ok(Some(status)) => {
                        self.reaped = true;
                        receipt.push_str(&format!("direct reap {status}\n"));
                        break;
                    }
                    Ok(None) => thread::sleep(Duration::from_millis(10)),
                    Err(error) => {
                        receipt.push_str(&format!("reap error {error:?}\n"));
                        break;
                    }
                }
            }
        }
        for member in self.owned.iter().chain(self.direct.iter()) {
            if Instant::now() >= cleanup_deadline {
                receipt.push_str("remaining stat observations exceed bounded cleanup deadline\n");
                break;
            }
            receipt.push_str(&format!("after TID {} original start {} observation {:?}\n", member.tid, member.start, generation(member.tid)));
        }
        let _ = fs::OpenOptions::new().write(true).create_new(true)
            .open(self.control.join("failed-cleanup.txt"))
            .and_then(|mut file| {
                use std::io::Write;
                file.write_all(receipt.as_bytes())
            });
        // All original files and partial outputs remain in the caller-owned case.
    }
}
