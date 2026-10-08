use super::{Call, Key};
use nf_contract::identity::{OperationId, RequestId};
use nf_kernel::supplies::{RejectedOutcome, RequestOutcome, RequestRejection, SuppliesRejection};
use nf_store::{StoreError, supplies::SuppliesStoreError as Error};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub struct Reference {
    pub keys: BTreeSet<Key>,
    pub requests: BTreeMap<RequestId, (Call, RequestOutcome)>,
    operations: BTreeMap<OperationId, (Call, RequestOutcome)>,
    decisions: Vec<(Call, RequestOutcome)>,
}
fn command_result(outcome: RequestOutcome) -> Result<RequestOutcome, Error> {
    match outcome {
        RequestOutcome::Rejected(_) => Err(Error::Rejected(SuppliesRejection::Limit)),
        _ => Ok(outcome),
    }
}
impl Reference {
    pub fn revision(&self) -> u64 {
        // A novel admitted terminal decision advances the journal, even when no asset moves.
        self.decisions.len().try_into().unwrap()
    }
    pub fn originals(&self) -> Vec<Call> {
        self.decisions.iter().map(|&(call, _)| call).collect()
    }
    pub fn totals(&self, key: Key) -> [u128; 6] {
        // Sum unique accepted asset effects from scratch; refusals contribute zero to every bucket.
        let (mut issued, mut reserved, mut burned) = (0_u128, 0_u128, 0_u128);
        for &(event, outcome) in &self.decisions {
            if event.key() != key || matches!(outcome, RequestOutcome::Rejected(_)) {
                continue;
            }
            match event {
                Call::Issue(v) => issued += u128::from(v.amount),
                Call::Reserve(v) => reserved += u128::from(v.amount),
                Call::Burn(v) => burned += u128::from(v.amount),
            }
        }
        assert!(
            issued >= reserved + burned,
            "reference accepted sequence must be funded"
        );
        let available = issued - reserved - burned;
        assert_eq!(issued, available + reserved + burned);
        let fields = [available, reserved, 0, 0, issued, burned];
        assert!(fields.into_iter().all(|n| n <= u128::from(u64::MAX)));
        fields
    }
    pub fn predict(&self, call: Call) -> Result<RequestOutcome, Error> {
        // Typed global identities, including retained refusals, precede new-effect affordability.
        if let Some(&(old, outcome)) = self.requests.get(&call.request()) {
            return if old == call {
                command_result(outcome)
            } else {
                Err(Error::Rejected(SuppliesRejection::Conflict))
            };
        }
        if let Some(&(old, outcome)) = self.operations.get(&call.operation()) {
            return if old.economic() == call.economic() {
                command_result(outcome)
            } else {
                Err(Error::Rejected(SuppliesRejection::Conflict))
            };
        }
        if call.amount() == 0 {
            return Err(Error::Rejected(SuppliesRejection::Limit));
        }
        let fields = self.totals(call.key());
        let amount = u128::from(call.amount());
        match call {
            Call::Issue(_)
                if fields[0] + amount > u128::from(u64::MAX)
                    || fields[4] + amount > u128::from(u64::MAX) =>
            {
                return Err(Error::Storage(StoreError::Limit));
            }
            Call::Reserve(_) | Call::Burn(_) if amount > fields[0] => {
                return Err(Error::Rejected(SuppliesRejection::Limit));
            }
            _ => {}
        }
        Ok(call.outcome(self.revision() + 1))
    }
    pub fn accept_prediction(&mut self, call: Call, prediction: Result<RequestOutcome, Error>) {
        self.keys.insert(call.key());
        let outcome = match prediction {
            Ok(outcome) => outcome,
            Err(Error::Rejected(SuppliesRejection::Limit))
                if call.amount() > 0 && matches!(call, Call::Reserve(_) | Call::Burn(_)) =>
            {
                if let Some(&(old, outcome)) = self.operations.get(&call.operation()) {
                    assert_eq!(
                        old.economic(),
                        call.economic(),
                        "refusal alias must preserve all typed terms"
                    );
                    outcome
                } else {
                    RequestOutcome::Rejected(RejectedOutcome {
                        operation: call.operation(),
                        revision: self.revision() + 1,
                        reason: RequestRejection::InsufficientAvailable,
                    })
                }
            }
            _ => return,
        };
        if let std::collections::btree_map::Entry::Vacant(entry) =
            self.operations.entry(call.operation())
        {
            entry.insert((call, outcome));
            self.decisions.push((call, outcome));
        }
        self.requests
            .entry(call.request())
            .or_insert((call, outcome));
    }
}
