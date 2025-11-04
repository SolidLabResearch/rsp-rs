//! # RSP-RS
//!
//! A high-performance RDF Stream Processing engine in Rust, supporting RSP-QL queries
//! with sliding windows and real-time analytics.
//!
//! This library provides:
//! - RSP-QL syntax support for continuous queries
//! - Sliding and tumbling window semantics
//! - SPARQL aggregation functions (COUNT, AVG, MIN, MAX, SUM)
//! - Real-time stream processing with async/await
//! - Integration with static background knowledge
//!
//! ## Example
//!
//! ```rust,no_run
//! use rsp_rs::RSPEngine;
//! use oxigraph::model::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let query = r#"
//!         PREFIX ex: <https://rsp.rs/>
//!         REGISTER RStream <output> AS
//!         SELECT *
//!         FROM NAMED WINDOW ex:w1 ON STREAM ex:stream1 [RANGE 10 STEP 2]
//!         WHERE {
//!             WINDOW ex:w1 { ?s ?p ?o }
//!         }
//!     "#;
//!
//!     let mut rsp_engine = RSPEngine::new(query.to_string());
//!     rsp_engine.initialize()?;
//!
//!     let stream = rsp_engine.get_stream("https://rsp.rs/stream1").unwrap();
//!
//!     // Add some data
//!     let quad = Quad::new(
//!         NamedNode::new("https://rsp.rs/subject")?,  
//!         NamedNode::new("https://rsp.rs/predicate")?,
//!         NamedNode::new("https://rsp.rs/object")?,    
//!         GraphName::DefaultGraph,
//!     );
//!
//!     stream.add_quads(vec![quad], 1000)?;
//!
//!     Ok(())
//! }
//! ```

mod csparql_window;
mod parsed_query;
mod quad_container;
mod r2r;
mod rsp_engine;
mod rspql_parser;
mod window_instance;

// Public API exports
pub use csparql_window::{execute_query, CSPARQLWindow, ReportStrategy, StreamType, Tick};
pub use parsed_query::{Operator, ParsedQuery, WindowDefinition};
pub use quad_container::QuadContainer;
pub use r2r::R2ROperator;
pub use rsp_engine::{BindingWithTimestamp, RDFStream, RSPEngine};
pub use rspql_parser::RSPQLParser;
pub use window_instance::WindowInstance;
