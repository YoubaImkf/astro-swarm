# Architecture Decision Record: Concurrency and Synchronization

## Context
The simulation involves multiple threads for robots, the station, and the UI. Shared state must be managed safely.

## Decision
We used `RwLock` for shared state and `mpsc` channels for communication.

## Rationale
- **Thread Safety**: `RwLock` ensures safe access to shared resources.
- **Asynchronous Communication**: Channels decouple threads and prevent blocking.

## Consequences
- Potential for deadlocks if locks are not managed carefully.
- Increased complexity in debugging concurrency issues.