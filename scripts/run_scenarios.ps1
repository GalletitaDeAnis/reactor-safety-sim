$ErrorActionPreference = "Stop"

cargo run -p cli -- --scenario normal --seconds 120 --dt-ms 50 --setpoint 350 | Select-Object -First 5
cargo run -p cli -- --scenario overheat --seconds 180 --dt-ms 50 --setpoint 380 --trip-temp 420 | Select-Object -First 5
cargo test --workspace
