# Streaming-First API Improvements for rsp-rs

This document describes the improvements made to rsp-rs to make it more streaming-friendly and easier to integrate with applications like Janus.

## Summary of Changes

All requested features have been implemented:

- [x] **#1**: Make `RDFStream` cloneable (High Priority)
- [x] **#2**: Expose window state inspection methods (Medium Priority)
- [x] **#3**: Improve result emission documentation (High Priority)
- [x] **#4**: Add debug logging control (Low Priority)
- [x] **#5**: Add convenience method for sentinel events (Medium Priority)
- [x] **#6**: Return stream clone instead of reference (High Priority)

## Detailed Changes

### 1. RDFStream is Now Cloneable (High Priority)

**File**: `src/engine/rsp_engine.rs`

```rust
#[derive(Clone)]
pub struct RDFStream {
    pub name: String,
    pub(crate) window_sender: mpsc::Sender<(QuadContainer, String)>,
}
```

**Why**: The internal `mpsc::Sender` is already cloneable, so this is safe for multi-threaded streaming.

**Impact**: 
- No more lifetime issues when storing stream references
- Streams can be passed between threads easily
- Cleaner API for Janus and other integrations

**Example**:
```rust
let stream = rsp_engine.get_stream("https://rsp.rs/stream1").unwrap();
let stream_clone = stream.clone(); // Works!

// Both can be used independently
stream.add_quads(vec![quad1], 1000)?;
stream_clone.add_quads(vec![quad2], 2000)?;
```

---

### 2. get_stream() Returns Clone Instead of Reference (High Priority)

**File**: `src/engine/rsp_engine.rs`

```rust
/// Get a stream by name (returns a clone for easier usage)
pub fn get_stream(&self, stream_name: &str) -> Option<RDFStream> {
    self.streams.get(stream_name).cloned()
}
```

**Why**: Eliminates lifetime complexity. Since `RDFStream` just wraps a channel sender (cheap to clone), this is efficient.

**Impact**:
- Can store the returned stream without borrowing issues
- Simplifies Janus integration significantly
- No need for complex lifetime annotations

**Before**:
```rust
// Had to call get_stream() every time or fight with lifetimes
rsp_engine.get_stream("uri").unwrap().add_quads(...)?;
rsp_engine.get_stream("uri").unwrap().add_quads(...)?;
```

**After**:
```rust
// Get once, store, and reuse
let stream = rsp_engine.get_stream("uri").unwrap();
stream.add_quads(...)?;
stream.add_quads(...)?;
```

---

### 3. close_stream() Convenience Method (Medium/High Priority)

**File**: `src/engine/rsp_engine.rs`

```rust
/// Add a sentinel event to trigger closure of all open windows
/// This should be called when the stream ends to emit final results
pub fn close_stream(&self, stream_uri: &str, final_timestamp: i64) -> Result<(), String>
```

**Why**: Explicit API for "I'm done sending events, please emit remaining results"

**Impact**: Makes the streaming model crystal clear - developers explicitly signal stream end.

**Example**:
```rust
// Add your events with TIMESTAMPS (not wall-clock time!)
// You could add all these instantly - the system only cares about the timestamp parameter
for i in 0..10 {
    stream.add_quads(vec![quad], i * 1000)?;  // timestamp = 0, 1000, 2000, ... 9000
}

// IMPORTANT: Close the stream to get final results
// This adds a sentinel event with timestamp=20000 to trigger remaining window closures
rsp_engine.close_stream("https://rsp.rs/stream1", 20000)?;
```

**Without `close_stream()`**: If your last event has timestamp=7000, windows that extend beyond that won't close and emit results.

**With `close_stream()`**: Adds a sentinel event with a high timestamp, closing all remaining windows and emitting their results.

---

### 4. Window State Inspection Methods (Medium Priority)

**File**: `src/windowing/csparql_window.rs`

```rust
/// Get the current number of active windows
pub fn get_active_window_count(&self) -> usize

/// Get the timestamp range of active windows
pub fn get_active_window_ranges(&self) -> Vec<(i64, i64)>
```

