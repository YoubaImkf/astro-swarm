# Architecture Decision Record: Communication Architecture

## Context
Robots and the station need to communicate efficiently to share knowledge, report events, and coordinate actions.

## Decision
We implemented a channel-based communication system using Rust's `mpsc` channels.

## Rationale
- **Concurrency**: Channels allow asynchronous communication between threads.
- **Decoupling**: Robots and the station can operate independently while exchanging messages.
- **Scalability**: The system can handle multiple robots without significant performance degradation.

## Consequences
- Requires careful handling of message queues to avoid bottlenecks.
- Debugging communication issues can be challenging.