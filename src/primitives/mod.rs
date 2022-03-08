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
    use crate::state::memory::MemoryState;
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
            let mut state = MemoryState::default();
            crate::process_events(&mut state, events, None);
        }

        #[test]
        fn deposits_always_succeed(
            available in arb_amount(1000.0),
            held in arb_amount(1000.0),
            locked: bool,
            deposit in arb_amount(100.0),
        ) {
            let mut state = MemoryState::default();
            let client: ClientId = 1.into();
            state.client_state.insert(client, ClientState { available, held, locked });
            prop_assert!(state.deposits.is_empty());

            let event = Event { event_type: EventType::Deposit, client, tx: 1.into(), amount: deposit };
            crate::process_events(&mut state, [event.clone()], None);

            prop_assert_eq!(state.client_state[&client].available, available + deposit);
            prop_assert_eq!(state.client_state[&client].held, held);
            prop_assert_eq!(state.deposits.len(), 1);
            prop_assert_eq!(&state.deposits[&1.into()].event, &event);
            prop_assert_eq!(state.deposits[&1.into()].is_disputed, false);
        }

        #[test]
        fn withdrawals_succeed_when_unlocked_and_sufficient_balance(
            available in arb_amount(1000.0),
            held in arb_amount(1000.0),
            locked: bool,
            withdrawal in arb_amount(100.0),
        ) {
            let mut state = MemoryState::default();
            let client: ClientId = 1.into();
            state.client_state.insert(client, ClientState { available, held, locked });

            let event = Event { event_type: EventType::Withdrawal, client, tx: 1.into(), amount: withdrawal };
            crate::process_events(&mut state, [event], None);

            if !locked && withdrawal <= available {
                // withdrawal should succeed
                prop_assert_eq!(state.client_state[&client].available, available - withdrawal);
            } else {
                // withdrawal should fail
                prop_assert_eq!(state.client_state[&client].available, available);
            }
            prop_assert_eq!(state.client_state[&client].held, held);
        }

        #[test]
        fn dispute_moves_available_funds_to_held(
            available in arb_amount(1000.0),
            held in arb_amount(1000.0),
            locked: bool,
            disputed_amount in arb_amount(1000.0),
        ) {
            prop_assume!(disputed_amount <= available);

            let mut state = MemoryState::default();
            let client: ClientId = 1.into();
            let tx: TransactionId = 1.into();

            state.client_state.insert(client, ClientState { available, held, locked });
            let deposit = Event { event_type: EventType::Deposit, client, tx, amount: disputed_amount };
            state.deposits.insert(deposit.tx, deposit.into());
            prop_assert!(!state.deposits[&tx].is_disputed);

            let dispute = Event { event_type: EventType::Dispute, client: 2.into(), tx, amount: Amount::ZERO};
            crate::process_events(&mut state, [dispute], None);

            prop_assert!(state.deposits[&tx].is_disputed);
            prop_assert_eq!(state.client_state[&client].available, available - disputed_amount);
            prop_assert_eq!(state.client_state[&client].held, held + disputed_amount);
        }

        #[test]
        fn resolve_moves_held_funds_to_available(
            available in arb_amount(1000.0),
            held in arb_amount(1000.0),
            locked: bool,
            disputed_amount in arb_amount(1000.0),
        ) {
            prop_assume!(disputed_amount <= held);

            let mut state = MemoryState::default();
            let client: ClientId = 1.into();
            let tx: TransactionId = 1.into();

            state.client_state.insert(client, ClientState { available, held, locked });
            let deposit = Event { event_type: EventType::Deposit, client, tx, amount: disputed_amount };
            state.deposits.insert(deposit.tx, crate::state::memory::DepositRecord { event: deposit, is_disputed: true });

            let resolve = Event { event_type: EventType::Resolve, client: 2.into(), tx, amount: Amount::ZERO};
            crate::process_events(&mut state, [resolve], None);

            prop_assert!(!state.deposits[&tx].is_disputed);
            prop_assert_eq!(state.client_state[&client].available, available + disputed_amount);
            prop_assert_eq!(state.client_state[&client].held, held - disputed_amount);
        }

        #[test]
        fn chargeback_burns_held_funds_and_locks(
            available in arb_amount(1000.0),
            held in arb_amount(1000.0),
            locked: bool,
            disputed_amount in arb_amount(1000.0),
        ) {
            prop_assume!(disputed_amount <= held);

            let mut state = MemoryState::default();
            let client: ClientId = 1.into();
            let tx: TransactionId = 1.into();

            state.client_state.insert(client, ClientState { available, held, locked });
            let deposit = Event { event_type: EventType::Deposit, client, tx, amount: disputed_amount };
            state.deposits.insert(deposit.tx, crate::state::memory::DepositRecord { event: deposit, is_disputed: true });

            let chargeback = Event { event_type: EventType::Chargeback, client: 2.into(), tx, amount: Amount::ZERO};
            crate::process_events(&mut state, [chargeback], None);

            prop_assert!(!state.deposits[&tx].is_disputed);
            prop_assert_eq!(state.client_state[&client].available, available);
            prop_assert_eq!(state.client_state[&client].held, held - disputed_amount);
            prop_assert!(state.client_state[&client].locked);
        }
    }
}
