// Library for RDF Stream Processing in Rust
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
