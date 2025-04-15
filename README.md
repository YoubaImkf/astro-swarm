# Astro Swarm

A robotic exploration system for alien planet surveying, built with Rust 🦀.

## Overview

Astro Swarm simulates a multi-robot ecosystem that autonomously explores, collects resources, and conducts scientific research on undiscovered terrain. 
The project features a terminal-based visualization system and realistic robot behaviors.

## Features

- **Specialized Robot Types**
  - Explorers: Map the terrain and identify resources
  - Collectors: Harvest energy and minerals
  - Scientists: Analyze points of scientific interest
- **Resource Management**
  - Energy (consumable), Minerals (consumable), Scientific points (non-consumable)
- **Swarm Intelligence**
  - Centralized communication, knowledge sharing, and autonomous decision-making
- **Interactive Display**
  - Real-time map visualization, resource tracking, and robot status monitoring

## Installation

```bash
git clone https://github.com/YoubaImkf/astro-swarm
cd astro-swarm
cargo run
```

## Controls

- `q`: Quit the application

## Architecture

- Procedural map generation using Perlin noise
- Thread-safe communication channels
- Resource management for consumable and non-consumable resources
- Terminal UI built with Ratatui

## License

This project is licensed under the MIT License. See the LICENSE file for details.