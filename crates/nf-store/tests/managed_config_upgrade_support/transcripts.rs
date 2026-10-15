//! Exact challenge/action observations. Parsed random fields are data, never grants.
use super::{
    cursor::Cursor,
    fixture::now_ms,
    records::{blob, hash, proof_bytes},
};
use nf_identity::model::{DeviceProof, MembershipState, PublicIdentity};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Stamp {
    pub bytes: Vec<u8>,
    pub installation: [u8; 16],
    pub generation: u64,
    pub account: [u8; 16],
    pub device: [u8; 16],
    pub incarnation: [u8; 16],
    pub term: u64,
    pub session: u64,
    pub profile: u32,
    pub schema: [u8; 32],
    pub ruleset: [u8; 32],
    pub member: Option<u64>,
    pub member_digest: [u8; 32],
}
pub fn stamp(input: &mut Cursor<'_>) -> Stamp {
    let start = input.at;
    let installation = input.take(16).try_into().unwrap();
    let generation = input.u64();
    let account = input.take(16).try_into().unwrap();
    let device = input.take(16).try_into().unwrap();
    let incarnation = input.take(16).try_into().unwrap();
    let term = input.u64();
    let session = input.u64();
    let profile = input.u32();
    let schema = input.take(32).try_into().unwrap();
    let ruleset = input.take(32).try_into().unwrap();
    let member = input
        .option(8)
        .map(|v| u64::from_be_bytes(v.try_into().unwrap()));
    let member_digest = input.take(32).try_into().unwrap();
    Stamp {
        bytes: input.bytes[start..input.at].to_vec(),
        installation,
        generation,
        account,
        device,
        incarnation,
        term,
        session,
        profile,
        schema,
        ruleset,
        member,
        member_digest,
    }
}
pub struct Audit {
    nonces: BTreeSet<[u8; 32]>,
    sequences: BTreeMap<[u8; 16], u64>,
    pub cutoff: u64,
    pub start: u64,
}
impl Audit {
    pub fn new(start: u64, cutoff: u64) -> Self {
        Self {
            nonces: BTreeSet::new(),
            sequences: BTreeMap::new(),
            start,
            cutoff,
        }
    }
    fn sequence(&mut self, incarnation: [u8; 16], sequence: u64) {
        assert!(sequence > self.sequences.get(&incarnation).copied().unwrap_or(0));
        self.sequences.insert(incarnation, sequence);
    }
    fn nonce(&mut self, nonce: [u8; 32], issued: u64, expires: u64) {
        assert_ne!(nonce, [0; 32]);
        assert!(self.nonces.insert(nonce));
        assert!(issued >= self.start && issued <= now_ms());
        assert!(issued < expires && expires <= self.cutoff);
        assert!(expires - issued <= 20_000);
    }
}
#[derive(Clone)]
pub struct Auth {
    pub stamp: Stamp,
    pub purpose: u32,
    pub sequence: u64,
    pub operation: [u8; 16],
    pub revision: u64,
    pub digest: [u8; 32],
    pub head: [u8; 32],
    pub payload: [u8; 32],
    pub auth_sequence: u64,
    pub nonce: [u8; 32],
    pub issued: u64,
    pub expires: u64,
}
pub fn assert_person(value: &Stamp, public: &PublicIdentity, member: &MembershipState) {
    assert_eq!(value.account, *public.account.as_bytes());
    assert_eq!(value.device, *public.device.as_bytes());
    assert_eq!(value.member, Some(member.revision));
    assert_eq!(
        value.member_digest,
        hash(&nf_identity::codec::encode_state(member).unwrap())
    );
    assert_ne!(value.installation, [0; 16]);
    assert_ne!(value.incarnation, [0; 16]);
}
pub fn assert_epoch(
    s: &Stamp,
    generation: u64,
    term: u64,
    session: u64,
    profile: u32,
    ruleset: [u8; 32],
) {
    assert_eq!(
        (s.generation, s.term, s.session, s.profile),
        (generation, term, session, profile)
    );
    assert_eq!(s.ruleset, ruleset);
    assert_eq!(
        s.schema,
        if profile == 1 {
            super::records::PROFILE1_SCHEMA
        } else {
            super::records::PROFILE4_SCHEMA
        }
    );
}
pub struct ProspectiveExpected<'a> {
    pub foot: &'a [u8],
    pub catalog: Option<(u64, [u8; 32], [u8; 32])>,
    pub public: &'a PublicIdentity,
    pub member: &'a MembershipState,
    pub paths: &'a [&'a str],
    pub composition: &'a [u8],
}
pub struct ExpectedPre<'a> {
    pub stamp: &'a [u8],
    pub revision: u64,
    pub digest: [u8; 32],
    pub head: [u8; 32],
}
pub fn prospective(
    bytes: &[u8],
    admission: &[u8],
    purpose: u32,
    expected: ProspectiveExpected<'_>,
    audit: &mut Audit,
) -> Stamp {
    let ProspectiveExpected {
        foot: expected_foot,
        catalog: expected_catalog,
        public,
        member,
        paths,
        composition: expected_composition,
    } = expected;
    let mut c = Cursor::new(bytes, b"NF-OWNER-PROSPECTIVE-1\0", 4096);
    assert_eq!(c.u32(), purpose);
    let installation: [u8; 16] = c.take(16).try_into().unwrap();
    assert_eq!(c.take(16), member.scope.universe.as_bytes());
    assert_eq!(c.take(16), member.scope.history.as_bytes());
    let catalog = c.option(72).map(|v| {
        (
            u64::from_be_bytes(v[..8].try_into().unwrap()),
            v[8..40].try_into().unwrap(),
            v[40..].try_into().unwrap(),
        )
    });
    assert_eq!(catalog, expected_catalog);
    let owner = stamp(&mut c);
    assert_eq!(owner.installation, installation);
    assert_person(&owner, public, member);
    assert_eq!(c.blob(2048), expected_foot);
    assert_eq!(c.blob(128), public.peer);
    assert_eq!(c.take(32), hash(admission));
    let sequence = c.u64();
    audit.sequence(owner.incarnation, sequence);
    let nonce = c.take(32).try_into().unwrap();
    let issued = c.u64();
    let expires = c.u64();
    audit.nonce(nonce, issued, expires);
    c.finish();
    let mut data = Cursor::new(admission, b"NF-OWNER-ADMISSION-1\0", 172_002);
    assert_eq!(data.u32(), purpose);
    assert_eq!(data.take(16), installation);
    assert_eq!(data.take(16), member.scope.universe.as_bytes());
    assert_eq!(data.take(16), member.scope.history.as_bytes());
    assert_eq!(data.blob(2048), expected_foot);
    let composition = data.blob(1923);
    assert_eq!(composition, expected_composition);
    let catalog_path = data.blob(4096);
    assert_eq!(std::str::from_utf8(catalog_path).unwrap(), paths[0]);
    let delta = data.blob(131072);
    assert!(delta.starts_with(b"NF-OWNER-ROW-DELTA-1\0\x01\0"));
    let operation = data.blob(32768);
    data.finish();
    // Every path has a canonical length-prefixed occurrence in signed admission/delta/operation.
    for path in paths {
        let mut encoded = Vec::new();
        blob(&mut encoded, path.as_bytes());
        assert!(
            admission.windows(encoded.len()).any(|v| v == encoded),
            "unbound path {path}"
        );
    }
    assert!(operation.starts_with(b"NF-OWNER-OPERATION-1\0\x01\0"));
    owner
}
pub fn owned(
    bytes: &[u8],
    purpose: u32,
    pre: ExpectedPre<'_>,
    payload: &[u8],
    audit: &mut Audit,
) -> Auth {
    let ExpectedPre {
        stamp: expected_stamp,
        revision,
        digest,
        head,
    } = pre;
    let mut c = Cursor::new(bytes, b"NF-OWNER-AUTH-1\0", 4096);
    let owner = stamp(&mut c);
    assert_eq!(owner.bytes, expected_stamp);
    assert_eq!(c.u32(), purpose);
    let sequence = c.u64();
    let operation = c.take(16).try_into().unwrap();
    assert!(sequence > 0);
    assert_ne!(operation, [0; 16]);
    assert_eq!(c.u64(), revision);
    assert_eq!(c.take(32), digest);
    assert_eq!(c.take(32), head);
    assert_eq!(c.take(32), hash(payload));
    let auth_sequence = c.u64();
    audit.sequence(owner.incarnation, auth_sequence);
    let nonce = c.take(32).try_into().unwrap();
    let issued = c.u64();
    let expires = c.u64();
    audit.nonce(nonce, issued, expires);
    c.finish();
    Auth {
        stamp: owner,
        purpose,
        sequence,
        operation,
        revision,
        digest,
        head,
        payload: hash(payload),
        auth_sequence,
        nonce,
        issued,
        expires,
    }
}
pub fn action(auth: &Auth, payload: &[u8], proof: &DeviceProof) -> Vec<u8> {
    assert_eq!(hash(payload), auth.payload);
    let mut out = b"NF-ACTIVE-ACTION-1\0".to_vec();
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&auth.purpose.to_le_bytes());
    out.extend_from_slice(&auth.stamp.bytes);
    out.extend_from_slice(&auth.sequence.to_be_bytes());
    out.extend_from_slice(&auth.operation);
    out.extend_from_slice(&auth.revision.to_be_bytes());
    out.extend_from_slice(&auth.digest);
    out.extend_from_slice(&auth.head);
    out.extend_from_slice(&auth.auth_sequence.to_be_bytes());
    out.extend_from_slice(&auth.nonce);
    out.extend_from_slice(&auth.issued.to_be_bytes());
    out.extend_from_slice(&auth.expires.to_be_bytes());
    blob(&mut out, &proof_bytes(proof));
    blob(&mut out, payload);
    assert!(out.len() <= 1_114_112);
    out
}
pub fn principal(
    bytes: &[u8],
    pre: ExpectedPre<'_>,
    frontier: &nf_kernel::Frontier,
    request: nf_contract::identity::RequestId,
    device: nf_contract::identity::DeviceId,
    audit: &mut Audit,
) -> (u64, [u8; 16], [u8; 32], u64, u64) {
    let ExpectedPre {
        stamp: expected_stamp,
        revision,
        digest,
        head,
    } = pre;
    let mut c = Cursor::new(bytes, b"NF-OWNER-PRINCIPAL-1\0", 4096);
    assert_eq!(stamp(&mut c).bytes, expected_stamp);
    let sequence = c.u64();
    let operation = c.take(16).try_into().unwrap();
    assert_eq!(c.u64(), revision);
    assert_eq!(c.take(32), digest);
    assert_eq!(c.take(32), head);
    assert_eq!(
        c.take(32),
        hash(&nf_kernel::encode_frontier(frontier).unwrap())
    );
    assert_eq!(c.take(16), request.as_bytes());
    assert_eq!(c.take(16), device.as_bytes());
    let intent = frontier
        .jobs()
        .iter()
        .find(|j| j.intent().request == request)
        .unwrap()
        .intent();
    assert_eq!(c.take(32), hash(&nf_kernel::encode_intent(intent).unwrap()));
    let nonce = c.take(32).try_into().unwrap();
    let issued = c.u64();
    let expires = c.u64();
    audit.nonce(nonce, issued, expires);
    c.finish();
    (sequence, operation, nonce, issued, expires)
}
