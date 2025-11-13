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

/// Get CPU metrics using sysinfo
fn get_cpu_metrics() -> (f32, usize) {
    let mut sys = sysinfo::System::new_all();
    sys.refresh_cpu_all();

    let cpu_usage: f32 =
        sys.cpus().iter().map(|cpu| cpu.cpu_usage()).sum::<f32>() / sys.cpus().len() as f32;
    let num_cpus = num_cpus::get();

    (cpu_usage, num_cpus)
}

/// Benchmark: CPU usage with varying data rates
fn benchmark_cpu_usage_varying_rates(c: &mut Criterion) {
    let mut group = c.benchmark_group("cpu_usage_varying_rates");
    group.sample_size(5);

    // Test with different quads per batch
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

                    let (initial_cpu, num_cpus) = get_cpu_metrics();
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

                    std::thread::sleep(std::time::Duration::from_millis(200));

                    let (peak_cpu, _) = get_cpu_metrics();
                    let elapsed = start.elapsed();

                    println!(
                        "\n[CPU Usage] Quads/batch: {}, Iterations: {}, Avg CPU: {:.2}%, Peak: {:.2}%, CPUs: {}, Time: {:.3}s",
                        quads_per_batch,
                        iters,
                        initial_cpu,
                        peak_cpu,
                        num_cpus,
                        elapsed.as_secs_f64()
                    );

                    elapsed
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: CPU efficiency (QPS per CPU core)
fn benchmark_cpu_efficiency(c: &mut Criterion) {
    let mut group = c.benchmark_group("cpu_efficiency");
    group.sample_size(5);
    group.measurement_time(std::time::Duration::from_secs(15));

    group.bench_function("qps_per_core", |b| {
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
            let mut total_quads = 0u64;

            for _iter in 0..iters {
                let mut quads = Vec::new();
                for q in 0..1000 {
                    quads.push(generate_quad(q % 100, q % 20, q));
                    total_quads += 1;
                }

                let timestamp = start.elapsed().as_millis() as i64;
                stream.add_quads(black_box(quads), timestamp).unwrap();

                // Brief sleep to simulate realistic scenario
                std::thread::sleep(std::time::Duration::from_micros(100));
            }

            let elapsed = start.elapsed();
            let qps = total_quads as f64 / elapsed.as_secs_f64();
            let num_cpus = num_cpus::get();
            let qps_per_core = qps / num_cpus as f64;

            println!(
                "\n[CPU Efficiency] Total Quads: {}, Time: {:.3}s, QPS: {:.0}, QPS/core: {:.0}, CPUs: {}",
                total_quads,
                elapsed.as_secs_f64(),
                qps,
                qps_per_core,
                num_cpus
            );

            elapsed
        });
    });

    group.finish();
}

/// Benchmark: CPU overhead with multiple windows
fn benchmark_cpu_multi_window_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("cpu_multi_window_overhead");
    group.sample_size(3);

    for num_windows in [1, 2, 4].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_windows),
            num_windows,
            |b, &num_windows| {
                b.iter_custom(|iters| {
                    // Build query with multiple windows
                    let mut where_clause = String::new();
                    let mut from_clause = String::new();

                    for i in 0..num_windows {
                        from_clause.push_str(&format!(
                            "NAMED WINDOW ex:w{} ON STREAM ex:stream1 [RANGE 5000 STEP 1000] ",
                            i
                        ));
                        where_clause.push_str(&format!("WINDOW ex:w{} {{ ?s{} ?p{} ?o{} }} ", i, i, i, i));
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
                    let stream = engine.get_stream("http://example.org/stream1").unwrap();

                    let (initial_cpu, _) = get_cpu_metrics();
                    let start = Instant::now();

                    for iter in 0..iters {
                        let mut quads = Vec::new();
                        for q in 0..500 {
                            let quad_id = (iter as usize * 500) + q;
                            quads.push(generate_quad(quad_id % 50, q % 10, quad_id));
                        }

                        let timestamp = (iter as i64) * 1000;
                        stream.add_quads(black_box(quads), timestamp).unwrap();
                    }

                    std::thread::sleep(std::time::Duration::from_millis(200));

                    let (peak_cpu, _) = get_cpu_metrics();
                    let elapsed = start.elapsed();

                    println!(
                        "\n[Multi-Window CPU] Windows: {}, Iterations: {}, Avg CPU: {:.2}%, Peak: {:.2}%, Time: {:.3}s",
                        num_windows,
                        iters,
                        initial_cpu,
                        peak_cpu,
                        elapsed.as_secs_f64()
                    );

                    elapsed
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: CPU scalability - sustained high throughput
fn benchmark_cpu_sustained_high_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("cpu_sustained_high_throughput");
    group.sample_size(3);
    group.measurement_time(std::time::Duration::from_secs(30));

    group.bench_function("sustained_high_throughput", |b| {
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

            let start = Instant::now();
            let mut total_quads = 0u64;
            let mut cpu_samples = Vec::new();

            while start.elapsed() < std::time::Duration::from_secs(30) {
                let mut quads = Vec::new();
                for q in 0..1000 {
                    quads.push(generate_quad(q % 100, q % 20, q));
                    total_quads += 1;
                }

                let timestamp = start.elapsed().as_millis() as i64;
                stream.add_quads(black_box(quads), timestamp).unwrap();

                // Sample CPU every 100 iterations
                if total_quads % 100_000 == 0 {
                    let (cpu, _) = get_cpu_metrics();
                    cpu_samples.push(cpu);
                }

                std::thread::sleep(std::time::Duration::from_millis(1));
            }

            let elapsed = start.elapsed();

            if !cpu_samples.is_empty() {
                let avg_cpu: f32 = cpu_samples.iter().sum::<f32>() / cpu_samples.len() as f32;
                let max_cpu = cpu_samples.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                let min_cpu = cpu_samples.iter().cloned().fold(f32::INFINITY, f32::min);

                let qps = total_quads as f64 / elapsed.as_secs_f64();
                let num_cpus = num_cpus::get();
                let qps_per_core = qps / num_cpus as f64;

                println!(
                    "\n[Sustained CPU] Total Quads: {}, Time: {:.3}s, QPS: {:.0}, QPS/core: {:.0}, Avg CPU: {:.2}%, Max: {:.2}%, Min: {:.2}%, CPUs: {}",
                    total_quads,
                    elapsed.as_secs_f64(),
                    qps,
                    qps_per_core,
                    avg_cpu,
                    max_cpu,
                    min_cpu,
                    num_cpus
                );
            }

            elapsed
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_cpu_usage_varying_rates,
    benchmark_cpu_efficiency,
    benchmark_cpu_multi_window_overhead,
    benchmark_cpu_sustained_high_throughput
);
criterion_main!(benches);
