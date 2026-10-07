use super::*;

impl NotifyBroker {
    pub(super) fn live(&self) -> Result<(), PeerError> {
        if let Some(end) = self.completion_end {
            super::super::until_cut(end)?;
        }
        let (created, lifetime) = if let Some(a) = &self.active {
            active_live(a)?;
            (a.created, u64::from(a.transcript.lifetime))
        } else if let Some(p) = &self.pending {
            (p.created, 5_u64.min(u64::from(p.transcript.lifetime)))
        } else {
            return Err(PeerError::Replay);
        };
        if Instant::now()
            .checked_duration_since(created)
            .is_none_or(|d| d >= Duration::from_secs(lifetime))
        {
            return Err(PeerError::Replay);
        }
        Ok(())
    }
    pub(crate) fn prepare_output(
        &mut self,
        record: NotifyRecord,
        peer: PeerId,
        connection: ConnectionId,
        repo: &mut ReceiptRepo,
    ) -> Result<crate::notification_effects::NotifyEmission, PeerError> {
        self.live()?;
        if let Some(a) = &self.active
            && let Some(n) = &a.awaited
            && n.created.elapsed() >= Duration::from_secs(5)
        {
            self.invalidate();
            return Err(PeerError::Replay);
        }
        let result = self
            .session
            .prepare_output(record, peer, connection, repo)?;
        self.live()?;
        Ok(result)
    }
    #[cfg(test)]
    pub(crate) fn complete_emission(
        &mut self,
        emission: crate::notification_effects::NotifyEmission,
        peer: PeerId,
        connection: ConnectionId,
        repo: &mut ReceiptRepo,
    ) -> Result<(), PeerError> {
        self.complete_inner(emission, peer, connection, repo, None)
    }
    pub(in crate::notification_effects) fn complete_emission_before(
        &mut self,
        emission: crate::notification_effects::NotifyEmission,
        peer: PeerId,
        connection: ConnectionId,
        repo: &mut ReceiptRepo,
        end: Instant,
    ) -> Result<(), PeerError> {
        self.complete_inner(emission, peer, connection, repo, Some(end))
    }
    fn complete_inner(
        &mut self,
        emission: crate::notification_effects::NotifyEmission,
        peer: PeerId,
        connection: ConnectionId,
        repo: &mut ReceiptRepo,
        end: Option<Instant>,
    ) -> Result<(), PeerError> {
        self.live()?;
        self.session.remote_from_repo(peer, connection, repo)?;
        self.live()?;
        #[cfg(test)]
        crate::notification_effects::lane::completion_test::after_current_read();
        self.live()?;
        if let Some(end) = end {
            super::super::until_cut(end)?;
        }
        let kind = self.session.flow.finish_emission(emission)?;
        if kind == 8 || kind == 9 {
            self.completion_end = None;
        }
        if kind == 8 {
            self.active.as_mut().ok_or(PeerError::Replay)?.flushed = true;
        }
        Ok(())
    }
}

impl NotifyBroker {
    pub(in crate::notification_effects) fn remaining(&self) -> Result<Duration, PeerError> {
        let a = self.active.as_ref().ok_or(PeerError::Replay)?;
        Duration::from_secs(u64::from(a.transcript.lifetime))
            .checked_sub(a.created.elapsed())
            .filter(|d| !d.is_zero())
            .ok_or(PeerError::Replay)
    }
}
