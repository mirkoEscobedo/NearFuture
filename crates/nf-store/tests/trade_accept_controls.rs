mod supplies_support;
mod trade_accept_controls_support;
mod trade_accept_support;
use nf_contract::identity::{OperationId, RequestId};
use nf_kernel::trade::{
    OfferId, TradeFinalKind, TradeReceipt, TradeRejection, TradeRequestOutcome,
};
use nf_store::{
    supplies::SuppliesStoreError,
    trade::{TradeChallenge, TradeStore, TradeStoreError as Error},
};
use std::sync::{Barrier, Mutex};
use trade_accept_controls_support::{Scenario, copy_proof, files, snapshot};

#[path = "trade_accept_controls_cases/coalescing.rs"]
mod coalescing;
#[path = "trade_accept_controls_cases/race.rs"]
mod race;
#[path = "trade_accept_controls_cases/received.rs"]
mod received;
#[path = "trade_accept_controls_cases/refusals.rs"]
mod refusals;
