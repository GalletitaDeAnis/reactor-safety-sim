# Reactor Safety Simulation (Portfolio Project)

![CI](https://img.shields.io/badge/CI-GitHub%20Actions-blue)
![Language](https://img.shields.io/badge/Rust-stable-orange)
![Focus](https://img.shields.io/badge/focus-safety--critical%20software-success)

> **Disclaimer**
> This repository is a **portfolio project** that demonstrates safety‑critical software engineering practices using a **generic thermal process simulation**.
> It **does not model a real nuclear reactor**, is **not validated for operational use**, and **must not** be used to operate, design, or assess any real facility.

## What this is

`reactor-safety-sim` is a modular simulation + control + protection system that mimics the **structure** of industrial safety software:

- A simplified **plant model** (generic thermal dynamics)
- A **controller** (PID + limits) that tries to hold a setpoint
- A **protection layer** (interlocks / trips) that forces a **safe shutdown (SCRAM)** when hazards are detected
- **Fault injection** to test how the system behaves when sensors fail
- **Traceable logs** and reproducible runs (deterministic seeds)

This is built to show the kind of rigor that matters in regulated, high‑reliability domains: **determinism, defensive programming, testing, documentation, and CI quality gates**.

---

## Key features

### Simulation & control
- Generic thermal plant model (heat source + cooling loop style dynamics)
- PID controller with:
  - output limits and anti‑windup
  - rate limiting (optional)
  - configurable setpoints and step profiles

### Protection logic (safety layer)
- Safety interlocks and trips such as:
  - over‑temperature → **SCRAM**
  - sensor out‑of‑range / invalid → **fail‑safe**
  - inconsistent sensors → degrade or trip
- **2oo3 voting** (two‑out‑of‑three) for redundant sensor channels

### Fault injection & scenario testing
- Simulated sensor faults:
  - stuck value
  - bias / drift
  - noise bursts
  - dropouts / NaNs / invalid readings
- Scenario runner for “normal”, “overheat”, “loss of cooling”, “sensor disagreement”, etc.

### Engineering quality
- Workspace layout (clear separation of concerns)
- Structured logging (recommended: JSON lines)
- Tests:
  - unit tests
  - integration scenario tests
  - optional property‑based tests (e.g., `proptest`)
- CI checks (format, lint, tests)

---

## Repository structure

```
reactor-safety-sim/
  crates/
    sim/         # plant dynamics + sensors + fault injection
    controller/  # PID, limits, setpoint profiles
    safety/      # interlocks, trip logic, 2oo3 voting, SCRAM state machine
    cli/         # command-line scenario runner
  docs/
    ARCHITECTURE.md
    HAZARD_ANALYSIS.md
    SAFETY_CASE.md
  tests/
    scenarios.rs
  scripts/
    run_scenarios.sh
  .github/workflows/
    ci.yml
```

---

## Quick start

### Requirements
- Rust (stable) + Cargo  
  Install from rustup.

### Build
```bash
cargo build
```

### Run tests
```bash
cargo test
```

### Format & lint (recommended before every push)
```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
```

---

## Running the simulator (CLI)

> The CLI is designed to be simple: pick a scenario, duration, step time, and optional fault injections.

Example commands (adjust flags to your final CLI implementation):

### Normal operation
```bash
cargo run -p cli -- --scenario normal --seconds 120 --dt-ms 50 --setpoint 350
```

### Overheat + automatic SCRAM
```bash
cargo run -p cli -- --scenario overheat --seconds 180 --dt-ms 50 --setpoint 380 --trip-temp 420
```

### Sensor fault injection (stuck sensor)
```bash
cargo run -p cli -- --scenario normal --seconds 120 --fault sensor_a:stuck@360
```

### Redundancy test (2oo3 voting)
```bash
cargo run -p cli -- --scenario redundant_sensors --seconds 120 --fault sensor_b:bias@+15
```

### Reproducible run
```bash
cargo run -p cli -- --scenario normal --seconds 120 --seed 12345
```

---

## Outputs

Typical outputs may include:
- `out/run_<timestamp>.jsonl` — structured log events
- `out/trace_<timestamp>.csv` — time series: temperature, power, valve, trip state, etc.

You can plot CSV traces in any tool (Excel, Python, Grafana, etc.).

---

## Safety design notes (portfolio-oriented)

This project intentionally mirrors common safety design principles:

- **Fail‑safe defaults:** if inputs are invalid, transition to a safe state.
- **Separation of concerns:** control tries to optimize; safety enforces limits.
- **Redundancy & voting:** 2oo3 sensor consensus reduces single-channel faults.
- **Determinism:** the same seed + parameters produce the same run.
- **Traceability:** every trip and state change is logged with context.

Read more in:
- `docs/ARCHITECTURE.md`
- `docs/HAZARD_ANALYSIS.md`
- `docs/SAFETY_CASE.md`

---

## CI / Quality gates (recommended)

Suggested checks enforced by GitHub Actions:
- `cargo fmt --check`
- `cargo clippy -D warnings`
- `cargo test`
- (optional) `cargo audit` for dependency vulnerability scanning

If you want to go further:
- fuzzing via `cargo fuzz`
- benchmarks via `criterion`
- coverage via `llvm-cov`

---

## Roadmap

Planned enhancements (great “resume bullets”):
- Formal state machine for SCRAM & reset (with explicit transitions)
- Stronger typing for units (°C, seconds, %, etc.)
- Property‑based safety invariants (e.g., “after SCRAM, power → 0 within N steps”)
- Golden scenario snapshots (regression tests for full time series)
- Simple TUI dashboard (terminal UI) for live runs

---

## Contributing

Contributions are welcome, especially:
- additional scenarios and fault models
- improved documentation and diagrams
- stricter invariants and tests

Suggested workflow:
1. Create a feature branch
2. Run `cargo fmt`, `cargo clippy`, `cargo test`
3. Open a PR with a clear description and rationale

---

## Security

This is not operational software. Still, responsible handling matters:
- Please report security issues privately if you find anything concerning.
- See `SECURITY.md` (if present) for disclosure guidance.

---

## License

Choose a license for your repo (MIT/Apache-2.0 are common).  
Add the license text to `LICENSE`.

---

## Contact

If you’re reviewing this for hiring:
- Author: **Paolo Pizarro Arispe**
- Focus: safety‑critical software practices, Rust/C++ systems engineering, testing & reliability
