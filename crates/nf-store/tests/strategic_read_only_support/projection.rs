//! Independent test-only projection of exactly selected getter APIs; no product encoder.
use super::{Case, hash};
use nf_identity::model::PublicIdentity;
use nf_kernel::{Capability, Domain, ProviderKind, WorldSpec};
use nf_store::{
    StrategicSupportImplementation, StrategicSupportObservationV1, StrategicSupportRulesetBinding,
    StrategicSupportUnavailable, SupportProtection,
};
use std::fmt::Write;
pub const EXPECTED: &str = r#"{"profile":1,"schema_version":1,"event_sequence":1,"store_revision":3,"membership_revision":0,"protected":{"scope":"checked","event":"checked","store_revision":"checked","membership_revision":"checked","authority_term":"not_applicable"},"counts":{"factions":1,"relations":0,"markets":1,"providers":1,"registry":1,"assignments":2,"pending":1,"pending_requests":1,"committed_requests":1,"reservations":0,"outbox":1,"members":1},"manifests":[{"provider":"provider0","kind":"manual","version":1,"state_schema":1,"read_domains":["faction","market","provider"],"write_domains":["market","provider"],"capabilities":["adjust_market"],"principal_count":1,"max_events":2,"implementation":{"classification":"registered_unverified","public_artifact":null,"sha256":null}}],"assignments":[{"aggregate":"aggregate0","provider":"provider0","generation":1,"activation_event":0,"ruleset_binding":"matches_persisted_ruleset"},{"aggregate":"aggregate1","provider":"provider0","generation":1,"activation_event":0,"ruleset_binding":"matches_persisted_ruleset"}],"unavailable":["current_owner","authority_term","selector","provider_lag","taint","native_transfer","transport_receipt","runtime_provenance"]}"#;
pub const EXPECTED_SHA256: [u8; 32] = [
    0x0d, 0xdd, 0xb6, 0x2f, 0x75, 0xa1, 0x9c, 0x60, 0x1a, 0x97, 0xad, 0x67, 0xb7, 0xd0, 0x5c, 0x17,
    0x40, 0x8e, 0x12, 0x6c, 0x33, 0x01, 0xa8, 0x02, 0xab, 0x52, 0x09, 0xe8, 0x9f, 0xe8, 0x26, 0x2d,
];
fn protection(value: SupportProtection) -> &'static str {
    match value {
        SupportProtection::Checked => "checked",
        SupportProtection::NotSupplied => "not_supplied",
        SupportProtection::NotApplicable => "not_applicable",
    }
}
fn domain(value: &Domain) -> &'static str {
    match value {
        Domain::Faction => "faction",
        Domain::Relation => "relation",
        Domain::Market => "market",
        Domain::Provider => "provider",
    }
}
fn capability(value: &Capability) -> &'static str {
    match value {
        Capability::SeekPeace => "seek_peace",
        Capability::AdjustRelation => "adjust_relation",
        Capability::AdjustMarket => "adjust_market",
    }
}
fn unavailable(value: &StrategicSupportUnavailable) -> &'static str {
    match value {
        StrategicSupportUnavailable::CurrentOwner => "current_owner",
        StrategicSupportUnavailable::AuthorityTerm => "authority_term",
        StrategicSupportUnavailable::Selector => "selector",
        StrategicSupportUnavailable::ProviderLag => "provider_lag",
        StrategicSupportUnavailable::Taint => "taint",
        StrategicSupportUnavailable::NativeTransfer => "native_transfer",
        StrategicSupportUnavailable::TransportReceipt => "transport_receipt",
        StrategicSupportUnavailable::RuntimeProvenance => "runtime_provenance",
    }
}
fn nullable(value: Option<u64>) -> String {
    value.map_or("null".into(), |v| v.to_string())
}
fn strings<T>(values: &[T], label: impl Fn(&T) -> &'static str) -> String {
    values
        .iter()
        .map(|v| format!("\"{}\"", label(v)))
        .collect::<Vec<_>>()
        .join(",")
}
pub fn projection(observation: &StrategicSupportObservationV1) -> String {
    let p = observation.protected();
    let c = observation.counts();
    let mut out = format!(
        "{{\"profile\":{},\"schema_version\":{},\"event_sequence\":{},\"store_revision\":{},\"membership_revision\":{},\"protected\":{{\"scope\":\"{}\",\"event\":\"{}\",\"store_revision\":\"{}\",\"membership_revision\":\"{}\",\"authority_term\":\"{}\"}},\"counts\":{{\"factions\":{},\"relations\":{},\"markets\":{},\"providers\":{},\"registry\":{},\"assignments\":{},\"pending\":{},\"pending_requests\":{},\"committed_requests\":{},\"reservations\":{},\"outbox\":{},\"members\":{}}},\"manifests\":[",
        observation.profile(),
        observation.schema_version(),
        observation.event_sequence(),
        observation.store_revision(),
        nullable(observation.membership_revision()),
        protection(p.scope()),
        protection(p.event()),
        protection(p.store_revision()),
        protection(p.membership_revision()),
        protection(p.authority_term()),
        c.factions(),
        c.relations(),
        c.markets(),
        c.providers(),
        c.registry(),
        c.assignments(),
        c.pending(),
        c.pending_requests(),
        c.committed_requests(),
        c.reservations(),
        c.outbox(),
        nullable(c.members()),
    );
    for (index, m) in observation.manifests().iter().enumerate() {
        if index != 0 {
            out.push(',');
        }
        let kind = match m.kind() {
            ProviderKind::SyntheticPeace => "synthetic_peace",
            ProviderKind::Manual => "manual",
        };
        let implementation = match m.implementation_classification() {
            StrategicSupportImplementation::RegisteredUnverified => "registered_unverified",
        };
        write!(out, "{{\"provider\":\"provider{}\",\"kind\":\"{}\",\"version\":{},\"state_schema\":{},\"read_domains\":[{}],\"write_domains\":[{}],\"capabilities\":[{}],\"principal_count\":{},\"max_events\":{},\"implementation\":{{\"classification\":\"{}\",\"public_artifact\":null,\"sha256\":null}}}}",
            m.provider_ordinal(), kind, m.version(), m.state_schema(), strings(m.read_domains(), domain),
            strings(m.write_domains(), domain), strings(m.capabilities(), capability),
            m.principal_count(), m.max_events(), implementation).unwrap();
    }
    out.push_str("],\"assignments\":[");
    for (index, a) in observation.assignments().iter().enumerate() {
        if index != 0 {
            out.push(',');
        }
        let binding = match a.ruleset_binding() {
            StrategicSupportRulesetBinding::MatchesPersistedRuleset => "matches_persisted_ruleset",
        };
        write!(out, "{{\"aggregate\":\"aggregate{}\",\"provider\":\"provider{}\",\"generation\":{},\"activation_event\":{},\"ruleset_binding\":\"{}\"}}",
            a.aggregate_ordinal(), a.provider_ordinal(), a.generation(), a.activation_event(), binding).unwrap();
    }
    write!(
        out,
        "],\"unavailable\":[{}]}}",
        strings(observation.unavailable(), unavailable)
    )
    .unwrap();
    assert!(out.len() <= 16384);
    out
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|v| format!("{v:02x}")).collect()
}
pub fn assert_private_fields_absent(
    text: &str,
    world: &WorldSpec,
    public: &PublicIdentity,
    member0: &[u8],
    member1: &[u8],
    case: &Case,
) {
    for marker in [
        "NF_FAKE_PRIVATE_CHAT_DO_NOT_EXPORT",
        "NF_FAKE_KEY_MATERIAL_DO_NOT_EXPORT",
        "NF_FAKE_SAVE_CONTENT_DO_NOT_EXPORT",
        "public-strategic-reader-fixture",
    ] {
        assert!(!text.contains(marker), "private marker exposed");
    }
    for bytes in [
        world.universe.as_bytes().as_slice(),
        world.history.as_bytes().as_slice(),
        world.seed.as_slice(),
        world.ruleset_hash.as_slice(),
        public.account.as_bytes().as_slice(),
        public.device.as_bytes().as_slice(),
        public.account_key.as_slice(),
        public.device_key.as_slice(),
    ] {
        assert!(
            !text.contains(&hex(bytes)),
            "raw identity/key/seed/ruleset exposed"
        );
        assert!(
            !text.contains(&format!("{bytes:?}")),
            "raw byte array exposed"
        );
    }
    for manifest in &world.registry {
        assert!(
            !text.contains(&hex(&manifest.implementation_hash)),
            "registered private fingerprint exposed"
        );
        assert!(
            !text.contains(&format!("{:?}", manifest.implementation_hash)),
            "registered raw fingerprint exposed"
        );
    }
    for bytes in [member0, member1] {
        assert!(
            !text.contains(&hex(&hash(bytes))),
            "membership fingerprint exposed"
        );
        assert!(
            !text.contains(&format!("{:?}", hash(bytes))),
            "raw membership fingerprint exposed"
        );
    }
    assert!(
        !text.contains(case.root().to_str().unwrap()),
        "private fixture path exposed"
    );
}
