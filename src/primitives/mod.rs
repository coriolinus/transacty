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
    Hash,
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
    Hash,
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
    #[serde(rename = "type")]
    pub event_type: EventType,
    pub client: ClientId,
    pub tx: TransactionId,
    #[serde(
        deserialize_with = "default_if_empty",
        skip_serializing_if = "Amount::is_zero"
    )]
    pub amount: Amount,
}

/// See https://github.com/BurntSushi/rust-csv/issues/109#issuecomment-372724808
fn default_if_empty<'de, D, T>(de: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Option::<T>::deserialize(de).map(|x| x.unwrap_or_else(|| T::default()))
}

impl Event {
    /// This event has no amount associated with it; any amount in the data is junk
    pub fn has_amount(&self) -> bool {
        match self.event_type {
            EventType::Deposit | EventType::Withdrawal => true, // these event types have amounts
            EventType::Dispute | EventType::Resolve | EventType::Chargeback => false, // these event types have no amounts
        }
    }
}

/// ClientState stores the fundamental data about a particular client.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct ClientState {
    pub available: Amount,
    pub held: Amount,
    // total is always computed dynamically, so the struct can't get out of sync with itself
    pub locked: bool,
}

impl ClientState {
    pub fn to_serialize(&self, client: ClientId) -> SerializeClientState {
        let ClientState {
            available,
            held,
            locked,
        } = self.clone();
        SerializeClientState {
            client,
            total: available + held,
            available,
            held,
            locked,
        }
    }
}

/// SerializeClientState stores client data in a serialization-friendly way.
#[derive(Serialize)]
pub struct SerializeClientState {
    pub client: ClientId,
    pub available: Amount,
    pub held: Amount,
    pub total: Amount,
    pub locked: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    prop_compose! {
        fn arb_client_id(upper_bound: u16)(id in 0..upper_bound) -> ClientId {
            ClientId(id)
        }
    }

    prop_compose! {
        fn arb_transaction_id()(id in any::<u32>()) -> TransactionId {
            TransactionId(id)
        }
    }

    fn arb_event_type() -> impl Strategy<Value = EventType> {
        prop_oneof![
            Just(EventType::Deposit),
            Just(EventType::Withdrawal),
            Just(EventType::Dispute),
            Just(EventType::Resolve),
            Just(EventType::Chargeback),
        ]
    }

    fn arb_amount(max: f64) -> impl Strategy<Value = Amount> {
        // reduce the max value to one which can't fail.
        let max = max.min(900719925474.0);
        (0.0..max).prop_map(|value| {
            value
                .try_into()
                .expect("values in this range should never fail to convert")
        })
    }

    prop_compose! {
        fn arb_event(client_upper_bound: u16, max_amount: f64)
        (
            event_type in arb_event_type(),
            client in arb_client_id(client_upper_bound),
            tx in arb_transaction_id(),
            amount in arb_amount(max_amount),
        ) -> Event {
            let mut event = Event { event_type, client, tx, amount };
            if !event.has_amount() {
                event.amount = Amount::ZERO;
            }
            event
        }
    }

    proptest! {
        // This test is somewhat slow and benefits when being run in release mode
        #[test]
        fn test_event_stream_never_crashes(events in proptest::collection::vec(arb_event(100, 1000.0), (10, 1000))) {
            let mut state = crate::state::memory::MemoryState::default();
            crate::process_events(&mut state, events, None);
        }
    }
}
