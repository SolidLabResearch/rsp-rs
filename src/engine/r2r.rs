use crate::quad_container::QuadContainer;
use oxigraph::model::Quad;
use oxigraph::sparql::QueryResults;
use oxigraph::store::Store;
use std::collections::HashSet;

/// R2R (Relation-to-Relation) Operator
/// Executes SPARQL queries over streaming data combined with static data
#[derive(Clone)]
pub struct R2ROperator {
    pub(crate) query: String,
    pub(crate) static_data: HashSet<Quad>,
}

impl R2ROperator {
    /// Create a new R2ROperator with a SPARQL query
    pub fn new(query: String) -> Self {
        Self {
            query,
            static_data: HashSet::new(),
        }
    }

    /// Add a static quad to the operator's static data store
    pub fn add_static_data(&mut self, quad: Quad) {
        self.static_data.insert(quad);
    }

    /// Execute the SPARQL query over the container's quads combined with static data
    pub fn execute(
        &self,
        container: &QuadContainer,
    ) -> Result<QueryResults, Box<dyn std::error::Error>> {
        // Create an in-memory store
        let store = Store::new()?;

        // Add all quads from the container
        for quad in &container.elements {
            store.insert(quad)?;
        }

        // Add all static quads
        for quad in &self.static_data {
            store.insert(quad)?;
        }

        #[cfg(debug_assertions)]
        {
            println!("[R2R] Executing query:");
            println!("{}", self.query);
            println!("[R2R] Container has {} quads", container.len());
            println!("[R2R] Static data has {} quads", self.static_data.len());
            for (i, quad) in container.elements.iter().enumerate() {
                println!("[R2R]   Quad {}: {:?}", i + 1, quad);
            }
        }

        // Execute the query
        // Note: Oxigraph doesn't support custom extension functions in the same way as Comunica
        // For custom functions like sqrt and pow, you would need to:
        // 1. Preprocess the query to replace custom functions with SPARQL built-ins
        // 2. Use SPARQL BIND expressions with standard math operations
        // 3. Or implement a query rewriter
        use oxigraph::sparql::SparqlEvaluator;
        SparqlEvaluator::new()
            .parse_query(&self.query)?
            .on_store(&store)
            .execute()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    /// Execute the SPARQL query and return results as a vector of solution mappings
    /// This is a convenience method that handles common result types
    pub fn execute_select(
        &self,
        container: &QuadContainer,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let results = self.execute(container)?;

        let mut output = Vec::new();

        if let QueryResults::Solutions(solutions) = results {
            for solution in solutions {
                let solution = solution?;
                output.push(format!("{:?}", solution));
            }
        }

        Ok(output)
    }

    /// Get a reference to the query string
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Get the number of static quads
    pub fn static_data_size(&self) -> usize {
        self.static_data.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxigraph::model::*;

    #[test]
    fn test_r2r_operator_creation() {
        let query = "SELECT * WHERE { ?s ?p ?o }".to_string();
        let operator = R2ROperator::new(query.clone());
        assert_eq!(operator.query(), query);
        assert_eq!(operator.static_data_size(), 0);
    }

    #[test]
    fn test_add_static_data() {
        let query = "SELECT * WHERE { ?s ?p ?o }".to_string();
        let mut operator = R2ROperator::new(query);

        let quad = Quad::new(
            NamedNode::new("http://example.org/subject").unwrap(),
            NamedNode::new("http://example.org/predicate").unwrap(),
            NamedNode::new("http://example.org/object").unwrap(),
            GraphName::DefaultGraph,
        );

        operator.add_static_data(quad);
        assert_eq!(operator.static_data_size(), 1);
    }

    #[test]
    fn test_execute_query() -> Result<(), Box<dyn std::error::Error>> {
        let query = "SELECT * WHERE { ?s ?p ?o }".to_string();
        let mut operator = R2ROperator::new(query);

        // Add some static data
        let static_quad = Quad::new(
            NamedNode::new("http://example.org/static").unwrap(),
            NamedNode::new("http://example.org/isStatic").unwrap(),
            Literal::new_simple_literal("true"),
            GraphName::DefaultGraph,
        );
        operator.add_static_data(static_quad.clone());

        // Create a container with some quads
        let mut container_quads = HashSet::new();
        let stream_quad = Quad::new(
            NamedNode::new("http://example.org/stream").unwrap(),
            NamedNode::new("http://example.org/hasValue").unwrap(),
            Literal::new_simple_literal("42"),
            GraphName::DefaultGraph,
        );
        container_quads.insert(stream_quad);

        let container = QuadContainer::new(container_quads, 0);

        // Execute the query
        let results = operator.execute(&container)?;

        // Check that we got results
        if let QueryResults::Solutions(solutions) = results {
            let count = solutions.count();
            assert_eq!(count, 2); // One from static data, one from container
        }

        Ok(())
    }
}
