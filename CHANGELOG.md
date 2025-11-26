# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.3] - 2024-11-26

### Changed

- **Removed all emojis and unicode symbols**: Replaced all unicode symbols (arrows, checkmarks, etc.) with plain ASCII characters
  - Replaced `->` with `->`
  - Replaced `<-` with `<-`
  - Replaced `[x]` with `[x]`
  - All documentation now uses only ASCII characters for maximum compatibility

---

## [0.3.2] - 2024-11-26

### Changed

- **Simplified README**: Reduced README from 419 lines to 143 lines (66% reduction) for better readability
  - Quick start example moved to the top
  - Removed verbose explanations and duplicate content
  - Detailed documentation still available in `docs/` folder
  - Added links to comprehensive docs for users who need more details

---

## [0.3.1] - 2024-11-26

### Fixed

**Critical Bug Fix: Graph Name Mismatch**

- **Issue**: RSP-QL queries transform `WINDOW ex:w1 { ?s ?p ?o }` to `GRAPH ex:w1 { ?s ?p ?o }`, but quads were stored with `graph_name: DefaultGraph`, causing queries to return no results
- **Fix**: Quads are now automatically assigned to the window's graph name when added to the window
- **Impact**: Query results now work correctly - this fixes a critical bug where WINDOW clauses would not match any quads
- **File**: `src/windowing/csparql_window.rs` - Modified `add()` method to rewrite quad graph names

### Added

- **Test**: `test_window_graph_names` - Verifies that quads are correctly assigned to window graphs and query results are returned

### Technical Details

When a quad is added to a window via `CSPARQLWindow::add()`, it is now rewritten to use the window's name as its graph:

```rust
let quad_with_window_graph = Quad::new(
    quad.subject.clone(),
    quad.predicate.clone(),
    quad.object.clone(),
    GraphName::NamedNode(NamedNode::new(&self.name).unwrap()),
);
```

This ensures that when the SPARQL query looks for quads in `GRAPH ex:w1`, it finds them.

---

## [0.3.0] - 2024-11-26

### Added

#### High Priority Features
- **Cloneable RDFStream**: `RDFStream` now implements `Clone`, eliminating lifetime issues when storing stream references
- **Stream Clone Return**: `get_stream()` now returns `Option<RDFStream>` instead of `Option<&RDFStream>` for easier API usage
- **close_stream() Method**: New convenience method `RSPEngine::close_stream(uri, timestamp)` to trigger closure of all open windows and emit final results
- **Comprehensive Documentation**: Added extensive documentation explaining when results are emitted and the timestamp-driven nature of window closure

#### Medium Priority Features  
- **Window State Inspection**: New methods for debugging window behavior:
  - `CSPARQLWindow::get_active_window_count()` - Returns number of currently active windows
  - `CSPARQLWindow::get_active_window_ranges()` - Returns time ranges of all active windows

#### Low Priority Features
- **Debug Logging Control**: New configurable debug mode for windows:
  - `CSPARQLWindow::debug_mode` field
  - `CSPARQLWindow::set_debug_mode(enabled)` method
  - Converted all debug `println!` to conditional `eprintln!` statements

#### Documentation & Examples
- **STREAMING_IMPROVEMENTS.md**: Detailed documentation of all streaming-first API improvements
- **WINDOW_SEMANTICS_FAQ.md**: Comprehensive FAQ explaining timestamp-driven window closure
- **examples/streaming_lifecycle.rs**: Complete example demonstrating streaming lifecycle and new features
- Updated `lib.rs` with detailed inline documentation
- Updated `README.md` with clearer examples and explanations

#### Testing
- **tests/test_new_api.rs**: New test suite covering all new features:
  - `test_stream_is_cloneable`
  - `test_get_stream_returns_clone`
  - `test_close_stream`
  - `test_window_inspection_methods`
  - `test_debug_mode_toggle`

### Changed
- Debug output now uses `eprintln!` instead of `println!` for better stderr handling
- Debug output is now conditional (controlled by `debug_mode` field) instead of always-on
- Improved clarity in all documentation about timestamp-driven vs time-driven window semantics

### Fixed
- No breaking changes - all existing code continues to work

### Performance
- Zero-cost abstractions: Clone only increments channel sender reference count
- Window inspection methods are O(n) where n = number of active windows (typically < 10)
- Debug mode has zero overhead when disabled

### Migration Guide

#### For Existing Users
The API is backwards compatible. The only change is that `get_stream()` now returns a clone instead of a reference, but this works seamlessly with existing code.

**Optional improvements you can make:**

```rust
// Before (still works)
let stream = rsp_engine.get_stream("uri").unwrap();
stream.add_quads(vec![quad], 1000)?;

// After (recommended - add close_stream)
let stream = rsp_engine.get_stream("uri").unwrap();
stream.add_quads(vec![quad], 1000)?;
rsp_engine.close_stream("uri", 20000)?; // Emit final results
```

#### For New Users
See the comprehensive documentation in:
- `examples/streaming_lifecycle.rs` - Complete working example
- `WINDOW_SEMANTICS_FAQ.md` - Answers to common questions
- `STREAMING_IMPROVEMENTS.md` - Detailed feature documentation

### Key Concepts Clarified

**Timestamp-Driven Window Closure**: Windows close based on EVENT TIMESTAMPS (the parameter you pass to `add_quads()`), NOT wall-clock time. You can add all events instantly - the system only cares about the timestamp parameter.

**Result Emission**: Results are emitted when windows CLOSE, which happens when a new event arrives with timestamp > window end time. Always call `close_stream()` after your last event to emit final results.

---

## [0.2.1] - Previous Release

Previous version features and changes.

---

## [0.2.0] - Previous Release

Initial stable release with core RSP-QL functionality.

[0.3.3]: https://github.com/argahsuknesib/rsp-rs/compare/v0.3.2...v0.3.3
[0.3.2]: https://github.com/argahsuknesib/rsp-rs/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/argahsuknesib/rsp-rs/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/argahsuknesib/rsp-rs/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/argahsuknesib/rsp-rs/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/argahsuknesib/rsp-rs/releases/tag/v0.2.0