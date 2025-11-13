# RSP-RS

An RDF Stream Processing Engine in Rust built on top of [Oxigraph](https://github.com/oxigraph/oxigraph/) for SPARQL querying with multi-threaded stream processing.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
rsp-rs = "0.2.0"
```

Or install with cargo:

```bash
cargo add rsp-rs
```

## Usage

You can define a query using the RSP-QL syntax. An example query is shown below:

```rust
use rsp_rs::RSPEngine;

let query = r#"
    PREFIX ex: <https://rsp.rs/>
    REGISTER RStream <output> AS
    SELECT *
    FROM NAMED WINDOW ex:w1 ON STREAM ex:stream1 [RANGE 10 STEP 2]
    WHERE {
        WINDOW ex:w1 { ?s ?p ?o }
    }
"#;
```

You can then create an instance of the RSPEngine and pass the query to it:

```rust
let mut rsp_engine = RSPEngine::new(query);
```

Initialize the engine to create windows and streams:

```rust
rsp_engine.initialize()?;
```

You can add stream elements to the RSPEngine using streams. First get a stream reference:

```rust
let stream = rsp_engine.get_stream("https://rsp.rs/stream1").unwrap();
```

Then add quads with timestamps:

```rust
use oxigraph::model::*;

let quad = Quad::new(
    NamedNode::new("https://rsp.rs/test_subject_1")?,
    NamedNode::new("https://rsp.rs/test_property")?,
    NamedNode::new("https://rsp.rs/test_object")?,
    GraphName::DefaultGraph,
);

stream.add_quads(vec![quad], timestamp_value)?;
```

Here's a complete example:

```rust
use oxigraph::model::*;
use rsp_rs::RSPEngine;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let query = r#"
        PREFIX ex: <https://rsp.rs/>
        REGISTER RStream <output> AS
        SELECT *
        FROM NAMED WINDOW ex:w1 ON STREAM ex:stream1 [RANGE 10 STEP 2]
        WHERE {
            WINDOW ex:w1 { ?s ?p ?o }
        }
    "#;

    let mut rsp_engine = RSPEngine::new(query.to_string());
    rsp_engine.initialize()?;

    let stream = rsp_engine.get_stream("https://rsp.rs/stream1").unwrap();

    // Start processing and get results receiver
    let result_receiver = rsp_engine.start_processing();

    // Generate some test data
    generate_data(10, &stream);

    // Collect results
    let mut results = Vec::new();
    while let Ok(result) = result_receiver.recv() {
        println!("Received result: {}", result.bindings);
        results.push(result.bindings);
    }

    println!("Total results: {}", results.len());
    Ok(())
}

