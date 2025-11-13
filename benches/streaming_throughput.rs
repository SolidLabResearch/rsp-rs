use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use oxigraph::model::*;
use rsp_rs::RSPEngine;
use std::time::Instant;

/// Generate a quad for benchmarking
fn generate_quad(subject_id: usize, property_id: usize, object_id: usize) -> Quad {
    Quad::new(
        NamedNode::new(&format!("http://example.org/sensor{}", subject_id)).unwrap(),
        NamedNode::new(&format!("http://example.org/property{}", property_id)).unwrap(),
        Literal::new_simple_literal(&format!("value_{}", object_id)),
        GraphName::DefaultGraph,
    )
}

/// Benchmark: Basic stream throughput with fixed data rate
fn benchmark_throughput_fixed_rate(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput_fixed_rate");
    group.sample_size(10);

    // Test with different data rates (quads per batch)
    for quads_per_batch in [100, 500, 1000, 5000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(quads_per_batch),
            quads_per_batch,
            |b, &quads_per_batch| {
                b.iter_custom(|iters| {
                    let query = r#"
                        PREFIX ex: <http://example.org/>
                        REGISTER RStream <output> AS
                        SELECT *
                        FROM NAMED WINDOW ex:w1 ON STREAM ex:stream1 [RANGE 5000 STEP 1000]
                        WHERE {
                            WINDOW ex:w1 { ?s ?p ?o }
                        }
                    "#;

                    let mut engine = RSPEngine::new(query.to_string());
                    engine.initialize().unwrap();

                    let _result_receiver = engine.start_processing();

                    let stream = engine.get_stream("http://example.org/stream1").unwrap();

                    let start = Instant::now();

                    for iter in 0..iters {
                        let mut quads = Vec::new();
                        for q in 0..quads_per_batch {
                            let quad_id = (iter as usize * quads_per_batch) + q;
                            quads.push(generate_quad(quad_id % 100, q % 10, quad_id));
                        }

                        let timestamp = (iter as i64) * 1000;
                        stream.add_quads(black_box(quads), timestamp).unwrap();
                    }

                    // Allow results to be processed
                    std::thread::sleep(std::time::Duration::from_millis(100));

                    start.elapsed()
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Throughput with varying window sizes
fn benchmark_throughput_window_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput_window_sizes");
    group.sample_size(10);

    // Test with different window sizes (width, slide)
    for (width, slide) in [(1000, 500), (5000, 1000), (10000, 5000), (60000, 10000)].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("w{}_s{}", width, slide)),
            &(*width, *slide),
            |b, &(width, slide)| {
                b.iter_custom(|iters| {
                    let mut engine = RSPEngine::new(format!(
                        r#"
                        PREFIX ex: <http://example.org/>
                        REGISTER RStream <output> AS
                        SELECT *
                        FROM NAMED WINDOW ex:w1 ON STREAM ex:stream1 [RANGE {} STEP {}]
                        WHERE {{
                            WINDOW ex:w1 {{ ?s ?p ?o }}
                        }}
                    "#,
                        width, slide
                    ));
                    engine.initialize().unwrap();

                    let _result_receiver = engine.start_processing();
                    let stream = engine.get_stream("http://example.org/stream1").unwrap();

                    let start = Instant::now();

                    for iter in 0..iters {
                        let mut quads = Vec::new();
                        for q in 0..100 {
                            let quad_id = (iter as usize * 100) + q;
                            quads.push(generate_quad(quad_id % 50, q % 5, quad_id));
                        }

                        let timestamp = (iter as i64) * 100;
                        stream.add_quads(black_box(quads), timestamp).unwrap();
                    }

                    std::thread::sleep(std::time::Duration::from_millis(100));
                    start.elapsed()
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Multi-stream throughput
fn benchmark_throughput_multi_stream(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput_multi_stream");
    group.sample_size(10);

    for num_streams in [1, 2, 4].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_streams),
            num_streams,
            |b, &num_streams| {
                b.iter_custom(|iters| {
                    // Build query with multiple windows
                    let mut where_clause = String::new();
                    let mut from_clause = String::new();

                    for i in 0..num_streams {
                        from_clause.push_str(&format!(
                            "NAMED WINDOW ex:w{} ON STREAM ex:stream{} [RANGE 5000 STEP 1000] ",
                            i, i
                        ));
                        where_clause
                            .push_str(&format!("WINDOW ex:w{} {{ ?s{} ?p{} ?o{} }} ", i, i, i, i));
                    }

                    let query = format!(
                        r#"
                        PREFIX ex: <http://example.org/>
                        REGISTER RStream <output> AS
                        SELECT *
                        FROM {}
                        WHERE {{ {} }}
                    "#,
                        from_clause, where_clause
                    );

                    let mut engine = RSPEngine::new(query);
                    engine.initialize().unwrap();

                    let _result_receiver = engine.start_processing();

                    let start = Instant::now();

                    for iter in 0..iters {
                        for stream_idx in 0..num_streams {
                            let stream = engine
                                .get_stream(&format!("http://example.org/stream{}", stream_idx))
                                .unwrap();

                            let mut quads = Vec::new();
                            for q in 0..50 {
                                let quad_id = (iter as usize * 50) + q;
                                quads.push(generate_quad(quad_id % 50, q % 5, quad_id));
                            }

                            let timestamp = (iter as i64) * 100;
                            stream
                                .add_quads(black_box(quads.clone()), timestamp)
                                .unwrap();
                        }
                    }

                    std::thread::sleep(std::time::Duration::from_millis(100));
                    start.elapsed()
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Sustained throughput over time
fn benchmark_throughput_sustained_load(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput_sustained_load");
    group.sample_size(5);
    group.measurement_time(std::time::Duration::from_secs(10));

    group.bench_function("sustained_1k_quads_per_sec", |b| {
        b.iter_custom(|iters| {
            let query = r#"
                PREFIX ex: <http://example.org/>
                REGISTER RStream <output> AS
                SELECT *
                FROM NAMED WINDOW ex:w1 ON STREAM ex:stream1 [RANGE 10000 STEP 2000]
                WHERE {
                    WINDOW ex:w1 { ?s ?p ?o }
                }
            "#;

            let mut engine = RSPEngine::new(query.to_string());
            engine.initialize().unwrap();

            let _result_receiver = engine.start_processing();
            let stream = engine.get_stream("http://example.org/stream1").unwrap();

            let start = Instant::now();

            for iter in 0..iters {
                let mut quads = Vec::new();
                for q in 0..1000 {
                    let quad_id = (iter as usize * 1000) + q;
                    quads.push(generate_quad(quad_id % 100, q % 20, quad_id));
                }

                let timestamp = (iter as i64) * 1000;
                stream.add_quads(black_box(quads), timestamp).unwrap();

                // Simulate realistic inter-arrival times
                std::thread::sleep(std::time::Duration::from_millis(1));
            }

            std::thread::sleep(std::time::Duration::from_secs(1));
            start.elapsed()
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_throughput_fixed_rate,
    benchmark_throughput_window_sizes,
    benchmark_throughput_multi_stream,
    benchmark_throughput_sustained_load
);
criterion_main!(benches);
