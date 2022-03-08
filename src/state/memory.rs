use std::collections::HashMap;

use crate::{
    primitives::{ClientId, ClientState, Event, EventType, TransactionId},
    state::StateManager,
    EventError,
};

/// Deposit records keep track of which deposits are under dispute
struct DepositRecord {
    event: Event,
    is_disputed: bool,
}

impl From<Event> for DepositRecord {
    fn from(event: Event) -> Self {
        DepositRecord {
            event,
            is_disputed: false,
        }
    }
}

/// MemoryState is a state manager which keeps everything resident in local memory.
///
/// It's simple and fast, but unsuitable for production; production data stores
/// would like to have something with persistence, and something which can better
/// handle large states.
#[derive(Default)]
pub struct MemoryState {
    client_state: HashMap<ClientId, ClientState>,
    deposits: HashMap<TransactionId, DepositRecord>,
}

impl StateManager for MemoryState {
    type Err = ();

    fn handle_event(&mut self, event: Event) -> Result<(), EventError<Self::Err>> {
        match event.event_type {
            EventType::Deposit => {
                self.client_state.entry(event.client).or_default().available += event.amount;
                if let Some(displaced) = self.deposits.insert(event.tx, event.into()) {
                    // given untrusted user input, this could produce some kind of validation error instead
                    panic!(
                        "expected globally unique transaction IDs but inserted duplicate tx ID: {}",
                        displaced.event.tx
                    );
                }
            }

            EventType::Withdrawal => {
                let state = self
                    .client_state
                    .get_mut(&event.client)
                    .ok_or(EventError::UnknownClient(event.client))?;

                if state.available < event.amount {
                    return Err(EventError::InsufficientFunds(event.client, event.tx));
                }
                if state.locked {
                    return Err(EventError::AccountLocked(event.client, event.tx));
                }
                state.available -= event.amount;
            }

            EventType::Dispute => {
                if let Some(record) = self.deposits.get(&event.tx) {
                    if record.event.event_type != EventType::Deposit {
                        return Err(EventError::IllegalDispute(
                            event.client,
                            event.tx,
                            record.event.event_type,
                        ));
                    }
                    let state = self
                        .client_state
                        .get_mut(&record.event.client)
                        .ok_or(EventError::UnknownClient(event.client))?;
                    state.available -= record.event.amount;
                    state.held += record.event.amount;
                } else {
                    // If the tx specified by the dispute doesn't exist you can ignore it and assume this is
                    // an error on our partners' side.
                }
            }

            EventType::Resolve => {
                if let Some(record) = self.deposits.get(&event.tx) {
                    if !record.is_disputed {
                        // If the tx isn't under dispute, you can ignore the resolve and assume this is an error
                        // on our partners' side.
                        return Ok(());
                    }

                    let state = self
                        .client_state
                        .get_mut(&record.event.client)
                        .ok_or(EventError::UnknownClient(event.client))?;
                    state.held -= record.event.amount;
                    state.available += record.event.amount;
                }
            }

            EventType::Chargeback => {
                if let Some(record) = self.deposits.get(&event.tx) {
                    if !record.is_disputed {
                        // If the tx isn't under dispute, you can ignore the resolve and assume this is an error
                        // on our partners' side.
                        return Ok(());
                    }

                    let state = self
                        .client_state
                        .get_mut(&record.event.client)
                        .ok_or(EventError::UnknownClient(event.client))?;
                    state.held -= record.event.amount;
                    state.locked = true;
                }
            }
        }

        Ok(())
    }
}
