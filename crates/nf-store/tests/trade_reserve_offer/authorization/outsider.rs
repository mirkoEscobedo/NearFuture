use crate::{Error, Scratch, TradeChallenge, TradeFixture, TradeRejection, TradeStore};
use nf_identity::{
    keys::{SecretSeed, generate_identity},
    model::{Invitation, MembershipState, Roles},
    signing::{admission_proof, device_digest, sign_invitation},
};
use nf_kernel::{
    supplies::{StatusQuery, SuppliesRejection},
    trade::OfferStatusQuery,
};
use nf_store::supplies::{ProofAttempt, SuppliesStoreError};
fn signed(store: &mut TradeStore, request: TradeChallenge<'_>, key: &SecretSeed) -> ProofAttempt {
    let issued = store.issue_challenge(request).unwrap();
    let mut proof = issued.template;
    proof.signature = key.sign(&device_digest(&proof).unwrap());
    ProofAttempt {
        ticket: issued.ticket,
        proof,
    }
}
#[test]
fn independently_signed_member_cannot_read_another_parties_refusal() {
    let scratch = Scratch::new();
    let f = TradeFixture::new();
    let (owner, owner_account, owner_key) = generate_identity(b"trade-owner".to_vec()).unwrap();
    let (maker, maker_account, maker_key) = generate_identity(b"trade-maker".to_vec()).unwrap();
    let (outsider, outsider_account, outsider_key) =
        generate_identity(b"trade-outsider".to_vec()).unwrap();
    let scope = f.supplies.membership.scope;
    let mut member = MembershipState::bootstrap(scope, &owner).unwrap();
    for (index, (recipient, account, key)) in [
        (&maker, &maker_account, &maker_key),
        (&outsider, &outsider_account, &outsider_key),
    ]
    .into_iter()
    .enumerate()
    {
        let invitation = Invitation {
            scope,
            id: [1 + index as u8; 16],
            issuer: owner.account,
            recipient: recipient.clone(),
            roles: Roles::PLAYER,
            expires_at: 100,
            issued_revision: member.revision,
            reusable: false,
        };
        let proof = admission_proof(&invitation, account, key).unwrap();
        member = member
            .redeem(
                &sign_invitation(invitation, &owner_account).unwrap(),
                &proof,
                1,
            )
            .unwrap();
    }
    let mut policy = f.policy.clone();
    policy.clock_authority = owner.account;
    policy.supplies.issuers[0].issuer = owner.account;
    policy.supplies.burners[0].issuer = owner.account;
    let mut store = TradeStore::create(scratch.db(), &policy, &member).unwrap();
    let mut value = f.reserve_offer();
    value.terms.maker = maker.account;
    value.terms.taker = owner.account;
    value.maker_device = maker.device;
    value.taker_device = owner.device;
    value.terms.policy = policy.digest().unwrap();
    let maker_proof = signed(&mut store, TradeChallenge::ReserveMaker(&value), &maker_key);
    let taker_proof = signed(&mut store, TradeChallenge::ReserveTaker(&value), &owner_key);
    assert_eq!(
        store.reserve_offer(&value, maker_proof, taker_proof),
        Err(Error::Rejected(TradeRejection::InsufficientAvailable))
    );
    let before = store.known_frontiers().unwrap();
    assert_eq!(before.revision, 1);
    let query = StatusQuery {
        actor: outsider.account,
        owner: outsider.account,
        device: outsider.device,
        universe: scope.universe,
        history: scope.history,
        request: value.request,
    };
    let proof = signed(&mut store, TradeChallenge::Status(&query), &outsider_key);
    let unauthorized = Error::Supplies(SuppliesStoreError::Rejected(
        SuppliesRejection::Unauthorized,
    ));
    assert_eq!(store.status(&query, proof), Err(unauthorized));
    let query = OfferStatusQuery {
        actor: outsider.account,
        device: outsider.device,
        universe: scope.universe,
        history: scope.history,
        offer: value.terms.offer,
        version: value.terms.version,
    };
    let proof = signed(
        &mut store,
        TradeChallenge::OfferStatus(&query),
        &outsider_key,
    );
    assert_eq!(store.offer_status(&query, proof), Err(unauthorized));
    assert_eq!(store.known_frontiers().unwrap(), before);
}