**Why**: Essential for debugging streaming behavior and understanding when windows will naturally close.

**Impact**: Developers can inspect window state without invasive logging.

**Example**:
```rust
if let Some(window) = rsp_engine.get_window("window_name") {
    let window_lock = window.lock().unwrap();
    
    println!("Active windows: {}", window_lock.get_active_window_count());
    
    for (start, end) in window_lock.get_active_window_ranges() {
        println!("Window: [{}, {})", start, end);
    }
}
```

**Use Cases**:
- Debug why results aren't being emitted
- Understand window lifecycle in real-time
- Verify window configuration is working as expected

---

### 5. Debug Logging Control (Low Priority)

**File**: `src/windowing/csparql_window.rs`

```rust
pub struct CSPARQLWindow {
    // ... existing fields
    pub debug_mode: bool,
}

/// Enable or disable debug mode for verbose logging
pub fn set_debug_mode(&mut self, enabled: bool)
```

**Why**: Previous `println!` debug statements were always on. Now they're conditional.

**Impact**: 
- Clean output for production use
- Verbose output for debugging when needed
- Better control over logging noise

**Example**:
```rust
if let Some(window) = rsp_engine.get_window("window_name") {
    let mut window_lock = window.lock().unwrap();
    window_lock.set_debug_mode(true); // Enable verbose logging
}
```

**Debug Output Includes**:
- When elements are received
- Which windows are processing elements
- When windows are scheduled for eviction
- When windows emit results
- Window lifecycle events

---

### 6. Comprehensive Documentation (High Priority)

**Files**: `src/lib.rs`, `README.md`

Added extensive documentation explaining:

#### When Are Results Emitted?

**Key Concept**: Results are emitted when windows **close**, not when events arrive.

**Critical Understanding**: Window closure is driven by **event timestamps**, NOT wall-clock time!
The system doesn't use timers - it only processes events when you call `add_quads()`.

Windows close when:
1. A new event arrives with a **timestamp** > window end time
2. The window's STEP interval is reached **based on event timestamps**

#### Example Timeline (RANGE 10000, STEP 2000):

**Important**: The timeline below shows EVENT TIMESTAMPS (not wall-clock time). You could add all these events in 1 millisecond of real time - the system only cares about the timestamp parameter!

```text
Event with timestamp=0:     Added to window [-8000, 2000)
Event with timestamp=500:   More events added
Event with timestamp=1000:  More events added
Event with timestamp=1500:  More events added
                            WARNING: NO RESULTS YET - windows still open

Event with timestamp=2000:  New event arrives
                            → Window [-8000, 2000) CLOSES
                            → ✓ FIRST RESULT EMITTED

Event with timestamp=4000:  New event arrives
                            → Window [-6000, 4000) CLOSES
                            → ✓ SECOND RESULT EMITTED

Event with timestamp=6000:  New event arrives
                            → Window [-4000, 6000) CLOSES
                            → ✓ THIRD RESULT EMITTED

Event with timestamp=7000:  Last event (no more events after this)
                            WARNING: NO MORE RESULTS - no event to trigger closure

Solution: Call close_stream("uri", 20000) to emit final results
         (This adds a sentinel event with timestamp=20000)
```

#### Common Pitfall

**Problem**: "I added 10 events but got no results!"

**Explanation**: If your last event has timestamp=1500 and your STEP is 2000, no window has closed yet. You need an event with timestamp=2000 or later to trigger the first window closure.

**Key Point**: It doesn't matter WHEN you add the events in real time. What matters is the TIMESTAMP parameter you pass to `add_quads()`. You could add all 10 events instantly, but if their timestamps don't trigger window closure, you won't get results.

**Solution**: Always call `close_stream()` after your last event to add a sentinel event with a high timestamp.

---

## API Documentation Updates

### RSPEngine

**New/Modified Methods**:
- `get_stream(name: &str) -> Option<RDFStream>` - Returns clone (was reference)
- `close_stream(uri: &str, timestamp: i64) -> Result<(), String>` - NEW
- `get_window(name: &str) -> Option<Arc<Mutex<CSPARQLWindow>>>` - For inspection

