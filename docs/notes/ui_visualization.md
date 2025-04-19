# Architecture Decision Record: UI and Visualization

## Context
The simulation requires a user interface to display the map, robot states, and statistics.

## Decision
We chose `ratatui` for rendering the UI and implemented custom widgets for map and statistics visualization.

## Rationale
- **Lightweight**: `ratatui` is efficient and suitable for terminal-based applications.
- **Customizability**: Allows creating tailored widgets for specific visualization needs.

## Consequences
- Limited to terminal-based UI, which may not appeal to all users.
- Requires additional effort to implement advanced visualizations.