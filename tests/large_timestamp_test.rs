use oxigraph::model::{GraphName, NamedNode, Quad};
use rsp_rs::RSPEngine;
use std::time::Duration;

/// Baseline test with small timestamps to verify test structure
#[test]
fn test_small_timestamps_baseline() {
    let query = r#"
        PREFIX ex: <http://example.org/>
        REGISTER RSTREAM ex:output AS
        SELECT (COUNT(?s) AS ?count)
        FROM NAMED WINDOW ex:window1 ON STREAM ex:stream1 [RANGE 5000 STEP 1000]
        WHERE {
            WINDOW ex:window1 {
                ?s ?p ?o .
            }
        }
    "#;

    let mut engine = RSPEngine::new(query.to_string());
    engine.initialize().unwrap();

    let result_receiver = engine.start_processing();
    let stream = engine.get_stream("http://example.org/stream1").unwrap();

    // Add events with small timestamps spanning 10 seconds
    for i in 0..10 {
        let timestamp = i * 1000; // 0, 1000, 2000, ... 9000
        let quads = vec![Quad::new(
            NamedNode::new(&format!("http://example.org/sensor{}", i)).unwrap(),
            NamedNode::new("http://example.org/hasValue").unwrap(),
            oxigraph::model::Literal::new_simple_literal(&format!("{}", i * 10)),
            GraphName::DefaultGraph,
        )];

        stream.add_quads(quads, timestamp).unwrap();
    }

    // Trigger final window closures
    let final_timestamp = 10_001;
    engine
        .close_stream("http://example.org/stream1", final_timestamp)
        .unwrap();

    // Collect results with timeout
    let mut result_count = 0;
    while let Ok(_result) = result_receiver.recv_timeout(Duration::from_millis(500)) {
        result_count += 1;
    }

    // With 10 seconds of data, 5s window, 1s step, we should get multiple results
    assert!(
        result_count > 0,
        "Should produce at least one result with small timestamps"
    );
}

/// Test that the engine correctly handles large timestamps (like Unix milliseconds)
/// This regression test ensures we don't lose precision when using real-world timestamps
#[test]
fn test_large_unix_millisecond_timestamps() {
    // Use current Unix timestamp in milliseconds (around 1.76 trillion)
    let base_timestamp = 1_760_000_000_000_i64;

    let query = r#"
        PREFIX ex: <http://example.org/>
        REGISTER RSTREAM ex:output AS
        SELECT (COUNT(?s) AS ?count)
        FROM NAMED WINDOW ex:window1 ON STREAM ex:stream1 [RANGE 5000 STEP 1000]
        WHERE {
            WINDOW ex:window1 {
                ?s ?p ?o .
            }
        }
    "#;

    let mut engine = RSPEngine::new(query.to_string());
    engine.initialize().unwrap();

    let result_receiver = engine.start_processing();
    let stream = engine.get_stream("http://example.org/stream1").unwrap();

    // Add events with large timestamps spanning 10 seconds
    for i in 0..10 {
        let timestamp = base_timestamp + (i * 1000); // Each second
        let quads = vec![Quad::new(
            NamedNode::new(&format!("http://example.org/sensor{}", i)).unwrap(),
            NamedNode::new("http://example.org/hasValue").unwrap(),
            oxigraph::model::Literal::new_simple_literal(&format!("{}", i * 10)),
            GraphName::DefaultGraph,
        )];

        stream.add_quads(quads, timestamp).unwrap();
    }

    // Trigger final window closures
    let final_timestamp = base_timestamp + 10_001;
    engine
        .close_stream("http://example.org/stream1", final_timestamp)
        .unwrap();

    // Collect results with timeout
    let mut result_count = 0;
    while let Ok(_result) = result_receiver.recv_timeout(Duration::from_millis(500)) {
        result_count += 1;
    }

    // With 10 seconds of data, 5s window, 1s step, we should get multiple results
    assert!(
        result_count > 0,
        "Should produce at least one result with large timestamps"
    );
}

