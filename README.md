# RSP-RS

A high-performance RDF Stream Processing engine in Rust built on [Oxigraph](https://github.com/oxigraph/oxigraph/).

## Installation

```toml
[dependencies]
rsp-rs = "0.3.2"
```

Or:
```bash
cargo add rsp-rs
```

## Quick Start

```rust
use rsp_rs::RSPEngine;
use oxigraph::model::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Define RSP-QL query
    let query = r#"
        PREFIX ex: <https://rsp.rs/>
        REGISTER RStream <output> AS
        SELECT *
        FROM NAMED WINDOW ex:w1 ON STREAM ex:stream1 [RANGE 10000 STEP 2000]
        WHERE {
            WINDOW ex:w1 { ?s ?p ?o }
        }
    "#;

    // Initialize engine
    let mut engine = RSPEngine::new(query.to_string());
    engine.initialize()?;

    // Get stream and start processing
    let stream = engine.get_stream("https://rsp.rs/stream1").unwrap();
    let results = engine.start_processing();

    // Add data with timestamps
    let quad = Quad::new(
        NamedNode::new("https://rsp.rs/subject")?,
        NamedNode::new("https://rsp.rs/predicate")?,
        NamedNode::new("https://rsp.rs/object")?,
        GraphName::DefaultGraph,
    );
    stream.add_quads(vec![quad], 1000)?;

    // Close stream to get final results
    engine.close_stream("https://rsp.rs/stream1", 10000)?;

    Ok(())
}
```

## Key Concepts

### Window Closure & Results

Results emit when windows **close**, triggered by event **timestamps** (not wall-clock time):

```rust
stream.add_quads(vec![quad1], 0)?;     // Added to window
stream.add_quads(vec![quad2], 1000)?;  // Added to window
stream.add_quads(vec![quad3], 2000)?;  // Closes window - results emitted!
```

**Important:** Always call `close_stream()` after your last event to trigger final window closures.

### Timestamps vs Wall-Clock

The system is **timestamp-driven**:
- You can add all events instantly
- Only the `timestamp` parameter matters
- Windows close when an event's timestamp exceeds the window's end time

## Features

- **RSP-QL Support** - Full RSP-QL syntax for continuous queries
- **Sliding Windows** - Time-based windows with configurable range and step
- **SPARQL Aggregations** - COUNT, AVG, MIN, MAX, SUM with GROUP BY
- **Stream-Static Joins** - Join streaming data with static knowledge
- **Multi-threaded** - Efficient concurrent processing
- **Cloneable Streams** - No lifetime issues, easy API

## API

### RSPEngine
- `new(query)` - Create engine with RSP-QL query
- `initialize()` - Initialize windows and streams
- `start_processing()` - Start processing, returns result receiver
- `get_stream(name)` - Get stream for adding data
- `close_stream(uri, timestamp)` - Trigger final window closures
- `add_static_data(quad)` - Add static background data

### RDFStream
- `add_quads(quads, timestamp)` - Add quads with event timestamp
- Cloneable - can be stored and reused

### Debugging
```rust
let window = engine.get_window("window_name").unwrap();
let mut w = window.lock().unwrap();

println!("Active windows: {}", w.get_active_window_count());
w.set_debug_mode(true); // Enable verbose logging
```

## Performance

- **Throughput**: Up to 1.28M quads/second
- **Latency**: ~400-700µs query execution on 30s windows
- **Memory**: ~2.5KB per quad in window

Run benchmarks:
```bash
cargo bench
```

## Examples

See `examples/streaming_lifecycle.rs` and `tests/integration/` for more examples.

## Documentation

- [API Docs](https://docs.rs/rsp-rs)
- [Window Semantics FAQ](docs/WINDOW_SEMANTICS_FAQ.md)
- [Streaming Improvements](docs/STREAMING_IMPROVEMENTS.md)

## License

MIT License - Copyright Ghent University - imec

## Acknowledgments

Rust port of [RSP-JS](https://github.com/pbonte/RSP-JS/). Thanks to the original authors for their excellent work.

## Contact

[Kush Bisen](mailto:mailkushbisen@gmail.com) or create an issue on GitHub.