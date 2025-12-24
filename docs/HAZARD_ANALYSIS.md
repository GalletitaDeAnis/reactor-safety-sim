# Hazard Analysis (Portfolio)

> This document is portfolio-oriented and describes a **generic** thermal system (not a real reactor).

## Example hazards
- H1: Over-temperature beyond safe limit
- H2: Sensor failure (invalid / stuck / drift) leading to unsafe control action
- H3: Sensor disagreement (redundancy breakdown)

## Mitigations
- M1: Over-temp trip → SCRAM (force power to zero)
- M2: Fail-safe on invalid sensor input → SCRAM
- M3: 2oo3 voting and disagreement detection → degrade or SCRAM
- M4: Deterministic scenario tests and regression snapshots
