use oxigraph::model::*;
use oxigraph::store::Store;

#[test]
fn test_oxigraph_basic_query() {
    let store = Store::new().unwrap();

    // Add a quad
    let quad = Quad::new(
        NamedNode::new("http://example.org/subject").unwrap(),
        NamedNode::new("http://example.org/predicate").unwrap(),
        Literal::new_simple_literal("object"),
        GraphName::DefaultGraph,
    );

    store.insert(&quad).unwrap();

    // Query
    let query = r#"
        PREFIX ex: <http://example.org/>
        SELECT ?s ?p ?o
        WHERE {
            ?s ?p ?o .
        }
    "#;

    let results = store.query(query).unwrap();

    if let oxigraph::sparql::QueryResults::Solutions(solutions) = results {
        let count = solutions.count();
        assert_eq!(count, 1, "Should return one result");
    } else {
        panic!("Expected Solutions result");
    }
}

#[test]
fn test_oxigraph_named_graph_query() {
    let store = Store::new().unwrap();

    // Add a quad with named graph
    let quad = Quad::new(
        NamedNode::new("http://example.org/sensor1").unwrap(),
        NamedNode::new("http://example.org/temperature").unwrap(),
        Literal::new_typed_literal(
            "21",
            NamedNode::new("http://www.w3.org/2001/XMLSchema#integer").unwrap(),
        ),
        GraphName::NamedNode(NamedNode::new("http://example.org/tempWindow").unwrap()),
    );

    store.insert(&quad).unwrap();

    // Query with GRAPH clause
    let query = r#"
        PREFIX ex: <http://example.org/>
        SELECT ?sensor ?temperature
        WHERE {
            GRAPH ex:tempWindow {
                ?sensor ex:temperature ?temperature .
            }
        }
    "#;

    let results = store.query(query).unwrap();

    if let oxigraph::sparql::QueryResults::Solutions(solutions) = results {
        let count = solutions.count();
        assert_eq!(count, 1, "Should return one result from named graph");
    } else {
        panic!("Expected Solutions result");
    }
}
