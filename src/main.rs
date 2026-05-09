//! drift-analyzer CLI — Precision drift analysis for constraint systems
//!
//! Usage:
//!   drift-analyzer --precision f32 --iterations 10000 --tolerance 1e-6
//!   drift-analyzer --compare-all --iterations 10000
//!   drift-analyzer --narrows
//!   drift-analyzer --eisenstein --a 3 --b 5 --iterations 10000

use drift_analyzer::{narrows_report, DriftAnalyzer, Precision};
use std::env;
use std::process;

const VERSION: &str = "0.1.0";

// ANSI colors
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";
const MAGENTA: &str = "\x1b[35m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const RESET: &str = "\x1b[0m";

fn print_banner() {
    eprintln!(
        "{}\
  {}╔═══════════════════════════════════════════════╗
  ║  {}⚙️  drift-analyzer v{}{}  {}                   ║
  ║  {}Precision drift analysis for constraints{}    ║
  ║  {}\"The boat sinks at iteration 28.\"{}          ║
  {}╚═══════════════════════════════════════════════╝{}",
        BOLD,
        CYAN,
        MAGENTA, VERSION, CYAN, BOLD,
        YELLOW, CYAN,
        DIM, CYAN,
        CYAN, RESET
    );
}

fn print_usage() {
    eprintln!("\n{}Usage:{}", BOLD, RESET);
    eprintln!("  drift-analyzer {}[OPTIONS]{}\n", DIM, RESET);
    eprintln!("{}Options:{}", BOLD, RESET);
    eprintln!("  {}--narrows{}              Run the Narrows scenario (all precisions)", GREEN, RESET);
    eprintln!("  {}--compare-all{}          Compare all precisions side-by-side", GREEN, RESET);
    eprintln!("  {}--eisenstein{}            Run Eisenstein multiply test", GREEN, RESET);
    eprintln!("  {}--precision <fmt>{}       Precision: f16, bf16, f32, f64, int8, int16, int32", YELLOW, RESET);
    eprintln!("  {}--iterations <n>{}        Number of iterations (default: 10000)", YELLOW, RESET);
    eprintln!("  {}--tolerance <t>{}          Constraint violation threshold (default: 1e-6)", YELLOW, RESET);
    eprintln!("  {}--a <n>{}                 Eisenstein parameter a (default: 3)", YELLOW, RESET);
    eprintln!("  {}--b <n>{}                 Eisenstein parameter b (default: 5)", YELLOW, RESET);
    eprintln!("  {}--help{}                  Show this help", CYAN, RESET);
    eprintln!("  {}--version{}               Show version", CYAN, RESET);
    eprintln!("\n{}Examples:{}", BOLD, RESET);
    eprintln!("  drift-analyzer {}--narrows{}", GREEN, RESET);
    eprintln!("  drift-analyzer {}--precision f32 --iterations 50000{}", GREEN, RESET);
    eprintln!("  drift-analyzer {}--compare-all --tolerance 1e-8{}", GREEN, RESET);
    eprintln!("  drift-analyzer {}--eisenstein --a 3 --b 5 --iterations 10000{}\n", GREEN, RESET);
}

fn parse_precision(s: &str) -> Option<Precision> {
    match s.to_lowercase().as_str() {
        "f16" | "fp16" | "half" => Some(Precision::F16),
        "bf16" | "bfloat16" => Some(Precision::Bf16),
        "f32" | "fp32" | "float" | "single" => Some(Precision::F32),
        "f64" | "fp64" | "double" => Some(Precision::F64),
        "i8" | "int8" => Some(Precision::Int8),
        "i16" | "int16" => Some(Precision::Int16),
        "i32" | "int32" => Some(Precision::Int32),
        _ => None,
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() {
        print_banner();
        print_usage();
        process::exit(0);
    }

    // Parse flags
    let mut flag_narrows = false;
    let mut flag_compare = false;
    let mut flag_eisenstein = false;
    let mut flag_precision: Option<Precision> = None;
    let mut iterations: u64 = 10_000;
    let mut tolerance: f64 = 1e-6;
    let mut eis_a: i32 = 3;
    let mut eis_b: i32 = 5;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                print_banner();
                print_usage();
                process::exit(0);
            }
            "--version" | "-V" => {
                println!("drift-analyzer {}", VERSION);
                process::exit(0);
            }
            "--narrows" => flag_narrows = true,
            "--compare-all" => flag_compare = true,
            "--eisenstein" => flag_eisenstein = true,
            "--precision" | "-p" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("{}Error: --precision requires an argument{}", RED, RESET);
                    process::exit(1);
                }
                flag_precision = parse_precision(&args[i]);
                if flag_precision.is_none() {
                    eprintln!(
                        "{}Error: unknown precision '{}' (try: f16, bf16, f32, f64, int8){}",
                        RED, &args[i], RESET
                    );
                    process::exit(1);
                }
            }
            "--iterations" | "-n" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("{}Error: --iterations requires an argument{}", RED, RESET);
                    process::exit(1);
                }
                iterations = args[i].parse().unwrap_or_else(|_| {
                    eprintln!("{}Error: invalid iteration count{}", RED, RESET);
                    process::exit(1);
                });
            }
            "--tolerance" | "-t" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("{}Error: --tolerance requires an argument{}", RED, RESET);
                    process::exit(1);
                }
                tolerance = args[i].parse().unwrap_or_else(|_| {
                    eprintln!("{}Error: invalid tolerance value{}", RED, RESET);
                    process::exit(1);
                });
            }
            "--a" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("{}Error: --a requires an argument{}", RED, RESET);
                    process::exit(1);
                }
                eis_a = args[i].parse().unwrap_or_else(|_| {
                    eprintln!("{}Error: invalid value for --a{}", RED, RESET);
                    process::exit(1);
                });
            }
            "--b" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("{}Error: --b requires an argument{}", RED, RESET);
                    process::exit(1);
                }
                eis_b = args[i].parse().unwrap_or_else(|_| {
                    eprintln!("{}Error: invalid value for --b{}", RED, RESET);
                    process::exit(1);
                });
            }
            other => {
                eprintln!("{}Error: unknown option '{}'{}", RED, other, RESET);
                eprintln!("Run with --help for usage.");
                process::exit(1);
            }
        }
        i += 1;
    }

    print_banner();

    // ── Narrows scenario ──
    if flag_narrows {
        println!("{}", narrows_report());
        return;
    }

    // ── Compare all ──
    if flag_compare {
        let reports = DriftAnalyzer::compare_all(tolerance, iterations);
        for r in &reports {
            println!("{}", r.display_colored(tolerance));
        }
        return;
    }

    // ── Eisenstein ──
    if flag_eisenstein {
        println!(
            "\n  {}Eisenstein Multiply Drift Test (a={}, b={}){}\n",
            BOLD, eis_a, eis_b, RESET
        );
        let report = DriftAnalyzer::eisenstein_multiply(eis_a, eis_b, iterations);
        println!("{}", report.display_colored(tolerance));
        return;
    }

    // ── Single precision analysis ──
    if let Some(p) = flag_precision {
        let angle_rad = 1.0_f64.to_radians();
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();

        let report = DriftAnalyzer::new(p, tolerance)
            .sample_interval(if iterations > 1000 { iterations / 20 } else { 10 })
            .analyze(
                |v| {
                    let x = v;
                    let x2 = x * x;
                    let y = if x2 <= 1.0 { (1.0 - x2).sqrt() } else { 0.0 };
                    x * cos_a - y * sin_a
                },
                1.0,
                iterations,
            );

        println!("{}", report.display_colored(tolerance));
        return;
    }

    // No action specified
    eprintln!(
        "\n  {}No analysis mode specified. Try --narrows, --compare-all, or --precision f32{}",
        YELLOW, RESET
    );
    eprintln!("  Run with {}--help{} for usage.\n", CYAN, RESET);
    process::exit(1);
}
