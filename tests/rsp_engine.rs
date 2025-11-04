use rsp_rs::RSPEngine;

#[test]
fn test_new_engine() {
    let query = r#"
        REGISTER RStream <http://example.org/output> AS
        PREFIX ex: <http://example.org/>
        SELECT ?sensor ?value
        FROM NAMED WINDOW ex:window1 ON STREAM ex:stream1 [RANGE 5000 STEP 1000]
        WHERE {
            WINDOW ex:window1 {
                ?sensor ex:value ?value .
            }
        }
    "#
    .to_string();
    let engine = RSPEngine::new(query.clone());
    // Engine should initialize without errors
    assert_eq!(engine.get_all_streams().len(), 0); // Not initialized yet
}

#[test]
fn test_engine_with_simple_query() {
    let query = r#"
        REGISTER RStream <http://example.org/output> AS
        SELECT ?s ?p ?o
        WHERE { ?s ?p ?o }
    "#
    .to_string();
    let engine = RSPEngine::new(query.clone());
    // Engine should parse successfully even without window definitions
    assert_eq!(engine.get_all_streams().len(), 0);
}
