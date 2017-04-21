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
