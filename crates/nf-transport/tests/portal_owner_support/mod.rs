use nf_transport::{
    portal_config::{PortalConfig, PortalMode},
    receipt_effects::ReceiptRepo,
};
pub fn server_config(repo: &ReceiptRepo) -> PortalConfig {
    let c = repo.config();
    PortalConfig {
        mode: PortalMode::Serve,
        local: nf_store::KnownFrontiers {
            scope: c.scope,
            event_sequence: nf_contract::identity::EventSeq(0),
            store_revision: 1,
            membership_revision: Some(1),
        },
        ruleset: c.ruleset,
        content: c.content,
        local_account: c.local_account,
        local_device: c.local_device,
        server: c.server_pin,
        addresses: Some(std::array::from_fn(|_| {
            "/ip4/127.0.0.1/tcp/0".parse().unwrap()
        })),
        originals: Vec::new(),
    }
}
