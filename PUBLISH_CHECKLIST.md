# Pre-Publication Checklist for crates.io

## Version Information
- [x] Version bumped to 0.3.5 in Cargo.toml
- [x] Version matches in all documentation
- [x] CHANGELOG.md updated with v0.3.5 entry
- [x] All new features documented in CHANGELOG

## Code Quality
- [x] All tests pass (30+ tests including 5 new large timestamp tests)
- [x] No compiler errors
- [x] Compiler warnings reviewed (only 1 minor lifetime warning, not critical)
- [x] No clippy warnings beyond acceptable ones
- [x] Code follows Rust conventions and best practices

## Documentation
- [x] README.md updated and complete
- [x] CHANGELOG.md comprehensive with all changes documented
- [x] docs/LARGE_TIMESTAMP_FIX.md - detailed technical documentation
- [x] examples/large_timestamps.rs - working example
- [x] Inline code comments appropriate and clear
- [x] Library documentation complete (lib.rs)

## Emoji and Unicode Removal
- [x] All emojis removed (✓, ✅, ❌, etc.)
- [x] All unicode arrows replaced (→ becomes ->, ← becomes <-)
- [x] All unicode tree characters replaced (├─, └─ become [1], [2], etc.)
- [x] All unicode symbols removed (µ becomes u)
- [x] Verified zero non-ASCII characters in documentation
- [x] Pure ASCII encoding throughout

## Cargo.toml Configuration
- [x] name = "rsp-rs" (correct, already on crates.io)
- [x] version = "0.3.5" (updated)
- [x] edition = "2021" (fixed from invalid "2024")
- [x] authors field populated
- [x] description present and descriptive
- [x] license = "MIT" (valid SPDX identifier)
- [x] repository URL points to GitHub
- [x] homepage URL valid
- [x] documentation URL valid
- [x] keywords relevant and appropriate (rdf, sparql, stream-processing, rsp-ql, real-time)
- [x] categories valid (database, asynchronous, data-structures)
- [x] All dependencies have valid versions
- [x] No dev-dependencies will be included in published package

## Dependency Review
- [x] oxigraph 0.5 - well-maintained, stable
- [x] regex 1 - standard, stable
- [x] dev-dependencies only used in tests/benchmarks
- [x] No unnecessary dependencies
- [x] All dependencies available on crates.io

## Build and Test Verification
- [x] cargo build succeeds
- [x] cargo build --release succeeds
- [x] cargo test passes all tests
- [x] cargo test --lib passes
- [x] cargo test --test large_timestamp_test passes (5/5 tests)
- [x] Integration tests pass (12/12 tests)
- [x] No warnings that would prevent publication

## Files and Structure
- [x] src/ directory properly organized
- [x] src/lib.rs exports public API correctly
- [x] All public APIs documented
- [x] examples/ directory contains working examples
- [x] tests/ directory has comprehensive test coverage
- [x] benches/ directory for performance testing
- [x] docs/ directory with additional documentation
- [x] LICENSE.md file present (MIT)
- [x] CHANGELOG.md properly formatted
- [x] README.md complete and helpful

## Breaking Changes
- [x] No breaking changes to public API
- [x] Bug fix to internal window calculation (not public API)
- [x] Existing code continues to work
- [x] v0.3.4 code works with v0.3.5

## New Features
- [x] Large timestamp support now works correctly
- [x] Comprehensive test coverage for new functionality
- [x] Example demonstrating new capability
- [x] Documentation explaining the fix and improvements

## Metadata
- [x] Author email is valid (mailkushbisen@gmail.com)
- [x] Repository is public and accessible
- [x] License matches license file
- [x] All required fields in Cargo.toml filled

## Pre-Publish Dry Run
To simulate publication, run:
```bash
cargo publish --dry-run
```

This will:
- Verify all files are included
- Check package contents
- Validate all metadata
- Ensure no errors would occur

## Final Checks Before Publishing
- [ ] Review generated documentation on docs.rs
- [ ] Verify all examples work
- [ ] Confirm package downloads correctly
- [ ] Check visibility on crates.io
- [ ] Update any external references to point to new version

## Publication Steps
1. Ensure all checks above are complete
2. Run: `cargo publish --dry-run` (verify no errors)
3. Run: `cargo publish` (publish to crates.io)
4. Wait for indexing (usually < 1 minute)
5. Visit https://crates.io/crates/rsp-rs to verify
6. Tag release in git: `git tag -a v0.3.5 -m "Version 0.3.5: Large timestamp precision fix"`
7. Push tag: `git push origin v0.3.5`

## Post-Publication
- [ ] Update project website/blog if applicable
- [ ] Announce release on social media/forums
- [ ] Update any downstream projects
- [ ] Monitor for issues/feedback

## Notes
- This version fixes a critical bug affecting real-world usage with Unix timestamps
- No breaking changes to the public API
- Backward compatible with v0.3.4 and earlier
- Ready for production use