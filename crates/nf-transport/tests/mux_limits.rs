use nf_transport::mux::LaneMuxConfig;
use nf_transport::records::Lane;
#[test]
fn physical_lane_backends_have_explicit_finite_credit_and_stream_ceilings() {
    let control = LaneMuxConfig::new(Lane::Control);
    let bulk = LaneMuxConfig::new(Lane::Bulk);
    assert_eq!(control.stream_limit(), 4);
    assert_eq!(control.receive_window(), 1048576);
    assert_eq!(control.buffer_limit(), 4);
    assert_eq!(bulk.stream_limit(), 1);
    assert_eq!(bulk.receive_window(), 262144);
    assert_eq!(bulk.buffer_limit(), 1);
}
