use super::{Author, ChatStoreError, IssuedChallenge, ProofAttempt, Result, codec};
use nf_identity::model::{IdentityError, MembershipState, ProtectedOperation};
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};
const MAX_TICKETS: usize = 64;
const TTL: Duration = Duration::from_secs(5);
struct Ticket {
    actor: Author,
    peer: Vec<u8>,
    binding: [u8; 32],
    challenge: [u8; 32],
    revision: u64,
    expires: Instant,
}
pub(super) struct AuthRuntime {
    tickets: BTreeMap<[u8; 16], Ticket>,
}
impl AuthRuntime {
    pub fn new() -> Self {
        Self {
            tickets: BTreeMap::new(),
        }
    }
    pub fn issue(
        &mut self,
        current: &MembershipState,
        actor: Author,
        peer: &[u8],
        binding: [u8; 32],
    ) -> Result<IssuedChallenge> {
        nf_identity::codec::validate_state(current)?;
        if peer.is_empty() || peer.len() > 128 {
            return Err(ChatStoreError::Limit);
        }
        let device = current
            .devices
            .get(&actor.device)
            .ok_or(IdentityError::UnknownDevice)?;
        if device.revoked {
            return Err(IdentityError::Revoked.into());
        }
        if device.account != actor.account || device.peer != peer {
            return Err(IdentityError::Signature.into());
        }
        let account = current
            .accounts
            .get(&actor.account)
            .ok_or(IdentityError::UnknownAccount)?;
        if !account.roles.contains(nf_identity::model::Roles::PLAYER) {
            return Err(IdentityError::RolePolicy.into());
        }
        let now = Instant::now();
        self.tickets.retain(|_, value| value.expires > now);
        if self.tickets.len() >= MAX_TICKETS {
            return Err(ChatStoreError::Limit);
        }
        let mut ticket = [0; 16];
        getrandom::fill(&mut ticket).map_err(|_| IdentityError::Entropy)?;
        let mut nonce = [0; 32];
        getrandom::fill(&mut nonce).map_err(|_| IdentityError::Entropy)?;
        if ticket == [0; 16] || nonce == [0; 32] || self.tickets.contains_key(&ticket) {
            return Err(ChatStoreError::Conflict);
        }
        let mut frame = b"NF-CHAT-ADMISSION-1\0".to_vec();
        frame.extend_from_slice(&binding);
        frame.extend_from_slice(&ticket);
        frame.extend_from_slice(&nonce);
        frame.extend_from_slice(actor.account.as_bytes());
        frame.extend_from_slice(actor.device.as_bytes());
        frame.extend_from_slice(&current.revision.to_be_bytes());
        frame.extend_from_slice(&(peer.len() as u16).to_be_bytes());
        frame.extend_from_slice(peer);
        let challenge = codec::hash(&frame);
        self.tickets.insert(
            ticket,
            Ticket {
                actor,
                peer: peer.to_vec(),
                binding,
                challenge,
                revision: current.revision,
                expires: now + TTL,
            },
        );
        Ok(IssuedChallenge {
            ticket,
            challenge,
            membership_revision: current.revision,
        })
    }
    pub fn consume(
        &mut self,
        current: &MembershipState,
        actor: Author,
        binding: Result<[u8; 32]>,
        attempt: ProofAttempt<'_>,
    ) -> Result<()> {
        // Consume once, even when the attempt is malformed, stale or unauthorized.
        let ticket = self
            .tickets
            .remove(&attempt.ticket)
            .ok_or(ChatStoreError::Replay)?;
        if Instant::now() >= ticket.expires {
            return Err(ChatStoreError::Expired);
        }
        let binding = binding?;
        if ticket.binding != binding
            || ticket.actor != actor
            || ticket.peer.as_slice() != attempt.peer
            || attempt.proof.account != actor.account
            || attempt.proof.device != actor.device
        {
            return Err(ChatStoreError::Signature);
        }
        if ticket.revision != current.revision {
            return Err(IdentityError::Frontier.into());
        }
        current.authorize(
            attempt.proof,
            attempt.peer,
            &ticket.challenge,
            ticket.revision,
            ProtectedOperation::Chat,
        )?;
        Ok(())
    }
}
