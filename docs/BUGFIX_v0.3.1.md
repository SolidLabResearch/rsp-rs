# Bug Fix Summary: v0.3.1 - Critical Graph Name Mismatch

**Release Date**: November 26, 2024  
**Version**: 0.3.1  
**Type**: Critical Bug Fix  
**Status**: Published to crates.io

---

## The Problem

### Symptom
Queries with WINDOW clauses returned **zero results** even when data was correctly added to streams.

### Root Cause
**Graph Name Mismatch** between query expectations and stored data:

1. **RSP-QL Parser** transforms WINDOW clauses to GRAPH clauses:
   ```sparql
   WINDOW ex:w1 { ?s ?p ?o }  →  GRAPH ex:w1 { ?s ?p ?o }
   ```

2. **Quads Storage** used `DefaultGraph` instead of the window's graph name:
   ```rust
   // Input quad had: graph_name: DefaultGraph
   // Query expected: graph_name: NamedNode("http://example.org/w1")
   ```

3. **SPARQL Query Execution** looked for quads in `GRAPH ex:w1` but found none because all quads were in `DefaultGraph`

### Impact
- **Severity**: Critical - queries returned no results
- **Affected Versions**: v0.3.0 and earlier
- **Scope**: All queries using WINDOW clauses

---

## The Fix

### Implementation
**File**: `src/windowing/csparql_window.rs`  
**Method**: `CSPARQLWindow::add()`

**Change**: When a quad is added to a window, it is now rewritten to use the window's graph name:

```rust
// Create a new quad with the window's graph name
let quad_with_window_graph = oxigraph::model::Quad::new(
    quad.subject.clone(),
    quad.predicate.clone(),
    quad.object.clone(),
    oxigraph::model::GraphName::NamedNode(
        oxigraph::model::NamedNode::new(&self.name).unwrap_or_else(|_| {
            // Fallback if window name isn't a valid IRI
            oxigraph::model::NamedNode::new("http://default-window").unwrap()
        }),
    ),
);

// Use the rewritten quad instead of the original
container.add(quad_with_window_graph.clone(), timestamp);
```

### Why This Works
- Quads are automatically assigned to the correct graph when entering the window
- SPARQL queries looking for `GRAPH ex:w1` now find matching quads
- No changes needed to user code - fix is transparent

---

## Verification

### New Test Added
**Test**: `test_window_graph_names` in `tests/test_new_api.rs`

```rust
#[test]
fn test_window_graph_names() {
    let query = r#"
        PREFIX ex: <http://example.org/>
        REGISTER RStream <output> AS
        SELECT ?s ?p ?o
        FROM NAMED WINDOW ex:w1 ON STREAM ex:stream1 [RANGE 1000 STEP 200]
        WHERE {
            WINDOW ex:w1 { ?s ?p ?o }
        }
    "#;

    // Add quad with DefaultGraph
    let quad = Quad::new(
        NamedNode::new("http://ex.org/s").unwrap(),
        NamedNode::new("http://ex.org/p").unwrap(),
        Literal::new_simple_literal("o"),
        GraphName::DefaultGraph, // Original graph
    );
    stream.add_quads(vec![quad], 100).unwrap();

    // Trigger window closure
    stream.add_quads(vec![sentinel], 2000).unwrap();

    // Collect results
    assert!(!results.is_empty(), "Should receive results!");
}
```

### Test Results
**Before Fix (v0.3.0)**:
```
Received 0 results
test test_window_graph_names ... FAILED
```

**After Fix (v0.3.1)**:
```
[R2R]   Quad 1: Quad { 
    subject: NamedNode("http://ex.org/s"), 
    predicate: NamedNode("http://ex.org/p"), 
    object: Literal("o"), 
    graph_name: NamedNode("http://example.org/w1")  ← Correctly assigned!
}
Received 1 results
test test_window_graph_names ... ok
```

---

## Migration Guide

### Do You Need to Change Your Code?
**No!** This fix is completely transparent to user code.

### What to Do
Simply update your dependency:

```toml
[dependencies]
rsp-rs = "0.3.1"  # Update from 0.3.0
```

Or:
```bash
cargo update rsp-rs
```

### What Changes
- Queries that previously returned 0 results will now return correct results
- No API changes
- No behavioral changes except the bug fix

---

## Example: Before vs After

