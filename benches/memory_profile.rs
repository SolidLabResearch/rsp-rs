use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use oxigraph::model::*;
use rsp_rs::RSPEngine;
use std::time::Instant;
use sysinfo::System;

/// Generate a quad for benchmarking
fn generate_quad(subject_id: usize, property_id: usize, object_id: usize) -> Quad {
    Quad::new(
        NamedNode::new(&format!("http://example.org/sensor{}", subject_id)).unwrap(),
        NamedNode::new(&format!("http://example.org/property{}", property_id)).unwrap(),
        Literal::new_simple_literal(&format!("value_{}", object_id)),
        GraphName::DefaultGraph,
    )
}

/// Get current memory usage in MB
fn get_memory_usage_mb() -> f64 {
    let mut sys = System::new_all();
    sys.refresh_memory();
    (sys.used_memory() as f64) / 1024.0 / 1024.0
}

/// Benchmark: Memory growth with increasing data volume
fn benchmark_memory_growth(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_growth");
    group.sample_size(10);

    // Test with different total quads processed
    for total_quads in [10_000, 50_000, 100_000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(total_quads),
            total_quads,
            |b, &total_quads| {
                b.iter_custom(|_iters| {
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

                    let initial_memory = get_memory_usage_mb();
                    let start = Instant::now();

                    let quads_per_batch = 1000;
                    let num_batches = total_quads / quads_per_batch;

                    for batch_idx in 0..num_batches {
                        let mut quads = Vec::new();
                        for q in 0..quads_per_batch {
                            let quad_id = batch_idx * quads_per_batch + q;
                            quads.push(generate_quad(quad_id % 200, q % 30, quad_id));
                        }

                        let timestamp = (batch_idx as i64) * 1000;
                        stream.add_quads(black_box(quads), timestamp).unwrap();
                    }

                    // Allow time for processing
                    std::thread::sleep(std::time::Duration::from_millis(500));

                    let peak_memory = get_memory_usage_mb();
                    let elapsed = start.elapsed();

                    println!(
                        "\n[Memory Growth] Total quads: {}, Initial: {:.2} MB, Peak: {:.2} MB, Delta: {:.2} MB, Time: {:.3}s",
                        total_quads,
                        initial_memory,
                        peak_memory,
                        peak_memory - initial_memory,
                        elapsed.as_secs_f64()
                    );

                    elapsed
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Memory stability over sustained operations
fn benchmark_memory_stability(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_stability");
    group.sample_size(10);
    group.measurement_time(std::time::Duration::from_secs(30));

    group.bench_function("sustained_memory_operations", |b| {
        b.iter_custom(|_iters| {
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
            let mut memory_samples = Vec::new();

            // Continuous stream for 30 seconds
            let mut iteration = 0;
            while start.elapsed() < std::time::Duration::from_secs(30) {
                let mut quads = Vec::new();
                for q in 0..500 {
                    let quad_id = iteration * 500 + q;
                    quads.push(generate_quad(quad_id % 100, q % 15, quad_id));
                }

                let timestamp = (iteration as i64) * 100;
                stream.add_quads(black_box(quads), timestamp).unwrap();

                // Sample memory every 10 iterations
                if iteration % 10 == 0 {
                    memory_samples.push(get_memory_usage_mb());
                }

                iteration += 1;
                std::thread::sleep(std::time::Duration::from_millis(10));
            }

            let elapsed = start.elapsed();

            if !memory_samples.is_empty() {
                let avg_memory: f64 = memory_samples.iter().sum::<f64>() / memory_samples.len() as f64;
                let max_memory = memory_samples
                    .iter()
                    .cloned()
                    .fold(f64::NEG_INFINITY, f64::max);
                let min_memory = memory_samples
                    .iter()
                    .cloned()
                    .fold(f64::INFINITY, f64::min);

                println!(
                    "\n[Memory Stability] Duration: {:.3}s, Iterations: {}, Avg Memory: {:.2} MB, Max: {:.2} MB, Min: {:.2} MB",
                    elapsed.as_secs_f64(),
                    iteration,
                    avg_memory,
                    max_memory,
                    min_memory
                );
            }

            elapsed
        });
    });

    group.finish();
}

/// Benchmark: Memory overhead of different window configurations
fn benchmark_memory_window_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_window_overhead");
    group.sample_size(10);

    for (width, slide) in [(1000, 500), (5000, 1000), (10000, 5000)].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("w{}_s{}", width, slide)),
            &(*width, *slide),
            |b, &(width, slide)| {
                b.iter_custom(|_iters| {
                    let query = format!(
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
                    );

                    let mut engine = RSPEngine::new(query);
                    engine.initialize().unwrap();

                    let _result_receiver = engine.start_processing();
                    let stream = engine.get_stream("http://example.org/stream1").unwrap();

                    let initial_memory = get_memory_usage_mb();
                    let start = Instant::now();

                    // Process 50k quads
                    for batch_idx in 0..50 {
                        let mut quads = Vec::new();
                        for q in 0..1000 {
                            let quad_id = batch_idx * 1000 + q;
                            quads.push(generate_quad(quad_id % 100, q % 20, quad_id));
                        }

                        let timestamp = (batch_idx as i64) * 100;
                        stream.add_quads(black_box(quads), timestamp).unwrap();
                    }

                    std::thread::sleep(std::time::Duration::from_millis(500));

                    let peak_memory = get_memory_usage_mb();
                    let elapsed = start.elapsed();

                    println!(
                        "\n[Window Overhead] W={} S={}, Memory Delta: {:.2} MB, Time: {:.3}s",
                        width,
                        slide,
                        peak_memory - initial_memory,
                        elapsed.as_secs_f64()
                    );

                    elapsed
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Memory impact with multiple concurrent streams
fn benchmark_memory_multi_stream(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_multi_stream");
    group.sample_size(10);

    for num_streams in [1, 2, 4].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_streams),
            num_streams,
            |b, &num_streams| {
                b.iter_custom(|_iters| {
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

                    let initial_memory = get_memory_usage_mb();
                    let start = Instant::now();

                    for batch_idx in 0..100 {
                        for stream_idx in 0..num_streams {
                            let stream = engine
                                .get_stream(&format!("http://example.org/stream{}", stream_idx))
                                .unwrap();

                            let mut quads = Vec::new();
                            for q in 0..100 {
                                let quad_id = batch_idx * 100 + q;
                                quads.push(generate_quad(quad_id % 50, q % 10, quad_id));
                            }

                            let timestamp = (batch_idx as i64) * 100;
                            stream
                                .add_quads(black_box(quads.clone()), timestamp)
                                .unwrap();
                        }
                    }

                    std::thread::sleep(std::time::Duration::from_millis(500));

                    let peak_memory = get_memory_usage_mb();
                    let elapsed = start.elapsed();

                    println!(
                        "\n[Multi-Stream Memory] Streams: {}, Memory Delta: {:.2} MB, Time: {:.3}s",
                        num_streams,
                        peak_memory - initial_memory,
                        elapsed.as_secs_f64()
                    );

                    elapsed
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_memory_growth,
    benchmark_memory_stability,
    benchmark_memory_window_overhead,
    benchmark_memory_multi_stream
);
criterion_main!(benches);
