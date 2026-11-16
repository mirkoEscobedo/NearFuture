use nf_modules::*;
use std::{
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};
#[test]
fn probe_binding_availability_and_cancellation_are_explicit() {
    let mut host = Host::new([1; 16], true).unwrap();
    let cancel = AtomicBool::new(false);
    let context = Context {
        deadline: Instant::now() + Duration::from_secs(1),
        cancelled: &cancel,
    };
    let mut r = Invocation {
        session: [1; 16],
        request_id: [2; 16],
        campaign_generation: 0,
        module: PROBE_MODULE,
        operation: 1,
        api_version: 1,
        required_capabilities: 1,
        payload: &[7; 32],
    };
    assert_eq!(host.dispatch(&r, 1, &context).outcome, Ok(vec![7; 32]));
    host.bind_campaign(3).unwrap();
    r.campaign_generation = 2;
    assert_eq!(
        host.dispatch(&r, 1, &context).outcome,
        Err(Error::StaleBinding)
    );
    r.campaign_generation = 3;
    r.api_version = 2;
    assert_eq!(
        host.dispatch(&r, 1, &context).outcome,
        Err(Error::UnsupportedVersion)
    );
    r.api_version = 1;
    r.module = 88;
    assert_eq!(
        host.dispatch(&r, 1, &context).outcome,
        Err(Error::UnsupportedModule)
    );
    r.module = 1;
    r.operation = 2;
    assert_eq!(
        host.dispatch(&r, 1, &context).outcome,
        Err(Error::UnsupportedOperation)
    );
    r.operation = 1;
    assert_eq!(
        host.dispatch(&r, 0, &context).outcome,
        Err(Error::UnsupportedCapability)
    );
    let disabled = Host::new([1; 16], false).unwrap();
    r.campaign_generation = 0;
    r.required_capabilities = 0;
    assert_eq!(
        disabled.dispatch(&r, 0, &context).outcome,
        Err(Error::Disabled)
    );
    cancel.store(true, std::sync::atomic::Ordering::Release);
    assert_eq!(
        host.dispatch(&r, 1, &context).outcome,
        Err(Error::Cancelled)
    );
    cancel.store(false, std::sync::atomic::Ordering::Release);
    let expired = Context {
        deadline: Instant::now(),
        cancelled: &cancel,
    };
    assert_eq!(host.dispatch(&r, 1, &expired).outcome, Err(Error::Deadline));
    host.stop();
    assert_eq!(host.available(), 0);
    assert_eq!(
        host.dispatch(&r, 1, &context).outcome,
        Err(Error::Unavailable)
    );
}

#[test]
fn snapshot_can_be_omitted_without_changing_the_probe() {
    let host = Host::new([1; 16], true)
        .unwrap()
        .with_snapshot(false)
        .unwrap();
    assert_eq!(host.available(), 1);
    let cancelled = AtomicBool::new(false);
    let context = Context {
        deadline: Instant::now() + Duration::from_secs(1),
        cancelled: &cancelled,
    };
    let request = Invocation {
        session: [1; 16],
        request_id: [2; 16],
        campaign_generation: 0,
        module: SNAPSHOT_MODULE,
        operation: 1,
        api_version: 1,
        required_capabilities: 0,
        payload: &[],
    };
    assert_eq!(
        host.dispatch(&request, 1, &context).outcome,
        Err(Error::Disabled)
    );
    #[cfg(not(feature = "snapshot"))]
    assert!(matches!(
        Host::new([1; 16], true).unwrap().with_snapshot(true),
        Err(Error::UnsupportedCapability)
    ));
}
