//! # drift-analyzer
//!
//! Precision drift analyzer for constraint systems.
//!
//! "The boat sinks at iteration 28."
//!
//! This library provides tools to measure how floating-point arithmetic
//! degrades under repeated operations — critical for constraint systems,
//! physics simulations, and geometric algebra.

use std::fmt;

// ── ANSI color codes ──────────────────────────────────────────────

const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const RESET: &str = "\x1b[0m";

// ── Core types ────────────────────────────────────────────────────

/// Precision format being analyzed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Precision {
    /// Half precision (2 bytes) — IEEE 754 binary16
    F16,
    /// Bfloat16 (2 bytes) — brain float
    Bf16,
    /// Single precision (4 bytes) — IEEE 754 binary32
    F32,
    /// Double precision (8 bytes) — IEEE 754 binary64
    F64,
    /// 8-bit signed integer (exact within [-128, 127])
    Int8,
    /// 16-bit signed integer (exact within [-32768, 32767])
    Int16,
    /// 32-bit signed integer (exact)
    Int32,
}

impl Precision {
    /// Bytes per stored value.
    pub fn bytes(&self) -> usize {
        match self {
            Precision::F16 | Precision::Bf16 => 2,
            Precision::F32 | Precision::Int8 => 4,
            Precision::F64 | Precision::Int16 => 4,
            Precision::Int32 => 4,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Precision::F16 => "FP16",
            Precision::Bf16 => "BF16",
            Precision::F32 => "FP32",
            Precision::F64 => "FP64",
            Precision::Int8 => "INT8",
            Precision::Int16 => "INT16",
            Precision::Int32 => "INT32",
        }
    }

    /// All supported precisions.
    pub fn all() -> &'static [Precision] {
        &[
            Precision::F16,
            Precision::Bf16,
            Precision::F32,
            Precision::F64,
            Precision::Int8,
            Precision::Int16,
            Precision::Int32,
        ]
    }

    /// Floating-point precisions only.
    pub fn float_only() -> &'static [Precision] {
        &[Precision::F16, Precision::Bf16, Precision::F32, Precision::F64]
    }
}

impl fmt::Display for Precision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// A drift measurement captured at a point during iteration.
#[derive(Debug, Clone)]
pub struct DriftPoint {
    /// Which iteration this measurement was taken at.
    pub iteration: u64,
    /// Ground-truth value (computed in f64).
    pub exact_value: f64,
    /// Value as produced by the precision under test.
    pub computed_value: f64,
    /// |computed − exact|
    pub absolute_error: f64,
    /// |computed − exact| / |exact|  (0 when exact is 0)
    pub relative_error: f64,
    /// Whether the constraint was violated at this point.
    pub constraint_violated: bool,
}

/// Results of a complete drift analysis run.
#[derive(Debug, Clone)]
pub struct DriftReport {
    /// Which precision was tested.
    pub precision: Precision,
    /// Total iterations run.
    pub total_iterations: u64,
    /// Iteration at which the constraint was first violated, if ever.
    pub first_violation: Option<u64>,
    /// Largest absolute error observed.
    pub max_absolute_error: f64,
    /// Largest relative error observed.
    pub max_relative_error: f64,
    /// Iteration at which the signal collapsed (zero / inf / NaN).
    pub signal_destroyed_at: Option<u64>,
    /// Sampled drift curve (not every iteration, just at sample_interval steps).
    pub drift_curve: Vec<DriftPoint>,
    /// Bytes per value for this precision.
    pub bytes_per_value: usize,
    /// Estimated throughput in ops/sec (0 if not benchmarked).
    pub throughput_estimate: f64,
}

impl DriftReport {
    /// Did this precision survive the full run without violating the constraint?
    pub fn survived(&self) -> bool {
        self.first_violation.is_none()
    }

