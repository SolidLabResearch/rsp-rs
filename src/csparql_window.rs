use crate::{QuadContainer, WindowInstance};
use oxigraph::model::Quad;
use oxigraph::store::Store;
use std::collections::{HashMap, HashSet};

pub struct CSPARQLWindow {
    pub width: i64,
    pub slide: i64,
    pub time: i64,
    pub t0: i64,
    pub active_windows: HashMap<WindowInstance, QuadContainer>,
}

impl CSPARQLWindow {
    pub fn new(width: i64, slide: i64, start_time: i64) -> Self {
        Self {
            width,
            slide,
            time: start_time,
            t0: start_time,
            active_windows: HashMap::new(),
        }
    }

    pub fn add(&mut self, quad: Quad, timestamp: i64) {
        self.scope(timestamp);
        for (window, container) in &mut self.active_windows {
            if window.open <= timestamp && timestamp < window.close {
                container.add(quad.clone(), timestamp);
            }
        }
        self.time = timestamp;
    }

    pub fn scope(&mut self, time_of_event: i64) {
        if self.t0 == 0 {
            self.t0 = time_of_event;
        }
        let mut o_i = ((time_of_event - self.t0 - self.width) / self.slide) * self.slide + self.t0;
        while o_i <= time_of_event {
            let window = WindowInstance::new(o_i, o_i + self.width);
            self.active_windows
                .entry(window)
                .or_insert_with(|| QuadContainer::new(HashSet::new(), 0));
            o_i += self.slide;
        }
    }

    pub fn get_content_from_window(&self, timestamp: i64) -> Option<&QuadContainer> {
        for (window, container) in &self.active_windows {
            if window.open <= timestamp && timestamp < window.close {
                return Some(container);
            }
        }
        None
    }
}

use oxigraph::sparql::QueryResults;

pub async fn execute_query(
    container: &QuadContainer,
    query: &str,
) -> Result<QueryResults, Box<dyn std::error::Error>> {
    let store = Store::new()?;
    for quad in &container.elements {
        store.insert(quad)?;
    }
    let results = store.query(query)?;
    Ok(results)
}