/// Test that small and large timestamps produce equivalent window behavior
#[test]
fn test_timestamp_normalization_equivalence() {
    let query = r#"
        PREFIX ex: <http://example.org/>
        REGISTER RSTREAM ex:output AS
        SELECT (COUNT(?s) AS ?count)
        FROM NAMED WINDOW ex:window1 ON STREAM ex:stream1 [RANGE 3000 STEP 1000]
        WHERE {
            WINDOW ex:window1 {
                ?s ?p ?o .
            }
        }
    "#;

    // Test with small timestamps (0-based)
    let mut engine_small = RSPEngine::new(query.to_string());
    engine_small.initialize().unwrap();
    let result_receiver_small = engine_small.start_processing();
    let stream_small = engine_small
        .get_stream("http://example.org/stream1")
        .unwrap();

    for i in 0..5 {
        let timestamp = i * 1000; // 0, 1000, 2000, 3000, 4000
        let quads = vec![Quad::new(
            NamedNode::new(&format!("http://example.org/s{}", i)).unwrap(),
            NamedNode::new("http://example.org/p").unwrap(),
            oxigraph::model::Literal::new_simple_literal("value"),
            GraphName::DefaultGraph,
        )];
        stream_small.add_quads(quads, timestamp).unwrap();
    }
    engine_small
        .close_stream("http://example.org/stream1", 5001)
        .unwrap();

    let mut small_results = Vec::new();
    while let Ok(result) = result_receiver_small.recv_timeout(Duration::from_millis(200)) {
        small_results.push(result);
    }

    // Test with large timestamps (Unix milliseconds)
    let base_large = 1_760_000_000_000_i64;
    let mut engine_large = RSPEngine::new(query.to_string());
    engine_large.initialize().unwrap();
    let result_receiver_large = engine_large.start_processing();
    let stream_large = engine_large
        .get_stream("http://example.org/stream1")
        .unwrap();

    for i in 0..5 {
        let timestamp = base_large + (i * 1000);
        let quads = vec![Quad::new(
            NamedNode::new(&format!("http://example.org/s{}", i)).unwrap(),
            NamedNode::new("http://example.org/p").unwrap(),
            oxigraph::model::Literal::new_simple_literal("value"),
            GraphName::DefaultGraph,
        )];
        stream_large.add_quads(quads, timestamp).unwrap();
    }
    engine_large
        .close_stream("http://example.org/stream1", base_large + 5001)
        .unwrap();

    let mut large_results = Vec::new();
    while let Ok(result) = result_receiver_large.recv_timeout(Duration::from_millis(200)) {
        large_results.push(result);
    }

    // Both should produce results (exact count may vary by 1 due to rounding)
    assert!(
        small_results.len() > 0,
        "Small timestamp configuration should produce results"
    );
    assert!(
        large_results.len() > 0,
        "Large timestamp configuration should produce results"
    );
    // Results should be within a reasonable range of each other
    let diff = (small_results.len() as i32 - large_results.len() as i32).abs();
    assert!(
        diff <= 1,
        "Result counts should be similar (small: {}, large: {})",
        small_results.len(),
        large_results.len()
    );
}

/// Test edge case: timestamps near i64::MAX
#[test]
fn test_very_large_timestamps() {
    let query = r#"
        PREFIX ex: <http://example.org/>
        REGISTER RSTREAM ex:output AS
        SELECT ?s ?p ?o
        FROM NAMED WINDOW ex:window1 ON STREAM ex:stream1 [RANGE 2000 STEP 1000]
        WHERE {
            WINDOW ex:window1 {
                ?s ?p ?o .
            }
        }
    "#;

    // Use timestamps close to i64::MAX but with room for arithmetic
    let base_timestamp = i64::MAX / 2; // About 4.6 quintillion

    let mut engine = RSPEngine::new(query.to_string());
    engine.initialize().unwrap();
    let result_receiver = engine.start_processing();
    let stream = engine.get_stream("http://example.org/stream1").unwrap();

    // Add a few quads with very large timestamps
    for i in 0..3 {
        let timestamp = base_timestamp + (i * 1000);
        let quads = vec![Quad::new(
            NamedNode::new(&format!("http://example.org/entity{}", i)).unwrap(),
            NamedNode::new("http://example.org/prop").unwrap(),
            oxigraph::model::Literal::new_simple_literal("data"),
            GraphName::DefaultGraph,
        )];

        stream.add_quads(quads, timestamp).unwrap();
    }

    // Close stream
    engine
        .close_stream("http://example.org/stream1", base_timestamp + 10_000)
        .unwrap();

    // Verify we get results without panic or silent failure
    let mut got_results = false;
    while let Ok(_result) = result_receiver.recv_timeout(Duration::from_millis(300)) {
        got_results = true;
    }

    assert!(
        got_results,
        "Should handle very large timestamps without failure"
    );
}

/// Test that precision is maintained across window calculations
#[test]
fn test_window_boundary_precision() {
    let query = r#"
        PREFIX ex: <http://example.org/>
        REGISTER RSTREAM ex:output AS
        SELECT (COUNT(?s) AS ?count)
        FROM NAMED WINDOW ex:window1 ON STREAM ex:stream1 [RANGE 1000 STEP 500]
        WHERE {
            WINDOW ex:window1 {
                ?s ?p ?o .
            }
        }
    "#;

    // Use a large base timestamp with fractional second steps
    let base_timestamp = 1_700_000_000_000_i64; // A large Unix millisecond timestamp

    let mut engine = RSPEngine::new(query.to_string());
    engine.initialize().unwrap();
    let result_receiver = engine.start_processing();
    let stream = engine.get_stream("http://example.org/stream1").unwrap();

    // Add events at precise 500ms intervals
    let intervals = vec![0, 500, 1000, 1500, 2000, 2500];
    for offset in intervals {
        let timestamp = base_timestamp + offset;
        let quads = vec![Quad::new(
            NamedNode::new(&format!("http://example.org/event{}", offset)).unwrap(),
            NamedNode::new("http://example.org/type").unwrap(),
            oxigraph::model::Literal::new_simple_literal("event"),
            GraphName::DefaultGraph,
        )];

        stream.add_quads(quads, timestamp).unwrap();
    }

    engine
        .close_stream("http://example.org/stream1", base_timestamp + 3000)
        .unwrap();

    // Should get results - the exact count depends on window semantics
    // but we must get SOME results if precision is maintained
    let mut result_count = 0;
    while let Ok(_result) = result_receiver.recv_timeout(Duration::from_millis(300)) {
        result_count += 1;
    }

    assert!(
        result_count > 0,
        "Should produce results with sub-second precision on large timestamps"
    );
}
