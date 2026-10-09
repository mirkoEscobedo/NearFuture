use std::{
    ffi::OsString,
    fs,
    path::PathBuf,
    process::{Command, Stdio},
    sync::atomic::Ordering,
    time::{Duration, Instant},
};

const EMPTY_TRACE: &str = "traversal|empty|OK|-|-|0|-|-\n";

#[derive(Clone, Copy)]
enum Arguments {
    Normal,
    Missing,
    Extra,
    InvalidMode,
}
struct Case {
    name: &'static str,
    input: Vec<u8>,
    arguments: Arguments,
    expected_stdout: Option<Vec<u8>>,
}
fn accept(name: &'static str, input: Vec<u8>, stdout: Vec<u8>) -> Case {
    Case {
        name,
        input,
        arguments: Arguments::Normal,
        expected_stdout: Some(stdout),
    }
}
fn refuse(name: &'static str, input: Vec<u8>) -> Case {
    Case {
        name,
        input,
        arguments: Arguments::Normal,
        expected_stdout: None,
    }
}

#[test]
#[ignore = "requires NF_SHADOW_JAVA and NF_SHADOW_CLASSPATH_FILE; exportDifferentialClasspath first"]
fn real_java_traversal_cli_protocol_bounds_and_refusals() {
    let mut cases = vec![
        accept("no_final_lf", b"empty|-".to_vec(), EMPTY_TRACE.as_bytes().to_vec()),
        accept("line_separator_forms", "empty|-\nempty|-\r\nempty|-\rempty|-\u{000b}empty|-\u{000c}empty|-\u{0085}empty|-\u{2028}empty|-\u{2029}empty|-".as_bytes().to_vec(), EMPTY_TRACE.repeat(9).into_bytes()),
        accept("data_rows_256", "empty|-\n".repeat(256).into_bytes(), EMPTY_TRACE.repeat(256).into_bytes()),
        accept("reached_missing_is_success_unavailable", b"missing|missing,00000000,false,true,false,false,U\n".to_vec(), b"traversal|missing|UNAVAILABLE\n".to_vec()),
        refuse("data_rows_257", "empty|-\n".repeat(257).into_bytes()),
        refuse("empty_file", Vec::new()),
        refuse("only_comments_and_blanks", b"# ignored\n \t\n".to_vec()),
        refuse("malformed_trailer_after_valid_prefix", b"empty|-\ntrailer|-|extra\n".to_vec()),
        refuse("missing_row_column", b"missing\n".to_vec()),
        refuse("invalid_case_id", b"Invalid|-\n".to_vec()),
        refuse("trailing_enemy_separator", b"bad|enemy,00000000,false,true,false,false,N;\n".to_vec()),
        refuse("missing_enemy_field", b"bad|enemy,00000000,false,true,false,N\n".to_vec()),
        refuse("invalid_enemy_id", b"bad|enemy1,00000000,false,true,false,false,N\n".to_vec()),
        refuse("invalid_raw_hex", b"bad|enemy,7F800000,false,true,false,false,N\n".to_vec()),
        refuse("invalid_boolean", b"bad|enemy,00000000,TRUE,true,false,false,N\n".to_vec()),
        refuse("invalid_return_even_when_skipped", b"bad|enemy,00000000,true,true,false,false,X\n".to_vec()),
    ];
    for (name, arguments) in [
        ("missing_cli_argument", Arguments::Missing),
        ("extra_cli_argument", Arguments::Extra),
        ("invalid_cli_mode", Arguments::InvalidMode),
    ] {
        let mut case = refuse(name, b"empty|-\n".to_vec());
        case.arguments = arguments;
        cases.push(case);
    }
    let mut malformed_utf8 = b"empty|-\n".to_vec();
    malformed_utf8.push(0xff);
    cases.push(refuse("malformed_utf8_after_valid_prefix", malformed_utf8));

    // Long comments bypass the data-row limit, but not the whole-file byte cap.
    let mut byte_limit = b"empty|-\n#".to_vec();
    byte_limit.resize(65_536, b'a');
    cases.push(accept(
        "corpus_bytes_65536",
        byte_limit.clone(),
        EMPTY_TRACE.as_bytes().to_vec(),
    ));
    byte_limit.push(b'a');
    cases.push(refuse("corpus_bytes_65537", byte_limit));

    // Both boundary records obey every grammar rule except the negative row length.
    let id = "a".repeat(32);
    let enemy = format!("{id},00000000,true,true,false,false,U");
    let facts = vec![enemy; 15].join(";");
    let case_id = "r".repeat(34);
    let row = format!("{case_id}|{facts}");
    assert_eq!(row.encode_utf16().count(), 1_024);
    let ids = vec![id.clone(); 15].join(",");
    let visits = vec![format!("{id}:RECENT_WAR"); 15].join(",");
    let stdout = format!("traversal|{case_id}|OK|{ids}|{visits}|0|-|-\n");
    cases.push(accept(
        "data_row_utf16_1024",
        row.into_bytes(),
        stdout.into_bytes(),
    ));
    let too_long = format!("r{case_id}|{facts}");
    assert_eq!(too_long.encode_utf16().count(), 1_025);
    cases.push(refuse("data_row_utf16_1025", too_long.into_bytes()));

    // Valid admitted ASCII data whose complete output exceeds the existing character cap.
    let enemy = format!("{id},00000000,false,true,true,false,U");
    let output_input = format!("a|{}\n", vec![enemy; 3].join(";")).repeat(256);
    assert!(output_input.len() <= 65_536);
    let visits = vec![format!("{id}:COMMISSIONED_PLAYER"); 3].join(",");
    let trace = format!("traversal|a|OK|{}|{visits}|0|-|-\n", vec![id; 3].join(","));
    assert!(trace.repeat(256).len() > 65_536);
    cases.push(refuse("buffered_output_cap", output_input.into_bytes()));

    assert_eq!(cases.len(), 25);
    for case in cases {
        probe(&case);
        println!("PASS: traversal CLI protocol {}", case.name);
    }
}

