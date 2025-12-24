# Contributing

Thanks for your interest in improving this portfolio project.

## Workflow
1. Create a feature branch.
2. Run quality checks locally:
   - `cargo fmt --all`
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
   - `cargo test --workspace --all-features`
3. Open a Pull Request with:
   - What changed
   - Why it changed
   - How it was tested

## Code style
- Prefer small, readable functions.
- Keep safety logic deterministic and side-effect free.
- Add tests for every behavior change.
