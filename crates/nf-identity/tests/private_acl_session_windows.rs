#![cfg(windows)]
#[path = "support/getter_contract/process.rs"]
mod process;
#[path = "support/temp_dir.rs"]
mod temp_dir;
use nf_identity::private_storage::PrivateVault;
use sha2::{Digest, Sha256};
use std::{ffi::OsStr, fs, path::Path};

#[test]
fn scoped_six_cut_append_preserves_original_initializer_and_literal_descriptors() {
    let baseline = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/support/getter_contract/original-private-acl.ps1"
    ));
    assert_eq!(
        format!("{:x}", Sha256::digest(fs::read(baseline).unwrap())),
        "d83b47582ed20336ac79ff674adfb40846bed3ccec4416aad7ef9107bf3ad0aa"
    );
    let oracle = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/support/getter_contract/descriptor.ps1"
    ));
    let scratch = temp_dir::disposable();
    let root = scratch.join("owner $ ' ` [λ-猫] directory");
    let saves = scratch.join("saves");
    fs::create_dir(&root).unwrap();
    fs::create_dir(&saves).unwrap();
    let old = root.join("blob-receipt-r0-g0");
    // Real blob framing and checksum are produced by the unchanged public one-shot entry point.
    initialize(baseline, &root, "root");
    let vault = PrivateVault::open(&root, &saves).expect("original protected directory setup");
    let baseline_file = root.join("original-file-baseline");
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&baseline_file)
        .expect("genuinely new independent empty baseline file");
    assert_eq!(fs::metadata(&baseline_file).unwrap().len(), 0);
    initialize(baseline, &baseline_file, "independent-file-baseline");
    vault.create_private_blob("receipt-r0-g0", &[3, 7]).unwrap();
    let root_descriptor = scratch.join("root-descriptor");
    let file_descriptor = scratch.join("file-descriptor");
    descriptor(oracle, &root, &root_descriptor, false);
    descriptor(oracle, &baseline_file, &file_descriptor, false);
    descriptor(oracle, &old, &file_descriptor, true);
    let old_bytes = fs::read(&old).unwrap();
    let names: [String; 64] =
        std::array::from_fn(|index| format!("receipt-r{}-g{}", index / 8, index % 8));
    let names: [&str; 64] = std::array::from_fn(|index| names[index].as_str());
    let mut scope = vault
        .protected_blob_scope()
        .expect("actual contained production ACL session");
    let mut seen = Vec::new();
    scope
        .scan_optional_private_blobs(&names, |index, bytes| {
            seen.push((index, bytes.map(Vec::from)));
            Ok(())
        })
        .unwrap();
    assert_eq!(seen.len(), 64);
    assert_eq!(seen[0], (0, Some(vec![3, 7])));
    assert!(seen[1..].iter().all(|(_, bytes)| bytes.is_none()));
    scope
        .create_private_blob("receipt-r0-g1", &[11, 13])
        .unwrap();
    seen.clear();
    scope
        .scan_optional_private_blobs(&names, |index, bytes| {
            seen.push((index, bytes.map(Vec::from)));
            Ok(())
        })
        .unwrap();
    assert_eq!(seen.len(), 64);
    assert_eq!(seen[0], (0, Some(vec![3, 7])));
    assert_eq!(seen[1], (1, Some(vec![11, 13])));
    assert!(seen[2..].iter().all(|(_, bytes)| bytes.is_none()));
    scope
        .finish_append()
        .expect("all numbered ACKs, DONE, actual successful exit and joined pipes");
    descriptor(oracle, &root, &root_descriptor, true);
    descriptor(oracle, &old, &file_descriptor, true);
    descriptor(
        oracle,
        &root.join("blob-receipt-r0-g1"),
        &file_descriptor,
        true,
    );
    assert_eq!(fs::read(&old).unwrap(), old_bytes);
    assert_eq!(vault.read_private_blob("receipt-r0-g1").unwrap(), [11, 13]);
}
fn initialize(script: &Path, path: &Path, stage: &'static str) {
    assert_eq!(
        process::run(
            script,
            &[
                OsStr::new("-PrivatePath"),
                path.as_os_str(),
                OsStr::new("-Initialize")
            ]
        ),
        0,
        "original Set-Acl setup failed: stage={stage}, exists={}, file={}, directory={}",
        path.exists(),
        path.is_file(),
        path.is_dir()
    );
}
fn descriptor(script: &Path, path: &Path, descriptor: &Path, compare: bool) {
    let mut arguments = vec![
        OsStr::new("-PrivatePath"),
        path.as_os_str(),
        OsStr::new("-DescriptorPath"),
        descriptor.as_os_str(),
    ];
    if compare {
        arguments.push(OsStr::new("-Compare"));
    }
    assert_eq!(
        process::run(script, &arguments),
        0,
        "independent Get-Acl descriptor oracle failed"
    );
}
