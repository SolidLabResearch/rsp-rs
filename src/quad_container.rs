use oxigraph::model::Quad;
use std::collections::HashSet;

// Representing a container for RDF Quads in the Window.
#[derive(Debug, Clone)]
pub struct QuadContainer {
    pub elements: HashSet<Quad>,
    pub last_timestamp_changed: i64,
}

impl QuadContainer {
    pub fn new(elements: HashSet<Quad>, ts: i64) -> Self {
        Self {
            elements,
            last_timestamp_changed: ts,
        }
    }

    pub fn len(&self) -> usize {
        self.elements.len()
    }

    pub fn add(&mut self, quad: Quad, ts: i64) {
        self.elements.insert(quad);
        self.last_timestamp_changed = ts;
    }

    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    pub fn remove(&mut self, quad: &Quad, ts: i64) {
        self.elements.remove(quad);
        self.last_timestamp_changed = ts;
    }

    pub fn contains(&self, quad: &Quad) -> bool {
        self.elements.contains(quad)
    }

    pub fn clear(&mut self, ts: i64) {
        self.elements.clear();
        self.last_timestamp_changed = ts;
    }
}

// Example usage of the Quad Container class.
#[cfg(test)]
mod tests {
    use super::*;
    use oxigraph::model::{Literal, NamedNode};

    #[test]
    fn test_quad_container() {
        let mut container = QuadContainer::new(HashSet::new(), 0);

        let quad1 = Quad::new(
            NamedNode::new("http://example.org/subject1").unwrap(),
            NamedNode::new("http://example.org/predicate1").unwrap(),
            Literal::new_simple_literal("object1"),
            NamedNode::new("http://example.org/graph1").unwrap(),
        );

        let quad2 = Quad::new(
            NamedNode::new("http://example.org/subject2").unwrap(),
            NamedNode::new("http://example.org/predicate2").unwrap(),
            Literal::new_simple_literal("object2"),
            NamedNode::new("http://example.org/graph2").unwrap(),
        );

        container.add(quad1.clone(), 1);
        assert_eq!(container.len(), 1);
        assert!(container.contains(&quad1));

        container.add(quad2.clone(), 2);
        assert_eq!(container.len(), 2);
        assert!(container.contains(&quad2));

        container.remove(&quad1, 3);
        assert_eq!(container.len(), 1);
        assert!(!container.contains(&quad1));

        container.clear(4);
        assert_eq!(container.len(), 0);
        assert!(container.is_empty());
    }
}