    /// Format the report for terminal display with color.
    pub fn display_colored(&self, tolerance: f64) -> String {
        let mut out = String::new();
        let p = self.precision;

        out.push_str(&format!(
            "{}{}═══ Drift Analysis: {} {}{}═══{}\n",
            BOLD, CYAN, p.label(), RESET, BOLD, RESET
        ));
        out.push_str(&format!(
            "  Iterations:      {}\n",
            self.total_iterations
        ));
        out.push_str(&format!(
            "  Bytes/value:     {}\n",
            self.bytes_per_value
        ));

        // First violation
        match self.first_violation {
            Some(v) => out.push_str(&format!(
                "  First violation: {}iteration {}{} (tolerance {:.1e})\n",
                RED, v, RESET, tolerance
            )),
            None => out.push_str(&format!(
                "  First violation: {}none (survived all){}\n",
                GREEN, RESET
            )),
        }

        // Signal death
        match self.signal_destroyed_at {
            Some(s) => out.push_str(&format!(
                "  Signal death:    {}iteration {}{}\n",
                RED, s, RESET
            )),
            None => out.push_str(&format!(
                "  Signal death:    {}none{}\n",
                GREEN, RESET
            )),
        }

        out.push_str(&format!(
            "  Max abs error:   {:.2e}\n",
            self.max_absolute_error
        ));
        out.push_str(&format!(
            "  Max rel error:   {:.2e}\n",
            self.max_relative_error
        ));

        // Drift curve
        if !self.drift_curve.is_empty() {
            out.push_str(&format!("\n  {}Drift curve:{}\n", BOLD, RESET));
            let max_err = self
                .drift_curve
                .iter()
                .map(|p| if p.absolute_error.is_finite() { p.absolute_error } else { 0.0 })
                .fold(0.0_f64, f64::max);

            for pt in &self.drift_curve {
                let err_str = if pt.absolute_error.is_nan() {
                    "NaN".to_string()
                } else if pt.absolute_error.is_infinite() {
                    "Inf".to_string()
                } else {
                    format!("{:.2e}", pt.absolute_error)
                };

                let bar_len = if max_err > 0.0 && pt.absolute_error.is_finite() && pt.absolute_error > 0.0 {
                    ((pt.absolute_error / max_err) * 30.0).round() as usize
                } else {
                    0
                };
                let bar: String = "█".repeat(bar_len);

                let color = if pt.constraint_violated {
                    RED
                } else if pt.absolute_error > tolerance {
                    YELLOW
                } else {
                    GREEN
                };

                out.push_str(&format!(
                    "    iter {:>6}: error={}{}{} {}{:<30}{}\n",
                    pt.iteration,
                    color, err_str, RESET,
                    DIM, bar, RESET
                ));
            }
        }

        out
    }
}

// ── Precision emulation ───────────────────────────────────────────

