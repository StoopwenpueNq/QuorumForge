# Changelog

All notable changes to QuorumForge are documented in this file.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Changed

- Rule tables are being reorganised for the next patch.

## [1.0.0] - 2026-06-09

### Added

- Stable contract: exit codes 0, 1 and 2, and the written bundle format in
  `docs/EVIDENCE.md`.
- Deterministic verdicts: identical inputs produce byte-identical reports.

## [0.9.0] - 2025-07-29

### Added

- `bundle` and `verify` so a deliberation can be archived and re-checked
  without the original inputs.
- The council viewer: an ANSI renderer and a self-contained HTML page.

## [0.8.0] - 2024-06-25

### Added

- Contradiction weighting: each claim carries the support and contradiction it
  survived, and the verdict names both.

## [0.7.0] - 2023-05-23

### Added

- The adjudication pass: normalize, weigh, then render a verdict per claim.
- JSON report with fixed key order and stable finding names.

## [0.6.0] - 2022-03-15

### Added

- Claim normalization rules with strict validation for ids and sources.
- `adjudicate` subcommand and the first report shape.

## [0.5.0] - 2020-12-01

### Added

- The minimal JSON codec, kept dependency-free so verdicts reproduce anywhere.
- Line numbers on every parse error instead of aborting the run.

## [0.4.0] - 2019-02-26

### Added

- Claim model with support and contradiction lists.
- Deterministic ordering for every list in the report.

## [0.3.0] - 2017-12-12

### Added

- Parser for the deliberation format, one record per line.
- `version` subcommand and the first output shape.

