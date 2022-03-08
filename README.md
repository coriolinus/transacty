# Readme

This repo defines a very simple toy transaction engine, modeling certain financial transactions.

## Design Decisions

### Newtypes

Newtypes are used extensively. It's almost not worth it for a problem of this size, but in a larger program,
maintainability is substantially improved when each type permits only allowed operations.

The most important newtype is `primitives::Amount`, which takes some care to discard dust and ensure that
arithmetic is not subject to floating-point errors.

### Synchronicity

This code is written in a synchronous manner for simplicity and ease of development. This is an intentional choice,
motivated largely by the YAGNI principle.

> What if your
> code was bundled in a server, and these CSVs came from thousands of
> concurrent TCP streams?

In this case, we'd want to write an asynchronous version of this code. This would be a pretty straightforward
modification, the most complicated part of which would be to update the `StateManager` trait to deal with `Future` trait objects
(as of this writing, async trait members are still not allowed in stable rust).

In that case we'd also need some more robust definition of the actual sequencing of events; several of the event effects
depend on the precise order of prior events, which would be unreliable given events flowing through thousands of
concurrent TCP streams.

### Data Storage

This program stores global state in memory. This is not an ideal solution for a production system for obvious reasons,
but for a toy it should be fine. For production code, we'd likely want to maintain global state in an external data store.
Either Redis or a SQL engine (Postgres, Sqlite) might be appropriate choices of data store depending on requirements.

Becuase it was simple to, I wrote a `StateManager` trait which allows us to swap out different data backends as required.
Actually extending the program to use an external data store should be very easy.

### Library-first design

This program is written first as a library, with a very thin executable wrapped around it. This design pattern is very useful
to ensure maximum reusability of components.

### Logging and Telemetry

... have been omitted. YAGNI for a toy project.

### Testing

Integration tests are presented as CSV files in the `inputs/` directory. Each input contains an example of the expected
output when run with the `--debug` flag. There is at least one simple integration test demonstrating each error type.

I did not write any integration testing code which would run each of these as a distinct test, because that was more work
than I wanted to put in at this point. That would be an obvious next step.

Unit tests appear occasionally, for complicated bits. These generally use the `proptest` crate to expand the space of
inputs tested.

### Error Handling

Errors are generally handled gracefully, with some work put into ensuring stability. When run with the `--debug` flag,
runtime errors (i.e. insufficient balance to withdraw) are reported to stderr; otherwise, they are silently suppressed.

There are no instances of `.unwrap()` in this codebase. Explicit assumptions are sometimes expressed via `.expect()`.

## Assumptions

- No test data will cause any `Amount` to overflow
- Only deposits can be disputed
- Only withdrawals are affected by locks; deposits are still permitted
- Test data is valid CSV throughout; invalid CSV data is not a state to guard against
- Writing to stderr never panics
- Disputes will never result in the available balance underflowing

These assumptions are sometimes reflected in error-handling simplifications and may panic if invalidated.

The last assumption is critical, and the most likely to be invalidated.
However, it's impossible to tell from the instructions given what the policy should be. Some plausible policies:

- Invent a `SignedAmount` type which can be used only for the `available` balance, compatible otherwise with `Amount`. Requires a fair amount of implementation.
- Invent a holding period during which deposits are automatically held, and after which transactions cannot be disputed. Requires a notion of time.