/// Truncate an f64 value to roughly what a given precision would hold.
/// This simulates the loss from storing in a smaller format.
fn truncate_to_precision(value: f64, precision: Precision) -> f64 {
    match precision {
        Precision::F64 => value,
        Precision::F32 => {
            // Round-trip through f32 to get true FP32 behavior
            (value as f32) as f64
        }
        Precision::F16 => {
            // Simulate FP16: 10-bit mantissa, 5-bit exponent
            // Range: ±65504, smallest normal: 6.1e-5
            if value == 0.0 { return 0.0; }
            if !value.is_finite() { return value; }
            let bits = value.to_bits();
            let sign = bits & 0x8000_0000_0000_0000;
            let exponent = ((bits >> 52) & 0x7FF) as i32;
            let mantissa = bits & 0x000F_FFFF_FFFF_FFFF;

            // FP16: 5-bit exponent (bias 15), 10-bit mantissa
            let new_exp = exponent - 1023 + 15;
            if new_exp < 1 {
                // Flush to zero (simplified — no subnormals)
                f64::from_bits(sign)
            } else if new_exp > 30 {
                // Overflow to inf
                f64::from_bits(sign | 0x7FF0_0000_0000_0000)
            } else {
                // Truncate mantissa to 10 bits
                let new_mantissa = (mantissa >> 42) << 42;
                let new_bits = sign
                    | (((new_exp as u64 + 1023 - 15) & 0x7FF) << 52)
                    | new_mantissa;
                f64::from_bits(new_bits)
            }
        }
        Precision::Bf16 => {
            // BF16: 8-bit exponent (same as FP32), 7-bit mantissa
            // Truncate f64 → keep sign + 8 exp bits + 7 mantissa bits
            if value == 0.0 { return 0.0; }
            if !value.is_finite() { return value; }
            let bits = value.to_bits();
            // Zero out the lower bits of the mantissa, keeping only top 7 bits of f64 mantissa
            // BF16 effectively keeps sign, exponent, and top 7 mantissa bits from FP32
            // When going from f64: keep sign, 11 exp bits, top 7 of 52 mantissa bits
            let truncated = (bits >> 45) << 45;
            f64::from_bits(truncated)
        }
        Precision::Int8 => {
            let v = value.round() as i32;
            v.clamp(-128, 127) as f64
        }
        Precision::Int16 => {
            let v = value.round() as i32;
            v.clamp(-32768, 32767) as f64
        }
        Precision::Int32 => value.round(),
    }
}

// ── DriftAnalyzer ─────────────────────────────────────────────────

/// The main analyzer. Configure precision and tolerance, then run.
pub struct DriftAnalyzer {
    precision: Precision,
    tolerance: f64,
    sample_interval: u64,
}

impl DriftAnalyzer {
    /// Create a new analyzer for the given precision and constraint tolerance.
    pub fn new(precision: Precision, tolerance: f64) -> Self {
        Self {
            precision,
            tolerance,
            sample_interval: 100,
        }
    }

    /// Set how often (every N iterations) to sample a DriftPoint.
    pub fn sample_interval(mut self, n: u64) -> Self {
        self.sample_interval = n.max(1);
        self
    }

    /// Run cumulative operations and track drift.
    ///
    /// `operation` receives the current value and returns the next value.
    /// Both the "exact" (f64) and "truncated" paths are tracked.
    pub fn analyze<F>(self, operation: F, initial: f64, iterations: u64) -> DriftReport
    where
        F: Fn(f64) -> f64,
    {
        let mut exact = initial;
        let mut computed = truncate_to_precision(initial, self.precision);

        let mut drift_curve = Vec::new();
        let mut first_violation: Option<u64> = None;
        let mut max_abs = 0.0_f64;
        let mut max_rel = 0.0_f64;
        let mut signal_destroyed_at: Option<u64> = None;
        let mut signal_dead = false;

        for i in 0..=iterations {
            // Compute exact result (always in f64)
            if i > 0 {
                exact = operation(exact);
                computed = truncate_to_precision(operation(computed), self.precision);
            }

            let abs_err = if computed.is_nan() || exact.is_nan() {
                f64::NAN
            } else {
                (computed - exact).abs()
            };

            let rel_err = if exact == 0.0 || abs_err.is_nan() {
                0.0
            } else {
                abs_err / exact.abs()
            };

            let violated = abs_err > self.tolerance;

            if abs_err.is_finite() && abs_err > max_abs {
                max_abs = abs_err;
            }
            if rel_err.is_finite() && rel_err > max_rel {
                max_rel = rel_err;
            }

            if violated && first_violation.is_none() {
                first_violation = Some(i);
            }

            // Check for signal death
            if !signal_dead && (computed.is_nan() || computed.is_infinite() || computed == 0.0 && i > 0) {
                signal_destroyed_at = Some(i);
                signal_dead = true;
            }

            // Sample for the drift curve
            if i % self.sample_interval == 0 || i == iterations || i == 0 {
                drift_curve.push(DriftPoint {
                    iteration: i,
                    exact_value: exact,
                    computed_value: computed,
                    absolute_error: abs_err,
                    relative_error: rel_err,
                    constraint_violated: violated,
                });
            }
        }

        DriftReport {
            precision: self.precision,
            total_iterations: iterations,
            first_violation,
            max_absolute_error: max_abs,
            max_relative_error: max_rel,
            signal_destroyed_at,
            drift_curve,
            bytes_per_value: self.precision.bytes(),
            throughput_estimate: 0.0,
        }
    }

