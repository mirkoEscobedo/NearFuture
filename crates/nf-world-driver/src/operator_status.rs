//! Redacted supplied-data projection for the miniature profile2.
//! Public snapshots can be constructed by callers. This API does not authenticate
//! admission, freshness, active authority, remote heads or recovery readiness.
use nf_store::miniature::MiniatureSnapshot;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StatusProjectionError {
    PendingCountLimit,
    OutputLimit,
}

/// Closed JSON data only. This value grants no I/O, restore or admin capability.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RedactedMiniatureStatusV1(String);
impl RedactedMiniatureStatusV1 {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

struct Counters {
    world_tick: u64,
    event_sequence: u64,
    store_revision: u64,
    membership_revision: Option<u64>,
    persisted_authority_term: Option<u64>,
    minimum_known_authority_term: u64,
    provider_revision: u64,
    pending_frontier: bool,
    pending_intents_count: usize,
}

/// Projects only supplied snapshot counters. No owner is opened or consulted.
pub fn project_miniature_status(
    snapshot: &MiniatureSnapshot,
) -> Result<RedactedMiniatureStatusV1, StatusProjectionError> {
    let metadata = snapshot.world.metadata();
    render(Counters {
        world_tick: metadata.tick.0,
        event_sequence: metadata.event_sequence.0,
        store_revision: snapshot.known.storage.store_revision,
        membership_revision: snapshot.known.storage.membership_revision,
        persisted_authority_term: snapshot.authority.map(|authority| authority.term.0),
        minimum_known_authority_term: snapshot.known.minimum_authority_term.0,
        provider_revision: metadata.provider_revision.0,
        pending_frontier: snapshot.pending.is_some(),
        pending_intents_count: snapshot
            .pending
            .as_ref()
            .map_or(0, |frontier| frontier.intents().len()),
    })
}

fn optional_decimal(value: Option<u64>) -> String {
    value.map_or_else(|| "null".to_owned(), |number| format!("\"{number}\""))
}

fn render(counters: Counters) -> Result<RedactedMiniatureStatusV1, StatusProjectionError> {
    if counters.pending_intents_count > 64 {
        return Err(StatusProjectionError::PendingCountLimit);
    }
    let json = format!(
        concat!(
            "{{\"format\":\"NF-OPERATOR-MINIATURE-STATUS-1\",",
            "\"store_profile\":\"profile2\",",
            "\"world_tick\":\"{}\",\"event_sequence\":\"{}\",\"store_revision\":\"{}\",",
            "\"membership_revision\":{},\"persisted_authority_term\":{},",
            "\"minimum_known_authority_term\":\"{}\",\"provider_revision\":\"{}\",",
            "\"pending_frontier\":{},\"pending_intents_count\":\"{}\",",
            "\"provider_ownership\":\"unavailable\",\"lag\":\"unavailable\",",
            "\"taint\":\"unavailable\",\"pending_native_bridge\":\"unavailable\",",
            "\"transfer_receipts\":\"unavailable_profile2\",\"backup_recovery\":\"unavailable\"}}"
        ),
        counters.world_tick,
        counters.event_sequence,
        counters.store_revision,
        optional_decimal(counters.membership_revision),
        optional_decimal(counters.persisted_authority_term),
        counters.minimum_known_authority_term,
        counters.provider_revision,
        counters.pending_frontier,
        counters.pending_intents_count,
    );
    if json.len() > 4096 {
        return Err(StatusProjectionError::OutputLimit);
    }
    Ok(RedactedMiniatureStatusV1(json))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nf_contract::identity::*;
    use nf_kernel::miniature::{MiniatureMetadata, MiniatureWorld, admit_miniature};
    use nf_store::KnownFrontiers;
    use nf_store::miniature::{MiniatureAuthority, MiniatureKnownFrontiers};
    use nf_world::Genesis;

    fn snapshot(marker: u8) -> MiniatureSnapshot {
        let universe = UniverseId::from_bytes([marker; 16]);
        let history = HistoryId::from_bytes([marker.wrapping_add(1); 16]);
        let component = nf_world::generate(Genesis {
            universe,
            history,
            seed: [marker; 32],
            accounts: [
                AccountId::from_bytes([marker; 16]),
                AccountId::from_bytes([marker.wrapping_add(1); 16]),
                AccountId::from_bytes([marker.wrapping_add(2); 16]),
            ],
        })
        .unwrap();
        MiniatureSnapshot {
            world: MiniatureWorld::new(
                MiniatureMetadata {
                    tick: WorldTick(0),
                    event_sequence: EventSeq(7),
                    aggregate: AggregateId::from_bytes([20; 16]),
                    provider: ProviderId::from_bytes([21; 16]),
                    provider_aggregate: AggregateId::from_bytes([22; 16]),
                    provider_revision: AggregateRevision(0),
                },
                component,
            )
            .unwrap(),
            authority: Some(MiniatureAuthority {
                term: AuthorityTerm(5),
                session: RuntimeSession(31),
                account: AccountId::from_bytes([marker; 16]),
                device: DeviceId::from_bytes([marker; 16]),
                membership_revision: 99,
            }),
            pending: None,
            known: MiniatureKnownFrontiers {
                storage: KnownFrontiers {
                    scope: nf_identity::model::Scope { universe, history },
                    event_sequence: EventSeq(66),
                    store_revision: 9,
                    membership_revision: Some(4),
                },
                minimum_authority_term: AuthorityTerm(3),
            },
        }
    }

    #[test]
    fn supplied_snapshot_has_exact_literal_allowlist_and_distinct_counter_sources() {
        let output = project_miniature_status(&snapshot(1)).unwrap();
        assert_eq!(
            output.as_str(),
            concat!(
                "{\"format\":\"NF-OPERATOR-MINIATURE-STATUS-1\",\"store_profile\":\"profile2\",",
                "\"world_tick\":\"0\",\"event_sequence\":\"7\",\"store_revision\":\"9\",",
                "\"membership_revision\":\"4\",\"persisted_authority_term\":\"5\",",
                "\"minimum_known_authority_term\":\"3\",\"provider_revision\":\"0\",",
                "\"pending_frontier\":false,\"pending_intents_count\":\"0\",",
                "\"provider_ownership\":\"unavailable\",\"lag\":\"unavailable\",",
                "\"taint\":\"unavailable\",\"pending_native_bridge\":\"unavailable\",",
                "\"transfer_receipts\":\"unavailable_profile2\",\"backup_recovery\":\"unavailable\"}"
            )
        );
        let keys: Vec<_> = output
            .as_str()
            .trim_matches(['{', '}'])
            .split(',')
            .map(|field| field.split_once(':').unwrap().0)
            .collect();
        assert_eq!(
            keys,
            [
                "\"format\"",
                "\"store_profile\"",
                "\"world_tick\"",
                "\"event_sequence\"",
                "\"store_revision\"",
                "\"membership_revision\"",
                "\"persisted_authority_term\"",
                "\"minimum_known_authority_term\"",
                "\"provider_revision\"",
                "\"pending_frontier\"",
                "\"pending_intents_count\"",
                "\"provider_ownership\"",
                "\"lag\"",
                "\"taint\"",
                "\"pending_native_bridge\"",
                "\"transfer_receipts\"",
                "\"backup_recovery\"",
            ]
        );
    }

    #[test]
    fn supplied_snapshot_redacts_identity_component_scope_and_authority_session() {
        let first = snapshot(1);
        let mut second = snapshot(91);
        second.authority.as_mut().unwrap().session = RuntimeSession(u64::MAX);
        second.authority.as_mut().unwrap().membership_revision = 888;
        second.known.storage.event_sequence = EventSeq(123);
        assert_ne!(first.world.component(), second.world.component());
        assert_ne!(first.authority, second.authority);
        assert_ne!(first.known.storage.scope, second.known.storage.scope);
        assert_eq!(
            project_miniature_status(&first),
            project_miniature_status(&second)
        );
    }

    #[test]
    fn missing_membership_and_persisted_authority_remain_null_not_zero() {
        let mut supplied = snapshot(1);
        supplied.authority = None;
        supplied.known.storage.membership_revision = None;
        let output = project_miniature_status(&supplied).unwrap();
        assert!(
            output
                .as_str()
                .contains("\"membership_revision\":null,\"persisted_authority_term\":null")
        );
        supplied.known.storage.membership_revision = Some(0);
        supplied.authority = snapshot(1).authority;
        supplied.authority.as_mut().unwrap().term = AuthorityTerm(0);
        let zero = project_miniature_status(&supplied).unwrap();
        assert!(
            zero.as_str()
                .contains("\"membership_revision\":\"0\",\"persisted_authority_term\":\"0\"")
        );
        assert_ne!(output, zero);
    }

    #[test]
    fn decimal_serializer_preserves_every_u64_max_and_fixed_unavailable_fields() {
        // Private renderer boundary test; not a claim that a world at this tick is admitted.
        let output = render(Counters {
            world_tick: u64::MAX,
            event_sequence: u64::MAX,
            store_revision: u64::MAX,
            membership_revision: Some(u64::MAX),
            persisted_authority_term: Some(u64::MAX),
            minimum_known_authority_term: u64::MAX,
            provider_revision: u64::MAX,
            pending_frontier: false,
            pending_intents_count: 0,
        })
        .unwrap();
        assert_eq!(
            output.as_str(),
            concat!(
                "{\"format\":\"NF-OPERATOR-MINIATURE-STATUS-1\",\"store_profile\":\"profile2\",",
                "\"world_tick\":\"18446744073709551615\",\"event_sequence\":\"18446744073709551615\",",
                "\"store_revision\":\"18446744073709551615\",\"membership_revision\":\"18446744073709551615\",",
                "\"persisted_authority_term\":\"18446744073709551615\",",
                "\"minimum_known_authority_term\":\"18446744073709551615\",\"provider_revision\":\"18446744073709551615\",",
                "\"pending_frontier\":false,\"pending_intents_count\":\"0\",",
                "\"provider_ownership\":\"unavailable\",\"lag\":\"unavailable\",",
                "\"taint\":\"unavailable\",\"pending_native_bridge\":\"unavailable\",",
                "\"transfer_receipts\":\"unavailable_profile2\",\"backup_recovery\":\"unavailable\"}"
            )
        );
        assert!(output.as_str().len() <= 4096);
        assert!(!output.as_str().contains('\n'));
    }

    #[test]
    fn empty_pending_frontier_is_present_without_becoming_native_pending() {
        let mut supplied = snapshot(1);
        supplied.pending = Some(
            admit_miniature(
                &supplied.world,
                Vec::new(),
                nf_kernel::AuthorityContext {
                    term: AuthorityTerm(5),
                    session: RuntimeSession(31),
                },
                4,
            )
            .unwrap(),
        );
        let output = project_miniature_status(&supplied).unwrap();
        assert!(
            output
                .as_str()
                .contains("\"pending_frontier\":true,\"pending_intents_count\":\"0\"")
        );
        assert!(
            output
                .as_str()
                .contains("\"pending_native_bridge\":\"unavailable\"")
        );
    }

    #[test]
    fn private_count_boundary_rejects_65_without_partial_json() {
        let values = |count| Counters {
            world_tick: 0,
            event_sequence: 0,
            store_revision: 0,
            membership_revision: None,
            persisted_authority_term: None,
            minimum_known_authority_term: 0,
            provider_revision: 0,
            pending_frontier: true,
            pending_intents_count: count,
        };
        assert!(
            render(values(64))
                .unwrap()
                .as_str()
                .contains("\"pending_intents_count\":\"64\"")
        );
        assert_eq!(
            render(values(65)),
            Err(StatusProjectionError::PendingCountLimit)
        );
    }
}
