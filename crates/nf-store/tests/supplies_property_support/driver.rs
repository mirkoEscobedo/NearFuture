use super::{Call, Key, own_device, reference::Reference};
use crate::supplies_support::{Fixture, Scratch};
use nf_kernel::supplies::{BalanceQuery, RequestOutcome, StatusQuery};
use nf_store::supplies::{ChallengeRequest, SuppliesStore, SuppliesStoreError as Error};
use std::path::PathBuf;

pub struct Driver<'a> {
    fixture: &'a Fixture,
    path: PathBuf,
    store: Option<SuppliesStore>,
    reference: Reference,
}
impl<'a> Driver<'a> {
    pub fn new(scratch: &Scratch, fixture: &'a Fixture) -> Self {
        let store =
            SuppliesStore::create(scratch.db(), &fixture.policy, &fixture.membership).unwrap();
        Self {
            fixture,
            path: scratch.db(),
            store: Some(store),
            reference: Reference::default(),
        }
    }
    pub fn watch(&mut self, key: Key) {
        self.reference.keys.insert(key);
    }
    pub fn originals(&self) -> Vec<Call> {
        self.reference.originals()
    }
    pub fn totals(&self, key: Key) -> [u128; 6] {
        self.reference.totals(key)
    }
    pub fn revision(&self) -> u64 {
        self.reference.revision()
    }
    pub fn apply(&mut self, call: Call) {
        let prediction = self.reference.predict(call);
        let store = self.store.as_mut().unwrap();
        let f = self.fixture;
        let actual: Result<Option<RequestOutcome>, Error> = match call {
            Call::Issue(v) => {
                let proof = f.attempt(store, ChallengeRequest::Issue(&v));
                store
                    .issue(&v, proof)
                    .map(|outcome| outcome.map(RequestOutcome::Issued))
            }
            Call::Reserve(v) => {
                let proof = f.attempt(store, ChallengeRequest::Reserve(&v));
                store
                    .reserve(&v, proof)
                    .map(|outcome| outcome.map(RequestOutcome::Reserved))
            }
            Call::Burn(v) => {
                let proof = f.attempt(store, ChallengeRequest::Burn(&v));
                store
                    .burn(&v, proof)
                    .map(|outcome| outcome.map(RequestOutcome::Burned))
            }
        };
        self.reference.accept_prediction(call, prediction);
        // An absent debit/credit is diagnosed by the independent numeric balance oracle first.
        self.assert_state();
        assert_eq!(
            actual,
            prediction.map(Some),
            "real signed operation agrees with independent semantic prediction: {call:?}"
        );
        let expected = self
            .reference
            .requests
            .get(&call.request())
            .map(|(_, outcome)| *outcome);
        self.assert_status(call, expected);
    }
    pub fn assert_state(&mut self) {
        let f = self.fixture;
        let store = self.store.as_mut().unwrap();
        for &key in &self.reference.keys {
            let query = BalanceQuery {
                actor: key.0,
                device: own_device(f, key.0),
                owner: key.0,
                universe: f.policy.universe,
                history: f.policy.history,
                content: key.1,
                origin: key.2,
            };
            let proof = f.attempt(store, ChallengeRequest::Balance(&query));
            let actual = store
                .balance(&query, proof)
                .expect("real signed own-stock balance");
            let fields = [
                actual.available,
                actual.reserved,
                actual.pending,
                actual.externalized,
                actual.minted,
                actual.burned,
            ]
            .map(u128::from);
            assert_eq!(
                fields,
                self.reference.totals(key),
                "unique-event u128 conservation for {key:?}"
            );
            assert_eq!(
                fields[4],
                fields[0] + fields[1] + fields[2] + fields[3] + fields[5]
            );
        }
        let retained: Vec<_> = self.reference.requests.values().copied().collect();
        for (call, outcome) in retained {
            self.assert_status(call, Some(outcome));
        }
        let known = self.store.as_ref().unwrap().known_frontiers().unwrap();
        assert_eq!(
            known.revision,
            self.reference.revision(),
            "only novel accepted or insufficient-stock decisions advance journal revision"
        );
        assert_eq!(known.membership_revision, f.membership.revision);
        assert_eq!(
            (known.scope.universe, known.scope.history),
            (f.policy.universe, f.policy.history)
        );
    }
    fn assert_status(&mut self, call: Call, expected: Option<RequestOutcome>) {
        let f = self.fixture;
        let owner = call.key().0;
        let query = StatusQuery {
            actor: owner,
            device: own_device(f, owner),
            owner,
            universe: f.policy.universe,
            history: f.policy.history,
            request: call.request(),
        };
        let store = self.store.as_mut().unwrap();
        let proof = f.attempt(store, ChallengeRequest::Status(&query));
        assert_eq!(
            store.status(&query, proof).unwrap(),
            expected,
            "original outcome for request {:?}",
            call.request()
        );
    }
    pub fn recover(&mut self, checkpoint: bool) {
        self.assert_state();
        let mut store = self.store.take().unwrap();
        let known = store.known_frontiers().unwrap();
        if checkpoint {
            store.compact().unwrap();
            assert_eq!(store.known_frontiers().unwrap(), known);
        }
        drop(store);
        self.store =
            Some(SuppliesStore::open_existing(&self.path, &self.fixture.policy, known).unwrap());
        self.assert_state();
    }
}