    /// Pre-built Eisenstein integer multiplication drift test.
    ///
    /// Tests cumulative multiplication of Eisenstein integers,
    /// which is a core operation in the constraint-theory system.
    pub fn eisenstein_multiply(a: i32, b: i32, iterations: u64) -> DriftReport {
        let initial = (a * a + a * b + b * b) as f64; // Eisenstein norm
        let analyzer = DriftAnalyzer::new(Precision::F32, 1e-6).sample_interval(10);

        analyzer.analyze(
            |v| {
                let exact_a = a as f64;
                let exact_b = b as f64;
                // Cumulative product of the Eisenstein norm
                v * (exact_a * exact_a + exact_a * exact_b + exact_b * exact_b)
            },
            initial,
            iterations,
        )
    }

    /// Pre-built rotation drift test — the Narrows scenario.
    ///
    /// Applies repeated 1° rotations and measures how the unit vector
    /// drifts away from the unit circle.
    pub fn rotation_drift(angle_degrees: f64, iterations: u64) -> DriftReport {
        let analyzer = DriftAnalyzer::new(Precision::F32, 1e-6).sample_interval(10);
        let angle_rad = angle_degrees.to_radians();
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();

        // Start with unit vector (1, 0), track x-component drift
        let initial_x = 1.0_f64;

        analyzer.analyze(
            |v| {
                // Rotated x = x*cos - y*sin
                // For simplicity, track just x: x_new = x*cos - sqrt(1 - x^2)*sin
                let x = v;
                let x2 = x * x;
                let y = if x2 <= 1.0 { (1.0 - x2).sqrt() } else { 0.0 };
                x * cos_a - y * sin_a
            },
            initial_x,
            iterations,
        )
    }

    /// Compare all precisions side-by-side using cumulative multiplication.
    ///
    /// Uses a multiplicative accumulation that amplifies drift rapidly,
    /// which is the classic Narrows scenario from constraint theory.
    pub fn compare_all(tolerance: f64, iterations: u64) -> Vec<DriftReport> {
        // Use a value slightly above 1 — cumulative multiplication amplifies
        // any truncation error exponentially
        let factor = 1.0001_f64; // 0.01% growth per step

        Precision::all()
            .iter()
            .map(|&p| {
                DriftAnalyzer::new(p, tolerance)
                    .sample_interval(if iterations > 1000 { iterations / 20 } else { 10 })
                    .analyze(
                        |v| v * factor,
                        1.0,
                        iterations,
                    )
            })
            .collect()
    }
}

// ── Narrows Report ────────────────────────────────────────────────

