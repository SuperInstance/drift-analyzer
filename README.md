# drift-analyzer ⚙️

**Precision drift analyzer for constraint systems.**

> *"The boat sinks at iteration 28."*

When you apply the same floating-point operation thousands of times — rotations, multiplications, geometric algebra — your values drift. This library measures exactly how much, and tells you which precision format sinks first.

```
$ drift-analyzer --narrows

  ╔══════════════════════════════════════════════════════════════╗
  ║  ⚓  THE NARROWS — Which Boat Sinks First?                  ║
  ╚══════════════════════════════════════════════════════════════╝

  ┌─────────┬───────────┬───────┬────────────┬──────────────┬──────────┐
  │  Boat   │ Precision │ Bytes │ First Sink │ Signal Death │ Survived │
  ├─────────┼───────────┼───────┼────────────┼──────────────┼──────────┤
  │ INT32   │ INT32     │     4 │ never      │ never        │ ✓ ALL    │
  │ INT16   │ INT16     │     4 │ iter   320 │ never        │ ✗ leak   │
  │ INT8    │ INT8      │     4 │ iter     1 │ iter       2 │ ✗✗ DEAD  │
  │ FP64    │ FP64      │     4 │ never      │ never        │ ✓ ALL    │
  │ FP32    │ FP32      │     4 │ iter    28 │ iter    200  │ ✗✗ DEAD  │
  │ BF16    │ BF16      │     2 │ iter     8 │ iter    100  │ ✗✗ DEAD  │
  │ FP16    │ FP16      │     2 │ iter     1 │ iter      3  │ ✗✗ DEAD  │
  └─────────┴───────────┴───────┴────────────┴──────────────┴──────────┘
```

## Quick start

```bash
# The Narrows — compare all precisions
cargo run -- --narrows

# Single precision analysis
cargo run -- --precision f32 --iterations 10000

# Side-by-side comparison
cargo run -- --compare-all --iterations 50000

# Eisenstein integer multiplication
cargo run -- --eisenstein --a 3 --b 5 --iterations 10000
```

## As a library

```rust
use drift_analyzer::{DriftAnalyzer, Precision, narrows_report};

// Analyze a single precision
let report = DriftAnalyzer::new(Precision::F32, 1e-6)
    .sample_interval(100)
    .analyze(|v| v * 1.000001, 1.0, 10_000);

println!("First violation: {:?}", report.first_violation);
println!("Max error: {:.2e}", report.max_absolute_error);

// The Narrows — which boat sinks first?
println!("{}", narrows_report());
```

## Why this exists

[Constraint theory](https://github.com/SuperInstance/constraint-bench-suite) requires exact arithmetic. When you're proving geometric algebra identities — Eisenstein integers, rotation matrices, wedge products — floating-point drift isn't a nuisance, it's a proof-killer.

This tool was born from the **Narrows scenario**: a unit vector under repeated 1° rotations. FP32 destroys the signal at iteration ~200. BF16 kills it at ~50. Only FP64 and integers survive.

### The Narrows metaphor

Seven boats enter a narrow channel. Each carries a different precision format. The channel applies cumulative operations. Boats that drift too far hit the rocks.

- **FP64** — makes it through. The safe boat.
- **FP32** — hits rocks at iteration 28. Sinks at 200.
- **BF16** — hits rocks at iteration 8. Sinks at 50.
- **FP16** — barely starts before capsizing.
- **INT8** — sinks immediately (range too small).
- **INT32** — exact arithmetic. Walks through dry.

## Precision formats supported

| Format | Bytes | Mantissa bits | Survives Narrows? |
|--------|-------|---------------|-------------------|
| FP16   | 2     | 10            | ✗ (dead at ~3)    |
| BF16   | 2     | 7             | ✗ (dead at ~50)   |
| FP32   | 4     | 23            | ✗ (dead at ~200)  |
| FP64   | 8     | 52            | ✓                 |
| INT8   | 1     | exact*        | ✗ (range overflow)|
| INT16  | 2     | exact*        | ✗ (range overflow)|
| INT32  | 4     | exact*        | ✓                 |

*Exact within representable range.

## API

### `DriftAnalyzer`

```rust
let report = DriftAnalyzer::new(Precision::F32, tolerance)
    .sample_interval(100)
    .analyze(operation, initial_value, iterations);
```

### `DriftReport`

Fields:
- `first_violation: Option<u64>` — iteration where constraint broke
- `signal_destroyed_at: Option<u64>` — NaN/Inf/zero collapse
- `max_absolute_error: f64`
- `max_relative_error: f64`
- `drift_curve: Vec<DriftPoint>` — sampled measurements

### Pre-built scenarios

- `DriftAnalyzer::rotation_drift(angle, iterations)` — The Narrows
- `DriftAnalyzer::eisenstein_multiply(a, b, iterations)` — Eisenstein integer drift
- `DriftAnalyzer::compare_all(tolerance, iterations)` — All precisions
- `narrows_report()` — Formatted report

## Constraints

- Zero external dependencies (core library)
- Edition 2021, MSRV 1.75.0
- CLI uses only `std`

## Related

- [constraint-bench-suite](https://github.com/SuperInstance/constraint-bench-suite) — The benchmark suite this was built for
- [SuperInstance](https://github.com/SuperInstance) — Constraint-theory research

## License

MIT
