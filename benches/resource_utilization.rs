use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use oxigraph::model::*;
use rsp_rs::RSPEngine;
use std::time::Instant;
use sysinfo::{System, Pid};

/// Generate a quad for benchmarking
fn generate_quad(subject_id: usize, property_id: usize, object_id: usize) -> Quad {
    Quad::new(
        NamedNode::new(&format!("http://example.org/sensor{}", subject_id)).unwrap(),
        NamedNode::new(&format!("http://example.org/property{}", property_id)).unwrap(),
        Literal::new_typed_literal(
            &format!("{}", object_id),
            NamedNode::new("http://www.w3.org/2001/XMLSchema#integer").unwrap(),
        ),
        GraphName::DefaultGraph,
    )
}

/// Get current process memory usage in MB
fn get_process_memory_mb() -> f64 {
    let mut sys = System::new_all();
    sys.refresh_all();
    let pid = Pid::from_u32(std::process::id());
    
    if let Some(process) = sys.process(pid) {
        (process.memory() as f64) / 1024.0 / 1024.0
    } else {
        0.0
    }
}

/// Benchmark: Memory usage for 30-second window with different data rates
fn benchmark_memory_usage_30s_window(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_30s_window");
    group.sample_size(10);

    // Test different data rates (quads per second)
    for quads_per_sec in [1, 5, 10, 20].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}qps", quads_per_sec)),
            quads_per_sec,
            |b, &quads_per_sec| {
                b.iter_custom(|iters| {
                    let mut total_duration = std::time::Duration::ZERO;
                    
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
                        let _result_receiver = engine.start_processing();
                        let stream = engine.get_stream("http://example.org/stream1").unwrap();

                        let initial_memory = get_process_memory_mb();
                        let start = Instant::now();
                        let base_timestamp = iter as i64 * 100000;

                        // Simulate 30 seconds of data at the specified rate
                        // We'll add data for 35 seconds to ensure full window coverage
                        let total_quads = quads_per_sec * 35;
                        
                        for i in 0..total_quads {
                            let timestamp = base_timestamp + ((i as i64 * 1000) / quads_per_sec as i64);
                            let quads = vec![generate_quad(i as usize % 10, i as usize % 5, i as usize)];
                            stream.add_quads(black_box(quads), timestamp).unwrap();
                        }

                        // Let the window process
                        std::thread::sleep(std::time::Duration::from_millis(50));

                        let final_memory = get_process_memory_mb();
                        let elapsed = start.elapsed();
                        
                        let memory_delta = final_memory - initial_memory;
                        
                        println!(
                            "\n[Memory] {}qps: Total quads: {}, Memory delta: {:.2} MB, Time: {:.3}s",
                            quads_per_sec, total_quads, memory_delta, elapsed.as_secs_f64()
                        );

                        total_duration += elapsed;
                    }
                    
                    total_duration
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: CPU usage during window processing
fn benchmark_cpu_usage_30s_window(c: &mut Criterion) {
    let mut group = c.benchmark_group("cpu_30s_window");
    group.sample_size(10);

    // Measure CPU usage with different query complexities
    let queries = vec![
        ("simple_select", r#"
            PREFIX ex: <http://example.org/>
            REGISTER RStream <output> AS
            SELECT ?s ?p ?o
            FROM NAMED WINDOW ex:w1 ON STREAM ex:stream1 [RANGE 30000 STEP 5000]
            WHERE {
                WINDOW ex:w1 { ?s ?p ?o }
            }
        "#),
        ("count_aggregation", r#"
            PREFIX ex: <http://example.org/>
            REGISTER RStream <output> AS
            SELECT (COUNT(?s) AS ?count)
            FROM NAMED WINDOW ex:w1 ON STREAM ex:stream1 [RANGE 30000 STEP 5000]
            WHERE {
                WINDOW ex:w1 { ?s ?p ?o }
            }
        "#),
        ("avg_aggregation", r#"
            PREFIX ex: <http://example.org/>
            REGISTER RStream <output> AS
            SELECT (AVG(?val) AS ?avgVal)
            FROM NAMED WINDOW ex:w1 ON STREAM ex:stream1 [RANGE 30000 STEP 5000]
            WHERE {
                WINDOW ex:w1 { ?s ?p ?val }
            }
        "#),
    ];

    for (name, query) in queries {
        group.bench_function(name, |b| {
            b.iter_custom(|iters| {
                let mut total_duration = std::time::Duration::ZERO;
                
                for iter in 0..iters {
                    let mut engine = RSPEngine::new(query.to_string());
                    engine.initialize().unwrap();
                    let _result_receiver = engine.start_processing();
                    let stream = engine.get_stream("http://example.org/stream1").unwrap();

                    let start = Instant::now();
                    let base_timestamp = iter as i64 * 100000;

                    // Add data at 10 quads/second for 35 seconds (350 quads total)
                    for i in 0..350 {
                        let timestamp = base_timestamp + (i * 100); // 10 quads per second
                        let quads = vec![generate_quad(i as usize % 10, i as usize % 5, i as usize)];
                        stream.add_quads(black_box(quads), timestamp).unwrap();
                    }

                    // Let processing complete
                    std::thread::sleep(std::time::Duration::from_millis(50));

                    let elapsed = start.elapsed();
                    total_duration += elapsed;
                }
                
                total_duration
            });
        });
    }

    group.finish();
}

/// Benchmark: Window overhead - just the windowing mechanism
fn benchmark_window_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("window_overhead");
    group.sample_size(10);

    group.bench_function("30s_window_operations", |b| {
        b.iter_custom(|iters| {
            let mut total_duration = std::time::Duration::ZERO;
            
            for iter in 0..iters {
                let query = r#"
                    PREFIX ex: <http://example.org/>
                    REGISTER RStream <output> AS
                    SELECT ?s ?p ?o
                    FROM NAMED WINDOW ex:w1 ON STREAM ex:stream1 [RANGE 30000 STEP 5000]
                    WHERE {
                        WINDOW ex:w1 { ?s ?p ?o }
                    }
                "#;

                let mut engine = RSPEngine::new(query.to_string());
                engine.initialize().unwrap();
                let _result_receiver = engine.start_processing();
                let stream = engine.get_stream("http://example.org/stream1").unwrap();

                let start = Instant::now();
                let base_timestamp = iter as i64 * 100000;

                // Minimal data - just enough to trigger windows
                for i in 0..10 {
                    let timestamp = base_timestamp + (i * 5000);
                    let quads = vec![generate_quad(0, 0, i as usize)];
                    stream.add_quads(black_box(quads), timestamp).unwrap();
                }

                let elapsed = start.elapsed();
                total_duration += elapsed;
            }
            
            total_duration
        });
    });

    group.finish();
}

/// Benchmark: Throughput under sustained load (quads/second)
fn benchmark_sustained_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("sustained_throughput");
    group.sample_size(10);

    group.bench_function("1000_quads_burst", |b| {
        b.iter_custom(|iters| {
            let mut total_duration = std::time::Duration::ZERO;
            
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
                let _result_receiver = engine.start_processing();
                let stream = engine.get_stream("http://example.org/stream1").unwrap();

                let start = Instant::now();
                let base_timestamp = iter as i64 * 100000;

                // Burst of 1000 quads as fast as possible
                for i in 0..1000 {
                    let timestamp = base_timestamp + i;
                    let quads = vec![generate_quad(i as usize % 100, i as usize % 10, i as usize)];
                    stream.add_quads(black_box(quads), timestamp).unwrap();
                }

                let elapsed = start.elapsed();
                
                let throughput = 1000.0 / elapsed.as_secs_f64();
                println!(
                    "\n[Throughput] 1000 quads in {:.3}s = {:.0} quads/sec",
                    elapsed.as_secs_f64(), throughput
                );

                total_duration += elapsed;
            }
            
            total_duration
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_memory_usage_30s_window,
    benchmark_cpu_usage_30s_window,
    benchmark_window_overhead,
    benchmark_sustained_throughput
);
criterion_main!(benches);
