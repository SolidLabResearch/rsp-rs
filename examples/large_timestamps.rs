//! Example demonstrating rsp-rs with large Unix timestamps (milliseconds)
//!
//! This example shows that rsp-rs (v0.3.5+) correctly handles real-world
//! Unix timestamps in milliseconds without requiring normalization workarounds.
//!
//! Prior to v0.3.5, large timestamps would cause precision issues due to
//! floating-point conversions. This has been fixed with pure integer arithmetic.

use oxigraph::model::{GraphName, NamedNode, Quad};
use rsp_rs::RSPEngine;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Large Timestamp Example ===\n");

    // Get current Unix timestamp in milliseconds
    let start_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    println!("Current Unix timestamp (ms): {}", start_time);
    println!(
        "This is approximately {} trillion\n",
        start_time as f64 / 1_000_000_000_000.0
    );

    // Define a query with a 5-second sliding window
    let query = r#"
        PREFIX sensor: <http://example.org/sensors/>
        PREFIX obs: <http://example.org/observations/>
        REGISTER RSTREAM sensor:output AS
        SELECT ?sensor (AVG(?temp) AS ?avg_temp) (COUNT(?temp) AS ?count)
        FROM NAMED WINDOW sensor:tempWindow ON STREAM sensor:stream1 [RANGE 5000 STEP 1000]
        WHERE {
            WINDOW sensor:tempWindow {
                ?sensor obs:temperature ?temp .
            }
        }
        GROUP BY ?sensor
    "#;

    // Initialize engine
    let mut engine = RSPEngine::new(query.to_string());
    engine.initialize()?;

    // Start processing and get result receiver
    let result_receiver = engine.start_processing();

    // Get stream reference
    let stream = engine
        .get_stream("http://example.org/sensors/stream1")
        .ok_or("Failed to get stream")?;

    println!("Simulating 10 seconds of sensor data with Unix timestamps...\n");

    // Simulate 10 sensor readings over 10 seconds using real Unix timestamps
    for i in 0..10 {
        let timestamp = start_time + (i * 1000); // Each second

        // Generate readings from 3 different sensors
        let mut quads = Vec::new();
        for sensor_id in 1..=3 {
            let temperature = 20.0 + (sensor_id as f64) + (i as f64 * 0.1);
            let quad = Quad::new(
                NamedNode::new(&format!("http://example.org/sensors/sensor{}", sensor_id))?,
                NamedNode::new("http://example.org/observations/temperature")?,
                oxigraph::model::Literal::new_typed_literal(
                    temperature.to_string(),
                    oxigraph::model::vocab::xsd::DOUBLE,
                ),
                GraphName::DefaultGraph,
            );
            quads.push(quad);
        }

        println!(
            "t={} (Unix: {}) - Adding {} temperature readings",
            i,
            timestamp,
            quads.len()
        );
        stream.add_quads(quads, timestamp)?;

        // Small delay for demonstration
        thread::sleep(Duration::from_millis(50));
    }

    println!("\nTriggering final window closures...");
    engine.close_stream("http://example.org/sensors/stream1", start_time + 11000)?;

    // Collect and display results
    println!("\n=== Results ===\n");
    let mut result_count = 0;

    while let Ok(result) = result_receiver.recv_timeout(Duration::from_millis(500)) {
        result_count += 1;
        println!(
            "Result #{}: Window=[{}, {})",
            result_count, result.timestamp_from, result.timestamp_to
        );

        // Display bindings
        println!("  Bindings: {}", result.bindings);
        println!();
    }

    println!("Total results received: {}", result_count);

    println!("\n=== Key Points ===");
    println!(
        "1. Used real Unix timestamps in milliseconds (~{} trillion)",
        start_time as f64 / 1_000_000_000_000.0
    );
    println!("2. No normalization or epoch subtraction needed");
    println!("3. Window boundaries calculated with perfect precision");
    println!("4. All {} results emitted correctly", result_count);
    println!("\nThis works thanks to the integer arithmetic fix in v0.3.5!");

    Ok(())
}