### RDFStream

**New Capability**:
- `Clone` trait implemented - streams can be cloned and stored

### CSPARQLWindow

**New Methods**:
- `get_active_window_count() -> usize` - NEW
- `get_active_window_ranges() -> Vec<(i64, i64)>` - NEW
- `set_debug_mode(enabled: bool)` - NEW

---

## Testing

All new features are tested in `tests/test_new_api.rs`:

- [x] `test_stream_is_cloneable` - Verifies Clone implementation
- [x] `test_get_stream_returns_clone` - Verifies clone semantics
- [x] `test_close_stream` - Verifies close_stream API
- [x] `test_window_inspection_methods` - Verifies state inspection
- [x] `test_debug_mode_toggle` - Verifies debug mode control

All existing tests pass - no breaking changes.

---

## Example Usage

See `examples/streaming_lifecycle.rs` for a comprehensive example demonstrating:

1. Creating and initializing an RSP engine
2. Getting and storing a cloned stream
3. Adding events with timestamps
4. Inspecting window state
5. Using `close_stream()` to emit final results
6. Enabling debug mode for troubleshooting

Run with:
```bash
cargo run --example streaming_lifecycle
```

---

## Migration Guide

### For Existing Code

**Before** (old API):
```rust
let stream = rsp_engine.get_stream("uri").unwrap();
stream.add_quads(vec![quad], 1000)?;
// Stream reference tied to rsp_engine lifetime
```

**After** (new API):
```rust
let stream = rsp_engine.get_stream("uri").unwrap();
stream.add_quads(vec![quad], 1000)?;
// Stream is cloned - can be stored independently

// Don't forget to close the stream!
rsp_engine.close_stream("uri", 20000)?;
```

### For Janus Integration

The new API makes Janus integration much cleaner:

```rust
// Store the stream (no lifetime issues!)
struct MyHandler {
    stream: RDFStream,
}

impl MyHandler {
    fn new(engine: &RSPEngine) -> Self {
        Self {
            stream: engine.get_stream("uri").unwrap()
        }
    }
    
    fn handle_event(&self, quad: Quad, timestamp: i64) {
        self.stream.add_quads(vec![quad], timestamp).unwrap();
    }
    
    fn finish(&self, engine: &RSPEngine) {
        engine.close_stream("uri", i64::MAX).unwrap();
    }
}
```

---

## Performance Impact

All changes are zero-cost or minimal-cost:

- **Clone**: Only clones a channel sender (single atomic reference count increment)
- **Inspection methods**: O(n) where n = number of active windows (typically < 10)
- **Debug mode**: Zero cost when disabled (simple boolean check)
- **close_stream()**: Same as adding one quad (just adds a sentinel event)

**Important Note**: The system is timestamp-driven, not time-driven. Adding 1 million events takes the same time regardless of the timestamp values you use. The timestamp parameter just determines when windows close relative to each other.

No performance regressions observed in existing benchmarks.

---

## Breaking Changes

**None**. All changes are backwards compatible:

- `get_stream()` still works the same way (just returns a clone instead of reference)
- New methods are additions, not replacements
- Debug output now requires explicit enabling (cleaner by default)

---

## Future Improvements

Potential enhancements based on this work:

1. **Result counting**: Add `get_emitted_result_count()` for debugging
2. **Batch close**: `close_all_streams(timestamp)` for multi-stream scenarios
3. **Window lifecycle events**: Callbacks for window open/close events
4. **Non-blocking result collection**: Async/await support for result receivers

---

## Summary

These changes transform rsp-rs from a reference-based API to a value-based streaming API:

[x] **Easier to use** - No lifetime fights  
[x] **More explicit** - Clear stream closure semantics  
[x] **Better debugging** - Inspection methods and controlled logging  
[x] **Well documented** - Crystal clear window lifecycle explanation  
[x] **Production ready** - Clean output, optional verbosity

**The minimal changes needed for Janus**: Just #1, #3, and #6 eliminate all API friction.

All other improvements are bonus features that make debugging and understanding the system easier.