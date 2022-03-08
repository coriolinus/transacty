/// The Amount type is complicated, so we've moved it into its own module for code organization purposes.
/// Logically, it lives among the other primitives.
mod amount;
pub use amount::Amount;

use derive_more::{Display, From, FromStr};

use serde::{Deserialize, Serialize};

/// The event type identifies the nature of the specified transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EventType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

/// A Client ID uniquely identifies a client.
///
/// It is known to be a valid `u16`.
///
/// No mathematical operations have been derived because it requires
/// only the semantics of an identifier.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    FromStr,
    Display,
    From,
    Serialize,
    Deserialize,
)]
pub struct ClientId(u16);

/// A Transaction ID uniquely identifies a transaction.
///
/// It is known to be a valid `u32`.
///
/// No mathematical operations have been derived because it requires
/// only the semantics of an identifier.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    FromStr,
    Display,
    From,
    Serialize,
    Deserialize,
)]
pub struct TransactionId(u32);

/// An Event is the fundamental unit of data flowing through this system.
///
/// It is an atomic unit of state change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    pub event_type: EventType,
    pub client: ClientId,
    pub tx: TransactionId,
    pub amount: Amount,
}

/// ClientState stores the fundamental data about a particular client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientState {
    pub available: Amount,
    pub held: Amount,
    // total is always computed dynamically, so the struct can't get out of sync with itself
    pub locked: bool,
}