fn generate_data(num_events: usize, stream: &rsp_rs::RDFStream) {
    for i in 0..num_events {
        let quad = Quad::new(
            NamedNode::new(&format!("https://rsp.rs/test_subject_{}", i)).unwrap(),
            NamedNode::new("https://rsp.rs/test_property").unwrap(),
            NamedNode::new("https://rsp.rs/test_object").unwrap(),
            GraphName::DefaultGraph,
        );

        stream.add_quads(vec![quad], i as i64).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
```

## Features

- **RSP-QL Support**: Full RSP-QL syntax for defining continuous queries
- **Multiple Windows**: Support for multiple sliding/tumbling windows
- **Stream-Static Joins**: Join streaming data with static background knowledge
- **SPARQL Aggregations**: COUNT, AVG, MIN, MAX, SUM with GROUP BY
- **Multi-threaded Processing**: Efficient concurrent stream processing using standard Rust threads
- **Named Graphs**: Full support for RDF named graphs in queries
- **Real-time Results**: Continuous query evaluation with RStream/IStream/DStream semantics

## Testing

Run the test suite:

```bash
cargo test
```

Run integration tests specifically:

```bash
cargo test --test integration_tests
```

## Performance Benchmarks

RSP-RS is designed for high-performance stream processing. Below are benchmark results from the included test suite:

### Streaming Throughput

Processing performance for different batch sizes (quads per operation):

| Batch Size | Processing Time | Throughput |
|------------|----------------|------------|
| 100 quads  | ~78 µs         | ~1.28M quads/sec |
| 500 quads  | ~576 µs        | ~868K quads/sec  |
| 1,000 quads | ~1.07 ms      | ~935K quads/sec  |
| 5,000 quads | ~6.9 ms       | ~725K quads/sec  |

### Query Execution Performance

SPARQL query execution times on streaming data:

| Query Type | Dataset Size | Execution Time |
|------------|-------------|----------------|
| Simple SELECT | 10 quads | ~20 µs |
| Simple SELECT | 100 quads | ~87 µs |
| Simple SELECT | 1,000 quads | ~795 µs |
| Static Join | 100 quads | ~129 µs |
| Static Join | 1,000 quads | ~547 µs |
| Complex (3 patterns) | 500 quads | ~448 µs |

### End-to-End Latency (30-Second Window)

For aggregation queries with a 30-second sliding window (STEP 5 seconds):

| Query Type | Window State | Data in Window | Processing Latency |
|------------|-------------|----------------|-------------------|
| COUNT aggregation | First window (t=5s) | 5 seconds of data | ~391 µs |
| COUNT aggregation | Full window (t=30s) | 30 seconds of data | ~717 µs |
| AVG aggregation | Full window (t=30s) | 30 seconds of data | ~646 µs |

**When do you see results?**
- With a window configuration of `RANGE 30000 STEP 5000` (30s range, 5s slide):
  - **First result**: After 5 seconds (when window closes at t=5s)
    - Contains 5 seconds of data
    - Processing latency: ~391 µs
  - **Subsequent results**: Every 5 seconds (at t=10s, t=15s, t=20s, t=25s, t=30s, ...)
  - **Full window coverage**: Starting at t=30s
    - Window contains full 30 seconds of historical data
    - Processing latency: ~717 µs for COUNT, ~646 µs for AVG
  - **Latency breakdown**:
    - Window close detection: < 1 µs
    - Query execution on window data: 390-720 µs (scales with data volume)
    - Result emission: < 10 µs

**Example Timeline:**
```
t=0s:  Data streaming starts
t=5s:  First result emitted (covers t=0-5s, ~5 data points, ~391µs latency)
t=10s: Second result (covers t=0-10s, ~10 data points)
t=15s: Third result (covers t=0-15s, ~15 data points)
...
t=30s: Sixth result (covers t=0-30s, ~30 data points, ~717µs latency - full window)
t=35s: Seventh result (covers t=5-35s, ~30 data points - sliding window)
```

### Memory and CPU Utilization

Measured with a 30-second window (RANGE 30000 STEP 5000) under different data rates:

**Memory Usage (30-second window):**

| Data Rate | Total Quads Processed | Memory Delta | Processing Time |
|-----------|----------------------|--------------|----------------|
| 1 quad/sec | 35 quads | ~0.09 MB | 65.5 ms |
| 5 quads/sec | 175 quads | ~0.45 MB | 64.7 ms |
| 10 quads/sec | 350 quads | ~0.90 MB | 64.2 ms |
| 20 quads/sec | 700 quads | ~1.80 MB | 63.6 ms |

**Key Memory Insights:**
- Memory scales linearly: ~2.5 KB per quad in window
- Base overhead: ~2-5 MB for engine structures
- Window eviction keeps memory bounded
- No memory leaks detected over sustained operations

**CPU Usage (30-second window, 10 quads/sec, 350 total quads):**

| Query Type | Processing Time | Notes |
|------------|----------------|-------|
| Simple SELECT | 55.1 ms | Baseline query performance |
| COUNT aggregation | 55.0 ms | Negligible aggregation overhead |
| AVG aggregation | 55.0 ms | Similar to COUNT performance |

**Window Management Overhead:**

| Metric | Value |
|--------|-------|
| Window operations (minimal data) | ~17 µs | Scoping, eviction, reporting |
| Sustained burst throughput | 8.9 ms for 1000 quads | ~112,000 quads/sec burst |

**CPU Efficiency:**
- Aggregations add minimal overhead (~0-1% vs simple SELECT)
- Multi-threaded: 1 background thread per window
- Efficient query execution: ~55ms for 350 quads
- Burst processing: Up to 1.4M quads/second peak throughput


### Running Benchmarks

To run the benchmarks yourself:

```bash
# Run all benchmarks
cargo bench

# Run specific benchmarks
cargo bench --bench streaming_throughput    # Throughput tests
cargo bench --bench end_to_end_latency     # Window latency tests
cargo bench --bench r2r_operator           # Query execution tests
cargo bench --bench resource_utilization   # Memory & CPU tests (fast, ~2 min)

# View HTML reports
open target/criterion/report/index.html
```

**Benchmark Categories:**
- `streaming_throughput`: Measures quads/second processing rates
- `end_to_end_latency`: Time from data arrival to result availability
- `r2r_operator`: SPARQL query execution performance
- `resource_utilization`: Memory and CPU usage (30s window scenarios)
- `memory_profile` & `cpu_utilization`: Long-running tests (10-30 min, not recommended)


## API Documentation

### RSPEngine

- `new(query: String)` - Create a new RSP engine with RSP-QL query
- `initialize()` - Initialize windows and streams from the query
- `start_processing()` - Start processing and return results receiver channel
- `get_stream(name: &str)` - Get a stream by name for adding data
- `add_static_data(quad: Quad)` - Add static background knowledge

### CSPARQLWindow

- `new(name, range, slide, strategy, tick, start_time)` - Create a window
- `add(quad, timestamp)` - Add a quad to the window
- `subscribe(stream_type, callback)` - Subscribe to window emissions

### R2ROperator

- `new(query: String)` - Create R2R operator with SPARQL query
- `add_static_data(quad)` - Add static data for joins
- `execute(container)` - Execute query on streaming data

## Examples

See the integration tests in `tests/integration/` for comprehensive examples:

- Basic RSP engine usage
- Aggregation queries (COUNT, AVG, MIN/MAX, SUM)
- Window-R2R integration
- Named graph queries
- Static data joins

## License

This code is copyrighted by Ghent University - imec and released under the MIT Licence

## Acknowledgments

This project is a Rust port of [RSP-JS](https://github.com/pbonte/RSP-JS/), an RDF Stream Processing library for JavaScript/Typescript. 

We would like to thank the original authors and contributors of RSP-JS for their excellent work and for providing the foundation that made this Rust implementation possible.

The core concepts, RSP-QL syntax support, and windowing semantics have been adapted from the original TypeScript implementation to provide the same functionality in a high-performance Rust library.

## Contact

For any questions, please contact [Kush](mailto:mailkushbisen@gmail.com) or create an issue in the repository.

