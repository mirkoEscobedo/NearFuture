use super::{
    ChatPolicy, ChatRatePolicy, ChatStoreError, Result, codec,
    rate_runtime::{Debit, Reload},
    receiver_rate::Balance,
};
use nf_contract::identity::AccountId;
use nf_identity::model::{MembershipState, Scope};
use rusqlite::{Connection, OptionalExtension, params};
use std::collections::BTreeMap;
pub(super) const SCHEMA: &str = "CREATE TABLE chat_rate_meta (singleton INTEGER PRIMARY KEY CHECK(singleton=1), universe BLOB NOT NULL CHECK(length(universe)=16), history BLOB NOT NULL CHECK(length(history)=16), generation BLOB NOT NULL CHECK(length(generation)=8), policy_body BLOB NOT NULL CHECK(length(policy_body)=61), policy_digest BLOB NOT NULL CHECK(length(policy_digest)=32)) STRICT;\nCREATE TABLE chat_rate_accounts (universe BLOB NOT NULL CHECK(length(universe)=16), history BLOB NOT NULL CHECK(length(history)=16), account BLOB NOT NULL CHECK(length(account)=16), generation BLOB NOT NULL CHECK(length(generation)=8), tokens INTEGER NOT NULL CHECK(tokens BETWEEN 0 AND 1024), fractional_ns BLOB NOT NULL CHECK(length(fractional_ns)=8), state_digest BLOB NOT NULL CHECK(length(state_digest)=32), PRIMARY KEY(universe,history,account)) STRICT;\n";
const DOMAIN: &[u8; 22] = b"NF-CHAT-RATE-POLICY-1\0";
pub(super) struct Snapshot {
    pub policy: ChatRatePolicy,
    pub generation: u64,
    pub accounts: BTreeMap<AccountId, Balance>,
}
pub(super) fn policy_bytes(scope: Scope, policy: ChatRatePolicy) -> Vec<u8> {
    let mut out = DOMAIN.to_vec();
    out.extend_from_slice(scope.universe.as_bytes());
    out.extend_from_slice(scope.history.as_bytes());
    out.extend_from_slice(&policy.capacity().to_be_bytes());
    out.extend_from_slice(&policy.refill_ms().to_be_bytes());
    out.push(1); // Receiver-local monotonic refill; no downtime/startup refill.
    out
}
fn decode_policy(scope: Scope, bytes: &[u8]) -> Result<ChatRatePolicy> {
    if bytes.len() != 61
        || &bytes[..22] != DOMAIN
        || &bytes[22..38] != scope.universe.as_bytes()
        || &bytes[38..54] != scope.history.as_bytes()
        || bytes[60] != 1
    {
        return Err(ChatStoreError::Corrupt);
    }
    let capacity = u16::from_be_bytes(
        bytes[54..56]
            .try_into()
            .map_err(|_| ChatStoreError::Corrupt)?,
    );
    let refill = u32::from_be_bytes(
        bytes[56..60]
            .try_into()
            .map_err(|_| ChatStoreError::Corrupt)?,
    );
    let policy = ChatRatePolicy::new(capacity, refill).map_err(|_| ChatStoreError::Corrupt)?;
    if policy_bytes(scope, policy) != bytes {
        return Err(ChatStoreError::Corrupt);
    }
    Ok(policy)
}
type Meta = (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>);
pub(super) fn meta(connection: &Connection, scope: Scope) -> Result<(ChatRatePolicy, u64)> {
    let count: i64 = connection.query_row(
        "SELECT count(*) FROM (SELECT 1 FROM chat_rate_meta LIMIT 2)",
        [],
        |r| r.get(0),
    )?;
    if count != 1 {
        return Err(ChatStoreError::Corrupt);
    }
    let (universe,history,generation,body,digest):Meta=connection.query_row("SELECT universe,history,generation,policy_body,policy_digest FROM chat_rate_meta WHERE singleton=1",[],|r|Ok((
        crate::persistence::bounded_blob(r,0,16)?,crate::persistence::bounded_blob(r,1,16)?,
        crate::persistence::bounded_blob(r,2,8)?,crate::persistence::bounded_blob(r,3,61)?,
        crate::persistence::bounded_blob(r,4,32)?
    )))?;
    if universe != scope.universe.as_bytes() || history != scope.history.as_bytes() {
        return Err(ChatStoreError::Scope);
    }
    if digest != codec::hash(&body) {
        return Err(ChatStoreError::Corrupt);
    }
    Ok((
        decode_policy(scope, &body)?,
        crate::schema::read_counter(&generation)?,
    ))
}
fn state_digest(scope: Scope, account: AccountId, generation: u64, balance: Balance) -> [u8; 32] {
    let mut bytes = b"NF-CHAT-RATE-STATE-1\0".to_vec();
    bytes.extend_from_slice(scope.universe.as_bytes());
    bytes.extend_from_slice(scope.history.as_bytes());
    bytes.extend_from_slice(account.as_bytes());
    bytes.extend_from_slice(&generation.to_be_bytes());
    bytes.extend_from_slice(&balance.tokens.to_be_bytes());
    bytes.extend_from_slice(&balance.fractional_ns.to_be_bytes());
    codec::hash(&bytes)
}
pub(super) fn initialize(
    connection: &Connection,
    policy: &ChatPolicy,
    members: &MembershipState,
    rate: ChatRatePolicy,
    fresh: bool,
) -> Result<()> {
    if members.scope != policy.scope || members.accounts.len() > 128 {
        return Err(ChatStoreError::Scope);
    }
    let body = policy_bytes(policy.scope, rate);
    if connection.execute(
        "INSERT INTO chat_rate_meta VALUES(1,?1,?2,?3,?4,?5)",
        params![
            policy.scope.universe.as_bytes(),
            policy.scope.history.as_bytes(),
            crate::schema::counter(0),
            body,
            codec::hash(&body)
        ],
    )? != 1
    {
        return Err(ChatStoreError::Corrupt);
    }
    let balance = if fresh {
        Balance::fresh(rate)
    } else {
        Balance::migrated_existing()
    };
    for account in members.accounts.keys() {
        insert_account(connection, policy.scope, *account, 0, balance)?;
    }
    Ok(())
}
fn insert_account(
    connection: &Connection,
    scope: Scope,
    account: AccountId,
    generation: u64,
    balance: Balance,
) -> Result<()> {
    if connection.execute(
        "INSERT INTO chat_rate_accounts VALUES(?1,?2,?3,?4,?5,?6,?7)",
        params![
            scope.universe.as_bytes(),
            scope.history.as_bytes(),
            account.as_bytes(),
            crate::schema::counter(generation),
            i64::from(balance.tokens),
            crate::schema::counter(balance.fractional_ns),
            state_digest(scope, account, generation, balance)
        ],
    )? != 1
    {
        return Err(ChatStoreError::Corrupt);
    }
    Ok(())
}
pub(super) fn add_accounts(
    connection: &Connection,
    scope: Scope,
    accounts: &[AccountId],
    policy: ChatRatePolicy,
    generation: u64,
) -> Result<()> {
    if accounts.len() > 128 || meta(connection, scope)? != (policy, generation) {
        return Err(ChatStoreError::Corrupt);
    }
    for account in accounts {
        insert_account(
            connection,
            scope,
            *account,
            generation,
            Balance::fresh(policy),
        )?;
    }
    Ok(())
}
pub(super) fn verify_all(connection: &Connection, members: &MembershipState) -> Result<Snapshot> {
    let (policy, generation) = meta(connection, members.scope)?;
    if members.accounts.len() > 128 {
        return Err(ChatStoreError::Limit);
    }
    let mut query=connection.prepare("SELECT universe,history,account,generation,tokens,fractional_ns,state_digest FROM chat_rate_accounts ORDER BY universe,history,account LIMIT 129")?;
    let mut rows = query.query([])?;
    let mut accounts = BTreeMap::new();
    while let Some(row) = rows.next()? {
        let universe = crate::persistence::bounded_blob(row, 0, 16)?;
        let history = crate::persistence::bounded_blob(row, 1, 16)?;
        let account: [u8; 16] = crate::persistence::bounded_blob(row, 2, 16)?
            .try_into()
            .map_err(|_| ChatStoreError::Corrupt)?;
        let row_generation =
            crate::schema::read_counter(&crate::persistence::bounded_blob(row, 3, 8)?)?;
        let tokens = u16::try_from(row.get::<_, i64>(4)?).map_err(|_| ChatStoreError::Corrupt)?;
        let fractional_ns =
            crate::schema::read_counter(&crate::persistence::bounded_blob(row, 5, 8)?)?;
        if universe != members.scope.universe.as_bytes()
            || history != members.scope.history.as_bytes()
            || row_generation != generation
            || accounts.len() >= 128
        {
            return Err(ChatStoreError::Corrupt);
        }
        let id = AccountId::from_bytes(account);
        if crate::persistence::bounded_blob(row, 6, 32)?
            != state_digest(
                members.scope,
                id,
                generation,
                Balance {
                    tokens,
                    fractional_ns,
                },
            )
        {
            return Err(ChatStoreError::Corrupt);
        }
        if !members.accounts.contains_key(&id)
            || accounts
                .insert(
                    id,
                    Balance {
                        tokens,
                        fractional_ns,
                    }
                    .validate(policy)?,
                )
                .is_some()
        {
            return Err(ChatStoreError::Corrupt);
        }
    }
    if accounts.len() != members.accounts.len() {
        return Err(ChatStoreError::Corrupt);
    }
    Ok(Snapshot {
        policy,
        generation,
        accounts,
    })
}
pub(super) fn balance(
    connection: &Connection,
    scope: Scope,
    account: AccountId,
    policy: ChatRatePolicy,
    generation: u64,
) -> Result<Balance> {
    if meta(connection, scope)? != (policy, generation) {
        return Err(ChatStoreError::Corrupt);
    }
    type StoredBalanceRow = (Vec<u8>, i64, Vec<u8>, Vec<u8>);
    let row:Option<StoredBalanceRow>=connection.query_row("SELECT generation,tokens,fractional_ns,state_digest FROM chat_rate_accounts WHERE universe=?1 AND history=?2 AND account=?3",
        params![scope.universe.as_bytes(),scope.history.as_bytes(),account.as_bytes()],
        |r|Ok((crate::persistence::bounded_blob(r,0,8)?,r.get(1)?,crate::persistence::bounded_blob(r,2,8)?,crate::persistence::bounded_blob(r,3,32)?))
    ).optional()?;
    let (row_generation, tokens, fractional, digest) = row.ok_or(ChatStoreError::Corrupt)?;
    if crate::schema::read_counter(&row_generation)? != generation {
        return Err(ChatStoreError::Corrupt);
    }
    let state = Balance {
        tokens: u16::try_from(tokens).map_err(|_| ChatStoreError::Corrupt)?,
        fractional_ns: crate::schema::read_counter(&fractional)?,
    }
    .validate(policy)?;
    if digest != state_digest(scope, account, generation, state) {
        return Err(ChatStoreError::Corrupt);
    }
    Ok(state)
}
fn replace_balance(
    connection: &Connection,
    scope: Scope,
    account: AccountId,
    before: Balance,
    after: Balance,
    old_generation: u64,
    new_generation: u64,
) -> Result<()> {
    if connection.execute("UPDATE chat_rate_accounts SET tokens=?1,fractional_ns=?2,generation=?3,state_digest=?10 WHERE universe=?4 AND history=?5 AND account=?6 AND tokens=?7 AND fractional_ns=?8 AND generation=?9 AND state_digest=?11",params![
        i64::from(after.tokens),crate::schema::counter(after.fractional_ns),crate::schema::counter(new_generation),
        scope.universe.as_bytes(),scope.history.as_bytes(),account.as_bytes(),i64::from(before.tokens),
        crate::schema::counter(before.fractional_ns),crate::schema::counter(old_generation),state_digest(scope,account,new_generation,after),state_digest(scope,account,old_generation,before)
    ])?!=1 { return Err(ChatStoreError::Conflict); }
    Ok(())
}
pub(super) fn debit(
    connection: &Connection,
    scope: Scope,
    generation: u64,
    debit: &Debit,
) -> Result<()> {
    replace_balance(
        connection,
        scope,
        debit.account,
        debit.before,
        debit.after,
        generation,
        generation,
    )
}
pub(super) fn reload(
    connection: &Connection,
    scope: Scope,
    old_policy: ChatRatePolicy,
    old_generation: u64,
    plan: &Reload,
) -> Result<()> {
    if meta(connection, scope)? != (old_policy, old_generation)
        || plan.generation != old_generation.checked_add(1).ok_or(ChatStoreError::Limit)?
    {
        return Err(ChatStoreError::Conflict);
    }
    let old = policy_bytes(scope, old_policy);
    let body = policy_bytes(scope, plan.policy);
    if connection.execute("UPDATE chat_rate_meta SET generation=?1,policy_body=?2,policy_digest=?3 WHERE singleton=1 AND universe=?4 AND history=?5 AND generation=?6 AND policy_body=?7 AND policy_digest=?8",params![
        crate::schema::counter(plan.generation),body,codec::hash(&body),scope.universe.as_bytes(),scope.history.as_bytes(),
        crate::schema::counter(old_generation),old,codec::hash(&old)
    ])?!=1 { return Err(ChatStoreError::Conflict); }
    for (account, (before, after)) in &plan.rows {
        after.validate(plan.policy)?;
        replace_balance(
            connection,
            scope,
            *account,
            *before,
            *after,
            old_generation,
            plan.generation,
        )?;
    }
    Ok(())
}
