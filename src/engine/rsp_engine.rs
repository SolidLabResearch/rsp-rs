use crate::parsed_query::WindowDefinition;
use crate::rspql_parser::RSPQLParser;
use crate::{CSPARQLWindow, QuadContainer, R2ROperator};
use oxigraph::model::Quad;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;

/// Represents a binding result with timestamp information
#[derive(Debug, Clone)]
pub struct BindingWithTimestamp {
    pub bindings: String,
    pub timestamp_from: i64,
    pub timestamp_to: i64,
}

/// Represents an RDF stream that feeds data into a window
#[derive(Clone)]
pub struct RDFStream {
    pub name: String,
    pub(crate) window_sender: mpsc::Sender<(QuadContainer, String)>,
}

impl RDFStream {
    pub fn new(name: String, window_sender: mpsc::Sender<(QuadContainer, String)>) -> Self {
        Self {
            name,
            window_sender,
        }
    }

    /// Add a quad container to the stream
    pub fn add(&self, container: QuadContainer) -> Result<(), String> {
        self.window_sender
            .send((container, self.name.clone()))
            .map_err(|e| format!("Failed to send data to window: {}", e))
    }

    /// Add a set of quads with a timestamp to the stream
    pub fn add_quads(&self, quads: Vec<Quad>, timestamp: i64) -> Result<(), String> {
        let mut elements = std::collections::HashSet::new();
        for quad in quads {
            elements.insert(quad);
        }
        let container = QuadContainer::new(elements, timestamp);
        self.add(container)
    }
}

/// The main RSP (RDF Stream Processing) Engine
pub struct RSPEngine {
    windows: HashMap<String, Arc<Mutex<CSPARQLWindow>>>,
    streams: HashMap<String, RDFStream>,
    r2r: R2ROperator,
    parsed_query: crate::parsed_query::ParsedQuery,
}

impl RSPEngine {
    /// Create a new RSP Engine from an RSPQL query
    pub fn new(query: String) -> Self {
        let parser = RSPQLParser::new(query);
        let parsed_query = parser.parse();

        #[cfg(debug_assertions)]
        {
            println!("[RSPEngine] Parsed SPARQL query:");
            println!("{}", parsed_query.sparql_query);
            println!();
        }

        let windows = HashMap::new();
        let streams = HashMap::new();
        let r2r = R2ROperator::new(parsed_query.sparql_query.clone());

        Self {
            windows,
            streams,
            r2r,
            parsed_query,
        }
    }

    /// Initialize the engine by creating windows and streams
    pub fn initialize(&mut self) -> Result<(), String> {
        // Create windows and streams based on parsed query
        for window_def in &self.parsed_query.s2r {
            let (tx, rx) = mpsc::channel::<(QuadContainer, String)>();

            // Create window with full parameters
            let window = Arc::new(Mutex::new(CSPARQLWindow::new(
                window_def.window_name.clone(),
                window_def.width,
                window_def.slide,
                crate::ReportStrategy::OnWindowClose,
                crate::Tick::TimeDriven,
                0,
            )));

            // Create stream
            let stream = RDFStream::new(window_def.stream_name.clone(), tx);

            // Store window and stream
            self.windows
                .insert(window_def.window_name.clone(), window.clone());
            self.streams.insert(window_def.stream_name.clone(), stream);

            // Spawn thread to handle incoming data
            let window_clone = window.clone();
            thread::spawn(move || {
                while let Ok((container, _stream_name)) = rx.recv() {
                    let mut win = window_clone.lock().unwrap();
                    // Add all quads from the container to the window
                    for quad in &container.elements {
                        win.add(quad.clone(), container.last_timestamp_changed);
                    }
                }
            });
        }

        Ok(())
    }

