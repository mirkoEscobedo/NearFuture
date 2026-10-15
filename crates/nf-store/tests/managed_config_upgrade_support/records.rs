//! Independent closed record encodings; existing kernel/identity codecs supply canonical values.
use nf_contract::identity::DeviceId;
use nf_identity::model::MembershipState;
use nf_kernel::{CommittedBatch, Intent, World};
use nf_store::{KnownFrontiers, migration::{Composition, ProviderVersion}};
use sha2::{Digest, Sha256};
pub const PROFILE1_SCHEMA: [u8; 32] = [
    0x7e, 0x06, 0x49, 0x12, 0x04, 0xd7, 0xaf, 0xa6, 0x93, 0x5f, 0x76, 0xf7, 0x69, 0xcb, 0x3f, 0xbf,
    0x98, 0x1c, 0x8b, 0x7c, 0x9c, 0x65, 0x10, 0x06, 0x73, 0xce, 0x3e, 0x32, 0x01, 0x23, 0x6e, 0xbb,
];
pub const PROFILE4_SCHEMA: [u8; 32] = [
    0x4a, 0x2c, 0xb2, 0x06, 0x36, 0x9d, 0x17, 0xa3, 0xf4, 0xc1, 0x34, 0x60, 0x4e, 0x01, 0xda, 0x01,
    0x56, 0x65, 0x73, 0x1b, 0xaa, 0x4d, 0xcb, 0x29, 0x92, 0xab, 0x8e, 0xc9, 0x91, 0x41, 0x93, 0xd3,
];

pub const FROM_COMPOSITION:[u8;32]=[0x7b,0xc6,0x6e,0x90,0xe7,0x9e,0x9b,0x12,0xe0,0x9e,0xa0,0xb9,0x46,0xd5,0x7c,0x2c,0x6b,0xb4,0x46,0x14,0x21,0x73,0xc7,0xc7,0x86,0x3f,0x91,0x1f,0x99,0xab,0x27,0x0e];
pub const TO_COMPOSITION:[u8;32]=[0x45,0x1c,0xf1,0x5e,0xae,0x2f,0xec,0x90,0xed,0x3b,0x35,0x20,0xf1,0x72,0x7d,0xc3,0x82,0x75,0x9e,0x7c,0xd2,0xed,0x1d,0x0c,0x49,0x2c,0x66,0xe0,0xfa,0x95,0xd8,0xfe];
pub fn hash(bytes: &[u8]) -> [u8; 32] { Sha256::digest(bytes).into() }
pub fn descriptor() -> ProviderVersion {
    ProviderVersion {
        provider: nf_contract::identity::ProviderId::from_bytes([41; 16]),
        implementation: [41; 32],
        version: 1,
        state_schema: 1,
    }
}
pub fn composition(storage: u32, schema: [u8; 32], config: u8) -> Composition {
    let value = Composition::new(storage, schema, [config; 32], [3; 32], vec![descriptor()]).unwrap();
    assert_eq!(value.digest(), hash(&composition_bytes(storage, schema, config)));
    assert_eq!(value.digest(),match (storage,config){(1,2)=>FROM_COMPOSITION,(4,9)=>TO_COMPOSITION,_=>panic!("outside fixed firstcase")});
    value
}
pub fn composition_bytes(storage: u32, schema: [u8; 32], config: u8) -> Vec<u8> {
    let mut bytes = b"NF-MIGRATION-COMPOSITION-1\0".to_vec();
    bytes.extend_from_slice(&storage.to_le_bytes());
    bytes.extend_from_slice(&schema);
    bytes.extend_from_slice(&[config; 32]);
    bytes.extend_from_slice(&[3; 32]);
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&[41; 16]);
    bytes.extend_from_slice(&[41; 32]);
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes
}

