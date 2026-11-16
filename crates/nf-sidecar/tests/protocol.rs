use nf_modules::*;
use nf_sidecar::protocol::*;
use zeroize::Zeroizing;
fn request() -> Invocation<'static> {
    Invocation {
        session: [1; 16],
        request_id: [2; 16],
        campaign_generation: 0,
        module: 1,
        operation: 1,
        api_version: 1,
        required_capabilities: 1,
        payload: &[7; 32],
    }
}
#[test]
fn mutual_negotiation_channel_and_response_correlation() {
    let token = Zeroizing::new([9; 32]);
    let h = hello(&token, [1; 16], 1234, [3; 16], &Offer::default());
    assert_eq!(h.len(), 112);
    let server = accept(&token, [1; 16], 1234, 1, [4; 16], &h).unwrap();
    assert_eq!(server.frame.len(), 120);
    let client = finish(&token, [1; 16], &h, &server.frame).unwrap();
    assert_eq!(client.status, 0);
    let r = request();
    let bytes = client.channel.encode_request(&r, 1, 500).unwrap();
    assert_eq!(bytes.len(), 136);
    let decoded = server.channel.decode_request(&bytes).unwrap();
    assert_eq!(decoded.invocation.payload, [7; 32]);
    let reply = Reply {
        session: r.session,
        request_id: r.request_id,
        campaign_generation: 0,
        outcome: Ok(vec![7; 32]),
    };
    let bytes = server.channel.encode_reply(&reply, 1).unwrap();
    assert_eq!(bytes.len(), 122);
    assert_eq!(client.channel.decode_reply(&bytes, &r, 1).unwrap(), reply);
    assert_eq!(
        client.channel.decode_reply(&bytes, &r, 2).unwrap_err(),
        Error::StaleBinding
    );
    let mut altered = request();
    altered.request_id = [8; 16];
    assert_eq!(
        client
            .channel
            .decode_reply(&bytes, &altered, 1)
            .unwrap_err(),
        Error::StaleBinding
    );
    altered = request();
    altered.session = [8; 16];
    assert_eq!(
        client
            .channel
            .decode_reply(&bytes, &altered, 1)
            .unwrap_err(),
        Error::StaleBinding
    );
    altered = request();
    altered.campaign_generation = 1;
    assert_eq!(
        client
            .channel
            .decode_reply(&bytes, &altered, 1)
            .unwrap_err(),
        Error::StaleBinding
    );
    let mut corrupt = bytes;
    corrupt[58] ^= 1;
    assert!(client.channel.decode_reply(&corrupt, &r, 1).is_err());
    let another = accept(&token, [1; 16], 1234, 1, [5; 16], &h).unwrap();
    assert!(
        another
            .channel
            .decode_request(&client.channel.encode_request(&r, 1, 100).unwrap())
            .is_err()
    );
}
#[test]
fn explicit_version_capability_limit_refusals_and_tamper() {
    let token = Zeroizing::new([9; 32]);
    for (offer, status) in [
        (
            Offer {
                minimum_version: 2,
                maximum_version: 3,
                ..Offer::default()
            },
            Error::UnsupportedVersion,
        ),
        (
            Offer {
                required: 2,
                requested: 3,
                ..Offer::default()
            },
            Error::UnsupportedCapability,
        ),
        (
            Offer {
                limits: Limits {
                    request_bytes: 100,
                    ..Limits::default()
                },
                ..Offer::default()
            },
            Error::Limit,
        ),
    ] {
        let h = hello(&token, [1; 16], 1234, [3; 16], &offer);
        let a = accept(&token, [1; 16], 1234, 1, [4; 16], &h).unwrap();
        assert_eq!(
            finish(&token, [1; 16], &h, &a.frame).unwrap().status,
            status as u16
        );
    }
    let mut h = hello(&token, [1; 16], 1234, [3; 16], &Offer::default());
    h[56] ^= 1;
    assert!(accept(&token, [1; 16], 1234, 1, [4; 16], &h).is_err());
    let h = hello(&token, [1; 16], 1234, [3; 16], &Offer::default());
    assert!(accept(&Zeroizing::new([8; 32]), [1; 16], 1234, 1, [4; 16], &h).is_err());
    assert!(accept(&token, [2; 16], 1234, 1, [4; 16], &h).is_err());
    let a = accept(&token, [1; 16], 1234, 1, [4; 16], &h).unwrap();
    assert!(a.channel.decode_request(&[0; 103]).is_err());
    let mut r = request();
    let large = vec![0; MAX_FRAME];
    r.payload = &large;
    assert_eq!(
        a.channel.encode_request(&r, 1, 100).unwrap_err(),
        Error::Limit
    );
}
