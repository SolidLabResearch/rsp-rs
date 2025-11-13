use criterion::{Criterion, black_box, criterion_group, criterion_main};
use oxigraph::model::*;
use rsp_rs::RSPEngine;
use std::time::Instant;

/// Benchmark: End-to-end latency for 30-second window with aggregation
fn benchmark_end_to_end_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("end_to_end_latency");
    group.sample_size(10);

    // Measure latency for first window (5 seconds of data)
    group.bench_function("window_30s_first_result_at_5s", |b| {
        b.iter_custom(|iters| {
            let mut total_latency = std::time::Duration::ZERO;

            for iter in 0..iters {
                let query = r#"
                    PREFIX ex: <http://example.org/>
                    REGISTER RStream <output> AS
                    SELECT (COUNT(?s) AS ?count)
                    FROM NAMED WINDOW ex:w1 ON STREAM ex:stream1 [RANGE 30000 STEP 5000]
                    WHERE {
                        WINDOW ex:w1 { ?s ?p ?o }
                    }
                "#;

                let mut engine = RSPEngine::new(query.to_string());
                engine.initialize().unwrap();
                let result_receiver = engine.start_processing();
                let stream = engine.get_stream("http://example.org/stream1").unwrap();

                let base_timestamp = iter as i64 * 100000;

                // Add data for 5 seconds
                for i in 0..5 {
                    let timestamp = base_timestamp + (i * 1000);
                    let quads = vec![Quad::new(
                        NamedNode::new(&format!("http://example.org/sensor{}", i % 10)).unwrap(),
                        NamedNode::new("http://example.org/temperature").unwrap(),
                        Literal::new_simple_literal(&format!("{}", 20 + i)),
                        GraphName::DefaultGraph,
                    )];
                    stream.add_quads(black_box(quads), timestamp).unwrap();
                }

                // Now measure processing time when triggering window close
                let start = Instant::now();

                // Add one more quad at 5001ms to trigger window close at 5000ms
                let timestamp = base_timestamp + 5001;
                let quads = vec![Quad::new(
                    NamedNode::new("http://example.org/sensor1").unwrap(),
                    NamedNode::new("http://example.org/temperature").unwrap(),
                    Literal::new_simple_literal("25"),
                    GraphName::DefaultGraph,
                )];
                stream.add_quads(black_box(quads), timestamp).unwrap();

                match result_receiver.recv_timeout(std::time::Duration::from_secs(2)) {
                    Ok(_result) => {
                        total_latency += start.elapsed();
                    }
                    Err(_) => {
                        total_latency += start.elapsed();
                    }
                }
            }
            total_latency
        });
    });

    // Measure latency for fully populated window (30 seconds of data)
    group.bench_function("window_30s_full_at_30s", |b| {
        b.iter_custom(|iters| {
            let mut total_latency = std::time::Duration::ZERO;

            for iter in 0..iters {
                let query = r#"
                    PREFIX ex: <http://example.org/>
                    REGISTER RStream <output> AS
                    SELECT (COUNT(?s) AS ?count)
                    FROM NAMED WINDOW ex:w1 ON STREAM ex:stream1 [RANGE 30000 STEP 5000]
                    WHERE {
                        WINDOW ex:w1 { ?s ?p ?o }
                    }
                "#;

                let mut engine = RSPEngine::new(query.to_string());
                engine.initialize().unwrap();
                let result_receiver = engine.start_processing();
                let stream = engine.get_stream("http://example.org/stream1").unwrap();

                let base_timestamp = iter as i64 * 100000;

                // Add data for full 30 seconds
                for i in 0..30 {
                    let timestamp = base_timestamp + (i * 1000);
                    let quads = vec![Quad::new(
                        NamedNode::new(&format!("http://example.org/sensor{}", i % 10)).unwrap(),
                        NamedNode::new("http://example.org/temperature").unwrap(),
                        Literal::new_simple_literal(&format!("{}", 20 + i)),
                        GraphName::DefaultGraph,
                    )];
                    stream.add_quads(black_box(quads), timestamp).unwrap();
                }

                // Now measure just the processing time for the next window result
                let start = Instant::now();

                // Add one more quad to trigger the window at t=30s
                let timestamp = base_timestamp + 30000;
                let quads = vec![Quad::new(
                    NamedNode::new("http://example.org/sensor1").unwrap(),
                    NamedNode::new("http://example.org/temperature").unwrap(),
                    Literal::new_simple_literal("25"),
                    GraphName::DefaultGraph,
                )];
                stream.add_quads(black_box(quads), timestamp).unwrap();

                // Wait for the result with full 30s window
                match result_receiver.recv_timeout(std::time::Duration::from_secs(2)) {
                    Ok(_result) => {
                        total_latency += start.elapsed();
                    }
                    Err(_) => {
                        total_latency += start.elapsed();
                    }
                }
            }
            total_latency
        });
    });

    // Measure latency for AVG aggregation with full window
    group.bench_function("window_30s_avg_full_at_30s", |b| {
        b.iter_custom(|iters| {
            let mut total_latency = std::time::Duration::ZERO;

            for iter in 0..iters {
                let query = r#"
                    PREFIX ex: <http://example.org/>
                    REGISTER RStream <output> AS
                    SELECT (AVG(?temp) AS ?avgTemp)
                    FROM NAMED WINDOW ex:w1 ON STREAM ex:stream1 [RANGE 30000 STEP 5000]
                    WHERE {
                        WINDOW ex:w1 { ?s ex:temperature ?temp }
                    }
                "#;

                let mut engine = RSPEngine::new(query.to_string());
                engine.initialize().unwrap();
                let result_receiver = engine.start_processing();
                let stream = engine.get_stream("http://example.org/stream1").unwrap();

                let base_timestamp = iter as i64 * 100000;

                // Add data for full 30 seconds
                for i in 0..30 {
                    let timestamp = base_timestamp + (i * 1000);
                    let quads = vec![Quad::new(
                        NamedNode::new(&format!("http://example.org/sensor{}", i % 10)).unwrap(),
                        NamedNode::new("http://example.org/temperature").unwrap(),
                        Literal::new_typed_literal(
                            &format!("{}", 20 + i),
                            NamedNode::new("http://www.w3.org/2001/XMLSchema#integer").unwrap(),
                        ),
                        GraphName::DefaultGraph,
                    )];
                    stream.add_quads(black_box(quads), timestamp).unwrap();
                }

                // Measure processing time for window at t=30s
                let start = Instant::now();

                let timestamp = base_timestamp + 30000;
                let quads = vec![Quad::new(
                    NamedNode::new("http://example.org/sensor1").unwrap(),
                    NamedNode::new("http://example.org/temperature").unwrap(),
                    Literal::new_typed_literal(
                        "25",
                        NamedNode::new("http://www.w3.org/2001/XMLSchema#integer").unwrap(),
                    ),
                    GraphName::DefaultGraph,
                )];
                stream.add_quads(black_box(quads), timestamp).unwrap();

                match result_receiver.recv_timeout(std::time::Duration::from_secs(2)) {
                    Ok(_result) => {
                        total_latency += start.elapsed();
                    }
                    Err(_) => {
                        total_latency += start.elapsed();
                    }
                }
            }
            total_latency
        });
    });

    group.finish();
}

criterion_group!(benches, benchmark_end_to_end_latency);
criterion_main!(benches);