pub(super) fn blob(bytes: &mut Vec<u8>, value: &[u8]) {
    bytes.extend_from_slice(&u32::try_from(value.len()).unwrap().to_le_bytes());
    bytes.extend_from_slice(value);
}
pub fn completed_core(
    world: &World,
    records: &[(&Intent, DeviceId, &CommittedBatch)],
) -> Vec<u8> {
    let mut bytes = b"NF-STORE-1\0\x01\0".to_vec();
    blob(&mut bytes, &nf_kernel::encode_snapshot(world).unwrap());
    bytes.push(0);
    let mut records = records.to_vec();
    records.sort_by_key(|(intent, _, _)| intent.request);
    bytes.extend_from_slice(&u32::try_from(records.len()).unwrap().to_le_bytes());
    for (intent, device, batch) in &records {
        assert!(batch.outcomes.iter().any(|o| o.operation == intent.operation && o.rejection.is_none()));
        blob(&mut bytes, &nf_kernel::encode_intent(intent).unwrap());
        bytes.extend_from_slice(device.as_bytes());
        bytes.push(2);
        bytes.extend_from_slice(&batch.sequence.0.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
    }
    bytes.extend_from_slice(&0u32.to_le_bytes());
    records.sort_by_key(|(intent, _, _)| intent.operation);
    bytes.extend_from_slice(&u32::try_from(records.len()).unwrap().to_le_bytes());
    for (intent, _, batch) in records {
        bytes.extend_from_slice(intent.operation.as_bytes());
        bytes.extend_from_slice(&batch.sequence.0.to_le_bytes());
        bytes.extend_from_slice(&hash(&nf_kernel::encode_batch(batch).unwrap()));
        bytes.extend_from_slice(&0u32.to_le_bytes());
    }
    assert!(bytes.len() <= 1_048_576);
    bytes
}
pub fn envelope(core: &[u8], member: &MembershipState, pending: Option<(u64, u64)>) -> Vec<u8> {
    let mut bytes = b"NF-ACTIVE-STATE-1\0".to_vec();
    bytes.extend_from_slice(&1u16.to_le_bytes());
    blob(&mut bytes, core);
    bytes.push(1);
    blob(&mut bytes, &nf_identity::codec::encode_state(member).unwrap());
    bytes.extend_from_slice(&65_536u32.to_le_bytes());
    match pending {
        None => bytes.push(0),
        Some((admitted, cutoff)) => {
            bytes.extend_from_slice(&[1, 1]);
            bytes.extend_from_slice(&admitted.to_be_bytes());
            bytes.extend_from_slice(&cutoff.to_be_bytes());
        }
    }
    bytes.extend_from_slice(&0u32.to_le_bytes());
    assert!(bytes.len() <= 1_884_160);
    bytes
}
pub fn source_binding(
    known: KnownFrontiers, head: [u8; 32], core: [u8; 32], member: [u8; 32], rows: [u8; 32],
) -> Vec<u8> {
    let mut bytes = b"NF-ACTIVE-SOURCE-1\0".to_vec();
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(known.scope.universe.as_bytes());
    bytes.extend_from_slice(known.scope.history.as_bytes());
    bytes.extend_from_slice(&known.store_revision.to_be_bytes());
    bytes.extend_from_slice(&known.event_sequence.0.to_be_bytes());
    bytes.push(1);
    bytes.extend_from_slice(&known.membership_revision.unwrap().to_be_bytes());
    bytes.extend_from_slice(&head);
    bytes.extend_from_slice(&core);
    bytes.push(1);
    bytes.extend_from_slice(&member);
    bytes.extend_from_slice(&65_536u32.to_le_bytes());
    bytes.extend_from_slice(&rows);
    bytes.extend_from_slice(&PROFILE1_SCHEMA);
    assert_eq!(bytes.len(), 243);
    bytes
}
pub fn upgrade_action(
    known: KnownFrontiers, head: [u8; 32], core: [u8; 32], member: [u8; 32], rows: [u8; 32],
) -> Vec<u8> {
    let mut bytes = b"NF-ACTIVE-UPGRADE-1\0".to_vec();
    bytes.extend_from_slice(&1u16.to_le_bytes());
    blob(&mut bytes, &source_binding(known, head, core, member, rows));
    blob(&mut bytes, &composition_bytes(1, PROFILE1_SCHEMA, 2));
    blob(&mut bytes, &composition_bytes(4, PROFILE4_SCHEMA, 9));
    assert_eq!(bytes.len(), 651);
    bytes
}
pub fn lineage(action: &[u8], revision: u64, before: [u8; 32], after: [u8; 32]) -> Vec<u8> {
    let offset = b"NF-ACTIVE-UPGRADE-1\0".len() + 2;
    let size = u32::from_le_bytes(action[offset..offset + 4].try_into().unwrap()) as usize;
    let source = &action[offset + 4..offset + 4 + size];
    let mut bytes = b"NF-ACTIVE-LINEAGE-1\0".to_vec();
    bytes.extend_from_slice(&1u16.to_le_bytes());
    blob(&mut bytes, source);
    blob(&mut bytes, action);
    bytes.extend_from_slice(&revision.to_be_bytes());
    bytes.extend_from_slice(&before);
    bytes.extend_from_slice(&after);
    bytes
}
pub fn journal_head(
    previous: [u8; 32], revision: u64, kind: u32, action: &[u8],
    before: [u8; 32], after: [u8; 32], lineage: [u8; 32],
) -> [u8; 32] {
    let mut bytes = b"NF-ACTIVE-JOURNAL-1\0".to_vec();
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&previous);
    bytes.extend_from_slice(&revision.to_be_bytes());
    bytes.extend_from_slice(&kind.to_le_bytes());
    bytes.extend_from_slice(&hash(action));
    bytes.extend_from_slice(&before);
    bytes.extend_from_slice(&after);
    bytes.extend_from_slice(&lineage);
    hash(&bytes)
}

