use super::*;

impl NotifySubscriber {
    /// A dirty hint requires a fresh authoritative receipt query. It is never an outcome.
    pub(crate) fn dirty(&self) -> bool {
        self.generation > self.covered
    }
    #[cfg(test)]
    pub(in crate::notification_effects) fn remaining_for_test(
        &self,
    ) -> Result<Duration, PeerError> {
        let a = self.active.as_ref().ok_or(PeerError::Replay)?;
        Duration::from_secs(u64::from(a.transcript.lifetime))
            .checked_sub(a.created.elapsed())
            .filter(|d| !d.is_zero())
            .ok_or(PeerError::Replay)
    }
    pub(in crate::notification_effects) fn active_live(&self) -> Result<(), PeerError> {
        let active = self.active.as_ref().ok_or(PeerError::Replay)?;
        if active.created.elapsed() >= Duration::from_secs(u64::from(active.transcript.lifetime)) {
            return Err(PeerError::Replay);
        }
        Ok(())
    }
    pub(in crate::notification_effects) fn not_after(&self) -> Result<Instant, PeerError> {
        self.active_live()?;
        let a = self.active.as_ref().ok_or(PeerError::Replay)?;
        a.created
            .checked_add(Duration::from_secs(u64::from(a.transcript.lifetime)))
            .ok_or(PeerError::Replay)
    }
    pub(crate) fn notice(
        &mut self,
        record: NotifyRecord,
        peer: PeerId,
        connection: ConnectionId,
        repo: &mut ReceiptRepo,
    ) -> Result<NotifyRecord, PeerError> {
        let result = self.accept_notice(record, peer, connection, repo);
        if result.is_err() {
            self.invalidate();
        }
        result
    }
    fn accept_notice(
        &mut self,
        record: NotifyRecord,
        peer: PeerId,
        connection: ConnectionId,
        repo: &mut ReceiptRepo,
    ) -> Result<NotifyRecord, PeerError> {
        self.active_live()?;
        let active = self.active.as_ref().ok_or(PeerError::Replay)?;
        let subscription = active.transcript.subscription;
        let selector = NotifySelector {
            request: active.original.request(),
            operation: active.original.operation(),
            binding: active.original.binding_digest(),
        };
        let context_digest = active.transcript.context_digest;
        let expected = active.next_sequence;
        let next = expected.checked_add(1).ok_or(PeerError::Limit)?;
        let generation = self.generation.checked_add(1).ok_or(PeerError::Limit)?;
        let inbound = self.session.flow.receive(&record)?;
        let transcript = self.session.with_repo(repo, |cut| {
            self.active_live()?;
            self.session
                .record(&record, peer, connection, cut.membership)?;
            let NotifyBody::Notice {
                subscription: got,
                sequence,
                selector: selected,
                nonce,
                proof,
            } = &record.body
            else {
                return Err(PeerError::Malformed);
            };
            if *got != subscription || *sequence != expected || *selected != selector {
                return Err(PeerError::Replay);
            }
            let transcript = crate::notification::NotifyNoticeTranscript {
                context_digest,
                subscription,
                sequence: *sequence,
                selector,
                nonce: *nonce,
                frontier: self.session.revision,
            };
            let prefix = crate::notification::signed_prefix_digest(
                &record,
                PROTOCOL,
                self.session.limits(),
            )?;
            self.session
                .verify(proof, transcript.challenge(1, prefix)?, cut.membership)?;
            Ok(transcript)
        })?;
        let notice_digest =
            crate::notification::signed_prefix_digest(&record, PROTOCOL, self.session.limits())?;
        // Reserve the complete bounded Ack before requesting its signature.
        let permit = self.session.flow.reserve_outbound(482)?;
        let mut ack = NotifyRecord {
            context: self.session.context(),
            body: NotifyBody::NoticeAck {
                subscription,
                sequence: expected,
                notice_digest,
                proof: nf_identity::model::DeviceProof {
                    scope: self.session.context().scope,
                    account: self.session.local.account,
                    device: self.session.local.device,
                    frontier: self.session.revision,
                    peer: self.session.local.peer.clone(),
                    challenge: [0; 32],
                    signature: [0; 64],
                },
            },
        };
        let prefix =
            crate::notification::signed_prefix_digest(&ack, PROTOCOL, self.session.limits())?;
        let challenge = transcript.challenge(2, prefix)?;
        let proof = self.session.with_repo(repo, |cut| {
            self.active_live()?;
            self.session.sign(challenge, cut.membership, cut.device_key)
        })?;
        if let NotifyBody::NoticeAck { proof: slot, .. } = &mut ack.body {
            *slot = proof;
        }
        self.active_live()?;
        self.session.flow.stage(&ack, permit)?;
        self.session.flow.complete_inbound(inbound)?;
        self.active.as_mut().ok_or(PeerError::Replay)?.next_sequence = next;
        self.generation = generation;
        Ok(ack)
    }
}
