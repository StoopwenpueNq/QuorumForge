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
            let mut delib = parse::parse_auto(&effective_path, &contents)
                .map_err(|e| CliError::parse(e.to_string()))?;
            normalize::normalize_deliberation(&mut delib);
            let rendered = inspect_json(&delib);
            emit(&output, &rendered)?;
            Ok(ExitCode::SUCCESS)
        }
        "verify" => {
            let ok = verify_bundle(&contents)?;
            if ok {
                emit(
                    &output,
                    "digest OK: bundle body matches its stored digest\n",
                )?;
                Ok(ExitCode::SUCCESS)
            } else {
                eprintln!(
                    "quorumforge: digest MISMATCH — bundle body does not match its stored digest"
                );
                Ok(ExitCode::from(4))
            }
        }
        other => Err(CliError::usage(format!("unknown command '{}'", other))),
    }
}

fn load_adjudicated(
    path: &str,
    contents: &str,
    policy: &Policy,
) -> Result<(quorumforge::Deliberation, quorumforge::Adjudication), CliError> {
    let mut delib =
        parse::parse_auto(path, contents).map_err(|e| CliError::parse(e.to_string()))?;
    normalize::normalize_deliberation(&mut delib);
    let adj = adjudicate(&delib, policy);
    Ok((delib, adj))
}

/// Re-parse a bundle JSON, re-derive the digest, and compare. This does not use
/// the library's `Bundle` struct round-trip; it recomputes the digest from the
/// serialised body exactly as `bundle::to_json` would, minus the digest field.
fn verify_bundle(contents: &str) -> Result<bool, CliError> {
    let root = json::parse(contents).map_err(|e| CliError::parse(e.message))?;
    let stored = root
        .get("digest")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CliError::parse("bundle has no 'digest' field"))?;

    // Reconstruct the body without the digest, in the same key order the writer
    // used, then compact-serialise and hash it.
    let entries = root
        .as_object()
        .ok_or_else(|| CliError::parse("bundle root is not an object"))?;
    let body: Vec<(String, json::Json)> = entries
        .iter()
        .filter(|(k, _)| k != "digest")
        .cloned()
        .collect();
    let body_json = json::Json::Obj(body);
    let recomputed = bundle::fnv1a_hex(json::to_string(&body_json).as_bytes());
    Ok(recomputed == stored)
}

fn inspect_json(delib: &quorumforge::Deliberation) -> String {
    let agents: Vec<json::Json> = delib
        .agents
        .values()
        .map(|a| {
            json::obj(vec![
                ("id", json::s(&a.id)),
                ("name", json::s(&a.name)),
                ("role", json::s(&a.role)),
                ("weight", json::num(a.weight)),
            ])
        })
        .collect();
    let claims: Vec<json::Json> = delib
        .claims
        .values()
        .map(|c| {
            json::obj(vec![
                ("id", json::s(&c.id)),
                ("topic", json::s(&c.topic)),
                ("text", json::s(&c.text)),
                ("normalized", json::s(&c.normalized)),
            ])
        })
        .collect();
    let positions: Vec<json::Json> = delib
        .positions
        .iter()
        .map(|p| {
            let cites: Vec<json::Json> = p
                .citations
                .iter()
                .map(|c| {
                    json::obj(vec![
                        ("source", json::s(&c.source)),
                        ("locator", json::s(&c.locator)),
                    ])
                })
                .collect();
            json::obj(vec![
                ("agent", json::s(&p.agent_id)),
                ("claim", json::s(&p.claim_id)),
                ("stance", json::s(p.stance.as_token())),
                ("confidence", json::num(p.confidence)),
                ("note", json::s(&p.note)),
                ("citations", json::Json::Arr(cites)),
            ])
        })
        .collect();
    let doc = json::obj(vec![
        ("id", json::s(&delib.id)),
        ("question", json::s(&delib.question)),
        ("agents", json::Json::Arr(agents)),
        ("claims", json::Json::Arr(claims)),
        ("positions", json::Json::Arr(positions)),
    ]);
    json::to_string_pretty(&doc)
}

fn read_input(input: &str, force_json: bool) -> Result<(String, String), CliError> {
    if input == "-" {
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| CliError::parse(format!("failed to read stdin: {}", e)))?;
        let path = if force_json { "stdin.json" } else { "stdin.qf" };
        Ok((buf, path.to_string()))
    } else {
        let contents = std::fs::read_to_string(input)
            .map_err(|e| CliError::parse(format!("cannot read '{}': {}", input, e)))?;
        let path = if force_json && !input.to_lowercase().ends_with(".json") {
            format!("{}.json", input)
        } else {
            input.to_string()
        };
        Ok((contents, path))
    }
}

fn emit(output: &Option<String>, text: &str) -> Result<(), CliError> {
    match output {
        Some(path) => {
            let mut file = std::fs::File::create(path)
                .map_err(|e| CliError::parse(format!("cannot write '{}': {}", path, e)))?;
            file.write_all(text.as_bytes())
                .map_err(|e| CliError::parse(format!("write failed: {}", e)))?;
            if !text.ends_with('\n') {
                let _ = file.write_all(b"\n");
            }
            Ok(())
        }
        None => {
            print!("{}", text);
            if !text.ends_with('\n') {
                println!();
            }
            Ok(())
        }
    }
}

fn take_value(args: &[String], i: &mut usize, name: &str) -> Result<String, CliError> {
    *i += 1;
    args.get(*i)
        .cloned()
        .ok_or_else(|| CliError::usage(format!("option {} requires a value", name)))
}

fn take_f64(args: &[String], i: &mut usize, name: &str) -> Result<f64, CliError> {
    let v = take_value(args, i, name)?;
    v.parse::<f64>()
        .map_err(|_| CliError::usage(format!("option {} expects a number, got '{}'", name, v)))
}
