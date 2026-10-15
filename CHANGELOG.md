# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

When releasing (see `RELEASING.md`), `scripts/set-version.sh` renames
`[Unreleased]` to the new version + date and starts a fresh `[Unreleased]`
section.

## [Unreleased]

### Compatibility

- **Rust source change**: `LayoutOptions` adds `fallback_glyph_metrics` (default:
  `None`). Full struct literals must add the field or use
  `..LayoutOptions::default()`; exhaustive patterns must add the field or `..`.
  Default/builder usage, the C ABI, and DisplayList JSON remain compatible.

### Added

- `\slashed` and `\centernot` commands.
- Mac Catalyst support in the Apple XCFramework and Swift package.
- Host-provided fallback glyph metrics for more accurate text spacing.
- Configurable PNG/SVG padding in the CLI.
- Borrowed lexer tokens via `Lexer::next_span()` and `TokenSpan`.

### Fixed

- PNG glyphs shifting by one pixel after cache reuse at different positions.
- mhchem spacing regression caused by escaped `##` source locations.
- Color inheritance for rules, boxes, and related symbols.
- Website rendering before fonts finish loading.
- iOS linking failures involving unwind symbols.

### Security

- Bound macro expansion, mhchem intermediates, and layout work, including
  `\edef`/`\xdef`, `\pmb`, and Array/CD padding.
- Reject overflowing `\char` values and oversized PNG allocations.
- Bound dashed-line rendering and clip work to the viewport.
- Enforce Android 16 KB ELF alignment and verify release packages.

See [resource limits](docs/EXPANSION_RESOURCE_LIMITS.md) for scope and limitations.

### Performance

- Cache glyph masks and emoji rasters; share font buffers and load fallback
  fonts on demand.
- Reuse built-in macros and delimiter measurements; reduce allocations in
  PNG encoding and SVG serialization.

Latest comparison with v0.1.14 (lower is better):

| Metric | v0.1.14 | Unreleased | Change |
|---|---:|---:|---:|
| PNG render | 717 μs | 339 μs | −53% |
| SVG standalone | 938 μs | 403 μs | −57% |
| End-to-end PNG | 1,030 μs | 506 μs | −51% |
| Peak RSS | 389.5 MiB | 218.6 MiB | −44% |

Measured on x86_64 macOS with Rust 1.95.0 and matching dependencies: 100 formulas
in release mode, one warmup and three measured renders per formula, median of
five alternating runs. RSS excludes compilation. These are warmed-render
results; cold renders and continually changing glyph positions may differ.
