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

