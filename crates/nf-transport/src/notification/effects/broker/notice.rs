use super::*;
impl NotifyBroker {
    pub(crate) fn next_notice(
        &mut self,
        peer: PeerId,
        connection: ConnectionId,
        repo: &mut ReceiptRepo,
    ) -> Result<Option<NotifyRecord>, PeerError> {
        let result = self.notice(peer, connection, repo);
        if result.is_err() {
            self.invalidate();
        }
        result
    }
    fn notice(
        &mut self,
        peer: PeerId,
        connection: ConnectionId,
        repo: &mut ReceiptRepo,
    ) -> Result<Option<NotifyRecord>, PeerError> {
        self.live()?;
        self.session.remote_from_repo(peer, connection, repo)?;
        let active = self.active.as_mut().ok_or(PeerError::Replay)?;
        if active
            .awaited
            .as_ref()
            .is_some_and(|n| n.created.elapsed() >= Duration::from_secs(5))
        {
            return Err(PeerError::Replay);
        }
        let observed = repo.observe_selector(&active.selector, self.session.revision)?;
        active_live(active)?;
        if observed.phase != active.baseline {
            active.dirty = true;
            active.baseline = observed.phase;
        }
        if !active.flushed || !active.dirty || active.awaited.is_some() {
            return Ok(None);
        }
        let permit = match self.session.flow.reserve_outbound(545) {
            Ok(p) => p,
            Err(PeerError::Backpressure) => return Ok(None),
            Err(e) => return Err(e),
        };
        let sequence = active.next_sequence;
        let next = sequence.checked_add(1).ok_or(PeerError::Limit)?;
        let nonce = crate::session::fresh()?;
        let transcript = crate::notification::NotifyNoticeTranscript {
            context_digest: active.transcript.context_digest,
            subscription: active.transcript.subscription,
            sequence,
            selector: active.transcript.selector,
            nonce,
            frontier: self.session.revision,
        };
        let mut record = NotifyRecord {
            context: self.session.context(),
            body: NotifyBody::Notice {
                subscription: transcript.subscription,
                sequence,
                selector: transcript.selector,
                nonce,
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
            crate::notification::signed_prefix_digest(&record, PROTOCOL, self.session.limits())?;
        let challenge = transcript.challenge(1, prefix)?;
        let proof = self.session.with_repo(repo, |cut| {
            active_live(active)?;
            self.session.sign(challenge, cut.membership, cut.device_key)
        })?;
        if let NotifyBody::Notice { proof: slot, .. } = &mut record.body {
            *slot = proof;
        }
        active_live(active)?;
        self.session.flow.stage(&record, permit)?;
        active.next_sequence = next;
        active.dirty = false;
        active.awaited = Some(Awaited {
            transcript,
            prefix,
            created: Instant::now(),
        });
        Ok(Some(record))
    }
    pub(crate) fn accept_ack(
        &mut self,
        record: NotifyRecord,
        peer: PeerId,
        connection: ConnectionId,
        repo: &mut ReceiptRepo,
    ) -> Result<(), PeerError> {
        let awaited = self
            .active
            .as_mut()
            .and_then(|a| a.awaited.take())
            .ok_or(PeerError::Replay)?;
        self.completion_end = Some(
            awaited
                .created
                .checked_add(Duration::from_secs(5))
                .ok_or(PeerError::Replay)?,
        );
        let result = self.ack(record, peer, connection, repo, awaited);
        if result.is_err() {
            self.invalidate();
        }
        result
    }
    fn ack(
        &mut self,
        record: NotifyRecord,
        peer: PeerId,
        connection: ConnectionId,
        repo: &mut ReceiptRepo,
        awaited: Awaited,
    ) -> Result<(), PeerError> {
        self.live()?;
        if awaited.created.elapsed() >= Duration::from_secs(5) {
            return Err(PeerError::Replay);
        }
        let inbound = self.session.flow.receive(&record)?;
        self.session.with_repo(repo, |cut| {
            self.live()?;
            super::super::elapsed_cut(awaited.created, Duration::from_secs(5))?;
            self.session
                .record(&record, peer, connection, cut.membership)?;
            let NotifyBody::NoticeAck {
                subscription,
                sequence,
                notice_digest,
                proof,
            } = &record.body
            else {
                return Err(PeerError::Malformed);
            };
            if *subscription != awaited.transcript.subscription
                || *sequence != awaited.transcript.sequence
                || *notice_digest != awaited.prefix
            {
                return Err(PeerError::Replay);
            }
            let prefix = crate::notification::signed_prefix_digest(
                &record,
                PROTOCOL,
                self.session.limits(),
            )?;
            self.session.verify(
                proof,
                awaited.transcript.challenge(2, prefix)?,
                cut.membership,
            )
        })?;
        self.live()?;
        super::super::elapsed_cut(awaited.created, Duration::from_secs(5))?;
        self.session.flow.complete_inbound(inbound)
    }
}
