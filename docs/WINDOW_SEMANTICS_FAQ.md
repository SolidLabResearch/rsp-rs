# Window Semantics FAQ

## Understanding Timestamp-Driven Window Closure

This document answers the most common questions about how windows work in rsp-rs.

---

## Q: When do windows close and emit results?

**A:** Windows close when you add an event whose **timestamp** exceeds the window's end time.

**Key Point:** It's the **timestamp parameter** you pass to `add_quads()` that matters, NOT:
- Wall-clock time
- How fast you add events
- When you call the method
- System time

---

## Q: What is "timestamp-driven" vs "time-driven"?

**Timestamp-driven (what rsp-rs uses):**
```rust
// You control window closure by the timestamps you provide
stream.add_quads(vec![quad1], 0)?;      // timestamp = 0
stream.add_quads(vec![quad2], 1000)?;   // timestamp = 1000
stream.add_quads(vec![quad3], 2000)?;   // timestamp = 2000 → triggers closure!
```

**Time-driven (NOT what rsp-rs uses):**
```rust
// System would use wall-clock time (rsp-rs doesn't do this!)
stream.add_quads(vec![quad1])?;  // Uses system clock
thread::sleep(Duration::from_secs(1));
stream.add_quads(vec![quad2])?;  // Uses system clock again
```

rsp-rs is **timestamp-driven** - you explicitly provide timestamps.

---

## Q: Can I add all events instantly?

**A:** Yes! You can add millions of events in milliseconds.

```rust
// Add all events instantly - no sleep needed
for i in 0..1000 {
    stream.add_quads(vec![quad], i * 1000)?;  // timestamps: 0, 1000, 2000, ...
}
// Windows will still close based on timestamps (2000, 4000, 6000, ...)
```

The `thread::sleep()` calls you see in examples are just to make the output readable for humans. The system doesn't need them.

---

## Q: Why am I not getting any results?

**Common causes:**

### Cause 1: Last event didn't trigger window closure
```rust
// RANGE 10000, STEP 2000
stream.add_quads(vec![quad1], 0)?;      // Added to windows
stream.add_quads(vec![quad2], 1000)?;   // Added to windows
stream.add_quads(vec![quad3], 1500)?;   // Added to windows
// ❌ NO RESULTS - no event with timestamp >= 2000 to close the window!
```

**Solution:** Add a sentinel event or use `close_stream()`:
```rust
stream.add_quads(vec![quad1], 0)?;
stream.add_quads(vec![quad2], 1000)?;
stream.add_quads(vec![quad3], 1500)?;
rsp_engine.close_stream("stream_uri", 20000)?;  // ✅ Triggers closure
```

### Cause 2: Not waiting for result processing
```rust
let result_receiver = rsp_engine.start_processing();
stream.add_quads(vec![quad], 2000)?;
// ❌ Results might still be in the channel!
```

**Solution:** Give time for processing:
```rust
let result_receiver = rsp_engine.start_processing();
stream.add_quads(vec![quad], 2000)?;
thread::sleep(Duration::from_millis(100)); // Wait for processing
while let Ok(result) = result_receiver.recv_timeout(Duration::from_millis(100)) {
    println!("Result: {:?}", result);
}
```

---

## Q: What does RANGE and STEP mean?

**RANGE:** How much historical data the window contains (in milliseconds)
**STEP:** How often windows slide (in milliseconds)

```
RANGE 10000 STEP 2000
```

**Visual timeline (based on EVENT TIMESTAMPS):**

```
Event timestamp=0:
  ├─ Window [-10000, 0)
  └─ Window [-8000, 2000)   ← Will close when timestamp >= 2000

Event timestamp=2000:
  ├─ Window [-8000, 2000) CLOSES → ✅ EMIT RESULTS
  ├─ Window [-6000, 4000)   ← Will close when timestamp >= 4000
  └─ Window [-4000, 6000)   ← Will close when timestamp >= 6000

Event timestamp=4000:
  ├─ Window [-6000, 4000) CLOSES → ✅ EMIT RESULTS
  ├─ Window [-4000, 6000)
  └─ Window [-2000, 8000)   ← Will close when timestamp >= 8000
```

---

## Q: Does the system use timers or background threads for window closure?

**A:** No! Window closure is **event-driven**, not timer-driven.

**What happens:**
1. You call `stream.add_quads(vec![quad], timestamp)`
2. System checks: "Does this timestamp close any open windows?"
3. If yes → emit results for those windows
4. Add event to appropriate windows

**What does NOT happen:**
- ❌ No background timer checking wall-clock time
- ❌ No automatic window closure after X seconds
- ❌ No polling or periodic checks

---

## Q: Why do I need to call close_stream()?

**A:** Because the last event you add doesn't trigger window closure.

