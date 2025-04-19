# Architecture Decision Record (ADR): Robot Simulation Architecture

## Submitters

- Youba.I

## Change Log

- [pending](https://github.com/YoubaImkf/astro-swarm/tree/main/CHANGELOG.md) 2025-04-19

## Referenced Use Case(s)

- [Astro Swarm Simulation Requirements](https://github.com/YoubaImkf/astro-swarm/tree/main/README.md)

## Context

The Astro Swarm project requires a simulation of autonomous robots exploring an exoplanet, collecting resources, and sharing knowledge. This architecture is significant because:

1. It involves concurrent processes (robots) operating independently
2. It requires thread-safe shared state (the map)
3. It models knowledge propagation between autonomous agents
4. It implements event sourcing for state tracking and robot communication

The high-level approach uses Rust's concurrency model with mpsc channels, Arc, and RwLock to create a thread-safe, message-passing architecture that simulates independent robot agents while maintaining system coherence.

## Proposed Design

### Services/Modules Impact

The system is structured around the following key modules:

1. **Map System**
   - `map::noise` - Procedural map generation using noise algorithms
   - `map::resources` - Resource management (energy, minerals, science points)
   - `map::map` - Core map state with thread-safe access

2. **Robot System**
   - `robots::robot` - Base robot structure and common behaviors
   - `robots::explorer` - Explorer robot specialization
   - `robots::collector` - Collector robot specialization
   - `robots::scientist` - Scientist robot specialization

3. **Communication System**
   - `communication::channels` - Event definitions and channel management
   - `communication::events` - Event sourcing implementation

4. **Station System**
   - `station::data_manager` - Knowledge merging and conflict resolution
   - `station::robot_factory` - Robot creation logic

### Model Impact

The system uses the following core data models:

1. **Map** - Thread-safe shared world state (`Arc<RwLock<Map>>`)
2. **Robot** - Independent agent with internal state and behavior
3. **Events** - Message types for inter-robot communication:
   - `ExplorationData`
   - `CollectionData` 
   - `ScienceData`
4. **Resources** - Consumable and discoverable items on the map

### API Impact

The system uses internal APIs for component communication:

1. **Robot-Map Interface**:
   ```rust
   fn move_to(&mut self, position: Position, map: &Arc<RwLock<Map>>)
   fn collect_resource(&mut self, resource_type: ResourceType, map: &Arc<RwLock<Map>>)
   fn scan_surroundings(&self, map: &Arc<RwLock<Map>>) -> ScanResult
   ```

2. **Event Channel Interface**:
   ```rust
   fn send_event(&self, event: RobotEvent)
   fn receive_events() -> Vec<RobotEvent>
   ```

3. **Station-Robot Interface**:
   ```rust
   fn merge_knowledge(&mut self, robot_knowledge: &RobotKnowledge)
   fn create_robot(&mut self, robot_type: RobotType) -> Result<Robot, Error>
   ```

### Configuration Impact

The system uses configuration for:
1. Map generation parameters (size, resource density)
2. Robot behavior parameters (energy consumption, view distance)
3. Station parameters (knowledge merging strategies)

### DevOps Impact

The concurrent nature of the application requires:
1. Proper thread management and shutdown procedures
2. Monitoring for deadlocks in shared resource access
3. Performance profiling for optimal thread count and synchronization strategies

## Considerations

1. **Alternative Approaches Considered**:
   - Pure ECS architecture: Rejected due to more complex synchronization needs
   - Lockless architecture with message-only passing: Rejected due to higher complexity and potential performance issues
   - Single-threaded simulation with time slicing: Rejected due to not taking advantage of modern multi-core systems

2. **Concerns Addressed**:
   - Thread safety: Addressed through careful use of Arc<RwLock<>>
   - Deadlock prevention: Addressed by consistent lock acquisition ordering
   - Message overflow: Addressed with bounded channels and back-pressure

3. **Performance Considerations**:
   - Fine-grained vs coarse-grained locking: Chose map-level locking for simplicity
   - Read vs write lock optimization: Using RwLock to allow concurrent reads

## Decision

I decided to implement the concurrent robot simulation using the following approach:

1. Each robot runs in its own thread, operating autonomously
2. The map is shared using Arc<RwLock<Map>> for thread-safe access
3. Communication occurs through typed channels carrying event objects
4. The main thread manages UI updates and central coordination
5. Station knowledge merging follows a git-like model with conflict resolution

### Implementation Details

- Robot spawning will use thread pools to control concurrency
- Map access will prioritize readers over writers for improved performance
- Event processing will be batched for efficiency
- Knowledge merging will use a last-write-wins strategy with conflict detection

### Future Considerations

1. Potential implementation of a more sophisticated conflict resolution algorithm
2. Exploration of lockless data structures for high-contention areas
3. Adaptive robot behavior based on swarm intelligence principles

## References

- [Rust Concurrency Documentation](https://doc.rust-lang.org/book/ch16-00-concurrency.html)
- [Event Sourcing Pattern](https://martinfowler.com/eaaDev/EventSourcing.html)