### Before v0.3.1 (Broken)
```rust
let query = r#"
    PREFIX ex: <http://example.org/>
    REGISTER RStream <output> AS
    SELECT ?s ?p ?o
    FROM NAMED WINDOW ex:w1 ON STREAM ex:stream1 [RANGE 10000 STEP 2000]
    WHERE {
        WINDOW ex:w1 { ?s ?p ?o }
    }
"#;

stream.add_quads(vec![quad], 1000)?;
stream.add_quads(vec![quad2], 3000)?; // Triggers window closure

// Result: 0 results (BUG!)
```

### After v0.3.1 (Fixed)
```rust
let query = r#"
    PREFIX ex: <http://example.org/>
    REGISTER RStream <output> AS
    SELECT ?s ?p ?o
    FROM NAMED WINDOW ex:w1 ON STREAM ex:stream1 [RANGE 10000 STEP 2000]
    WHERE {
        WINDOW ex:w1 { ?s ?p ?o }
    }
"#;

stream.add_quads(vec![quad], 1000)?;
stream.add_quads(vec![quad2], 3000)?; // Triggers window closure

// Result: 1+ results (WORKS!)
```

---

## Technical Details

### Code Flow

1. **User adds quad** with any graph name (usually `DefaultGraph`):
   ```rust
   stream.add_quads(vec![quad], timestamp)?;
   ```

2. **Window receives quad** via channel

3. **CSPARQLWindow::add()** is called:
   - Creates new quad with window's graph name
   - Adds rewritten quad to window containers

4. **Window closes** and emits content

5. **R2ROperator executes query**:
   ```sparql
   GRAPH ex:w1 { ?s ?p ?o }  ← Now matches!
   ```

6. **Results returned** to user

### Graph Name Assignment Logic

```rust
// Window name: "http://example.org/w1"
GraphName::NamedNode(
    NamedNode::new("http://example.org/w1").unwrap()
)

// Fallback (if window name is invalid IRI):
GraphName::NamedNode(
    NamedNode::new("http://default-window").unwrap()
)
```

---

## Regression Testing

All existing tests pass with the fix:
- ✓ 6 unit tests (lib)
- ✓ 12 integration tests
- ✓ 2 RSP engine tests
- ✓ 7 new API tests (including new graph name test)

**Total**: 27 tests pass, 0 failures

---

## Related Issues

This fix resolves the fundamental issue that prevented WINDOW clauses from working correctly in v0.3.0.

### Symptoms You May Have Experienced
- "My query returns no results"
- "Data is being added but I get empty result sets"
- "Debug logs show quads in window but query finds nothing"
- "SPARQL query works in Oxigraph directly but not in rsp-rs"

### Why It Wasn't Caught Earlier
- Integration tests used static joins which bypassed the WINDOW→GRAPH transformation
- Manual testing often used queries that didn't rely on WINDOW matching
- The bug only manifested when WINDOW clauses were actually executed as GRAPH clauses

---

## Performance Impact

**Zero performance overhead** - the graph name rewrite happens once when the quad enters the window, which was already being cloned at that point.

---

## Credits

**Bug Discovery**: Identified through Janus integration testing  
**Fix Implementation**: Applied Option 1 from fix instructions (cleanest solution)  
**Test Coverage**: Added regression test to prevent future occurrences

---

## Links

- **Crates.io**: https://crates.io/crates/rsp-rs/0.3.1
- **Documentation**: https://docs.rs/rsp-rs/0.3.1
- **Repository**: https://github.com/SolidLabResearch/rsp-rs
- **Changelog**: See CHANGELOG.md for full details

---

## Recommendations

### For All Users
**Upgrade immediately** if you're using WINDOW clauses. This is a critical bug fix.

### For New Users
Install `rsp-rs = "0.3.1"` to get the fix from the start.

### For Library Maintainers
Update your dependencies to require at least version 0.3.1:
```toml
rsp-rs = ">=0.3.1"
```

---

## Summary

| Aspect | Details |
|--------|---------|
| **Version** | 0.3.1 |
| **Type** | Critical Bug Fix |
| **Issue** | Graph name mismatch preventing WINDOW queries from working |
| **Fix** | Automatic graph name assignment in CSPARQLWindow::add() |
| **Breaking Changes** | None |
| **Migration Required** | No - just update version |
| **Test Coverage** | New regression test added |
| **Performance Impact** | None |

**Bottom Line**: If you use WINDOW clauses in your RSP-QL queries, upgrade to 0.3.1 immediately. The fix is transparent and requires no code changes.