```rust
// RANGE 10000, STEP 2000
stream.add_quads(vec![quad], 5000)?;   // Last event, timestamp=5000

// Windows [-5000, 5000), [-3000, 7000), [-1000, 9000) are still OPEN
// They won't close until an event with timestamp >= 7000, 9000, 11000 arrives
```

`close_stream()` adds a sentinel event with a high timestamp to close all remaining windows:

```rust
rsp_engine.close_stream("stream_uri", 100000)?;
// Adds sentinel event with timestamp=100000 → closes ALL open windows
```

---

## Q: What happens if I add events out of order?

**A:** The system logs "OUT OF ORDER NOT HANDLED" and behavior is undefined.

```rust
stream.add_quads(vec![quad1], 5000)?;   // timestamp=5000
stream.add_quads(vec![quad2], 3000)?;   // ❌ timestamp=3000 < 5000 (out of order)
```

**Best Practice:** Always add events in increasing timestamp order.

---

## Q: Can I use negative timestamps?

**A:** Yes! Timestamps are just i64 values. Negative timestamps work fine.

```rust
stream.add_quads(vec![quad1], -5000)?;  // Valid
stream.add_quads(vec![quad2], -3000)?;  // Valid
stream.add_quads(vec![quad3], 0)?;      // Triggers closure of early windows
```

---

## Q: How do I debug window behavior?

**A:** Use the inspection methods:

```rust
if let Some(window) = rsp_engine.get_window("window_name") {
    let window_lock = window.lock().unwrap();
    
    // How many windows are currently open?
    println!("Active windows: {}", window_lock.get_active_window_count());
    
    // What are their time ranges?
    for (start, end) in window_lock.get_active_window_ranges() {
        println!("Window [{}, {}) is open, will close when timestamp >= {}", 
                 start, end, end);
    }
    
    // Enable verbose logging
    window_lock.set_debug_mode(true);
}
```

---

## Q: What's the difference between window start/end time and event timestamps?

**Window times** are calculated by the system:
```
RANGE 10000, STEP 2000
Event with timestamp=5000 creates windows:
  [-5000, 5000)
  [-3000, 7000)
  [-1000, 9000)
  [1000, 11000)
  etc.
```

**Event timestamps** are what YOU provide:
```rust
stream.add_quads(vec![quad], YOUR_TIMESTAMP)?;
```

When YOUR_TIMESTAMP >= window.end, that window closes.

---

## Q: Why do windows have negative start times?

**A:** Because windows look backward in time to capture historical data.

```
RANGE 10000, STEP 2000
Event arrives with timestamp=5000

Windows created:
  [-5000, 5000)   ← Looks back 10000ms from 5000
  [-3000, 7000)   ← Looks back 10000ms from 7000
  [-1000, 9000)   ← Looks back 10000ms from 9000
```

This is normal! It means "capture events from the last 10 seconds (RANGE)" relative to the window's end time.

---

## Q: Can I have real-time streaming with wall-clock time?

**A:** Yes, just use wall-clock time as your timestamp:

```rust
use std::time::{SystemTime, UNIX_EPOCH};

let timestamp = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap()
    .as_millis() as i64;

stream.add_quads(vec![quad], timestamp)?;
```

But remember: the system doesn't automatically close windows based on wall-clock time. You still need events with future timestamps to trigger closure.

---

## Q: What's a "sentinel event"?

**A:** A dummy event with a high timestamp used to trigger window closure.

```rust
// Manual sentinel
let sentinel = Quad::new(
    NamedNode::new("urn:sentinel")?,
    NamedNode::new("urn:type")?,
    Literal::new_simple_literal("end"),
    GraphName::DefaultGraph,
);
stream.add_quads(vec![sentinel], i64::MAX)?;

// Or use the convenience method
rsp_engine.close_stream("stream_uri", i64::MAX)?;
```

The sentinel event itself isn't important - it's the **timestamp** that triggers window closure.

---

## Quick Reference

| Concept | What it is | Example |
|---------|-----------|---------|
| **Event timestamp** | The timestamp YOU provide | `add_quads(vec![quad], 5000)` |
| **Window RANGE** | How much history to keep | `10000` = 10 seconds of data |
| **Window STEP** | How often windows slide | `2000` = new window every 2 seconds |
| **Window closure** | When timestamp >= window.end | Event at 2000 closes window [-8000, 2000) |
| **Result emission** | Happens when window closes | Not when event arrives! |
| **Sentinel event** | Event with high timestamp | Triggers closure of remaining windows |
| **close_stream()** | Convenience method | Adds sentinel event automatically |

---

## TL;DR

1. **Windows close based on EVENT TIMESTAMPS, not wall-clock time**
2. **You can add all events instantly - only timestamps matter**
3. **Results emit when windows CLOSE, not when events arrive**
4. **Always call `close_stream()` to get final results**
5. **Use inspection methods to debug window state**
