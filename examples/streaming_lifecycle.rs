//! Comprehensive example demonstrating the streaming lifecycle in rsp-rs
//!
//! This example shows:
//! 1. How to create and initialize an RSP engine
//! 2. When results are emitted (on window closure)
//! 3. How to use the close_stream() method
//! 4. How to inspect window state for debugging
//! 5. How to enable debug logging
//!
//! IMPORTANT: Window closure is driven by EVENT TIMESTAMPS, not wall-clock time!
//! The timestamps you pass to add_quads() determine when windows close.
//! You could add all events instantly, but results only emit when an event's
//! timestamp triggers window closure.

use oxigraph::model::*;
use rsp_rs::RSPEngine;
use std::thread;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== RSP-RS Streaming Lifecycle Example ===\n");

    // Define an RSP-QL query with a 10-second window, sliding every 2 seconds
    let query = r#"
        PREFIX ex: <https://rsp.rs/>
        REGISTER RStream <output> AS
        SELECT ?s ?p ?o
        FROM NAMED WINDOW ex:w1 ON STREAM ex:stream1 [RANGE 10000 STEP 2000]
        WHERE {
            WINDOW ex:w1 { ?s ?p ?o }
        }
    "#;

    println!("Query Configuration:");
    println!("  - Window RANGE: 10000ms (10 seconds)");
    println!("  - Window STEP:  2000ms (2 seconds)");
    println!("  - Stream: https://rsp.rs/stream1\n");

    // Create and initialize the RSP engine
    let mut rsp_engine = RSPEngine::new(query.to_string());
    rsp_engine.initialize()?;

    // Get a cloned stream (can be stored and reused - no lifetime issues!)
    let stream = rsp_engine
        .get_stream("https://rsp.rs/stream1")
        .expect("Stream should exist after initialization");

    println!("Stream obtained: {}\n", stream.name);

    // Enable debug mode to see window lifecycle in detail
    if let Some(window) = rsp_engine.get_window("https://rsp.rs/w1") {
        let window_lock = window.lock().unwrap();
        // Uncomment to see detailed debug output:
        // window_lock.set_debug_mode(true);
        drop(window_lock); // Release lock
    }

    // Start processing and get results receiver
    let result_receiver = rsp_engine.start_processing();

    // Spawn a thread to collect and display results
    let result_thread = thread::spawn(move || {
        let mut count = 0;
        println!("--- Starting Result Collection ---\n");
        // Use recv_timeout to avoid blocking forever
        loop {
            match result_receiver.recv_timeout(Duration::from_millis(500)) {
                Ok(result) => {
                    count += 1;
                    println!(
                        "[x] Result #{}: Window [{}, {})",
                        count, result.timestamp_from, result.timestamp_to
                    );
                    println!("  Bindings: {}\n", result.bindings);
                }
                Err(_) => {
                    // Timeout or disconnected - check if we should exit
                    // Give a bit more time in case more results are coming
                    thread::sleep(Duration::from_millis(100));
                    if result_receiver
                        .recv_timeout(Duration::from_millis(100))
                        .is_err()
                    {
                        break;
                    }
                }
            }
        }
        println!("--- Result Collection Complete ---");
        println!("Total results received: {}\n", count);
    });

    // Give the result thread time to start
    thread::sleep(Duration::from_millis(100));

    println!("=== Adding Events to Stream ===\n");
    println!("NOTE: The 't=' values below are EVENT TIMESTAMPS, not wall-clock time!");
    println!("We could add all these events instantly - the system only cares about");
    println!("the timestamp parameter we pass to add_quads().\n");

    // Add events with timestamp=0, 500, 1000, 1500
    println!("timestamp=0:    Adding event (subject_0)");
    let quad0 = Quad::new(
        NamedNode::new("https://rsp.rs/subject_0")?,
        NamedNode::new("https://rsp.rs/predicate")?,
        NamedNode::new("https://rsp.rs/object")?,
        GraphName::DefaultGraph,
    );
    stream.add_quads(vec![quad0], 0)?; // timestamp = 0
    thread::sleep(Duration::from_millis(50)); // Sleep just to slow down output

    println!("timestamp=500:  Adding event (subject_1)");
    let quad1 = Quad::new(
        NamedNode::new("https://rsp.rs/subject_1")?,
        NamedNode::new("https://rsp.rs/predicate")?,
        NamedNode::new("https://rsp.rs/object")?,
        GraphName::DefaultGraph,
    );
    stream.add_quads(vec![quad1], 500)?; // timestamp = 500
    thread::sleep(Duration::from_millis(50));

    // Inspect window state
    if let Some(window) = rsp_engine.get_window("https://rsp.rs/w1") {
        let window_lock = window.lock().unwrap();
        println!("\nWindow State Inspection (after 2 events, before t=2000):");
        println!(
            "  Active windows: {}",
            window_lock.get_active_window_count()
        );
        println!("  Window ranges:");
        for (start, end) in window_lock.get_active_window_ranges() {
            println!("    [{}, {})", start, end);
        }
        println!("  Note: Windows are open but haven't emitted yet!");
        println!("  They're waiting for an event with timestamp >= 2000 to trigger closure!\n");
    }

    println!("timestamp=1000: Adding event (subject_2)");
    let quad2 = Quad::new(
        NamedNode::new("https://rsp.rs/subject_2")?,
        NamedNode::new("https://rsp.rs/predicate")?,
        NamedNode::new("https://rsp.rs/object")?,
        GraphName::DefaultGraph,
    );
    stream.add_quads(vec![quad2], 1000)?; // timestamp = 1000
    thread::sleep(Duration::from_millis(50));

    println!("timestamp=1500: Adding event (subject_3)");
    let quad3 = Quad::new(
        NamedNode::new("https://rsp.rs/subject_3")?,
        NamedNode::new("https://rsp.rs/predicate")?,
        NamedNode::new("https://rsp.rs/object")?,
        GraphName::DefaultGraph,
    );
    stream.add_quads(vec![quad3], 1500)?; // timestamp = 1500
    thread::sleep(Duration::from_millis(100));

    println!("\nWARNING: Still no results! Windows are open but not closed yet.");
    println!("   Need an event with timestamp >= 2000 to trigger closure!\n");

    // Add event with timestamp=2000 to trigger first window closure
    println!("timestamp=2000: Adding event (subject_4)");
    println!("                -> This should CLOSE window [-8000, 2000) and emit first result!");
    let quad4 = Quad::new(
        NamedNode::new("https://rsp.rs/subject_4")?,
        NamedNode::new("https://rsp.rs/predicate")?,
        NamedNode::new("https://rsp.rs/object")?,
        GraphName::DefaultGraph,
    );
    stream.add_quads(vec![quad4], 2000)?; // timestamp = 2000 (triggers window closure!)
    thread::sleep(Duration::from_millis(100));

    // Add more events to trigger additional window closures
    println!("\ntimestamp=4000: Adding event (subject_5)");
    println!("                -> This should CLOSE window [-6000, 4000) and emit second result!");
    let quad5 = Quad::new(
        NamedNode::new("https://rsp.rs/subject_5")?,
        NamedNode::new("https://rsp.rs/predicate")?,
        NamedNode::new("https://rsp.rs/object")?,
        GraphName::DefaultGraph,
    );
    stream.add_quads(vec![quad5], 4000)?; // timestamp = 4000 (triggers window closure!)
    thread::sleep(Duration::from_millis(100));

    println!("\ntimestamp=6000: Adding event (subject_6)");
    println!("                -> This should CLOSE window [-4000, 6000) and emit third result!");
    let quad6 = Quad::new(
        NamedNode::new("https://rsp.rs/subject_6")?,
        NamedNode::new("https://rsp.rs/predicate")?,
        NamedNode::new("https://rsp.rs/object")?,
        GraphName::DefaultGraph,
    );
    stream.add_quads(vec![quad6], 6000)?; // timestamp = 6000 (triggers window closure!)
    thread::sleep(Duration::from_millis(100));

    // Demonstrate the problem: last event doesn't trigger closure
    println!("\ntimestamp=7000: Adding final event (subject_7)");
    println!("                This is our LAST event.");
    let quad7 = Quad::new(
        NamedNode::new("https://rsp.rs/subject_7")?,
        NamedNode::new("https://rsp.rs/predicate")?,
        NamedNode::new("https://rsp.rs/object")?,
        GraphName::DefaultGraph,
    );
    stream.add_quads(vec![quad7], 7000)?; // timestamp = 7000
    thread::sleep(Duration::from_millis(200));

    println!("\nWARNING: Without close_stream(), remaining windows won't emit!");
    println!("   We need an event with a higher timestamp to trigger closure.");

    // Inspect window state again
    if let Some(window) = rsp_engine.get_window("https://rsp.rs/w1") {
        let window_lock = window.lock().unwrap();
        println!("\nWindow State Before close_stream():");
        println!(
            "  Active windows: {}",
            window_lock.get_active_window_count()
        );
        println!("  Window ranges:");
        for (start, end) in window_lock.get_active_window_ranges() {
            println!("    [{}, {})", start, end);
        }
        println!("  These windows have data but haven't emitted because they're still open!\n");
    }

    // IMPORTANT: Close the stream to emit final results
    println!("=== Calling close_stream() ===");
    println!("This adds a sentinel event with timestamp=20000 to close all remaining windows.");
    println!("Remember: it's the TIMESTAMP that matters, not wall-clock time!\n");
    rsp_engine.close_stream("https://rsp.rs/stream1", 20000)?;

    // Give time for final results to be emitted
    thread::sleep(Duration::from_millis(500));

    // Drop the stream to close the channel
    drop(stream);

    // Wait for result collection to complete
    result_thread.join().unwrap();

    println!("\n=== Key Takeaways ===");
    println!("1. Windows emit results when they CLOSE, not when events arrive");
    println!("2. Window closure is TIMESTAMP-driven, not wall-clock-driven!");
    println!("3. A window closes when a new event with timestamp > window.end arrives");
    println!("4. You can add all events instantly - only the timestamp parameter matters");
    println!("5. Always call close_stream() at the end to emit final results");
    println!("6. Use get_active_window_count() and get_active_window_ranges() for debugging");
    println!("7. Enable debug_mode for detailed window lifecycle logging");
    println!("\n=== Example Complete ===");

    Ok(())
}
