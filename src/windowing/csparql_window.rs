use crate::{QuadContainer, WindowInstance};
use oxigraph::model::Quad;
use oxigraph::store::Store;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Report strategy for window content emission
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportStrategy {
    NonEmptyContent,
    OnContentChange,
    OnWindowClose,
    Periodic,
}

/// Tick mechanism for window progression
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tick {
    TimeDriven,
    TupleDriven,
    BatchDriven,
}

/// Output stream type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StreamType {
    RStream,
    IStream,
    DStream,
}

/// Callback type for window content emission
pub type WindowCallback = Arc<dyn Fn(QuadContainer) + Send + Sync>;

/// CSPARQL Window implementation
pub struct CSPARQLWindow {
    pub name: String,
    pub width: i64,
    pub slide: i64,
    pub time: i64,
    pub t0: i64,
    pub active_windows: HashMap<WindowInstance, QuadContainer>,
    pub report: ReportStrategy,
    pub tick: Tick,
    callbacks: HashMap<StreamType, Vec<WindowCallback>>,
}

impl CSPARQLWindow {
    pub fn new(
        name: String,
        width: i64,
        slide: i64,
        report: ReportStrategy,
        tick: Tick,
        start_time: i64,
    ) -> Self {
        Self {
            name,
            width,
            slide,
            report,
            tick,
            time: start_time,
            t0: start_time,
            active_windows: HashMap::new(),
            callbacks: HashMap::new(),
        }
    }

    /// Get window content at a specific timestamp
    /// Returns the window with the smallest close time that contains the timestamp
    pub fn get_content(&self, timestamp: i64) -> Option<&QuadContainer> {
        let mut max_window: Option<&WindowInstance> = None;
        let mut max_time = i64::MAX;

        for (window, _container) in &self.active_windows {
            if window.open <= timestamp && timestamp <= window.close {
                if window.close < max_time {
                    max_time = window.close;
                    max_window = Some(window);
                }
            }
        }

        max_window.and_then(|w| self.active_windows.get(w))
    }

    /// Add a quad to the window at the given timestamp
    pub fn add(&mut self, quad: Quad, timestamp: i64) {
        #[cfg(debug_assertions)]
        println!(
            "Window {} Received element ({:?},{}) ",
            self.name, quad, timestamp
        );

        let mut to_evict = Vec::new();
        let t_e = timestamp;

        if self.time > t_e {
            eprintln!("OUT OF ORDER NOT HANDLED");
        }

        self.scope(t_e);

        // Add element to appropriate windows
        for (window, container) in &mut self.active_windows {
            #[cfg(debug_assertions)]
            println!(
                "Processing Window {} [{},{}) for element ({:?},{})",
                self.name, window.open, window.close, quad, timestamp
            );

            if window.open <= t_e && t_e < window.close {
                #[cfg(debug_assertions)]
                println!(
                    "Adding element to Window [{},{})",
                    window.open, window.close
                );
                container.add(quad.clone(), timestamp);
                #[cfg(debug_assertions)]
                println!(
                    "Window [{},{}) now has {} quads",
                    window.open,
                    window.close,
                    container.len()
                );
            } else if t_e >= window.close {
                #[cfg(debug_assertions)]
                println!("Scheduling for Eviction [{},{})", window.open, window.close);
                // Don't add to eviction list yet - windows need to report before being evicted
                // to_evict.push(window.clone());
            }
        }

        // Find the window to report
        #[cfg(debug_assertions)]
        println!(
            "Active windows before reporting check: {}",
            self.active_windows.len()
        );

        let mut max_window: Option<WindowInstance> = None;
        let mut max_time = 0i64;

        for (window, container) in &self.active_windows {
            if self.compute_report(window, container, timestamp) {
                #[cfg(debug_assertions)]
                println!(
                    "Window [{},{}) should report (has {} quads)",
                    window.open,
                    window.close,
                    container.len()
                );
                if window.close > max_time {
                    max_time = window.close;
                    max_window = Some(window.clone());
                }
                // Mark window for eviction after it reports
                to_evict.push(window.clone());
            }
        }

        // Emit window content if conditions are met
        if let Some(window) = max_window {
            #[cfg(debug_assertions)]
            println!(
                "Max window selected for reporting: [{},{})",
                window.open, window.close
            );
            if self.tick == Tick::TimeDriven {
                if timestamp > self.time {
                    self.time = timestamp;
                    if let Some(content) = self.active_windows.get(&window) {
                        #[cfg(debug_assertions)]
                        println!(
                            "Window [{},{}),triggers. Content: {} quads",
                            window.open,
                            window.close,
                            content.len()
                        );
                        self.emit(StreamType::RStream, content.clone());
                    } else {
                        #[cfg(debug_assertions)]
                        println!(
                            "ERROR: Window [{},{}) not found in active_windows!",
                            window.open, window.close
                        );
                    }
                }
            }
        }

        // Evict old windows
        for window in to_evict {
            #[cfg(debug_assertions)]
            println!("Evicting [{},{})", window.open, window.close);
            self.active_windows.remove(&window);
        }
    }

