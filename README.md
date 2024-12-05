# payment\_engine\_challenge

[Documentation](https://yo2b.github.io/payment_engine_challenge)

## Wording of the challenge
> [...] implement a simple [payment] engine that [manages] transactions, [handling possible] disputes and chargebacks [...]

## Approach
Although practicing async & concurrent Rust is not a strong prerequisite for this challenge, I still decided to go the async way for better efficiency and further compliance, as it is stated it could possibly be server-side bundled and listen to TCP streams. Also it allows values to be streamed and processed on-the-fly rather than in-memory loading an entire dataset upfront.

Nevertheless, to handle transaction disputes, we need to keep track of achieved transactions in case it is disputed later on. Given that we need to aggregate all the transactions into balanced accounts, it also means that we cannot produce any output before having processed all incoming transactions (except maybe once an account is frozen). The capacity to stream data throughout the whole process bottlenecks because of this requirement, and it limits the potential of a fully streamlined data flow.

Regarding requirements about the decimal precision, I decided on purpose not to use an existing crate but to take up the challenge to implement it. It means I have only dealt with the minimum feature set required to meet the challenge, and may be missing common, usual operations. It also definitely lacks of intensive testing for accuracy and complete documentation for boundaries.

As a **strong hypothesis** prior to this challenge, I made the following assumptions:
- A transaction is considered as a one-way operation, ie. it is not possible for the same transaction to concern/refer to two different clients as a two-way (+/-) operation.
- A deposit cannot be disputed, only a withdrawal can.
- Once resolved or charged back, a transaction is considered completed and cannot be disputed again; as a consequence, it can be rolled out of transaction history.
- When an account is locked/frozen, should further transactions occur, it is considered they should just be discarded without any kind of track keeping except logging.
- It seems the wording for the expected behavior of a dispute could be erroneous; it will be considered that only held funds should increase by the amount disputed, and that clients' available funds should **not** be decreased.

Based on this assumptions:
- Any I/O errors or CSV-format (de)serialization errors are considered unrecoverable and will stop the process immediately.
  - If an error occurs while reading inputs for aggregation, no output other than the error is produced.
  - If an error occurs while writing outputs after aggregation, any previous output can be considered as a valid record but any further output is lost.
  - This behavior can easily be adapted in the `io::process()` function.
- Any processing errors due to transaction inconsistency or funds availability are considered recoverable and will just be logged then discarded. This behavior can easily be adapted in the `Processor::process()` function.
- Client's funds and transaction amount will be managed as unsigned decimal numbers, with the required decimal precision of up to four places past the decimal.
- Transaction history will only be kept in an in-memory cache with a limited size (see `process::MAX_TRANSACTION_CAPACITY` const), meaning "old" transactions could be rolled out at some point. An additional persistent cache system should be implemented as a fallback for "oldest" transactions before returning a transaction does not exists.

I also wanted to be careful about documenting and testing, as I would with any standard project.

## Workspace & projects
This workspace is made up of the following projects:
* `transaction`: a `lib` crate dedicated to handling transactions with minimal overhead and blazing-fast performances.
* `payment-engine`: a `bin` crate dedicated to dealing with transactions input/output files through a simple command-line interface.

## Features
The `transaction` crate aims to provide an efficient shared library with a minimal set of features to handle large transaction set in an asynchronous way.

It is mainly composed of:
- `io`: a module providing transaction I/O features, with helper functions to configure CSV reader/writer and initiate a whole transaction process.
- `num`: a module providing transaction numeric features, with a const-generic `Decimal<N>` to handle fixed-precision with up to `N` places past the decimal.
- `process`: a module providing transaction processing features, with `Processor` to handle an asynchronous stream of transactions on-the-fly.
- all necessary common types to deal with transactions and client accounts and their (de)serialization in CSV files.

## Dependencies
The crates in this workspace may rely on other renowned, widely tried and tested crates developed by the ever-growing Rust community, amongst others:

* [``futures``](https://crates.io/crates/futures) crate for futures/streams/sinks adaptations.
* [``tokio``](https://crates.io/crates/tokio) crate for event-driven, async I/O capabilities.
* [``csv-async``](https://crates.io/crates/csv-async) crate for async CSV-format (de)serialization capabilities.
* [``serde``](https://crates.io/crates/serde) crate for (de)serialization of data structures.
* [``tracing``](https://crates.io/crates/rocket) crate for logging capabilities.
* [``clap``](https://crates.io/crates/clap) crate for CLI management and command-line arguments parsing.
