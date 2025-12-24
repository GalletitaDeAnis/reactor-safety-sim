#!/usr/bin/env bash
set -euo pipefail

cargo run -p cli -- --scenario normal --seconds 120 --dt-ms 50 --setpoint 350 | head -n 5
cargo run -p cli -- --scenario overheat --seconds 180 --dt-ms 50 --setpoint 380 --trip-temp 420 | head -n 5
cargo test --workspace
