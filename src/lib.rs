pub mod primitives;
pub mod state;

use primitives::{ClientId, Event, EventType, TransactionId};
use state::StateManager;

/// Process a stream of events, updating global state appropriately.
///
/// If `errors` is not `None`, errors will be sent along that channel.
/// This is a `SyncSender` insetad of a `Sender` because unbuffered channels
/// are dangerous in a server context.
pub fn process_events<State, I>(
    state: &mut State,
    events: I,
    errors: Option<std::sync::mpsc::SyncSender<EventError<<State as StateManager>::Err>>>,
) where
    State: StateManager,
    I: IntoIterator<Item = Event>,
{
    for event in events.into_iter() {
        if let Err(err) = state.handle_event(event) {
            if let Some(errors) = &errors {
                if let Err(_) = errors.send(err) {
                    eprintln!("event processing terminated early due to send error");
                    break;
                }
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EventError<E> {
    #[error("client {0} has insufficient funds to withdraw as requested by transaction {1}")]
    InsufficientFunds(ClientId, TransactionId),
    #[error("client {0} cannot withdraw per transaction {1} because their account is locked")]
    AccountLocked(ClientId, TransactionId),
    #[error("client {0} attempted to dispute transaction {1} ({2:?}), but only deposits may be disputed")]
    IllegalDispute(ClientId, TransactionId, EventType),
    #[error("client {0} does not exist")]
    UnknownClient(ClientId),
    #[error("state error")]
    StateError(#[source] E),
}
