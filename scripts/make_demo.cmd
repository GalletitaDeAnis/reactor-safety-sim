@echo off
if not exist out mkdir out
cargo run -p cli -- --scenario overheat --seconds 180 --dt-ms 50 --setpoint 380 --trip-temp 420 > out\demo_overheat.jsonl
echo Wrote out\demo_overheat.jsonl
