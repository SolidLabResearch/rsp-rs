//! # RSP-RS
//!
//! A high-performance RDF Stream Processing engine in Rust, supporting RSP-QL queries
//! with sliding windows and real-time analytics.
//!
//! This library provides:
//! - RSP-QL syntax support for continuous queries
//! - Sliding and tumbling window semantics
//! - SPARQL aggregation functions (COUNT, AVG, MIN, MAX, SUM)
//! - Real-time stream processing with multi-threading
//! - Integration with static background knowledge
//!
//! ## When Are Results Emitted?
//!
//! **Important:** Results are emitted when windows **close**, which happens when:
//! 1. A new event arrives with a **timestamp** > window end time
//! 2. The window's STEP interval is reached **based on event timestamps**
//!
//! **Key Concept:** Window closure is driven by **event timestamps**, NOT wall-clock time!
//! The system doesn't use timers - it only processes events when you call `add_quads()`.
//!
//! ### Example with RANGE 10000 STEP 2000:
//!
//! ```text
//! - Events at t=0, 500, 1000, 1500 are added to windows
//! - No results yet (windows still open)
//! - Event with timestamp=2000 arrives → closes window [-8000, 2000) → results emitted
//! - Event with timestamp=4000 arrives → closes window [-6000, 4000) → results emitted
//!
//! Note: Wall-clock time doesn't matter! You could add all these events instantly,
//! but results only emit when an event's TIMESTAMP triggers window closure.
//! ```
//!
//! **Important:** If your last event has timestamp=1500, NO results will be emitted
//! because no subsequent event with a higher timestamp triggered window closure.
//! Use `close_stream()` to add a "sentinel" event with a future timestamp to trigger
//! remaining window closures.
//!
//! ### Complete Example with Stream Closure:
//!
//! ```rust,no_run
//! use rsp_rs::RSPEngine;
//! use oxigraph::model::*;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let query = r#"
//!         PREFIX ex: <https://rsp.rs/>
//!         REGISTER RStream <output> AS
//!         SELECT *
//!         FROM NAMED WINDOW ex:w1 ON STREAM ex:stream1 [RANGE 10000 STEP 2000]
//!         WHERE {
//!             WINDOW ex:w1 { ?s ?p ?o }
//!         }
//!     "#;
//!
//!     let mut rsp_engine = RSPEngine::new(query.to_string());
//!     rsp_engine.initialize()?;
//!
//!     // Get a cloned stream (can be stored and reused)
//!     let stream = rsp_engine.get_stream("https://rsp.rs/stream1").unwrap();
//!
//!     // Start processing results
//!     let result_receiver = rsp_engine.start_processing();
//!
//!     // Add events with TIMESTAMPS (not wall-clock time!)
//!     // These could be added instantly or over hours - doesn't matter
//!     let quad1 = Quad::new(
//!         NamedNode::new("https://rsp.rs/subject1")?,
//!         NamedNode::new("https://rsp.rs/predicate")?,
//!         NamedNode::new("https://rsp.rs/object")?,
//!         GraphName::DefaultGraph,
//!     );
//!     stream.add_quads(vec![quad1], 1000)?;  // timestamp = 1000
//!
//!     let quad2 = Quad::new(
//!         NamedNode::new("https://rsp.rs/subject2")?,
//!         NamedNode::new("https://rsp.rs/predicate")?,
//!         NamedNode::new("https://rsp.rs/object")?,
//!         GraphName::DefaultGraph,
//!     );
//!     stream.add_quads(vec![quad2], 1500)?;  // timestamp = 1500
//!
//!     // IMPORTANT: Close the stream to emit final results
//!     // This adds a sentinel event with timestamp=10000 to trigger window closures
//!     rsp_engine.close_stream("https://rsp.rs/stream1", 10000)?;
//!
//!     // Collect results
//!     while let Ok(result) = result_receiver.recv() {
//!         println!("Result: {} (window: {} to {})",
//!                  result.bindings,
//!                  result.timestamp_from,
//!                  result.timestamp_to);
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Understanding Window Lifecycle
//!
//! Windows don't emit results when events arrive - they emit when **closed** by future events.
//!
//! **Critical:** The timeline below shows EVENT TIMESTAMPS (not wall-clock time):
//!
//! ### Timeline Example (RANGE 10s, STEP 2s):
//!
//! ```text
//! Event with timestamp=0:     Added to window
//! Event with timestamp=1000:  More events added to window
//! Event with timestamp=2000:  → window [-8000, 2000) closes → RESULTS EMITTED
//! Event with timestamp=4000:  → window [-6000, 4000) closes → RESULTS EMITTED
//! Event with timestamp=6000:  → window [-4000, 6000) closes → RESULTS EMITTED
//! ...
//! Event with timestamp=15000: Last event added to stream
//!                             NO MORE RESULTS (no event to trigger closure!)
//!
//! Solution: Call close_stream("stream_uri", 20000) to emit final results
//!
//! Note: You can add ALL these events in 1 millisecond of real time! The system only
//! cares about the timestamps you provide, not how fast you send events.
//! ```
//!
//! ## Debugging Window Behavior
//!
//! You can inspect window state for debugging:
//!
//! ```rust,no_run
//! # use rsp_rs::RSPEngine;
//! # let mut engine = RSPEngine::new("".to_string());
//! # engine.initialize().unwrap();
//! if let Some(window) = engine.get_window("window_name") {
//!     let window_lock = window.lock().unwrap();
//!
//!     // Check how many windows are active
//!     println!("Active windows: {}", window_lock.get_active_window_count());
//!
//!     // See the time ranges of active windows
//!     for (start, end) in window_lock.get_active_window_ranges() {
//!         println!("Window: [{}, {})", start, end);
//!     }
//!
//!     // Enable verbose debug logging
//!     // window_lock.set_debug_mode(true);
//! }
//! ```

mod engine;
mod parsing;
mod quad_container;
mod windowing;

// Re-export modules for easier access
pub use engine::*;
pub use parsing::*;
pub use windowing::*;

// Public API exports
pub use engine::r2r::R2ROperator;
pub use engine::rsp_engine::{BindingWithTimestamp, RDFStream, RSPEngine};
pub use parsing::parsed_query::{Operator, ParsedQuery, WindowDefinition};
pub use parsing::rspql_parser::RSPQLParser;
pub use quad_container::QuadContainer;
pub use windowing::csparql_window::{
    CSPARQLWindow, ReportStrategy, StreamType, Tick, execute_query,
};
pub use windowing::window_instance::WindowInstance;
