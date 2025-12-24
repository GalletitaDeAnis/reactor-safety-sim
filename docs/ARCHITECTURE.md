# Architecture

## Overview
This repository is organized as a Rust workspace with separated crates:

- `sim`: generic thermal plant model, sensors, fault injection
- `controller`: PID controller and setpoint handling
- `safety`: interlocks, trips, 2oo3 voting, SCRAM state
- `cli`: scenario runner producing logs/traces

## Data flow (high-level)
1. Plant state evolves in fixed time steps (`dt`).
2. Sensors provide measurements (with optional faults).
3. Control computes actuator commands (power/cooling).
4. Safety layer evaluates hazards and can force SCRAM.
5. Traces/logs are emitted for analysis and regression tests.
