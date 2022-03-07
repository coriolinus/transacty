mod amount;
pub use amount::Amount;

use derive_more::{Display, From, FromStr};

use serde::{Deserialize, Serialize};

/// The `TxType` identifies the nature of the specified transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TxType {
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
