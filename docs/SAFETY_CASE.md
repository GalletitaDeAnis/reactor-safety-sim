# Safety Case (Portfolio)

This is a portfolio safety case for a **generic thermal process**.

## Safety goals
- SG1: Prevent sustained operation above configured temperature limit.
- SG2: Default to a safe state on invalid or inconsistent sensor data.
- SG3: Provide traceability for every safety decision.

## Evidence (planned)
- Unit tests for trips, voting, and validation.
- Integration tests for scenarios (overheat, sensor faults).
- CI enforcement for formatting, linting, and tests.
- Optional: property-based tests for safety invariants.
