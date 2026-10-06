use crate::{
    codec::validate_state,
    model::*,
    signing::{invitation_digest, verify},
};

impl MembershipState {
    /// Explicit local founder initialization, never remote admission or recovery.
    pub fn bootstrap(scope: Scope, founder: &PublicIdentity) -> Result<Self, IdentityError> {
        if founder.peer.is_empty() || founder.peer.len() > 128 {
            return Err(IdentityError::Limit);
        }
        Ok(Self {
            scope,
            revision: 0,
            owner: founder.account,
            accounts: [(
                founder.account,
                Account {
                    key: founder.account_key,
                    roles: Roles::ALL,
                },
            )]
            .into(),
            devices: [(
                founder.device,
                Device {
                    account: founder.account,
                    key: founder.device_key,
                    peer: founder.peer.clone(),
                    revoked: false,
                },
            )]
            .into(),
            consumed: Default::default(),
        })
    }
    pub fn redeem(
        &self,
        signed: &SignedInvitation,
        proof: &AdmissionProof,
        now: u64,
    ) -> Result<Self, IdentityError> {
        crate::codec::validate_state(self)?;
        let invitation = &signed.invitation;
        if invitation.scope != self.scope {
            return Err(IdentityError::Scope);
        }
        if invitation.expires_at <= now {
            return Err(IdentityError::Expired);
        }
        if invitation.issued_revision > self.revision {
            return Err(IdentityError::Frontier);
        }
        if invitation.reusable {
            return Err(IdentityError::RolePolicy);
        }
        if self.consumed.contains(&invitation.id) {
            return Err(IdentityError::Replay);
        }
        let issuer = self
            .accounts
            .get(&invitation.issuer)
            .ok_or(IdentityError::UnknownAccount)?;
        if invitation.issuer != self.owner
            && (!issuer.roles.contains(Roles::MODERATOR) || invitation.roles != Roles::PLAYER)
        {
            return Err(IdentityError::RolePolicy);
        }
        let digest = invitation_digest(invitation)?;
        verify(&issuer.key, &digest, &signed.signature)?;
        verify(
            &invitation.recipient.account_key,
            &digest,
            &proof.account_signature,
        )?;
        verify(
            &invitation.recipient.device_key,
            &digest,
            &proof.device_signature,
        )?;
        if self.accounts.contains_key(&invitation.recipient.account)
            || self.devices.contains_key(&invitation.recipient.device)
        {
            return Err(IdentityError::Conflict);
        }
        let mut next = self.clone();
        next.revision = next.revision.checked_add(1).ok_or(IdentityError::Limit)?;
        next.accounts.insert(
            invitation.recipient.account,
            Account {
                key: invitation.recipient.account_key,
                roles: invitation.roles,
            },
        );
        next.devices.insert(
            invitation.recipient.device,
            Device {
                account: invitation.recipient.account,
                key: invitation.recipient.device_key,
                peer: invitation.recipient.peer.clone(),
                revoked: false,
            },
        );
        next.consumed.insert(invitation.id);
        validate_state(&next)?;
        Ok(next)
    }
}