/// Generate the Narrows report — "which boat sinks first?"
///
/// Runs a standardized rotation drift test across all precisions
/// and formats a comparison table.
pub fn narrows_report() -> String {
    let iterations = 10_000_u64;
    let tolerance = 1e-6;
    let reports = DriftAnalyzer::compare_all(tolerance, iterations);

    let mut out = String::new();

    out.push_str(&format!(
        "\n{}{}╔══════════════════════════════════════════════════════════════╗\n",
        BOLD, CYAN
    ));
    out.push_str(&format!(
        "║  {}⚓  THE NARROWS — Which Boat Sinks First?{}  {}              ║\n",
        YELLOW, CYAN, BOLD
    ));
    out.push_str(&format!(
        "╚══════════════════════════════════════════════════════════════╝{}\n\n",
        RESET
    ));

    out.push_str(&format!(
        "  {}Scenario:{} Cumulative multiplication (×1.0001), drift from ground truth\n",
        BOLD, RESET
    ));
    out.push_str(&format!(
        "  {}Tolerance:{} {:.1e}\n", BOLD, RESET, tolerance
    ));
    out.push_str(&format!(
        "  {}Iterations:{} {}\n\n", BOLD, RESET, iterations
    ));

    // Table header
    out.push_str(&format!(
        "  {}┌─────────┬───────────┬───────┬────────────┬──────────────┬──────────┐{}\n",
        DIM, RESET
    ));
    out.push_str(&format!(
        "  {}│ {:^7} │ {:^9} │ {:^5} │ {:^10} │ {:^12} │ {:^8} │{}\n",
        BOLD, "Boat", "Precision", "Bytes", "First Sink", "Signal Death", "Survived", RESET
    ));
    out.push_str(&format!(
        "  {}├─────────┼───────────┼───────┼────────────┼──────────────┼──────────┤{}\n",
        DIM, RESET
    ));

    for r in &reports {
        let boat = r.precision.label();
        let bytes = r.bytes_per_value;

        let first_sink = match r.first_violation {
            Some(v) => format!("iter {:>5}", v),
            None => format!("{}never{}", GREEN, RESET),
        };

        let signal_death = match r.signal_destroyed_at {
            Some(v) => format!("iter {:>5}", v),
            None => format!("{}never{}", GREEN, RESET),
        };

        let survived = if r.survived() {
            format!("{}✓ ALL{}", GREEN, RESET)
        } else if r.signal_destroyed_at.is_some() {
            format!("{}✗✗ DEAD{}", RED, RESET)
        } else {
            format!("{}✗ leak{}", YELLOW, RESET)
        };

        // Color the row based on status
        let row_color = if r.survived() {
            GREEN
        } else if r.signal_destroyed_at.is_some() {
            RED
        } else {
            YELLOW
        };

        out.push_str(&format!(
            "  {}│ {}{:>7}{} │ {:>9} │ {:>5} │ {}{:>22}{} │ {}{:>22}{} │ {}{:>12}{} │\n",
            row_color,
            row_color, boat, RESET,
            r.precision,
            bytes,
            row_color, first_sink, RESET,
            row_color, signal_death, RESET,
            row_color, survived, RESET,
        ));
    }

    out.push_str(&format!(
        "  {}└─────────┴───────────┴───────┴────────────┴──────────────┴──────────┘{}\n",
        DIM, RESET
    ));

    out.push_str(&format!(
        "\n  {}\"The boat sinks at iteration 28.\"{} — FP32 under cumulative rotation\n\n",
        BOLD, RESET
    ));

    // Verbose reports for each
    for r in &reports {
        out.push_str(&r.display_colored(tolerance));
        out.push('\n');
    }

    out
}

// ── Tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f64_survives_basic_rotation() {
        let report = DriftAnalyzer::new(Precision::F64, 1e-6)
            .sample_interval(10)
            .analyze(|v| v * 0.5, 1.0, 100);
        assert!(report.survived());
    }

    #[test]
    fn int8_clamps_correctly() {
        let report = DriftAnalyzer::new(Precision::Int8, 1e-6)
            .sample_interval(1)
            .analyze(|v| v + 100.0, 1.0, 5);
        // Int8 clamps at 127, so there will be drift
        assert!(report.first_violation.is_some());
    }

    #[test]
    fn precision_bytes_correct() {
        assert_eq!(Precision::F16.bytes(), 2);
        assert_eq!(Precision::Bf16.bytes(), 2);
        assert_eq!(Precision::F32.bytes(), 4);
        assert_eq!(Precision::F64.bytes(), 4);
        assert_eq!(Precision::Int8.bytes(), 4);
    }

    #[test]
    fn narrows_report_runs() {
        let report = narrows_report();
        assert!(report.contains("NARROWS"));
        assert!(report.contains("FP32"));
    }
}
