use oxigraph::model::*;
use rsp_rs::R2ROperator;

#[test]
fn test_r2r_basic_query() {
    let query = r#"
        PREFIX ex: <http://example.org/>
        SELECT ?s ?p ?o
        WHERE {
            ?s ?p ?o .
        }
    "#
    .to_string();

    let r2r = R2ROperator::new(query);

    // Create a container with some quads
    let mut container = rsp_rs::QuadContainer::new(std::collections::HashSet::new(), 1000);

    let quad = Quad::new(
        NamedNode::new("http://example.org/subject1").unwrap(),
        NamedNode::new("http://example.org/predicate1").unwrap(),
        Literal::new_simple_literal("object1"),
        GraphName::DefaultGraph,
    );

    container.add(quad, 1000);

    // Execute query
    let results = r2r.execute(&container);
    assert!(results.is_ok(), "Query execution should succeed");
}

#[test]
fn test_r2r_with_named_graph() {
    let query = r#"
        PREFIX ex: <http://example.org/>
        SELECT ?sensor ?temperature
        WHERE {
            GRAPH ex:tempWindow {
                ?sensor ex:temperature ?temperature .
            }
        }
    "#
    .to_string();

    let r2r = R2ROperator::new(query);

    // Create a container with quad in named graph
    let mut container = rsp_rs::QuadContainer::new(std::collections::HashSet::new(), 1000);

    let quad = Quad::new(
        NamedNode::new("http://example.org/sensor1").unwrap(),
        NamedNode::new("http://example.org/temperature").unwrap(),
        Literal::new_typed_literal(
            "25",
            NamedNode::new("http://www.w3.org/2001/XMLSchema#integer").unwrap(),
        ),
        GraphName::NamedNode(NamedNode::new("http://example.org/tempWindow").unwrap()),
    );

    container.add(quad, 1000);

    // Execute query
    let results = r2r.execute(&container).unwrap();

    if let oxigraph::sparql::QueryResults::Solutions(solutions) = results {
        let count = solutions.count();
        assert_eq!(count, 1, "Should return exactly one result");
    } else {
        panic!("Expected Solutions result");
    }
}

#[test]
fn test_r2r_with_static_data() {
    let query = r#"
        PREFIX ex: <http://example.org/>
        SELECT ?sensor ?location
        WHERE {
            ?sensor ex:location ?location .
        }
    "#
    .to_string();

    let mut r2r = R2ROperator::new(query);

    // Add static data
    let static_quad = Quad::new(
        NamedNode::new("http://example.org/sensor1").unwrap(),
        NamedNode::new("http://example.org/location").unwrap(),
        Literal::new_simple_literal("Room A"),
        GraphName::DefaultGraph,
    );

    r2r.add_static_data(static_quad);

    // Create empty container
    let container = rsp_rs::QuadContainer::new(std::collections::HashSet::new(), 1000);

    // Execute query - should find static data
    let results = r2r.execute(&container).unwrap();

    if let oxigraph::sparql::QueryResults::Solutions(solutions) = results {
        let count = solutions.count();
        assert_eq!(count, 1, "Should return static data result");
    } else {
        panic!("Expected Solutions result");
    }
}
