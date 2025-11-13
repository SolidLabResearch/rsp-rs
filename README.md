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

