$ErrorActionPreference = "Stop"

New-Item -ItemType Directory -Force out | Out-Null

# Overheat demo (should SCRAM)
cargo run -p cli -- --scenario overheat --seconds 180 --dt-ms 50 --setpoint 380 --trip-temp 420 `
  | Set-Content -Encoding utf8 out\demo_overheat.jsonl

Write-Host "Wrote out\demo_overheat.jsonl"