    /// Compute whether to report this window based on the report strategy
    fn compute_report(
        &self,
        window: &WindowInstance,
        _content: &QuadContainer,
        timestamp: i64,
    ) -> bool {
        match self.report {
            ReportStrategy::OnWindowClose => window.close < timestamp,
            ReportStrategy::NonEmptyContent => !_content.is_empty(),
            ReportStrategy::OnContentChange => true, // TODO : Tracking content changes needed here but for now always true as a placeholder for future implementation
            ReportStrategy::Periodic => true, // TODO : Implement periodic reporting logic here as content is always true for now
        }
    }

    /// Calculate and create windows based on the event time
    pub fn scope(&mut self, t_e: i64) {
        if self.t0 == 0 {
            self.t0 = t_e;
        }

        let c_sup = ((t_e - self.t0).abs() as f64 / self.slide as f64).ceil() as i64 * self.slide;
        let mut o_i = c_sup - self.width;

        #[cfg(debug_assertions)]
        println!(
            "Calculating the Windows to Open. First one opens at [{}] and closes at [{}]",
            o_i, c_sup
        );

        while o_i <= t_e {
            #[cfg(debug_assertions)]
            println!("Computing Window [{},{}) if absent", o_i, o_i + self.width);

            let window = WindowInstance::new(o_i, o_i + self.width);
            self.compute_window_if_absent(window);
            o_i += self.slide;
        }
    }

    /// Add window if it doesn't already exist
    fn compute_window_if_absent(&mut self, key: WindowInstance) {
        self.active_windows
            .entry(key)
            .or_insert_with(|| QuadContainer::new(HashSet::new(), 0));
    }

    /// Subscribe a callback to window emissions
    pub fn subscribe<F>(&mut self, stream_type: StreamType, callback: F)
    where
        F: Fn(QuadContainer) + Send + Sync + 'static,
    {
        let callbacks = self.callbacks.entry(stream_type).or_insert_with(Vec::new);
        callbacks.push(Arc::new(callback));
    }

    /// Emit window content to subscribers
    fn emit(&self, stream_type: StreamType, content: QuadContainer) {
        if let Some(callbacks) = self.callbacks.get(&stream_type) {
            for callback in callbacks {
                callback(content.clone());
            }
        }
    }

    /// Get content from window at specific timestamp (alternative method name for compatibility)
    pub fn get_content_from_window(&self, timestamp: i64) -> Option<&QuadContainer> {
        self.get_content(timestamp)
    }
}

use oxigraph::sparql::QueryResults;

pub fn execute_query<'a>(
    container: &'a QuadContainer,
    query: &str,
) -> Result<QueryResults<'a>, Box<dyn std::error::Error>> {
    let store = Store::new()?;
    for quad in &container.elements {
        store.insert(quad)?;
    }
    use oxigraph::sparql::SparqlEvaluator;
    let results = SparqlEvaluator::new()
        .parse_query(query)?
        .on_store(&store)
        .execute()
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    Ok(results)
}
