# Architecture Decision Record: Map and Resource Management

## Context
The simulation map is procedurally generated using Perlin noise, and resources are distributed across the map. The map serves as the environment where robots operate.

## Decision
We decided to use Perlin noise for map generation and manage resources as discrete entities.

## Rationale
- **Perlin Noise**: Provides a natural-looking terrain with smooth transitions, suitable for a simulation environment.
- **Resource Management**: Resources are categorized into types (e.g., Energy, Minerals, Science Points) to align with robot capabilities.

## Consequences
- The map generation process is computationally intensive but provides a realistic environment.
- Resource management requires careful balancing to ensure fair distribution.