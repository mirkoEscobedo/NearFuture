use super::chat_support::Fixture;
use nf_identity::{
    model::{DeviceProof, Scope},
    private_storage::LocalIdentity,
    signing::device_digest,
};
use nf_store::chat::IssuedChallenge;
use std::{collections::BTreeMap, fs};

pub(super) fn proof(actor: &LocalIdentity, scope: Scope, issued: IssuedChallenge) -> DeviceProof {
    let mut proof = DeviceProof {
        scope,
        account: actor.public.account,
        device: actor.public.device,
        frontier: issued.membership_revision,
        peer: actor.public.peer.clone(),
        challenge: issued.challenge,
        signature: [0; 64],
    };
    proof.signature = actor.device_key.sign(&device_digest(&proof).unwrap());
    proof
}
#[derive(PartialEq, Eq)]
pub(super) struct Snapshot(BTreeMap<String, Vec<u8>>);
impl Snapshot {
    pub(super) fn take(fixture: &Fixture) -> Self {
        Self(
            fs::read_dir(&fixture.scratch.0)
                .unwrap()
                .map(|entry| {
                    let entry = entry.unwrap();
                    assert!(entry.file_type().unwrap().is_file());
                    (
                        entry.file_name().into_string().unwrap(),
                        fs::read(entry.path()).unwrap(),
                    )
                })
                .collect(),
        )
    }
    pub(super) fn assert_unchanged(&self, fixture: &Fixture) {
        assert!(
            *self == Self::take(fixture),
            "CHAT_REVOCATION_REFUSAL_CHANGED_DATABASE_OR_SIDECARS"
        );
    }
}
