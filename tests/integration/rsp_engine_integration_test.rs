use oxigraph::model::*;
use rsp_rs::RSPEngine;

#[tokio::test]
async fn test_rsp_engine_basic() {
    let query = r#"
        REGISTER RStream <http://example.org/output> AS
        PREFIX ex: <http://example.org/>
        PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>
        SELECT ?sensor ?temperature
        FROM NAMED WINDOW ex:tempWindow ON STREAM ex:temperatureStream [RANGE 5000 STEP 1000]
        WHERE {
            WINDOW ex:tempWindow {
                ?sensor ex:temperature ?temperature .
            }
        }
    "#
    .to_string();

    // Create the RSP engine
    let mut engine = RSPEngine::new(query);

    // Initialize the engine (creates windows and streams)
    engine
        .initialize()
        .expect("Engine initialization should succeed");

    // Add some static data (e.g., sensor metadata)
    engine.add_static_data(Quad::new(
        NamedNode::new("http://example.org/sensor1").unwrap(),
        NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type").unwrap(),
        NamedNode::new("http://example.org/TemperatureSensor").unwrap(),
        GraphName::DefaultGraph,
    ));

    // Start processing (this spawns background tasks)
    let mut result_receiver = engine.start_processing();

    // Get the stream to add data to
    let stream_name = "http://example.org/temperatureStream";

    // Simulate streaming data
    if let Some(stream) = engine.get_stream(stream_name) {
        for i in 1..=5 {
            let timestamp = i * 1000;

            let quads = vec![Quad::new(
                NamedNode::new("http://example.org/sensor1").unwrap(),
                NamedNode::new("http://example.org/temperature").unwrap(),
                Literal::new_typed_literal(
                    &format!("{}", 20 + i),
                    NamedNode::new("http://www.w3.org/2001/XMLSchema#integer").unwrap(),
                ),
                GraphName::NamedNode(NamedNode::new("http://example.org/tempWindow").unwrap()),
            )];

            stream
                .add_quads(quads, timestamp)
                .expect("Adding quads should succeed");

            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }

    // Listen for results with timeout
    let mut count = 0;
    let max_results = 5;
    let timeout = tokio::time::Duration::from_secs(2);
    let start = tokio::time::Instant::now();

    while count < max_results && start.elapsed() < timeout {
        tokio::select! {
            result = result_receiver.recv() => {
                if let Some(_binding_result) = result {
                    count += 1;
                } else {
                    break;
                }
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
                // Continue waiting
            }
        }
    }

    assert!(count > 0, "RSP engine should produce results");
}
