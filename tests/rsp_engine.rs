use rsp_rs::RSPEngine;

#[test]
fn test_new_engine() {
    let query = "SELECT * WHERE { ?s ?p ?o }".to_string();
    let engine = RSPEngine::new(query.clone());
    assert_eq!(engine.query, query);
}

#[test]
fn test_empty_query() {
    let query = "".to_string();
    let engine = RSPEngine::new(query.clone());
    assert_eq!(engine.query, query);
}
