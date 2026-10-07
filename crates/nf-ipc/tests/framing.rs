use nf_ipc::{FrameDecoder, IpcError};
#[test]
fn partial_header_and_body_return_only_one_complete_frame_and_reject_declared_oversize() {
    let mut decoder = FrameDecoder::new(8).unwrap();
    assert_eq!(decoder.push(&[0, 0]).unwrap().consumed, 2);
    assert!(decoder.push(&[0, 3, 7]).unwrap().frame.is_none());
    let completed = decoder.push(&[8, 9, 0, 0, 0, 1, 10]).unwrap();
    assert_eq!(completed.consumed, 2);
    assert_eq!(completed.frame, Some(vec![7, 8, 9]));
    let next = decoder.push(&[0, 0, 0, 1, 10]).unwrap();
    assert_eq!(next.frame, Some(vec![10]));
    assert_eq!(decoder.push(&[0, 0, 0, 9]), Err(IpcError::Limit));
}
#[test]
fn truncated_stream_and_illegal_prefix_are_rejected_without_retaining_state() {
    let mut decoder = FrameDecoder::new(4).unwrap();
    decoder.push(&[0, 0, 0, 2, 7]).unwrap();
    assert_eq!(decoder.finish(), Err(IpcError::Incomplete));
    decoder.reset();
    assert_eq!(decoder.finish(), Ok(()));
    assert_eq!(decoder.push(&[0, 0, 0, 0]), Err(IpcError::Malformed));
    assert_eq!(decoder.buffered_bytes(), 0);
    assert_eq!(decoder.push(&[0, 0, 0, 1, 8]).unwrap().frame, Some(vec![8]));
}