fn probe(case: &Case) {
    let java = std::env::var_os("NF_SHADOW_JAVA").expect("explicit Java executable required");
    let classpath_file =
        std::env::var_os("NF_SHADOW_CLASSPATH_FILE").expect("explicit oracle classpath required");
    let classpath = fs::read_to_string(classpath_file).unwrap();
    assert!(!classpath.trim().is_empty() && classpath.len() < 65_536);
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".tmp");
    fs::create_dir_all(&base).unwrap();
    let base = base.canonicalize().unwrap();
    let root = base.join(format!(
        "traversal-cli-{}-{}",
        std::process::id(),
        super::NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    assert!(root.starts_with(&base));
    fs::create_dir(&root).unwrap();
    // Reuse the existing, unchanged parent ownership and Drop kill/wait/cleanup.
    let mut owned = super::Owned {
        child: None,
        root,
        base,
    };
    let input = owned.root.join("input");
    use std::io::Write;
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&input)
        .unwrap()
        .write_all(&case.input)
        .unwrap();
    let output = owned.root.join("out");
    let errors = owned.root.join("err");
    let out = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)
        .unwrap();
    let err = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&errors)
        .unwrap();
    let mode = if matches!(case.arguments, Arguments::InvalidMode) {
        "invalid"
    } else {
        "traversal"
    };
    let mut args = vec![
        OsString::from("-cp"),
        OsString::from(classpath.trim()),
        OsString::from("nf.nex.reference.DifferentialOracleMain"),
        OsString::from(mode),
    ];
    if !matches!(case.arguments, Arguments::Missing) {
        args.push(OsString::from("input"));
    }
    if matches!(case.arguments, Arguments::Extra) {
        args.push(OsString::from("extra"));
    }
    let mut command = Command::new(java);
    command
        .current_dir(&owned.root)
        .args(args)
        .stdin(Stdio::null())
        .stdout(out)
        .stderr(err);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    owned.child = Some(command.spawn().unwrap());
    let deadline = Instant::now() + Duration::from_secs(10);
    let status = loop {
        assert!(
            fs::metadata(&output).unwrap().len() <= 65_536
                && fs::metadata(&errors).unwrap().len() <= 4_096,
            "bounded oracle output"
        );
        if let Some(status) = owned.child.as_mut().unwrap().try_wait().unwrap() {
            break status;
        }
        assert!(Instant::now() < deadline, "bounded oracle deadline");
        std::thread::sleep(Duration::from_millis(5));
    };
    assert!(fs::metadata(&output).unwrap().len() <= 65_536);
    assert!(fs::metadata(&errors).unwrap().len() <= 4_096);
    let stdout = fs::read(output).unwrap();
    let stderr = fs::read(errors).unwrap();
    if let Some(expected) = &case.expected_stdout {
        assert_eq!(
            status.code(),
            Some(0),
            "{} exit; stdout={:?}; stderr={:?}",
            case.name,
            stdout,
            stderr
        );
        assert_eq!(&stdout, expected, "{} stdout", case.name);
        assert!(stderr.is_empty(), "{} stderr", case.name);
    } else {
        assert_eq!(
            status.code(),
            Some(1),
            "{} exit; stdout={:?}; stderr={:?}",
            case.name,
            stdout,
            stderr
        );
        assert!(
            stdout.is_empty(),
            "{} must not emit partial stdout",
            case.name
        );
        let expected: &[u8] = if cfg!(windows) {
            b"REFERENCE_UNAVAILABLE\r\n"
        } else {
            b"REFERENCE_UNAVAILABLE\n"
        };
        assert_eq!(stderr, expected, "{} stderr", case.name);
    }
}
