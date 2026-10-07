#[path = "support/blob_scan.rs"]
mod support;
use std::time::Instant;
#[test]
fn complete_empty_and_maximal_present_passes_remain_bounded() {
    let (_root, vault) = support::fixture();
    let names = support::names();
    let names: Vec<_> = names.iter().map(String::as_str).collect();
    let start = Instant::now();
    let mut absent = 0;
    vault
        .scan_optional_private_blobs(&names, |_, payload| {
            assert_eq!(payload, None);
            absent += 1;
            Ok(())
        })
        .unwrap();
    assert_eq!(absent, 64);
    println!(
        "SCAN_EMPTY_64 elapsed_ms={} callbacks=64",
        start.elapsed().as_millis()
    );
    let payload = [7u8; 4096];
    for name in &names {
        vault.create_private_blob(name, &payload).unwrap();
    }
    let start = Instant::now();
    let mut present = 0;
    let mut maximum = 0;
    vault
        .scan_optional_private_blobs(&names, |_, bytes| {
            let bytes = bytes.unwrap();
            assert_eq!(bytes, payload);
            maximum = maximum.max(bytes.len());
            present += 1;
            Ok(())
        })
        .unwrap();
    assert_eq!((present, maximum), (64, 4096));
    println!(
        "SCAN_PRESENT_64 elapsed_ms={} callbacks=64 max_payload=4096",
        start.elapsed().as_millis()
    );
}