    /// Register a callback for processing window content
    /// Returns a receiver for binding results
    pub fn register(
        windows: HashMap<String, Arc<Mutex<CSPARQLWindow>>>,
        r2r: R2ROperator,
        window_defs: Vec<WindowDefinition>,
    ) -> mpsc::Receiver<BindingWithTimestamp> {
        let (tx, rx) = mpsc::channel();

        // For each window, subscribe to its RStream output
        for (window_name, window_arc) in windows.iter() {
            let r2r_clone = r2r.clone();
            let tx_clone = tx.clone();
            let all_windows = windows.clone();
            let window_def = window_defs
                .iter()
                .find(|w| w.window_name == *window_name)
                .cloned();
            let window_name_owned = window_name.clone();

            // Subscribe to window emissions using the callback system
            {
                let mut window = window_arc.lock().unwrap();
                window.subscribe(crate::StreamType::RStream, move |mut container| {
                    let timestamp = container.last_timestamp_changed;

                    // Merge content from other windows
                    for (other_name, other_window_arc) in &all_windows {
                        if other_name != &window_name_owned {
                            if let Ok(other_window) = other_window_arc.lock() {
                                if let Some(other_container) =
                                    other_window.get_content_from_window(timestamp)
                                {
                                    for quad in &other_container.elements {
                                        container.add(quad.clone(), timestamp);
                                    }
                                }
                            }
                        }
                    }

                    // Execute R2R query
                    if let Ok(results) = r2r_clone.execute(&container) {
                        if let Some(def) = &window_def {
                            if let oxigraph::sparql::QueryResults::Solutions(solutions) = results {
                                for solution in solutions {
                                    if let Ok(binding) = solution {
                                        let binding_str = format!("{:?}", binding);
                                        let result = BindingWithTimestamp {
                                            bindings: binding_str,
                                            timestamp_from: timestamp,
                                            timestamp_to: timestamp + def.width,
                                        };
                                        let _ = tx_clone.send(result);
                                    }
                                }
                            }
                        }
                    }
                });
            }
        }

        rx
    }

    /// Convenience method to register using the engine's own data
    pub fn start_processing(&self) -> mpsc::Receiver<BindingWithTimestamp> {
        Self::register(
            self.windows.clone(),
            self.r2r.clone(),
            self.parsed_query.s2r.clone(),
        )
    }

    /// Get a stream by name (returns a clone for easier usage)
    pub fn get_stream(&self, stream_name: &str) -> Option<RDFStream> {
        self.streams.get(stream_name).cloned()
    }

    /// Add static data to the R2R operator
    pub fn add_static_data(&mut self, quad: Quad) {
        self.r2r.add_static_data(quad);
    }

    /// Get all stream names
    pub fn get_all_streams(&self) -> Vec<String> {
        self.streams.keys().cloned().collect()
    }

    /// Add a sentinel event to trigger closure of all open windows
    /// This should be called when the stream ends to emit final results
    pub fn close_stream(&self, stream_uri: &str, final_timestamp: i64) -> Result<(), String> {
        if let Some(stream) = self.get_stream(stream_uri) {
            // Add a dummy quad with timestamp far in the future
            let sentinel = oxigraph::model::Quad::new(
                oxigraph::model::NamedNode::new("urn:rsp:sentinel")
                    .map_err(|e| format!("Failed to create sentinel node: {}", e))?,
                oxigraph::model::NamedNode::new("urn:rsp:type")
                    .map_err(|e| format!("Failed to create sentinel node: {}", e))?,
                oxigraph::model::Literal::new_simple_literal("end"),
                oxigraph::model::GraphName::DefaultGraph,
            );
            stream.add_quads(vec![sentinel], final_timestamp)?;
            Ok(())
        } else {
            Err(format!("Stream {} not found", stream_uri))
        }
    }

    /// Get the parsed query
    pub fn parsed_query(&self) -> &crate::parsed_query::ParsedQuery {
        &self.parsed_query
    }

    /// Get a window by name
    pub fn get_window(&self, window_name: &str) -> Option<Arc<Mutex<CSPARQLWindow>>> {
        self.windows.get(window_name).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rsp_engine_creation() {
        let query = r#"
            REGISTER RStream <http://example.org/output> AS
            PREFIX ex: <http://example.org/>
            SELECT ?s ?p ?o
            FROM NAMED WINDOW :win1 ON STREAM :stream1 [RANGE 10 STEP 5]
            WHERE {
                WINDOW :win1 { ?s ?p ?o }
            }
        "#
        .to_string();

        let engine = RSPEngine::new(query);
        assert_eq!(engine.parsed_query.s2r.len(), 1);
    }

    #[test]
    fn test_initialize_engine() {
        let query = r#"
            REGISTER RStream <http://example.org/output> AS
            PREFIX ex: <http://example.org/>
            SELECT ?s ?p ?o
            FROM NAMED WINDOW :win1 ON STREAM :stream1 [RANGE 10 STEP 5]
            WHERE {
                WINDOW :win1 { ?s ?p ?o }
            }
        "#
        .to_string();

        let mut engine = RSPEngine::new(query);
        let result = engine.initialize();
        assert!(result.is_ok());
        assert_eq!(engine.get_all_streams().len(), 1);
    }
}
