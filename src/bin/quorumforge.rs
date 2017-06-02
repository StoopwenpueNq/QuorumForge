//! The `quorumforge` command-line interface.
//!
//! Usage:
//!
//! ```text
//! quorumforge <command> [options] <evidence-file>
//!
//! Commands:
//!   adjudicate   Parse, normalize, and print a text or JSON verdict report.
//!   bundle       Emit a deterministic, digest-stamped evidence bundle (JSON).
//!   verify       Re-derive a bundle's digest and confirm it is intact.
//!   inspect      Print the parsed, normalized deliberation as JSON.
//!   help         Show this message.
//!
//! Options:
//!   --format <text|json>     Report format for `adjudicate` (default: text).
//!   --consensus <0..1>       Consensus polarity threshold (default: 0.66).
//!   --dissent <0..1>         Dissent ceiling (default: 0.34).
//!   --min-mass <n>           Minimum decisive mass to escape unsupported.
//!   -o, --output <path>      Write output to a file instead of stdout.
//!   -                        Read evidence from stdin (format assumed .qf
//!                            unless --json is given).
//!   --json                   Treat stdin/ambiguous input as JSON.
//! ```
//!
//! Exit codes: `0` success, `2` usage error, `3` parse/validation error,
//! `4` verify mismatch.

use quorumforge::adjudicate::{adjudicate, Policy};
use quorumforge::{bundle, json, normalize, parse, report};
use std::io::{Read, Write};
use std::process::ExitCode;

const USAGE: &str = "\
quorumforge — multi-agent evidence adjudication engine

USAGE:
    quorumforge <command> [options] <evidence-file | ->

COMMANDS:
    adjudicate   Parse, normalize, and print a verdict report (text or JSON).
    bundle       Emit a deterministic, digest-stamped evidence bundle (JSON).
    verify       Re-derive a bundle's digest from its body and confirm it.
    inspect      Print the parsed, normalized deliberation as JSON.
    help         Show this message.

OPTIONS:
    --format <text|json>   Report format for `adjudicate` (default: text).
    --consensus <0..1>     Consensus polarity threshold (default: 0.66).
    --dissent <0..1>       Dissent ceiling (default: 0.34).
    --min-mass <n>         Minimum decisive mass to escape unsupported.
    -o, --output <path>    Write output to a file instead of stdout.
    --json                 Treat stdin / extensionless input as JSON.

EXAMPLES:
    quorumforge adjudicate samples/cache-coherence.qf
    quorumforge adjudicate --format json samples/vaccine-efficacy.json
    quorumforge bundle samples/cache-coherence.qf -o bundle.json
    quorumforge verify bundle.json
    cat samples/cache-coherence.qf | quorumforge adjudicate -
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("quorumforge: {}", err.message);
            err.code
        }
    }
}

struct CliError {
    message: String,
    code: ExitCode,
}

impl CliError {
    fn usage(msg: impl Into<String>) -> Self {
        CliError {
            message: msg.into(),
            code: ExitCode::from(2),
        }
    }
    fn parse(msg: impl Into<String>) -> Self {
        CliError {
            message: msg.into(),
            code: ExitCode::from(3),
        }
    }
}

fn run(args: &[String]) -> Result<ExitCode, CliError> {
    if args.is_empty() {
        print!("{}", USAGE);
        return Ok(ExitCode::from(2));
    }

    let command = args[0].as_str();
    if command == "help" || command == "--help" || command == "-h" {
        print!("{}", USAGE);
        return Ok(ExitCode::SUCCESS);
    }

    // Parse options and collect the positional input path.
    let mut format = "text".to_string();
    let mut policy = Policy::default();
    let mut output: Option<String> = None;
    let mut input: Option<String> = None;
    let mut force_json = false;

    let mut i = 1;
    while i < args.len() {
        let arg = args[i].as_str();
        match arg {
            "--format" => {
                format = take_value(args, &mut i, "--format")?;
            }
            "--consensus" => {
                policy.consensus_threshold = take_f64(args, &mut i, "--consensus")?;
            }
            "--dissent" => {
                policy.dissent_ceiling = take_f64(args, &mut i, "--dissent")?;
            }
            "--min-mass" => {
                policy.minimum_mass = take_f64(args, &mut i, "--min-mass")?;
            }
            "-o" | "--output" => {
                output = Some(take_value(args, &mut i, "--output")?);
            }
            "--json" => {
                force_json = true;
            }
            other if other.starts_with("--") => {
                return Err(CliError::usage(format!("unknown option '{}'", other)));
            }
            _ => {
                if input.is_some() {
                    return Err(CliError::usage(format!(
                        "unexpected extra argument '{}'",
                        arg
                    )));
                }
                input = Some(arg.to_string());
            }
        }
        i += 1;
    }

    policy
        .validate()
        .map_err(|e| CliError::usage(format!("invalid policy: {}", e)))?;

    let input =
        input.ok_or_else(|| CliError::usage("no evidence file given (use '-' for stdin)"))?;

    let (contents, effective_path) = read_input(&input, force_json)?;

    match command {
        "adjudicate" => {
            let (delib, adj) = load_adjudicated(&effective_path, &contents, &policy)?;
            let rendered = match format.as_str() {
                "text" => report::render_text(&delib, &adj),
                "json" => report::render_json(&delib, &adj),
                other => {
                    return Err(CliError::usage(format!(
                        "unknown --format '{}', expected text or json",
                        other
                    )))
                }
            };
            emit(&output, &rendered)?;
            Ok(ExitCode::SUCCESS)
        }
        "bundle" => {
            let (delib, adj) = load_adjudicated(&effective_path, &contents, &policy)?;
            let b = bundle::build(&delib, &adj);
            let rendered = bundle::to_json(&b);
            emit(&output, &rendered)?;
            Ok(ExitCode::SUCCESS)
        }
        "inspect" => {
