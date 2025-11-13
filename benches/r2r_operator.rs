use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use oxigraph::model::*;
use rsp_rs::{QuadContainer, R2ROperator};
use std::collections::HashSet;
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

/// Generate static data quads
fn generate_static_quad(subject_id: usize, property_id: usize, object_id: usize) -> Quad {
    Quad::new(
        NamedNode::new(&format!("http://example.org/static_sensor{}", subject_id)).unwrap(),
        NamedNode::new(&format!(
            "http://example.org/static_property{}",
            property_id
        ))
        .unwrap(),
        Literal::new_simple_literal(&format!("static_value_{}", object_id)),
        GraphName::DefaultGraph,
    )
}

/// Benchmark: Simple R2R query execution
fn benchmark_r2r_simple_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("r2r_simple_query");
    group.sample_size(20);

    // Test with different numbers of streaming quads
    for streaming_quads in [10, 50, 100, 500, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(streaming_quads),
            streaming_quads,
            |b, &streaming_quads| {
                b.iter_custom(|iters| {
                    let query = r#"
                        PREFIX ex: <http://example.org/>
                        SELECT ?sensor ?temp
                        WHERE {
                            ?sensor ex:property1 ?temp .
                        }
                    "#
                    .to_string();

                    let mut r2r = R2ROperator::new(query);

                    // Add minimal static data
                    r2r.add_static_data(Quad::new(
                        NamedNode::new("http://example.org/static_metadata").unwrap(),
                        NamedNode::new("http://example.org/type").unwrap(),
                        NamedNode::new("http://example.org/SensorMetadata").unwrap(),
                        GraphName::DefaultGraph,
                    ));

                    let start = Instant::now();

                    for _iter in 0..iters {
                        let mut quads = HashSet::new();
                        for q in 0..streaming_quads {
                            quads.insert(generate_quad(q % 50, 1, q));
                        }

                        let container = QuadContainer::new(quads, 1000);
                        let _ = r2r.execute(black_box(&container));
                    }

                    start.elapsed()
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: R2R query with static data joins
fn benchmark_r2r_static_join(c: &mut Criterion) {
    let mut group = c.benchmark_group("r2r_static_join");
    group.sample_size(10);

    // Test with different static data sizes
    for static_data_size in [100, 500, 1000, 5000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(static_data_size),
            static_data_size,
            |b, &static_data_size| {
                b.iter_custom(|iters| {
                    let query = r#"
                        PREFIX ex: <http://example.org/>
                        SELECT ?sensor ?temp ?metadata
                        WHERE {
                            ?sensor ex:property1 ?temp .
                            ?sensor ex:static_property0 ?metadata .
                        }
                    "#
                    .to_string();

                    let mut r2r = R2ROperator::new(query);

                    // Add static data for joins
                    for i in 0..static_data_size {
                        r2r.add_static_data(generate_static_quad(i, 0, i));
                    }

                    let start = Instant::now();

                    for _iter in 0..iters {
                        let mut quads = HashSet::new();
                        for q in 0..100 {
                            quads.insert(generate_quad(q % (static_data_size.min(50)), 1, q));
                        }

                        let container = QuadContainer::new(quads, 1000);
                        let _ = r2r.execute(black_box(&container));
                    }

                    start.elapsed()
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: R2R complex query with multiple patterns
fn benchmark_r2r_complex_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("r2r_complex_query");
    group.sample_size(10);

    // Test with different complexity levels
    let queries = vec![
        (
            "simple",
            r#"
            PREFIX ex: <http://example.org/>
            SELECT ?sensor ?prop1 ?prop2
            WHERE {
                ?sensor ex:property1 ?prop1 .
            }
        "#,
        ),
        (
            "two_patterns",
            r#"
            PREFIX ex: <http://example.org/>
            SELECT ?sensor ?prop1 ?prop2
            WHERE {
                ?sensor ex:property1 ?prop1 .
                ?sensor ex:property2 ?prop2 .
            }
        "#,
        ),
        (
            "three_patterns",
            r#"
            PREFIX ex: <http://example.org/>
            SELECT ?sensor ?prop1 ?prop2 ?prop3
            WHERE {
                ?sensor ex:property1 ?prop1 .
                ?sensor ex:property2 ?prop2 .
                ?sensor ex:property3 ?prop3 .
            }
        "#,
        ),
    ];

    for (name, query) in queries {
        group.bench_with_input(BenchmarkId::from_parameter(name), query, |b, query| {
            b.iter_custom(|iters| {
                let mut r2r = R2ROperator::new(query.to_string());

                // Add some static data
                for i in 0..100 {
                    r2r.add_static_data(generate_static_quad(i, 0, i));
                }

                let start = Instant::now();

                for _iter in 0..iters {
                    let mut quads = HashSet::new();
                    for q in 0..500 {
                        quads.insert(generate_quad(q % 50, (q % 3) + 1, q));
                    }

                    let container = QuadContainer::new(quads, 1000);
                    let _ = r2r.execute(black_box(&container));
                }

                start.elapsed()
            });
        });
    }

    group.finish();
}

/// Benchmark: R2R execution time vs cardinality
fn benchmark_r2r_cardinality_impact(c: &mut Criterion) {
    let mut group = c.benchmark_group("r2r_cardinality_impact");
    group.sample_size(10);

    let query = r#"
        PREFIX ex: <http://example.org/>
        SELECT ?sensor ?temp
        WHERE {
            ?sensor ex:property1 ?temp .
        }
    "#
    .to_string();

    // Test with different streaming data cardinality
    for cardinality in [10, 50, 100, 500].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(cardinality),
            cardinality,
            |b, &cardinality| {
                b.iter_custom(|iters| {
                    let mut r2r = R2ROperator::new(query.clone());

                    let start = Instant::now();

                    for _iter in 0..iters {
                        let mut quads = HashSet::new();

                        // Generate data with varying cardinality
                        for q in 0..1000 {
                            quads.insert(generate_quad(q % cardinality, 1, q));
                        }

                        let container = QuadContainer::new(quads, 1000);
                        let _ = r2r.execute(black_box(&container));
                    }

                    start.elapsed()
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: R2R in streaming context (integrated with RSPEngine)
fn benchmark_r2r_in_streaming_pipeline(c: &mut Criterion) {
    use rsp_rs::RSPEngine;

    let mut group = c.benchmark_group("r2r_in_streaming_pipeline");
    group.sample_size(5);

    // Test with different query complexity within RSP pipeline
    let queries = vec![
        (
            "basic_select",
            r#"
            PREFIX ex: <http://example.org/>
            REGISTER RStream <output> AS
            SELECT ?sensor ?temperature
            FROM NAMED WINDOW ex:w1 ON STREAM ex:stream1 [RANGE 5000 STEP 1000]
            WHERE {
                WINDOW ex:w1 {
                    ?sensor ex:property1 ?temperature .
                }
            }
        "#,
        ),
        (
            "with_filter",
            r#"
            PREFIX ex: <http://example.org/>
            REGISTER RStream <output> AS
            SELECT ?sensor ?temperature
            FROM NAMED WINDOW ex:w1 ON STREAM ex:stream1 [RANGE 5000 STEP 1000]
            WHERE {
                WINDOW ex:w1 {
                    ?sensor ex:property1 ?temperature .
                    FILTER (regex(?temperature, "^[0-9]"))
                }
            }
        "#,
        ),
    ];

    for (name, query) in queries {
        group.bench_with_input(BenchmarkId::from_parameter(name), query, |b, query| {
            b.iter_custom(|iters| {
                let mut engine = RSPEngine::new(query.to_string());
                engine.initialize().unwrap();

                let _result_receiver = engine.start_processing();
                let stream = engine.get_stream("http://example.org/stream1").unwrap();

                let start = Instant::now();

                for iter in 0..iters {
                    let mut quads = Vec::new();
                    for q in 0..500 {
                        let quad_id = (iter as usize * 500) + q;
                        quads.push(generate_quad(quad_id % 100, 1, quad_id));
                    }

                    let timestamp = (iter as i64) * 1000;
                    stream.add_quads(black_box(quads), timestamp).unwrap();
                }

                std::thread::sleep(std::time::Duration::from_millis(200));

                start.elapsed()
            });
        });
    }

    group.finish();
}

/// Benchmark: R2R with optional patterns (SPARQL OPTIONAL)
fn benchmark_r2r_optional_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("r2r_optional_patterns");
    group.sample_size(10);

    let queries = vec![
        (
            "no_optional",
            r#"
            PREFIX ex: <http://example.org/>
            SELECT ?sensor ?prop1
            WHERE {
                ?sensor ex:property1 ?prop1 .
            }
        "#,
        ),
        (
            "one_optional",
            r#"
            PREFIX ex: <http://example.org/>
            SELECT ?sensor ?prop1 ?prop2
            WHERE {
                ?sensor ex:property1 ?prop1 .
                OPTIONAL { ?sensor ex:property2 ?prop2 }
            }
        "#,
        ),
        (
            "two_optional",
            r#"
            PREFIX ex: <http://example.org/>
            SELECT ?sensor ?prop1 ?prop2 ?prop3
            WHERE {
                ?sensor ex:property1 ?prop1 .
                OPTIONAL { ?sensor ex:property2 ?prop2 }
                OPTIONAL { ?sensor ex:property3 ?prop3 }
            }
        "#,
        ),
    ];

    for (name, query) in queries {
        group.bench_with_input(BenchmarkId::from_parameter(name), query, |b, query| {
            b.iter_custom(|iters| {
                let mut r2r = R2ROperator::new(query.to_string());

                let start = Instant::now();

                for _iter in 0..iters {
                    let mut quads = HashSet::new();
                    for q in 0..300 {
                        quads.insert(generate_quad(q % 50, (q % 3) + 1, q));
                    }

                    let container = QuadContainer::new(quads, 1000);
                    let _ = r2r.execute(black_box(&container));
                }

                start.elapsed()
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_r2r_simple_query,
    benchmark_r2r_static_join,
    benchmark_r2r_complex_query,
    benchmark_r2r_cardinality_impact,
    benchmark_r2r_in_streaming_pipeline,
    benchmark_r2r_optional_patterns
);
criterion_main!(benches);