pub fn proof_bytes(proof: &nf_identity::model::DeviceProof) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(proof.scope.universe.as_bytes());
    out.extend_from_slice(proof.scope.history.as_bytes());
    out.extend_from_slice(proof.account.as_bytes());
    out.extend_from_slice(proof.device.as_bytes());
    out.extend_from_slice(&proof.frontier.to_be_bytes());
    blob(&mut out, &proof.peer);
    out.extend_from_slice(&proof.challenge);
    out.extend_from_slice(&proof.signature);
    out
}
pub fn prepared_core(
    world: &World, first: &Intent, device: DeviceId, batch: &CommittedBatch,
    next: &Intent, frontier: &nf_kernel::Frontier, reservation: &nf_store::Reservation,
) -> Vec<u8> {
    assert!(first.request < next.request);
    let mut out = b"NF-STORE-1\0\x01\0".to_vec();
    blob(&mut out, &nf_kernel::encode_snapshot(world).unwrap());
    out.push(1); blob(&mut out, &nf_kernel::encode_frontier(frontier).unwrap());
    out.extend_from_slice(&2u32.to_le_bytes());
    for (intent, committed) in [(first, true), (next, false)] {
        blob(&mut out, &nf_kernel::encode_intent(intent).unwrap());
        out.extend_from_slice(device.as_bytes());
        if committed {
            out.push(2); out.extend_from_slice(&batch.sequence.0.to_le_bytes());
            out.extend_from_slice(&0u32.to_le_bytes());
        } else { out.push(1); }
    }
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(reservation.operation.as_bytes());
    out.extend_from_slice(reservation.market.as_bytes());
    out.extend_from_slice(&reservation.amount.to_le_bytes());
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(first.operation.as_bytes());
    out.extend_from_slice(&batch.sequence.0.to_le_bytes());
    out.extend_from_slice(&hash(&nf_kernel::encode_batch(batch).unwrap()));
    out.extend_from_slice(&0u32.to_le_bytes());
    out
}
pub fn settlement_payload(results: &[nf_kernel::JobResult], policy: nf_kernel::MissingPolicy) -> Vec<u8> {
    let mut inputs = b"NF-ACTIVE-SETTLEMENT-1\0".to_vec();
    inputs.extend_from_slice(&1u16.to_le_bytes());
    inputs.push(match policy { nf_kernel::MissingPolicy::Pause => 1, nf_kernel::MissingPolicy::Recompute => 2 });
    let mut results = results.to_vec();
    results.sort_by_key(|result| result.job);
    inputs.extend_from_slice(&u32::try_from(results.len()).unwrap().to_le_bytes());
    for result in results {
        inputs.extend_from_slice(result.job.as_bytes());
        match result.result {
            Err(reason) => { inputs.push(1); inputs.extend_from_slice(&(reason as u32 + 1).to_le_bytes()); }
            Ok(proposal) => {
                inputs.push(2);
                inputs.extend_from_slice(proposal.job.as_bytes());
                inputs.extend_from_slice(proposal.provider.as_bytes());
                inputs.extend_from_slice(&proposal.provider_version.to_le_bytes());
                inputs.extend_from_slice(&proposal.implementation_hash);
                inputs.extend_from_slice(&proposal.input_hash);
                inputs.extend_from_slice(&proposal.session.0.to_be_bytes());
                for map in [&proposal.reads, &proposal.writes] {
                    inputs.extend_from_slice(&u32::try_from(map.len()).unwrap().to_le_bytes());
                    for (id, revision) in map {
                        inputs.extend_from_slice(id.as_bytes());
                        inputs.extend_from_slice(&revision.0.to_be_bytes());
                    }
                }
                inputs.extend_from_slice(&u32::try_from(proposal.events.len()).unwrap().to_le_bytes());
                for event in proposal.events {
                    match event {
                        nf_kernel::Event::RelationChanged { relation, delta } => {
                            inputs.push(1); inputs.extend_from_slice(relation.as_bytes());
                            inputs.extend_from_slice(&delta.to_le_bytes());
                        }
                        nf_kernel::Event::MarketAdjusted { market, delta } => {
                            inputs.push(2); inputs.extend_from_slice(market.as_bytes());
                            inputs.extend_from_slice(&delta.to_le_bytes());
                        }
                        nf_kernel::Event::ProviderAdvanced { provider, draws, cooldown_until } => {
                            inputs.push(3); inputs.extend_from_slice(provider.as_bytes());
                            inputs.extend_from_slice(&draws.to_be_bytes());
                            inputs.extend_from_slice(&cooldown_until.0.to_be_bytes());
                        }
                    }
                }
            }
        }
    }
    assert!(inputs.len() <= 1_048_576);
    let mut payload = vec![1];
    blob(&mut payload, &inputs);
    payload
}
pub struct PrepareTimes{pub nonce:[u8;32],pub issued:u64,pub expires:u64,pub admitted:u64,pub cutoff:u64}
pub fn prepare_payload(frontier:&nf_kernel::Frontier,device:DeviceId,reservation:&nf_store::Reservation,
    principal:&nf_identity::model::DeviceProof,times:PrepareTimes)->Vec<u8>{
    let PrepareTimes{nonce,issued,expires,admitted,cutoff}=times;
    assert_eq!(frontier.jobs().len(), 1);
    let request = frontier.jobs()[0].intent().request;
    let mut out = Vec::new();
    blob(&mut out, &nf_kernel::encode_frontier(frontier).unwrap());
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(request.as_bytes()); out.extend_from_slice(device.as_bytes());
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(reservation.operation.as_bytes());
    out.extend_from_slice(reservation.market.as_bytes());
    out.extend_from_slice(&reservation.amount.to_be_bytes());
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(request.as_bytes()); out.extend_from_slice(&nonce);
    out.extend_from_slice(&issued.to_be_bytes()); out.extend_from_slice(&expires.to_be_bytes());
    blob(&mut out, &proof_bytes(principal));
    out.push(1); out.extend_from_slice(&admitted.to_be_bytes());
    out.extend_from_slice(&cutoff.to_be_bytes());
    out
}

pub fn request_binding(intent:&Intent,device:DeviceId)->[u8;32]{
    nf_contract::canonical::binding::RequestBinding{
        request_id:intent.request,account_id:intent.actor,device_id:device,
        universe_id:intent.universe,history_id:intent.history,operation_kind:3,
        payload_digest:nf_kernel::intent_digest(intent).unwrap(),
    }.digest()
}
