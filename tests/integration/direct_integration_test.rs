use oxigraph::model::*;
use rsp_rs::{CSPARQLWindow, R2ROperator, ReportStrategy, StreamType, Tick};
use std::sync::{Arc, Mutex};

#[test]
fn test_direct_window_r2r_integration() {
    // Create a window
    let window = Arc::new(Mutex::new(CSPARQLWindow::new(
        "http://example.org/tempWindow".to_string(),
        5000, // 5 second window
        1000, // 1 second slide
        ReportStrategy::OnWindowClose,
        Tick::TimeDriven,
        0,
    )));

    // Create R2R operator with query
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

    // Subscribe to window emissions
    let r2r_clone = r2r.clone();
    let result_count = Arc::new(Mutex::new(0));
    let result_count_clone = result_count.clone();

    {
        let mut win = window.lock().unwrap();
        win.subscribe(StreamType::RStream, move |container| {
            // Execute query on the window content
            match r2r_clone.execute(&container) {
                Ok(results) => {
                    if let oxigraph::sparql::QueryResults::Solutions(solutions) = results {
                        for solution in solutions {
                            if let Ok(_sol) = solution {
                                *result_count_clone.lock().unwrap() += 1;
                            }
                        }
                    }
                }
                Err(_) => {}
            }
        });
    }

    // Add streaming data
    for i in 1..=10 {
        let timestamp = i * 1000;

        let quad = Quad::new(
            NamedNode::new("http://example.org/sensor1").unwrap(),
            NamedNode::new("http://example.org/temperature").unwrap(),
            Literal::new_typed_literal(
                &format!("{}", 20 + i),
                NamedNode::new("http://www.w3.org/2001/XMLSchema#integer").unwrap(),
            ),
            GraphName::NamedNode(NamedNode::new("http://example.org/tempWindow").unwrap()),
        );

        let mut win = window.lock().unwrap();
        win.add(quad, timestamp);
    }

    // Verify results were produced
    let total_results = *result_count.lock().unwrap();
    assert!(total_results > 0, "Should have produced query results");
}
