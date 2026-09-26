# Changelog

All notable changes to this crate are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/). Before 1.0 the minor slot is where
breaking changes go.

## [Unreleased]

### Added

- Repository scaffold, templated from thumb-asm: CI gates (dev/qa/release,
  scorecard), REUSE licensing, and the Renesas manuals under `spec/`.
- `Target` (H8/300, H8/300H, H8S/2000, H8S/2600, H8SX), `Mode` (normal,
  middle, advanced, maximum), and `Target::supports`.
