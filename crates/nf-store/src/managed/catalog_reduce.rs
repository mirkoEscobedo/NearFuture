use super::{bytes::{hash, next}, catalog_action::Action, catalog_payload::Payload,
    catalog_rules as rules, catalog_state::CatalogState, challenge::Owned,
    delta::RegistryPurpose, error::{ManagedError, Result}, maps::{Maps, ReceiptRow},
    prospective::Prospective, stamp::CatalogFrontier, summary::Summary};

#[derive(Clone, Default)]
pub(super) struct ReplayGuard { pub open: Option<(Summary, u64)> }
impl ReplayGuard {
    pub fn advance(&mut self, before: Option<&CatalogState>, frontier: CatalogFrontier,
        after: &CatalogState, old_maps: &Maps, action: &Action) -> Result<Maps> {
        action.encode()?; after.encode()?;
        if let Payload::Owner { adopt: true, transcript, stamp, foot, .. } = &action.payload {
            if before.is_some() || frontier.revision != 0 || frontier.state_digest != [0; 32]
                || frontier.head != [0; 32] || self.open.is_some()
            { return Err(ManagedError::Corrupt); }
            let auth = Prospective::decode(transcript)?;
            let maps = action.delta.apply(old_maps, RegistryPurpose::Adopt, None)?;
            let (registry, receipts) = maps.digests()?;
            let expected = CatalogState { installation: auth.installation, scope: auth.scope, phase: 2,
                generation: 1, term: 1, session: 1, owner: Some(rules::identity(stamp)), consumed: 0,
                selected: Some(maps.registry.get(&1).ok_or(ManagedError::Corrupt)?.artifact),
                source: maps.registry.get(&1).ok_or(ManagedError::Corrupt)?.artifact,
                current: Some(foot.clone()), original: foot.clone(), target: None, retirement: None,
                pending: None, source_policy: None, utc: after.utc, registry, receipts };
            if auth.purpose != 9 || auth.catalog.is_some() || stamp.installation != expected.installation
                || (stamp.generation, stamp.term, stamp.session, stamp.profile) != (1, 1, 1, 1)
                || stamp.incarnation == [0; 16] || foot.profile != 1 || foot.policy.is_some()
                || stamp.schema != foot.schema || stamp.membership_revision != foot.known.membership_revision
                || Some(stamp.membership_digest) != foot.member || foot.known.scope != auth.scope
                || after.utc < auth.times.issued || after.utc >= auth.times.expires || after != &expected
            { return Err(ManagedError::Corrupt); }
            rules::registry(&maps, after)?; return Ok(maps);
        }
        let before = before.ok_or(ManagedError::Corrupt)?;
        if frontier.state_digest != before.digest()? || after.utc < before.utc { return Err(ManagedError::Corrupt); }
        let mut expected = before.clone(); expected.utc = after.utc;
        let mut purpose = RegistryPurpose::None; let mut expected_receipt = None;
        match &action.payload {
            Payload::Owner { adopt: false, transcript, stamp, foot, .. } => {
                if before.pending.is_some() || self.open.is_some() || before.current.as_ref() != Some(foot) {
                    return Err(ManagedError::OperationConflict);
                }
                let auth = Prospective::decode(transcript)?;
                rules::prospective(before, after, frontier, &auth)?;
                rules::claim_stamp(before, stamp, foot)?;
                expected.phase = match before.phase { 1 | 2 if foot.profile == 1 => 2,
                    3 if foot.profile == 1 => 3, 6 | 7 if foot.profile == 4 => 7,
                    _ => return Err(ManagedError::OwnerMismatch) };
                expected.term = stamp.term; expected.session = stamp.session;
                expected.owner = Some(rules::identity(stamp));
            },
            Payload::Pending(summary) => {
                if before.pending.is_some() || self.open.is_some() || summary.sequence != next(before.consumed)?
                    || summary.stage != 1 || summary.pre_pending.is_some()
                    || (summary.purpose == 8 && before.phase != 3)
                    || (summary.purpose != 8 && !matches!(before.phase, 2 | 7))
                { return Err(ManagedError::OperationConflict); }
                rules::pending(summary, before)?;
                expected.pending = summary.post_pending; expected.consumed = summary.sequence;
                if summary.purpose == 8 { expected.phase = 4; }
                self.open = Some((summary.clone(), after.utc));
            },
            Payload::Attempt(summary) => {
                let (prior, utc) = self.open.as_ref().ok_or(ManagedError::Corrupt)?;
                let mut compare = prior.clone(); compare.stage = 2;
                compare.pre_pending = prior.post_pending; compare.post_pending = summary.post_pending;
                if before.phase != 4 || prior.stage != 1 || prior.purpose != 8 || before.pending != prior.post_pending
                    || compare.encode()? != summary.encode()? || after.utc != *utc
                { return Err(ManagedError::Corrupt); }
                expected.pending = summary.post_pending; self.open = Some((summary.clone(), *utc));
            },
            Payload::Outcome { summary, receipt, disposition } => {
                self.match_outcome(summary, after.utc)?;
                if before.pending != summary.post_pending || receipt.revision != next(frontier.revision)?
                    || before.consumed != summary.sequence || summary.intended != after.digest()?
                { return Err(ManagedError::Corrupt); }
                expected.current = Some(summary.post.clone()); expected.pending = None;
                expected.source_policy = if summary.post.profile == 1 { summary.post.policy } else { None };
                if summary.purpose == 8 {
                    if summary.stage != 2 || before.phase != 4 || *disposition != 2 || after.retirement.is_none() {
                        return Err(ManagedError::Corrupt);
                    }
                    expected.phase = 5; expected.owner = None; expected.retirement = after.retirement;
                } else if *disposition == 2 {
                    if summary.purpose != 5 { return Err(ManagedError::Corrupt); }
                    expected.phase = if summary.post.profile == 1 { 1 } else { 6 }; expected.owner = None;
                } else if *disposition != 1 { return Err(ManagedError::Corrupt); }
                expected_receipt = Some(ReceiptRow::new(receipt)?); self.open = None;
            },
            Payload::Quiesce { foot, cutoff, transcript, .. } => {
                require_no_pending(before, &self.open)?;
                let auth = Owned::decode(transcript)?; rules::signed_step(before, after, &auth)?;
                if before.phase != 2 || foot.profile != 1 || foot.policy.is_some()
                    || before.current.as_ref() != Some(foot) || *cutoff <= after.utc
                { return Err(ManagedError::NotDrained); }
                expected.phase = 3;
            },
            Payload::Register { registry, transcript, .. } => {
                require_no_pending(before, &self.open)?;
                let auth = Owned::decode(transcript)?; rules::signed_step(before, after, &auth)?;
                if before.phase != 3 || before.target.is_some() || before.retirement.is_some()
                    || after.target.is_none() { return Err(ManagedError::OperationConflict); }
                let source = before.current.as_ref().ok_or(ManagedError::Corrupt)?;
                let target = after.target.as_ref().ok_or(ManagedError::Corrupt)?;
                if source.profile != 1 || target.profile != 4 || target.schema != super::schema::active_schema_digest()
                    || target.known.scope != source.known.scope || target.known.store_revision != next(source.known.store_revision)?
                    || target.known.event_sequence != source.known.event_sequence
                    || target.known.membership_revision != source.known.membership_revision
                    || target.member != source.member || target.quota != source.quota || target.policy.is_some()
                    || target.lineage.is_none()
                { return Err(ManagedError::TargetMismatch); }
                expected.target = Some(target.clone()); expected.registry = *registry; purpose = RegistryPurpose::Stage;
            },
            Payload::Select { retirement, source, target, binary, transcript, .. } => {
                require_no_pending(before, &self.open)?;
                let auth = Prospective::decode(transcript)?; rules::prospective(before, after, frontier, &auth)?;
                let candidate = &auth.candidate;
                if before.phase != 5 || before.owner.is_some() || before.current.as_ref() != Some(source)
                    || before.target.as_ref() != Some(target) || before.retirement != Some(*retirement)
                    || source.profile != 5 || target.profile != 4 || candidate.generation != next(before.generation)?
                    || candidate.term != next(before.term)? || candidate.session != next(before.session)?
                    || candidate.profile != 4 || candidate.schema != target.schema
                    || candidate.membership_revision != target.known.membership_revision
                    || Some(candidate.membership_digest) != target.member
                    || old_maps.registry.get(&5).ok_or(ManagedError::Corrupt)?.manifest_digest() != *binary
                { return Err(ManagedError::TargetMismatch); }
                expected.generation = candidate.generation; expected.phase = 6; expected.owner = None;
                expected.current = Some(target.clone()); expected.source_policy = None;
                expected.selected = Some(old_maps.registry.get(&3).ok_or(ManagedError::Corrupt)?.artifact);
            },
            Payload::Abort { summary, receipt, disposition } => {
                self.match_outcome(summary, after.utc)?;
                if before.pending != summary.post_pending || summary.stage != 1
                    || before.consumed != summary.sequence || receipt.revision != next(frontier.revision)?
                    || (summary.purpose == 8 && (*disposition != 2 || before.phase != 4))
                    || (summary.purpose != 8 && *disposition != 1)
                { return Err(ManagedError::Corrupt); }
                expected.current = Some(summary.pre.clone()); expected.pending = None;
                expected.source_policy = if summary.pre.profile == 1 { summary.pre.policy } else { None };
                if summary.purpose == 8 { expected.phase = 2; }
                expected_receipt = Some(ReceiptRow::new(receipt)?); self.open = None;
            },
            Payload::Checkpoint { revision, state, head, rows } => {
                require_no_pending(before, &self.open)?;
                if *revision != frontier.revision || *state != frontier.state_digest || *head != frontier.head
                    || *rows != rules::checkpoint_rows(old_maps)? || after != before
                { return Err(ManagedError::Corrupt); }
            },
            Payload::Owner { adopt: true, .. } => return Err(ManagedError::AlreadyExists),
        }
        let maps = action.delta.apply(old_maps, purpose, expected_receipt.as_ref())?;
        let (registry, receipts) = maps.digests()?; expected.registry = registry; expected.receipts = receipts;
        if expected != *after { return Err(ManagedError::Corrupt); }
        rules::registry(&maps, after)?; Ok(maps)
    }
    fn match_outcome(&self, summary: &Summary, utc: u64) -> Result<()> {
        let (open, pinned) = self.open.as_ref().ok_or(ManagedError::Corrupt)?;
        if open.encode()? != summary.encode()? || utc != *pinned { return Err(ManagedError::Corrupt); }
        Ok(())
    }
}
fn require_no_pending(state: &CatalogState, open: &Option<(Summary, u64)>) -> Result<()> {
    if state.pending.is_some() || open.is_some() { return Err(ManagedError::OperationConflict); }
    Ok(())
}
